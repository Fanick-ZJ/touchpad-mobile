use anyhow::{Result, anyhow};
use std::path::Path;

use serde::Deserialize;

#[derive(Debug, Deserialize, PartialEq)]
pub enum LogLevel {
    #[serde(rename = "debug")]
    Debug,
    #[serde(rename = "info")]
    Info,
    #[serde(rename = "warn")]
    Warn,
    #[serde(rename = "error")]
    Error,
}

#[derive(Debug, Deserialize)]
pub struct TouchpadConfig {
    // 发现服务的端口
    #[serde(default = "default_port")]
    pub discover_port: u16,
    // 登录服务的端口
    #[serde(default = "default_login_port")]
    pub login_port: u16,
    // 后端服务的端口
    #[serde(default = "default_backend_port")]
    pub backend_port: u16,
    // 将发现服务绑定到制定的地址，如果为None,则为自动，可能会被绑定到ipv6的地址中
    pub ip: Option<String>,
    #[serde(default = "default_log_level")]
    pub log_level: LogLevel,
    pub cert_pem: Option<String>,
    pub key_pem: Option<String>,
}

fn default_port() -> u16 {
    8521
}

fn default_login_port() -> u16 {
    8522
}

fn default_backend_port() -> u16 {
    8523
}

fn default_log_level() -> LogLevel {
    LogLevel::Info
}

impl TouchpadConfig {
    pub fn from(file_path: &impl AsRef<Path>) -> Result<Self> {
        let config_path = Path::new(file_path.as_ref());
        let ext = config_path.extension().and_then(|e| e.to_str());
        if ![Some("yaml"), Some("yml")].contains(&ext) {
            return Err(anyhow!(config::ConfigError::NotFound(
                config_path.to_string_lossy().into_owned(),
            )));
        }
        let config_path = if config_path.is_relative() {
            config_path.canonicalize()?
        } else {
            config_path.to_path_buf()
        };
        if !config_path.exists() {
            return Err(anyhow!(config::ConfigError::NotFound(
                config_path.to_string_lossy().into_owned(),
            )));
        }
        eprintln!("Trying to read: {:?}", config_path);
        let config = config::Config::builder()
            .add_source(config::File::from(config_path))
            .build()?
            .try_deserialize()?;
        Ok(config)
    }
}
