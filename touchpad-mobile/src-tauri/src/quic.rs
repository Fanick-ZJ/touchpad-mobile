use std::{net::SocketAddr, str::FromStr, sync::Arc};

use anyhow::{anyhow, Result};
use log::info;
use prost::Message;
use quinn::{
    crypto::rustls::QuicClientConfig,
    rustls::{self, pki_types::CertificateDer},
};
use tokio::sync::RwLock;
use touchpad_proto::{
    codec::ProtoStream,
    proto::v1::{wrapper::Payload, Exit},
};

pub struct QuicClient {
    cert_der: Vec<u8>,
    endpoint: Option<quinn::Endpoint>,
    proto_stream: Option<ProtoStream>,
    remote_addr: Option<SocketAddr>,
    paused: Arc<RwLock<bool>>,
    touch_pack_count: Arc<RwLock<u32>>,
}

impl QuicClient {
    pub fn new(cert_der: Vec<u8>) -> Self {
        Self {
            cert_der,
            endpoint: None,
            proto_stream: None,
            remote_addr: None,
            paused: Arc::new(RwLock::new(false)),
            touch_pack_count: Arc::new(RwLock::new(0)),
        }
    }

    pub fn remote_host(&self) -> Result<String> {
        if let Some(addr) = self.remote_addr {
            Ok(addr.ip().to_string())
        } else {
            Err(anyhow!(format!("此客户端暂未连接")))
        }
    }

    pub async fn connect(&mut self, addr: &str) -> Result<()> {
        info!("开始连接 QUIC 服务器: {}", addr);

        // self.cert_der 已经是 DER 格式的二进制数据
        let cert_der = CertificateDer::from(self.cert_der.clone());
        let mut roots = rustls::RootCertStore::empty();
        if let Err(e) = roots.add(cert_der) {
            info!("添加根证书失败: {:?}", e);
            return Err(e.into());
        }
        info!("根证书配置成功");

        let mut client_crypto = rustls::ClientConfig::builder()
            .with_root_certificates(roots)
            .with_no_client_auth();

        client_crypto.key_log = Arc::new(rustls::KeyLogFile::new());
        let client_config =
            quinn::ClientConfig::new(Arc::new(QuicClientConfig::try_from(client_crypto)?));
        let remote_addr = SocketAddr::from_str(addr)?;
        let bind_addr: SocketAddr = "0.0.0.0:0".parse()?;

        let mut endpoint = quinn::Endpoint::client(bind_addr)?;
        endpoint.set_default_client_config(client_config);

        let connecting = endpoint.connect(remote_addr, "touchpad.internal")?;
        let conn = connecting.await?;

        self.endpoint = Some(endpoint);
        // 客户端需要主动打开双向流，而不是 accept_bi()
        let (send, recv) = conn.open_bi().await?;
        info!("双向流打开成功!");
        let proto_stream = ProtoStream::new(Box::new(send), Box::new(recv));
        self.proto_stream.replace(proto_stream);
        self.remote_addr.replace(remote_addr);
        Ok(())
    }

    pub async fn send<M: Message + 'static>(&mut self, msg: &M) -> Result<()> {
        if *self.paused.read().await {
            return Err(anyhow::anyhow!("Client is paused"));
        }
        if let Some(proto_stream) = self.proto_stream.as_mut() {
            proto_stream.send_message(msg).await?;
        }
        Ok(())
    }

    pub async fn recv(&mut self) -> Result<Payload> {
        if *self.paused.read().await {
            return Err(anyhow::anyhow!("Client is paused"));
        }
        if let Some(proto_stream) = self.proto_stream.as_mut() {
            let msg = proto_stream.receive_message().await?;
            Ok(msg)
        } else {
            Err(anyhow::anyhow!("No proto stream"))
        }
    }

    pub async fn pause(&mut self) -> Result<()> {
        *self.paused.write().await = true;
        Ok(())
    }

    pub async fn resume(&mut self) -> Result<()> {
        *self.paused.write().await = false;
        Ok(())
    }

    pub async fn disconnect(&mut self) -> Result<()> {
        if let Some(endpoint) = self.endpoint.take() {
            let now = chrono::Utc::now().timestamp() as u64;
            self.send(&Exit { ts_ms: now }).await?;
            // 2. 等待一小段时间，确保消息发送完成
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

            endpoint.close((0 as u8).into(), b"Goodbye");

            self.remote_addr.take();
            self.proto_stream.take();
        }
        Ok(())
    }

    pub async fn touch_pack_count(&mut self) -> u32 {
        *self.touch_pack_count.read().await
    }

    pub async fn increment_touch_pack_count(&mut self) -> Result<()> {
        *self.touch_pack_count.write().await += 1;
        Ok(())
    }
}
