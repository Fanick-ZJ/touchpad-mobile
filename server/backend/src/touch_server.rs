use std::{
    collections::HashMap,
    fmt::Debug,
    net::{IpAddr, SocketAddr},
    str::FromStr,
    sync::Arc,
};

use anyhow::Result;

use quinn::{
    Connection, Endpoint, ServerConfig,
    rustls::pki_types::{CertificateDer, PrivatePkcs8KeyDer},
};
use tokio::sync::{
    Mutex, RwLock,
    mpsc::{self, Receiver},
    watch,
};
use touchpad_proto::{
    codec::ProtoStream,
    proto::v1::{TouchEventType, wrapper::Payload},
};
use tracing::{debug, error, info};

use server_core_kit::{
    device::Device,
    driver::{Driver, TouchPoint, TouchStatus},
};

/// 创建服务段的配置
pub fn configure_server(
    cert_der: CertificateDer<'static>,
    key_der: PrivatePkcs8KeyDer<'static>,
) -> Result<ServerConfig> {
    let mut server_config = ServerConfig::with_single_cert(vec![cert_der], key_der.into())?;
    let transport_config = Arc::get_mut(&mut server_config.transport).unwrap();
    // 最大双工通讯连接数量
    transport_config.max_concurrent_bidi_streams(100_u8.into());

    Ok(server_config)
}

pub struct TouchServerConfig {
    pub server_port: u16,
    pub cert_der: CertificateDer<'static>,
    pub key_der: PrivatePkcs8KeyDer<'static>,
}

#[derive(Clone, Debug, PartialEq)]
enum ShutdownSignal {
    Empty,
    ServerStop,
    ConnectionClose(usize),
}

pub struct TouchServer {
    // 一个端点都对应一个UDP套接字
    pub endpoint: Endpoint,
    pub addr: SocketAddr,
    shutdown_tx: Mutex<Option<watch::Sender<ShutdownSignal>>>,
    connections: Arc<RwLock<HashMap<u64, ConnectionInfo>>>,
    server_handle: RwLock<Option<tokio::task::JoinHandle<()>>>,
    connected_device: Arc<Mutex<HashMap<IpAddr, Device>>>,
    touch_driver: Arc<std::sync::Mutex<Driver>>,
    touch_event_tx: mpsc::UnboundedSender<TouchPoint>,
}

struct ConnectionInfo {
    conn: Connection,
    task_handle: tokio::task::JoinHandle<()>,
}

impl TouchServer {
    pub async fn new(
        config: &TouchServerConfig,
        device_map: Arc<Mutex<HashMap<IpAddr, Device>>>,
    ) -> Result<Self> {
        let server_config = Self::server_config(config).await?;
        let ip_addr = SocketAddr::new(server_core_kit::inner_const::ANY_V4, config.server_port);
        let endpoint = Endpoint::server(server_config, ip_addr)?;
        info!("listening on {}", endpoint.local_addr()?);
        let (touch_event_tx, touch_event_rx) = mpsc::unbounded_channel();
        let touch_driver = Arc::new(std::sync::Mutex::new(Driver::new(1920, 1080)?));

        // 启动触控事件处理任务
        let driver_clone = Arc::clone(&touch_driver);
        tokio::spawn(async move {
            Self::touch_event_processor(driver_clone, touch_event_rx).await;
        });

        let touch_server = Self {
            endpoint,
            addr: ip_addr,
            shutdown_tx: Mutex::new(None),
            connections: Arc::new(RwLock::new(HashMap::new())),
            server_handle: RwLock::new(None),
            connected_device: device_map,
            touch_driver,
            touch_event_tx,
        };
        Ok(touch_server)
    }

    /// 触控事件处理器 - 专门的任务处理触控事件，不阻塞网络 I/O
    async fn touch_event_processor(
        driver: Arc<std::sync::Mutex<Driver>>,
        mut event_rx: mpsc::UnboundedReceiver<TouchPoint>,
    ) {
        // 批量处理缓冲区
        let mut buffer = Vec::with_capacity(64);

        loop {
            buffer.clear();

            // 批量接收事件（最多 64 个或等待 1ms）
            let first = match event_rx.recv().await {
                Some(event) => event,
                None => break, // channel 关闭
            };
            buffer.push(first);

            // 收集更多事件（非阻塞）
            for _ in 0..63 {
                match event_rx.try_recv() {
                    Ok(event) => buffer.push(event),
                    Err(_) => break,
                }
            }

            // 批量处理 - 使用 std::sync::Mutex，快速且不跨 await
            if let Ok(mut driver) = driver.lock() {
                for point in &buffer {
                    debug!("Emitting touch event: {:?}", point);
                    if let Err(e) = driver.emit_multitouch(std::slice::from_ref(point)) {
                        error!("Failed to emit touch event: {}", e);
                    }
                }
            }
        }
    }

    /// 创建服务段的配置
    async fn server_config(config: &TouchServerConfig) -> Result<ServerConfig> {
        let server_config = configure_server(config.cert_der.clone(), config.key_der.clone_key())?;
        info!("Server configuration created successfully");
        Ok(server_config)
    }

    pub async fn wait_connect(self: &Arc<Self>) -> Result<()> {
        info!("Waiting for connection...");
        let (shutdown_tx, mut shutdown_rx) = watch::channel(ShutdownSignal::Empty);
        self.shutdown_tx.lock().await.replace(shutdown_tx.clone());
        loop {
            let shutdown_subscribe = shutdown_tx.subscribe();
            // 同时监听 shutdown 信号和新的双向流
            tokio::select! {
                _ = shutdown_rx.changed() => {
                    let value = shutdown_rx.borrow().clone();
                    if value == ShutdownSignal::ServerStop {
                        info!("Shutdown signal received");
                        // 关闭所有连接
                        let connections = self.connections.clone();
                        let conns = connections.read().await;
                        for (id, info) in conns.iter() {
                            info!("Closing connection: {}", id);
                            // 关闭连接
                            info.conn.close(0u8.into(), b"shutdown");
                        }
                        drop(conns);
                        // 等待所有连接完成
                        let mut conns = self.connections.write().await;
                        for (id, info) in conns.drain() {
                            info!("Waiting for connection: {}", id);
                            let _ = info.task_handle.await;
                            info!("Connection closed: {}", id);
                        }
                        break;
                    }
                },
                incoming = self.endpoint.accept() => {
                    match incoming {
                        Some(incoming) => {
                            match incoming.await {
                                Ok(conn) => {
                                    // 将接受到的连接记录，并给他开启任务处理器
                                    let conn_id = conn.stable_id() as u64;
                                    let connection_map = Arc::clone(&self.connections);
                                    let connected_device = Arc::clone(&self.connected_device);
                                    info!("New connection: {}", conn_id);
                                    let conn_clone = conn.clone();
                                    let conn_ip = conn.remote_address().ip();
                                    let touch_event_tx = self.touch_event_tx.clone();
                                    let task_handle = tokio::spawn(async move {
                                        let mut conn_client = ConnectedExector::new(
                                            conn_clone,
                                            Arc::clone(&connected_device),
                                            shutdown_subscribe,
                                            touch_event_tx
                                        );
                                        if let Err(err) = conn_client.start().await {
                                            error!("Failed to client running: {}", err);
                                        }
                                        connection_map.write().await.remove(&conn_id);
                                        connected_device.lock().await.remove(&conn_ip);
                                    });

                                    // 保存句柄
                                    let conn_info = ConnectionInfo {
                                        conn: conn.clone(),
                                        task_handle,
                                    };
                                    self.connections.write().await.insert(conn_id, conn_info);
                                },
                                Err(_) => {
                                    error!("Failed to accept connection");
                                }
                            }
                        },
                        None => todo!(),
                    }
                }
            }
        }
        Ok(())
    }

    pub async fn start(self: &Arc<Self>) -> Result<()> {
        info!("Starting server loop");
        let this = self.clone();
        let handle = tokio::spawn(async move {
            if let Err(e) = this.wait_connect().await {
                error!("Failed to accept connection: {}", e);
            }
        });
        *self.server_handle.write().await = Some(handle);

        Ok(())
    }

    pub async fn close(self: &Arc<Self>) {
        if let Some(tx) = self.shutdown_tx.lock().await.take() {
            let _ = tx.send(ShutdownSignal::ServerStop);
        }
    }
}

struct ConnectedExector {
    conn: quinn::Connection,
    done: bool,
    connected_device: Arc<Mutex<HashMap<IpAddr, Device>>>,
    touch_event_tx: mpsc::UnboundedSender<TouchPoint>,
    /// 停止信号
    stop_signal: watch::Receiver<ShutdownSignal>,
}

impl ConnectedExector {
    fn new(
        conn: quinn::Connection,
        connected_device: Arc<Mutex<HashMap<IpAddr, Device>>>,
        stop_signal: watch::Receiver<ShutdownSignal>,
        touch_event_tx: mpsc::UnboundedSender<TouchPoint>,
    ) -> Self {
        ConnectedExector {
            conn,
            done: false,
            connected_device,
            stop_signal,
            touch_event_tx,
        }
    }

    pub async fn start(&mut self) -> Result<bool> {
        // 读取数据直到流结束
        loop {
            tokio::select! {
                _ = self.stop_signal.changed() => {
                    let value = self.stop_signal.borrow();
                    info!("Shutdown signal received");
                    if *value == ShutdownSignal::ConnectionClose(self.conn.stable_id()) {
                        info!("Closing connection");
                        self.conn.close((0 as u8).into(), b"");
                        break;
                    }
                },
                accept_result = self.conn.accept_bi() => {
                    match accept_result {
                        Ok((send, recv)) => {
                            self.handle_stream(send, recv).await?;
                            if self.done {
                                break;
                            }
                        },
                        Err(e) => {
                            error!("Error accepting bidirectional stream: {}", e);
                            return Err(e.into());
                        }
                    }
                }
            }
        }
        Ok(true)
    }

    async fn handle_stream(
        &mut self,
        send: quinn::SendStream,
        recv: quinn::RecvStream,
    ) -> Result<()> {
        let mut proto_stream = ProtoStream::new(Box::new(send), Box::new(recv));
        // 处理消息
        while let Ok(message) = proto_stream.receive_message().await {
            let need_continue = self.handle_message(message).await?;
            if !need_continue {
                self.done = true;
                break;
            }
        }

        Ok(())
    }

    /// 处理消息，OK(False)代表推出连接
    async fn handle_message(&self, message: Payload) -> Result<bool> {
        match message {
            Payload::RegisterDevice(device) => {
                let device = Device {
                    name: device.device_name,
                    ip: IpAddr::from_str(&device.ip)?,
                    width: device.width,
                    height: device.height,
                };
                self.connected_device.lock().await.insert(device.ip, device);

                Ok(true)
            },
            Payload::TouchPacket(touch_packet) => {
                debug!("接受触控事件: {:?}", touch_packet);
                // 通过 channel 发送触控事件，不阻塞网络 I/O
                for pointer in touch_packet.pointers {
                    let tracking_id = if pointer.event_type != TouchEventType::Up as i32 {
                        pointer.id
                    } else {
                        -1
                    };
                    let status = match TouchEventType::try_from(pointer.event_type) {
                        Ok(TouchEventType::Down) => TouchStatus::Down,
                        Ok(TouchEventType::Move) => TouchStatus::Move,
                        Ok(TouchEventType::Up) => TouchStatus::Up,
                        Ok(TouchEventType::Cancel) => TouchStatus::Up, // 如果需要处理 Cancel
                        Ok(TouchEventType::Unspecified) => continue,   // 跳过未指定的
                        Err(_) => continue,                            // 跳过无效值
                    };

                    let touch_point = TouchPoint {
                        slot: pointer.id,
                        tracking_id,
                        x: pointer.abs_x as i32,
                        y: pointer.abs_y as i32,
                        status,
                    };

                    // 非阻塞发送，如果 channel 满了则丢弃（避免阻塞网络处理）
                    let _ = self.touch_event_tx.send(touch_point);
                }
                Ok(true)
            },
            Payload::Exit(exit) => {
                info!("Exiting connection: {:?}", self.conn.remote_address());
                self.conn.close((0 as u8).into(), b"");
                Ok(false)
            },
            _ => Ok(true),
        }
    }
}
