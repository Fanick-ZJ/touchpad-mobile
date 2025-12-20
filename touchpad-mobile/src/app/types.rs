use serde::{Deserialize, Serialize};
use std::net::IpAddr;

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct DiscoverDevice {
    name: String,
    address: IpAddr,
    login_port: u16,
}
