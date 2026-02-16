use std::{
    collections::HashMap,
    fmt::Debug,
    net::{IpAddr, SocketAddr},
    str::FromStr,
    sync::Arc,
    time::Duration,
};

use anyhow::Result;

use quinn::{
    Connection, Endpoint, IdleTimeout, ServerConfig, VarInt,
    rustls::pki_types::{CertificateDer, PrivatePkcs8KeyDer},
};
use tokio::sync::{
    Mutex, RwLock,
    mpsc::{self},
    watch,
};
use touchpad_proto::{
    codec::ProtoStream,
    proto::v1::{TouchEventType, TuneSetting, setting_request, wrapper::Payload},
};
use tracing::{debug, error, info};

use server_core_kit::{
    device::Device,
    driver::{Driver, TouchPoint, TouchStatus},
};

use crate::latency::{LatencyDisplay, RealtimeLatencyTracker};

/// 创建服务段的配置
pub fn configure_server(
    cert_der: CertificateDer<'static>,
    key_der: PrivatePkcs8KeyDer<'static>,
) -> Result<ServerConfig> {
    let mut server_config = ServerConfig::with_single_cert(vec![cert_der], key_der.into())?;
    let transport_config = Arc::get_mut(&mut server_config.transport).unwrap();
    // 最大双工通讯连接数量
    transport_config.max_concurrent_bidi_streams(100_u8.into());
    transport_config
        .max_idle_timeout(Some(IdleTimeout::from(VarInt::from_u32(1000 * 60 * 60 * 24))));
    transport_config.keep_alive_interval(Some(Duration::from_secs(25)));

    Ok(server_config)
}

pub struct TouchServerConfig {
    pub server_port: u16,
    pub cert_der: CertificateDer<'static>,
    pub key_der: PrivatePkcs8KeyDer<'static>,
}

#[derive(Clone, Debug, PartialEq)]
enum ProcesserType {
    Touch,
    Latency,
    Connection,
}

#[derive(Clone, Debug, PartialEq)]
enum ShutdownSignal {
    Empty,
    ProcesserStop(ProcesserType),
    ConnectionClose(usize),
}

#[derive(Debug)]
enum TouchpadEvent {
    TouchPoint(TouchPoint),
    TuneSetting(TuneSetting),
}

struct TouchServerChannel {
    /// 关闭通道 (watch::Receiver 可以 clone，不需要 Mutex)
    shutdown_tx: watch::Sender<ShutdownSignal>,
    shutdown_rx: Arc<Mutex<watch::Receiver<ShutdownSignal>>>,
    /// 触摸事件
    touch_event_tx: mpsc::UnboundedSender<TouchpadEvent>,
    touch_event_rx: Arc<Mutex<mpsc::UnboundedReceiver<TouchpadEvent>>>,
    /// 延迟计算
    latency_tx: Option<mpsc::UnboundedSender<LatencyDisplay>>,
    latency_rx: Option<Arc<Mutex<mpsc::UnboundedReceiver<LatencyDisplay>>>>,
}

/// 所有通讯服务处理句柄的集合
struct ServiceHandlers {
    /// 总体服务间通讯通道
    server_handler: RwLock<Option<tokio::task::JoinHandle<()>>>,
    /// 触摸事件服务
    touch_handler: RwLock<Option<tokio::task::JoinHandle<()>>>,
    /// 延迟计算服务
    latency_handler: RwLock<Option<tokio::task::JoinHandle<()>>>,
}

pub struct TouchServer {
    // 一个端点都对应一个UDP套接字
    pub endpoint: Endpoint,
    pub addr: SocketAddr,
    connections: Arc<RwLock<HashMap<u64, ConnectionInfo>>>,
    connected_device: Arc<Mutex<HashMap<IpAddr, Device>>>,
    touch_driver: Arc<std::sync::Mutex<Driver>>,
    /// 延迟跟踪器
    latency_tracker: Arc<std::sync::Mutex<RealtimeLatencyTracker>>,
    /// 服务间通讯通道
    server_channel: TouchServerChannel,
    service_handlers: ServiceHandlers,
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
        let touch_driver = match Driver::new(8192, 8192) {
            Ok(driver) => Arc::new(std::sync::Mutex::new(driver)),
            Err(err) => {
                error!("Failed to initialize touch driver: {}", err);
                return Err(err.into());
            },
        };

        let (touch_event_tx, touch_event_rx) = mpsc::unbounded_channel();
        let (latency_tx, latency_rx) = mpsc::unbounded_channel();
        let (shutdown_tx, shutdown_rx) = watch::channel(ShutdownSignal::Empty);

        let server_channel = TouchServerChannel {
            touch_event_tx,
            touch_event_rx: Arc::new(Mutex::new(touch_event_rx)),
            shutdown_tx,
            shutdown_rx: Arc::new(Mutex::new(shutdown_rx)),
            latency_tx: Some(latency_tx),
            latency_rx: Some(Arc::new(Mutex::new(latency_rx))),
        };

        let service_handlers = ServiceHandlers {
            server_handler: RwLock::new(None),
            touch_handler: RwLock::new(None),
            latency_handler: RwLock::new(None),
        };

        let touch_server = Self {
            endpoint,
            addr: ip_addr,
            connections: Arc::new(RwLock::new(HashMap::new())),
            connected_device: device_map,
            touch_driver,
            latency_tracker: Arc::new(std::sync::Mutex::new(RealtimeLatencyTracker::new(100))),
            server_channel,
            service_handlers,
        };
        Ok(touch_server)
    }

    /// 触控事件处理器 - 专门的任务处理触控事件，不阻塞网络 I/O
    async fn touch_event_processor(self: &Arc<Self>) {
        // Clone watch::Receiver（可以 clone，不需要 Mutex）
        let mut shutdown_rx = self.server_channel.shutdown_tx.subscribe();
        loop {
            tokio::select! {
                _ = shutdown_rx.changed() => {
                    let value = shutdown_rx.borrow().clone();
                    if value == ShutdownSignal::ProcesserStop(ProcesserType::Touch) {
                        break;
                    }
                },
                event = async {
                    let mut rx = self.server_channel.touch_event_rx.lock().await;
                    rx.recv().await
                } => {
                    let event = match event {
                        Some(event) => event,
                        None => break, // channel 关闭
                    };
                    match event {
                        TouchpadEvent::TouchPoint(touch_point) => {
                            if let Ok(mut driver) = self.touch_driver.lock() {
                                if let Err(e) = driver.emit_multitouch(&vec![touch_point]) {
                                    error!("Failed to emit touch event: {}", e);
                                }
                            }
                        },
                        TouchpadEvent::TuneSetting(tune_setting) => {
                            info!("tune setting: {:?}", tune_setting);
                            if let Ok(mut driver) = self.touch_driver.lock() {
                                driver.set_invert_x(tune_setting.invert_x);
                                driver.set_invert_y(tune_setting.invert_y);
                                driver.set_sensitivity(tune_setting.sensitivity);
                            }
                        },
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

    pub async fn connection_handler(self: &Arc<Self>) -> Result<()> {
        info!("Waiting for connection...");
        loop {
            let mut shutdown_subscribe = self.server_channel.shutdown_tx.subscribe();
            // 同时监听 shutdown 信号和新的双向流
            tokio::select! {
                _ = shutdown_subscribe.changed() => {
                    let value = shutdown_subscribe.borrow().clone();
                    if value == ShutdownSignal::ProcesserStop(ProcesserType::Connection) {
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
                                    let touch_event_tx = self.server_channel.touch_event_tx.clone();
                                    let latency_tracker = Arc::clone(&self.latency_tracker);
                                    let latency_tx = self.server_channel.latency_tx.clone();
                                    let task_handle = tokio::spawn(async move {
                                        let mut conn_client = ConnectedExector::new(
                                            conn_clone,
                                            Arc::clone(&connected_device),
                                            shutdown_subscribe.clone(),
                                            touch_event_tx,
                                            Some(latency_tracker),
                                            latency_tx,
                                        );
                                        if let Err(err) = conn_client.start().await {
                                            error!("Failed to client running: {}", err);
                                        }
                                        connection_map.write().await.remove(&conn_id);
                                        connected_device.lock().await.remove(&conn_ip);
                                        info!("Connection {} closed", conn_ip);
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

    /// 延迟数据广播处理器
    async fn latency_handler(self: &Arc<Self>) {
        // Clone watch::Receiver（可以 clone，不需要 Mutex）
        let mut shutdown_rx = self.server_channel.shutdown_tx.subscribe();

        loop {
            tokio::select! {
                _ = shutdown_rx.changed() => {
                    let value = shutdown_rx.borrow().clone();
                    if value == ShutdownSignal::ProcesserStop(ProcesserType::Latency) {
                        break;
                    }
                },
                Some(latency_data) = async {
                    let rx_opt = self.server_channel.latency_rx.as_ref();
                    match rx_opt {
                        Some(rx) => {
                            let mut locked_rx = rx.lock().await;
                            locked_rx.recv().await
                        },
                        None => std::future::pending().await
                    }
                } => {
                    // 这里可以通过 Tauri 事件发送到前端
                    // 暂时只记录日志
                    if latency_data.total_packets % 100 == 0 {
                        info!(
                            "📊 延迟统计: {:.2}ms (平均: {:.2}ms, 最小: {:.2}ms, 最大: {:.2}ms, 丢包率: {:.2}%)",
                            latency_data.current_ms,
                            latency_data.avg_ms,
                            latency_data.min_ms,
                            latency_data.max_ms,
                            latency_data.packet_loss_percent
                        );
                    }
                }
            }
        }
    }

    pub async fn start(self: &Arc<Self>) -> Result<()> {
        info!("Starting server loop");

        // 启动连接处理任务
        let this = self.clone();
        let server_handle = tokio::spawn(async move {
            if let Err(e) = this.connection_handler().await {
                error!("Failed to accept connection: {}", e);
            }
        });

        // 启动触控事件处理任务
        let this = self.clone();
        let touch_handler = tokio::spawn(async move {
            this.touch_event_processor().await;
        });

        // 启动延迟数据广播任务
        let this = self.clone();
        let latency_handler_task = tokio::spawn(async move {
            this.latency_handler().await;
        });

        self.service_handlers
            .server_handler
            .write()
            .await
            .replace(server_handle);
        self.service_handlers
            .touch_handler
            .write()
            .await
            .replace(touch_handler);
        self.service_handlers
            .latency_handler
            .write()
            .await
            .replace(latency_handler_task);

        Ok(())
    }

    pub async fn close(self: &Arc<Self>) {
        let shutdown_tx = &self.server_channel.shutdown_tx;
        let _ = shutdown_tx.send(ShutdownSignal::ProcesserStop(ProcesserType::Connection));
        let _ = shutdown_tx.send(ShutdownSignal::ProcesserStop(ProcesserType::Touch));
        let _ = shutdown_tx.send(ShutdownSignal::ProcesserStop(ProcesserType::Latency));
    }

    /// 获取当前延迟统计数据
    pub fn get_latency_stats(&self) -> LatencyDisplay {
        if let Ok(tracker) = self.latency_tracker.lock() {
            tracker.get_current_stats().to_display()
        } else {
            LatencyDisplay {
                current_ms: 0.0,
                avg_ms: 0.0,
                min_ms: 0.0,
                max_ms: 0.0,
                packet_loss_percent: 0.0,
                total_packets: 0,
            }
        }
    }

    /// 重置延迟统计数据
    pub fn reset_latency_stats(&self) {
        if let Ok(mut tracker) = self.latency_tracker.lock() {
            tracker.reset();
        }
    }

    /// 设置时钟偏移（用于同步手机和服务器时间）
    pub fn set_clock_offset(&self, offset_ms: i64) {
        if let Ok(mut tracker) = self.latency_tracker.lock() {
            tracker.set_clock_offset(offset_ms);
        }
    }
}

struct ConnectedExector {
    conn: quinn::Connection,
    done: bool,
    connected_device: Arc<Mutex<HashMap<IpAddr, Device>>>,
    touch_event_tx: mpsc::UnboundedSender<TouchpadEvent>,
    /// 停止信号
    stop_signal: watch::Receiver<ShutdownSignal>,
    /// 延迟跟踪器
    latency_tracker: Option<Arc<std::sync::Mutex<RealtimeLatencyTracker>>>,
    /// 延迟数据发送器
    latency_tx: Option<mpsc::UnboundedSender<LatencyDisplay>>,
}

impl ConnectedExector {
    fn new(
        conn: quinn::Connection,
        connected_device: Arc<Mutex<HashMap<IpAddr, Device>>>,
        stop_signal: watch::Receiver<ShutdownSignal>,
        touch_event_tx: mpsc::UnboundedSender<TouchpadEvent>,
        latency_tracker: Option<Arc<std::sync::Mutex<RealtimeLatencyTracker>>>,
        latency_tx: Option<mpsc::UnboundedSender<LatencyDisplay>>,
    ) -> Self {
        ConnectedExector {
            conn,
            done: false,
            connected_device,
            stop_signal,
            touch_event_tx,
            latency_tracker,
            latency_tx,
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
                // 保存客户端发送时间戳用于时钟同步
                let client_send_ts = device.send_ts;

                let device = Device {
                    name: device.device_name,
                    ip: IpAddr::from_str(&device.ip)?,
                    width: device.width,
                    height: device.height,
                };
                self.connected_device.lock().await.insert(device.ip, device);

                // 计算时钟偏移并设置
                if let (Some(tracker), Some(latency_tx)) = (&self.latency_tracker, &self.latency_tx)
                {
                    let server_recv_ts_ms = (self.get_timestamp_us() / 1000) as i64;
                    let clock_offset_ms = client_send_ts - server_recv_ts_ms;
                    if let Ok(mut tracker) = tracker.lock() {
                        tracker.set_clock_offset(clock_offset_ms);
                        info!(
                            "⏱️  时钟同步完成: 偏移量 = {}ms (客户端时间: {}ms, 服务器时间: {}ms)",
                            clock_offset_ms, client_send_ts, server_recv_ts_ms
                        );
                    }
                }

                Ok(true)
            },
            Payload::TouchPacket(touch_packet) => {
                // 记录延迟
                if let (Some(tracker), Some(latency_tx)) = (&self.latency_tracker, &self.latency_tx)
                {
                    let server_ts_us = self.get_timestamp_us();
                    if let Ok(mut tracker) = tracker.lock() {
                        if let Some(latency_data) = tracker.record_packet(
                            touch_packet.seq,
                            touch_packet.ts_ms,
                            server_ts_us,
                        ) {
                            // 发送延迟数据到前端
                            debug!("Latency data: {:?}", latency_data);
                            let _ = latency_tx.send(latency_data.to_display());
                        }
                    }
                }

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
                    let _ = self
                        .touch_event_tx
                        .send(TouchpadEvent::TouchPoint(touch_point));
                }
                Ok(true)
            },
            Payload::SettingRequest(setting) => {
                if let Some(value) = setting.value {
                    let _ = match value {
                        setting_request::Value::TuneSetting(tune_setting) => self
                            .touch_event_tx
                            .send(TouchpadEvent::TuneSetting(tune_setting)),
                    };
                    Ok(true)
                } else {
                    error!("Invalid setting request");
                    Ok(true)
                }
            },
            Payload::Exit(_exit) => {
                info!("Exiting connection: {:?}", self.conn.remote_address());
                self.conn.close((0 as u8).into(), b"");
                Ok(false)
            },
            _ => Ok(true),
        }
    }

    /// 获取当前时间戳（微秒）
    fn get_timestamp_us(&self) -> u64 {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_micros() as u64
    }
}
