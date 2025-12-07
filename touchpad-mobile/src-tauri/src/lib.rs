use std::{
    net::IpAddr,
    sync::{Arc, Mutex},
};

use mdns_sd::ServiceDaemon;
use tauri::State;
mod utils;

struct DiscoverDevice {
    name: String,
    address: IpAddr,
}

type SharedServiceDaemon = Arc<Mutex<ServiceDaemon>>;
struct ManagedState {
    daemon: SharedServiceDaemon,
    devices: Arc<Mutex<Vec<DiscoverDevice>>>,
    backend_screen: Arc<Mutex<bool>>,
}

impl ManagedState {
    pub fn new() -> Self {
        Self {
            daemon: initialize_shared_daemon(),
            devices: Arc::new(Mutex::new(vec![])),
            backend_screen: Arc::new(Mutex::new(false)),
        }
    }
}

fn initialize_shared_daemon() -> SharedServiceDaemon {
    let daemon = ServiceDaemon::new().expect("Failed to create daemon");
    if let Err(err) = daemon.disable_interface(utils::enumerate_mdns_incapable_interfaces()) {
        log::warn!("Failed to disable interface: {err:?}, continuing anyway");
    }
    Arc::new(Mutex::new(daemon))
}

#[tauri::command]
fn start_discover_service(state: State<ManagedState>) -> Result<(), String> {
    let daemon = state
        .daemon
        .lock()
        .map_err(|e| format!("Failed to lock daemon: {e:?}"))?;
    // daemon.stop_browse(ty_domain)
    Ok(())
}
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    use chrono::Utc;
    use tauri_plugin_log::{fern, Target, TargetKind};

    let log_targets = vec![
        Target::new(TargetKind::Stdout),
        Target::new(TargetKind::Webview),
    ];
    let colors = fern::colors::ColoredLevelConfig::default();
    tauri::Builder::default()
        .plugin(
            tauri_plugin_log::Builder::new()
                .targets(log_targets)
                .level(tauri_plugin_log::log::LevelFilter::Info)
                .format(move |out, message, record| {
                    let now = Utc::now();
                    let level = format!("{:<5}", colors.color(record.level()));
                    out.finish(format_args!(
                        "{date} {level} {target}: {message}",
                        date = now.format("%Y-%m-%dT%H:%M:%S%.6fZ"),
                        level = level,
                        target = record.target(),
                        message = message
                    ))
                })
                .build(),
        )
        .plugin(tauri_plugin_opener::init())
        .manage(ManagedState::new())
        .invoke_handler(tauri::generate_handler![])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
