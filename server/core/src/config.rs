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
pub struct Config {
    pub listen_addr: String,
    pub log_level: LogLevel,
}

impl Config {
    pub fn from(file_path: &impl AsRef<Path>) -> Result<Self, config::ConfigError> {
        let config_path = Path::new(file_path.as_ref());
        let ext = config_path.extension().and_then(|e| e.to_str());
        if ![Some("yaml"), Some("yml")].contains(&ext) {
            return Err(config::ConfigError::NotFound(
                config_path.to_string_lossy().into_owned(),
            ));
        }
        eprintln!("Trying to read: {:?}", config_path.canonicalize());
        let config = config::Config::builder()
            .add_source(config::File::from(config_path))
            .build()?
            .try_deserialize()
            .map_err(|err| config::ConfigError::FileParse {
                uri: Some(config_path.to_string_lossy().into_owned()),
                cause: Box::new(err),
            })?;
        Ok(config)
    }
}
