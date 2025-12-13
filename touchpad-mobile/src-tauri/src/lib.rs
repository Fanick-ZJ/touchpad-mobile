use std::{
    net::IpAddr,
    sync::{Arc, Mutex},
};

use mdns_sd::{IfKind, ServiceDaemon};
use shared_utils::execute_params;
use tauri::State;

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
    let interface = shared_utils::interface::enumerate_mdns_incapable_interfaces()
        .iter()
        .map(|inter_name| IfKind::from(inter_name))
        .collect::<Vec<IfKind>>();
    if let Err(err) = daemon.disable_interface(interface) {
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

    // 先停止服务
    daemon
        .stop_browse(execute_params::mdns_server_type())
        .map_err(|e| format!("Failed to stop discover_service"))?;
    let daemon = daemon.clone();
    tauri::async_runtime::spawn(async move {
        let receiver = match daemon.browse(execute_params::mdns_server_type()) {
            Ok(receiver) => receiver,
            Err(e) => {
                log::error!("Failed to browse for service types: {e:?}");
                return;
            }
        };
        while let Ok(event) = receiver.recv_async().await {
            match event {
                mdns_sd::ServiceEvent::SearchStarted(service_type) => {
                    log::info!("start mdns discover service: {service_type}")
                }
                mdns_sd::ServiceEvent::ServiceFound(service_type, full_name) => {
                    log::info!("found service: {full_name}")
                }
                mdns_sd::ServiceEvent::ServiceResolved(resolved_service) => {
                    log::info!("service resolved: {:?}", resolved_service)
                }
                mdns_sd::ServiceEvent::ServiceRemoved(_, _) => todo!(),
                mdns_sd::ServiceEvent::SearchStopped(_) => todo!(),
                _ => todo!(),
            }
        }
    });
    Ok(())
}
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    use chrono::Utc;
    use tauri_plugin_log::{fern, Target, TargetKind};

    let log_targets = vec![
        Target::new(TargetKind::Stdout),
        Target::new(TargetKind::LogDir { file_name: None }),
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
        .invoke_handler(tauri::generate_handler![start_discover_service])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
