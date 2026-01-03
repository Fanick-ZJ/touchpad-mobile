use std::{
    collections::HashMap,
    net::{IpAddr, SocketAddr},
    sync::Arc,
};

use anyhow::Result;

use quinn::{
    Connection, Endpoint, ServerConfig,
    rustls::pki_types::{CertificateDer, PrivatePkcs8KeyDer},
};
use tokio::sync::{Mutex, RwLock, watch};
use touchpad_proto::{codec::ProtoStream, proto::v1::wrapper::Payload};
use tracing::{error, info};

use server_core_kit::{device::Device, inner_const::LOCALHOST_V4};

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
    valid_device: Arc<Mutex<HashMap<IpAddr, Device>>>,
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
        Ok(Self {
            endpoint,
            addr: ip_addr,
            shutdown_tx: Mutex::new(None),
            connections: Arc::new(RwLock::new(HashMap::new())),
            server_handle: RwLock::new(None),
            valid_device: device_map,
        })
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
                                    info!("New connection: {}", conn_id);
                                    let conn_clone = conn.clone();
                                    let task_handle = tokio::spawn(async move {
                                        let mut conn_client = ConnectedExector::new(conn_clone, shutdown_subscribe);
                                        if let Err(err) = conn_client.start().await {
                                            error!("Failed to client running: {}", err);
                                        }
                                        connection_map.write().await.remove(&conn_id);
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
    /// 停止信号
    stop_signal: watch::Receiver<ShutdownSignal>,
}

impl ConnectedExector {
    fn new(conn: quinn::Connection, stop_signal: watch::Receiver<ShutdownSignal>) -> Self {
        ConnectedExector { conn, stop_signal }
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
            self.handle_message(message).await?;
            todo!()
        }

        Ok(())
    }

    /// 处理消息，OK(False)代表推出连接
    async fn handle_message(&self, message: Payload) -> Result<()> {
        match message {
            Payload::Welcome(welcome) => todo!(),
            Payload::TouchPacket(touch_packet) => todo!(),
            Payload::HeartBeat(heart_beat) => todo!(),
            Payload::DiscoverValidation(discover_validation) => todo!(),
            Payload::Reject(reject) => todo!(),
        }
    }
}
