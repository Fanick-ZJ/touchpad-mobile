use anyhow::Result;
use std::{collections::HashMap, str::FromStr, sync::LazyLock};

static ENV: LazyLock<HashMap<String, String>> = LazyLock::new(|| {
    //touchpad/.env
    const FILE_CONTENT: &str = include_str!("../../../.env");
    let mut map = HashMap::new();
    for line in FILE_CONTENT.lines() {
        let tokens: Vec<String> = line
            .split("=")
            .map(String::from_str)
            .filter_map(Result::ok)
            .collect();
        if tokens.len() != 2 {
            panic!(".env content: {} is not valid", line)
        } else {
            map.insert(tokens[0].clone(), tokens[1].clone());
        }
    }
    map
});

pub fn get_env(key: &str) -> Option<&str> {
    ENV.get(key).map(|s| s.as_str())
}
pub fn hash_seed() -> Option<&'static str> {
    get_env("SEED")
}
