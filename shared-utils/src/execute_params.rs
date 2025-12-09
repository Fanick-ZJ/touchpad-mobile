use serde::Deserialize;
use std::sync::LazyLock;

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
struct ExecutionParam {
    shared_config: SharedConfig,
}

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
struct SharedConfig {
    seed: String,
    mdns_server_type: String,
}

static EXE_PARAM: LazyLock<ExecutionParam> = LazyLock::new(|| {
    //touchpad/.env
    const FILE_CONTENT: &str = include_str!("../../execution_params.toml");
    let config: ExecutionParam = toml::from_str(FILE_CONTENT).unwrap();
    config
});

pub fn hash_seed() -> &'static str {
    EXE_PARAM.shared_config.seed.as_str()
}

pub fn mdns_server_type() -> &'static str {
    EXE_PARAM.shared_config.mdns_server_type.as_str()
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_seed_constant() {
        assert_eq!(hash_seed(), "0x1234567890ABCDEF");
    }
}
