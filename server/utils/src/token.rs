use anyhow::{Result, anyhow};
use std::{
    collections::HashMap,
    net::IpAddr,
    sync::{LazyLock, RwLock},
};

use xxhash_rust::xxh3::xxh3_64;

use crate::env;

static PREV_TOKENS: LazyLock<RwLock<HashMap<IpAddr, String>>> = LazyLock::new(|| {
    let tokens = RwLock::new(HashMap::new());
    tokens
});

pub fn get_token(ip: IpAddr) -> Option<String> {
    PREV_TOKENS.read().unwrap().get(&ip).cloned()
}

pub fn get_first_token(ip: &IpAddr, random_key: &str, device_name: &str) -> Result<String> {
    let mut prev_tokens = PREV_TOKENS.write().unwrap();
    if let Some(_) = prev_tokens.get(&ip) {
        return Err(anyhow!(format!(
            "Is not first to get token in {}",
            ip.to_string()
        )));
    }
    let seed = env::hash_seed().expect("Failed to get .env field:hash seed");
    let token =
        xxh3_64(&format!("{}{}{}{}", random_key, ip.to_string(), device_name, seed).into_bytes())
            .to_string();
    prev_tokens.insert(*ip, token.clone());
    Ok(token)
}

pub fn gen_token(ip: &IpAddr) -> String {
    let seed = env::hash_seed().expect("Failed to get .env field:hash seed");
    let prev_tokens = PREV_TOKENS.read().unwrap();
    let prev_token = if let Some(token) = prev_tokens.get(&ip) {
        token
    } else {
        ""
    };
    let token =
        xxh3_64(&format!("{}{}{}", ip.to_string(), seed, prev_token).into_bytes()).to_string();
    token
}

pub fn set_token(ip: IpAddr) {
    let token = gen_token(&ip);
    let mut prev_tokens = PREV_TOKENS.write().unwrap();
    prev_tokens.insert(ip, token);
}

pub fn token_valid(ip: &IpAddr, token: String) -> bool {
    let seed = env::hash_seed().expect("Failed to get .env field:hash seed");
    let prev_tokens = PREV_TOKENS.read().unwrap();
    let prev_token = if let Some(token) = prev_tokens.get(&ip) {
        token
    } else {
        ""
    };
    let expected_token =
        xxh3_64(&format!("{}{}{}", ip.to_string(), seed, prev_token).into_bytes()).to_string();
    expected_token == token
}
