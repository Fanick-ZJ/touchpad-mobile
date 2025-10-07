use clap::Parser;
use config;
use core_kit::{config::TouchpadConfig, logger::init_tracing};
use tracing::{error, info};

#[derive(Parser, Debug)]
#[command(name = "touchpad", version = "0.1.0", about = "A simple touchpad utility", long_about = None)]
struct Cli {
    #[arg(short = 'c', long = "config", required = true)]
    config_file: std::path::PathBuf,
}

fn main() -> Result<(), config::ConfigError> {
    let _guard = init_tracing();
    let cli = Cli::parse();
    let config = TouchpadConfig::from(&cli.config_file).map_err(|e| {
        error!("Error: {}", e);
        e
    });
    info!("success to load config");
    Ok(())
}
