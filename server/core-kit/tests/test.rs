use server_core_kit::config::{LogLevel, TouchpadConfig};

#[test]
fn config_parse() {
    let config = TouchpadConfig::from(&"tests/config.yml").unwrap();
    assert_eq!(config.backend_port, 8521);
    assert_eq!(config.log_level, LogLevel::Info);
}
