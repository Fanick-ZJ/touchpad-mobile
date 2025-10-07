use std::{net::SocketAddr, sync::Arc, time::Instant};

use anyhow::Result;
use quinn::{
    ClientConfig, Connection, Endpoint, RecvStream, VarInt,
    rustls::{self, pki_types::CertificateDer},
};
use tracing::{error, info};

use crate::inner_const::{RECEIVE_SUCCESS, SERVER_STOP_CODE};

fn configure_client(server_certs: &[&[u8]]) -> Result<ClientConfig> {
    let mut certs = rustls::RootCertStore::empty();
    for cert in server_certs {
        certs.add(CertificateDer::from(*cert))?
    }
    Ok(ClientConfig::with_root_certificates(Arc::new(certs))?)
}

fn make_client_endpoint(bind_addr: SocketAddr, server_certs: &[&[u8]]) -> Result<Endpoint> {
    let config = configure_client(server_certs)?;
    let mut endpoint = Endpoint::client(bind_addr)?;
    endpoint.set_default_client_config(config);
    Ok(endpoint)
}

pub struct Client {
    // 一个端点都对应一个UDP套接字
    pub endpoint: Endpoint,
    server_name: String,
    server_addr: SocketAddr,
    connection: Option<Connection>,
}

impl Client {
    pub fn new(
        local_addr: SocketAddr,
        server_addr: SocketAddr,
        server_certs: &[&[u8]],
        server_name: String,
    ) -> Result<Self> {
        let endpoint = make_client_endpoint(local_addr, server_certs)?;
        Ok(Self {
            endpoint,
            server_name,
            server_addr,
            connection: None,
        })
    }

    pub async fn connect(&mut self) -> Result<()> {
        let connection = self
            .endpoint
            .connect(self.server_addr, &self.server_name)?
            .await?;
        self.connection = Some(connection);
        Ok(())
    }

    pub async fn disconnect(&mut self) -> Result<()> {
        if let Some(connection) = self.connection.take() {
            connection.close(VarInt::from_u32(0), b"");
        }
        Ok(())
    }

    pub async fn send(&mut self, packet: &[u8]) -> Result<()> {
        if let None = self.connection {
            self.connect().await?;
        }
        let start_time = Instant::now();
        let connection = self.connection.as_ref().unwrap();
        let (mut send, recv) = connection.open_bi().await?;
        send.write_all(packet).await?;
        send.finish()?;
        self.receive(recv).await;
        let elapsed_time = start_time.elapsed();
        info!("Packet sent in {}μs", elapsed_time.as_micros());
        Ok(())
    }

    async fn receive(&self, mut recv: RecvStream) {
        let mut bytes = Vec::new();
        let mut buff = [0_u8; 1024];
        while let Ok(Some(length)) = recv.read(&mut buff).await {
            bytes.extend_from_slice(&buff[..length]);
        }
        let success_len = RECEIVE_SUCCESS.as_bytes().len();
        if bytes.len() < success_len {
            // Handle unexpected response
            error!("Unexpected response from server: {:?}", bytes);
            return;
        }
        if &bytes[..success_len] != RECEIVE_SUCCESS.as_bytes() {
            // Handle unexpected response
            error!("Unexpected response from server: {:?}", bytes);
        }
    }

    pub async fn finish(&mut self) -> Result<()> {
        self.send(SERVER_STOP_CODE.as_bytes()).await?;
        if let Some(connection) = self.connection.take() {
            connection.close(VarInt::from_u32(0), b"");
        }
        info!("Client finished");
        Ok(())
    }
}
