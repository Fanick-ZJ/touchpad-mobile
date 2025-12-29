use std::{
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
    sync::Arc,
};

use anyhow::{Result, anyhow};
use clap::Parser;
use server_backend::{discover_service::DiscoverService, touch_server::TouchServer};
use server_core_kit::{config::TouchpadConfig, device::Device, logger::init_tracing};
use shared_utils::{
    execute_params,
    interface::{enumerate_mdns_capable_interfaces, get_ip_by_name},
};
use tracing::{error, info};

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
    let check_seed = execute_params::hash_seed();

    // 获取指定的ip地址
    let discover_service_ip = if config.ip.is_some() {
        let addr = config.ip.as_ref().unwrap();
        let ipv4 = addr.parse::<Ipv4Addr>();
        let ipv6 = addr.parse::<Ipv6Addr>();
        if ipv4.is_ok() {
            IpAddr::V4(ipv4.unwrap())
        } else if ipv6.is_ok() {
            IpAddr::V6(ipv6.unwrap())
        } else {
            error!("The discover service address is invalid");
            return Err(anyhow!("Invalid discover service address"));
        }
    } else {
        let inter_names = enumerate_mdns_capable_interfaces();
        if inter_names.is_empty() {
            error!("No network interface found");
            return Err(anyhow!("No network interface found"));
        }
        let ip = get_ip_by_name(&inter_names[0], true);
        if let Some(ip) = ip {
            ip
        } else {
            error!("Failed to get IP address");
            return Err(anyhow!("Failed to get IP address"));
        }
    };

    let callback: Box<dyn Fn(&Device, Vec<&Device>) + Send + Sync> =
        Box::new(|device, device_list| {
            // 在这里添加回调逻辑
            info!("Device found: {:?}", device);
            info!("Device list: {:?}", device_list);
        });

    let discover_service = Arc::new(DiscoverService::new(
        config.login_port,
        config.backend_port,
        config.discover_port,
        check_seed.to_string(),
        discover_service_ip,
        Some(callback),
    ));
    // 启动发现服务
    discover_service.discover().await?;
    let listening_device = discover_service.listening_derive();
    let touch_service = Arc::new(TouchServer::new(&config).await?);
    touch_service.start().await?;
    tokio::signal::ctrl_c().await?;
    touch_service.close().await;
    Ok(())
}
