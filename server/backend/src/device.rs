use std::net::IpAddr;

#[derive(Clone, Debug)]
pub struct Device {
    pub name: String,
    pub ip: IpAddr,
    pub width: u32,
    pub height: u32,
}
