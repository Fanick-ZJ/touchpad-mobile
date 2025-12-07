use std::{
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
    sync::Arc,
    vec,
};

use anyhow::{Result, anyhow};
use clap::Parser;
use core_kit::{config::TouchpadConfig, logger::init_tracing};
use discover::{device::Device, discover_service::DiscoverService};
use tracing::{error, info};
use utils::env::get_env;

#[derive(Parser, Debug)]
#[command(name = "touchpad", version = "0.1.0", about = "A simple touchpad utility", long_about = None)]
struct Cli {
    #[arg(short = 'c', long = "config", required = true)]
    config_file: std::path::PathBuf,
}

#[tokio::main]
async fn main() -> Result<()> {
    let _guard = init_tracing();
    let cli = Cli::parse();
    let config = TouchpadConfig::from(&cli.config_file).map_err(|e| {
        error!("Error: {}", e);
        e
    })?;
    info!("success to load config");
    let check_seed = get_env("SEED").ok_or(anyhow!(
        "The .env file is missing or the SEED environment variable is not set."
    ))?;

    // 获取指定的ip地址
    let discover_service_ip = if config.discover_address.is_some() {
        let addr = config.discover_address.unwrap();
        let ipv4 = addr.parse::<Ipv4Addr>();
        let ipv6 = addr.parse::<Ipv6Addr>();
        if ipv4.is_ok() {
            Some(vec![IpAddr::V4(ipv4.unwrap())])
        } else if ipv6.is_ok() {
            Some(vec![IpAddr::V6(ipv6.unwrap())])
        } else {
            error!("The discover service address is invalid");
            None
        }
    } else {
        None
    };

    let callback: Box<dyn Fn(&Device, Vec<&Device>) + Send + Sync> =
        Box::new(|device, device_list| {
            // 在这里添加回调逻辑
            info!("Device found: {:?}", device);
            info!("Device list: {:?}", device_list);
        });

    let discover_service = Arc::new(DiscoverService::new(
        10,
        config.discover_port,
        check_seed.to_string(),
        discover_service_ip,
        Some(callback),
    ));
    discover_service.discover().await?;
    // 启动发现服务
    tokio::signal::ctrl_c().await?;
    Ok(())
}
