use core::config::{Config, LogLevel};

#[test]
fn config_parse() {
    let config = Config::from(&"tests/config.yml").unwrap();
    assert_eq!(config.listen_addr, "8521");
    assert_eq!(config.log_level, LogLevel::Info);
}
