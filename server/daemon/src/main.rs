use clap::Parser;
use core::logger;
use daemon::command::Command;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use tracing::info;

#[tokio::main]
async fn main() {
    let _guard = logger::init_tracing();
    let args = Command::parse();
    info!("Daemon started");
    info!("Daemon parameters: {:?}", args);
    let server_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), args.port);
}
