use std::{collections::HashMap, net::SocketAddr, path::Path, sync::Arc};

use anyhow::Result;

use quinn::{
    Connection, Endpoint, ServerConfig,
    rustls::pki_types::{CertificateDer, PrivatePkcs8KeyDer},
};
use tokio::sync::{Notify, RwLock};
use tracing::{error, info};

use server_core_kit::{
    common::{read_cert, read_key},
    config::TouchpadConfig,
    inner_const::{LOCALHOST_V4, RECEIVE_SUCCESS, SERVER_STOP_CODE},
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

pub struct TouchServer {
    // 一个端点都对应一个UDP套接字
    pub endpoint: Endpoint,
    pub addr: SocketAddr,
    shutdown: Arc<Notify>,
    connections: Arc<RwLock<HashMap<u64, ConnectionInfo>>>,
    server_handle: RwLock<Option<tokio::task::JoinHandle<()>>>,
}

struct ConnectionInfo {
    conn: Connection,
    task_handle: tokio::task::JoinHandle<()>,
}

impl TouchServer {
    pub async fn new(config: &TouchpadConfig) -> Result<Self> {
        let server_config = Self::server_config(config).await?;
        let ip_addr = SocketAddr::new(LOCALHOST_V4, config.backend_port);
        let endpoint = Endpoint::server(server_config, ip_addr)?;
        let shutdown = Arc::new(Notify::new());
        info!("listening on {}", endpoint.local_addr()?);
        Ok(Self {
            endpoint,
            addr: ip_addr,
            shutdown: Arc::clone(&shutdown),
            connections: Arc::new(RwLock::new(HashMap::new())),
            server_handle: RwLock::new(None),
        })
    }

    /// 创建服务段的配置
    async fn server_config(config: &TouchpadConfig) -> Result<ServerConfig> {
        let cert_der_path = Path::new(&config.cert_pem);
        // 获取密钥文件
        let cert_der = read_cert(&cert_der_path).await?;
        let key_der_path = Path::new(&config.key_pem);
        let key_der = read_key(&key_der_path).await?;
        let server_config = configure_server(cert_der, key_der)?;
        info!("Server configuration created successfully");
        Ok(server_config)
    }

    pub async fn wait_connect(&self) -> Result<()> {
        info!("Waiting for connection...");
        loop {
            // 同时监听 shutdown 信号和新的双向流
            tokio::select! {
                _ = self.shutdown.notified() => {
                    info!("Shutdown signal received");
                    // 关闭所有连接
                    let conns = self.connections.read().await;
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
                },
                _ = async {
                    // 不停的等待连接
                    if let Some(incoming) = self.endpoint.accept().await {
                        match incoming.await {
                            Ok(conn) => {
                                // 将接受到的连接记录，并给他开启任务处理器
                                let conn_id = conn.stable_id() as u64;
                                let shutdown = Arc::clone(&self.shutdown);
                                let connection_map = Arc::clone(&self.connections);
                                info!("New connection: {}", conn_id);
                                let conn_clone = conn.clone();
                                let task_handle = tokio::spawn(async move {
                                    if let Err(e) = Self::handle_connect(conn_clone, shutdown).await {
                                        error!("Failed to handle connection: {}", e);
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
                    }
                } => {
                    info!("New connection established");
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

    async fn handle_connect(conn: Connection, shutdown: Arc<Notify>) -> Result<()> {
        loop {
            tokio::select! {
                _ = shutdown.notified() => {
                    info!("Shutdown signal received");
                    break;
                },
                accept_result = conn.accept_bi() => {
                    match accept_result {
                        Ok((send, recv)) => {
                            Self::handle_stream(send, recv).await?;
                        },
                        Err(e) => {
                            error!("Error accepting bidirectional stream: {}", e);
                            return Err(e.into());
                        }
                    }
                }
            }
        }
        Ok(())
    }

    async fn handle_stream(
        mut send: quinn::SendStream,
        mut recv: quinn::RecvStream,
    ) -> Result<bool> {
        let mut buff = [0u8; 64 * 1024];
        let mut bytes = Vec::new();

        // 读取数据直到流结束
        loop {
            match recv.read(&mut buff).await {
                Ok(Some(length)) => {
                    bytes.extend_from_slice(&buff[..length]);
                }
                Ok(None) => {
                    // 流正常结束
                    break;
                }
                Err(e) => {
                    error!("Error reading from stream: {}", e);
                    return Err(e.into());
                }
            }
        }

        info!("Received bytes length: {}", bytes.len());

        // 写入完成信号
        send.write_all(RECEIVE_SUCCESS.as_bytes()).await?;
        send.finish()?;

        info!(
            "Received bytes content: {:?}",
            String::from_utf8_lossy(&bytes)
        );
        Ok(true)
    }

    pub async fn close(&self) {
        self.shutdown.notify_waiters();
    }
}
