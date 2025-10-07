use anyhow::Result;
use clap::Parser;
use core_kit::{config::TouchpadConfig, logger, server::Server};
use daemon::command::Command;
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    let _guard = logger::init_tracing();
    let args = Command::parse();
    if !args.config_file.exists() {
        panic!("Config file not found");
    }
    let config = TouchpadConfig::from(&args.config_file).unwrap();
    info!("Daemon started");
    info!("Daemon parameters: {:?}", args);

    let server = Server::new(&config).await?;
    tokio::join!(async move {
        info!("Server start");
        match server.endpoint.accept().await {
            Some(connection) => {
                let conn = connection.await.unwrap();
                println!(
                    "[server] incoming connection: addr={}",
                    conn.remote_address()
                );
            }
            None => {
                eprintln!("The connection was closed");
            }
        }
    });
    Ok(())
}
