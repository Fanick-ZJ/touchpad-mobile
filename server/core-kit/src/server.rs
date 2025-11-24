use std::{net::SocketAddr, path::Path, sync::Arc};

use anyhow::{Result, anyhow};

use quinn::{
    Connection, Endpoint, ServerConfig, VarInt,
    rustls::pki_types::{CertificateDer, PrivatePkcs8KeyDer},
};
use tokio::sync::{Notify, RwLock};
use tracing::{error, info};

use crate::{
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

pub struct Server {
    // 一个端点都对应一个UDP套接字
    pub endpoint: Endpoint,
    pub addr: SocketAddr,
    shutdown: Arc<Notify>,
    shutdown_tx: Arc<Notify>,
    connection: RwLock<Option<Connection>>,
}

impl Server {
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
            shutdown_tx: shutdown,
            connection: RwLock::new(None),
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
        let mut connection = self.connection.write().await;
        *connection = Some(
            self.endpoint
                .accept()
                .await
                .ok_or(anyhow!("Failed to accept connection"))?
                .await?,
        );
        info!(
            "Connection established, ip: {:?}",
            connection.as_ref().unwrap().local_ip()
        );
        Ok(())
    }

    pub async fn run_work(self: Arc<Self>) -> Result<()> {
        info!("Starting server loop iteration");
        loop {
            // 先确保有连接
            if !self.has_connection().await {
                info!("No active connection, waiting for one...");
                if let Err(e) = self.wait_connect().await {
                    error!("Failed to accept connection: {}", e);
                    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                    continue;
                }
            }

            // 获取当前连接（已存在）
            let conn = {
                let guard = self.connection.read().await;
                guard.as_ref().unwrap().clone()
            };
            // 现在，我们有两个可等待的事件：
            // 1. shutdown 信号
            // 2. 新的双向流
            tokio::select! {
                _ = self.shutdown.notified() => {
                    info!("Shutdown signal received");
                    tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
                    break;
                }
                stream_res = conn.accept_bi() => {
                    match stream_res {
                        Ok(stream) => {
                            info!("New stream accepted");
                            self.handle_stream(stream).await?;
                        }
                        Err(e) => {
                            error!("Connection error (maybe closed): {}", e);
                            // 清除连接，下一轮重新 wait_connect
                            self.disconnect().await;
                        }
                    }
                }
            }
        }
        Ok(())
    }

    async fn disconnect(&self) {
        if let Some(conn) = self.connection.read().await.as_ref() {
            conn.close(VarInt::from_u32(0), b"Server shutdown");
        }
        *self.connection.write().await = None;
    }

    async fn has_connection(&self) -> bool {
        self.connection.read().await.is_some()
    }

    async fn handle_stream(
        &self,
        (mut send, mut recv): (quinn::SendStream, quinn::RecvStream),
    ) -> Result<bool> {
        let mut buff = [0u8; 64 * 1024];
        let mut bytes = Vec::new();
        while let Ok(Some(length)) = recv.read(&mut buff).await {
            if length == 0 {
                break;
            }
            bytes.extend_from_slice(&buff[..length]);
        }
        info!("Received bytes length: {}", bytes.len());
        // 写入完成信号
        send.write_all(RECEIVE_SUCCESS.as_bytes()).await?;
        send.finish()?;
        // 判断关闭信号
        if bytes == SERVER_STOP_CODE.as_bytes() {
            info!("Received stop code");
            self.shutdown_tx.notify_one();
            return Ok(false);
        }
        info!("Received bytes content: {:?}", String::from_utf8(bytes));
        Ok(true)
    }

    pub async fn close(&mut self) {
        self.shutdown_tx.notify_one();
    }
}
