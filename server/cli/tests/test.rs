use std::{net::SocketAddr, path::Path, sync::Arc};

use anyhow::Result;
use server_core_kit::{
    client::Client, common::read_cert, config::TouchpadConfig, inner_const::LOCALHOST_V4, logger,
    server::Server,
};

use tracing::{error, info};

#[tokio::test]
async fn server() -> Result<()> {
    let _guard = logger::init_tracing();
    let config = TouchpadConfig::from(&"tests/config.yml").unwrap();
    info!("Daemon started");

    let server = Arc::new(Server::new(&config).await?);
    // 在后台启动服务器
    let _ = tokio::join!(async { server.run_work().await });
    Ok(())
}

#[tokio::test]
async fn client() -> Result<()> {
    let _guard = logger::init_tracing();
    let config = TouchpadConfig::from(&"tests/config.yml").unwrap();
    info!("Daemon started");

    let server_addr = SocketAddr::new(LOCALHOST_V4, config.backend_port);
    let cert = read_cert(Path::new(&config.cert_pem)).await?;
    let local_addr = SocketAddr::new(LOCALHOST_V4, 0);
    let mut client = Client::new(local_addr, server_addr, &[&cert], "localhost".into())?;
    client.connect().await?;
    for _ in 0..1000 {
        info!("Send message");
        tokio::join!(async {
            if let Err(e) = client.send("Hello".as_bytes()).await {
                error!("Send message failed: {}", e);
            }
        });
    }
    client.finish().await?;
    Ok(())
}
