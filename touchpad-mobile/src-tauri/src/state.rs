use std::{
    net::IpAddr,
    sync::{Arc, Mutex},
};

use mdns_sd::{IfKind, ServiceDaemon};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct DiscoverDevice {
    pub name: String,
    pub address: IpAddr,
    pub full_name: String,
    pub login_port: u16,
    pub backend_port: u16,
}

pub type SharedServiceDaemon = Arc<Mutex<ServiceDaemon>>;
pub struct ManagedState {
    pub daemon: SharedServiceDaemon,
    pub devices: Arc<Mutex<Vec<DiscoverDevice>>>,
    pub current_device: Arc<Mutex<Option<DiscoverDevice>>>,
    pub backend_screen: Arc<Mutex<bool>>,
    pub token: Arc<Mutex<Option<String>>>,
}

impl ManagedState {
    pub fn new() -> Self {
        Self {
            daemon: initialize_shared_daemon(),
            devices: Arc::new(Mutex::new(vec![])),
            current_device: Arc::new(Mutex::new(None)),
            backend_screen: Arc::new(Mutex::new(false)),
            token: Arc::new(Mutex::new(None)),
        }
    }
}

fn initialize_shared_daemon() -> SharedServiceDaemon {
    let daemon = ServiceDaemon::new().expect("Failed to create daemon");
    let interface = shared_utils::interface::enumerate_mdns_incapable_interfaces()
        .iter()
        .map(|inter_name| IfKind::from(inter_name))
        .collect::<Vec<IfKind>>();
    if let Err(err) = daemon.disable_interface(interface) {
        log::warn!("Failed to disable interface: {err:?}, continuing anyway");
    }
    Arc::new(Mutex::new(daemon))
}
