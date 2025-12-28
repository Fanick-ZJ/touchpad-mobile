use std::{
    net::IpAddr,
    sync::{Arc, Mutex},
};

use anyhow::Result;
use mdns_sd::{IfKind, ServiceDaemon};
use serde::{Deserialize, Serialize};
use shared_utils::execute_params;
use tauri::{utils::acl::resolved, AppHandle, Emitter, EventTarget, State};

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
struct DiscoverDevice {
    name: String,
    address: IpAddr,
    login_port: u16,
}

type SharedServiceDaemon = Arc<Mutex<ServiceDaemon>>;
struct ManagedState {
    daemon: SharedServiceDaemon,
    devices: Arc<Mutex<Vec<DiscoverDevice>>>,
    current_device: Arc<Mutex<Option<DiscoverDevice>>>,
    backend_screen: Arc<Mutex<bool>>,
}

impl ManagedState {
    pub fn new() -> Self {
        Self {
            daemon: initialize_shared_daemon(),
            devices: Arc::new(Mutex::new(vec![])),
            current_device: Arc::new(Mutex::new(None)),
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
fn start_discover_service(app: AppHandle, state: State<ManagedState>) -> Result<(), String> {
    let daemon = state
        .daemon
        .lock()
        .map_err(|e| format!("Failed to lock daemon: {e:?}"))?;

    let service_type = execute_params::mdns_server_type();
    // 先停止服务
    daemon
        .stop_browse(service_type)
        .map_err(|e| format!("Failed to stop discover_service"))?;
    let daemon = daemon.clone();
    let devices_arc = state.devices.clone();
    tauri::async_runtime::spawn(async move {
        let receiver = match daemon.browse(service_type) {
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
                    log::info!("found service: {service_type}")
                }
                mdns_sd::ServiceEvent::ServiceResolved(resolved_service) => {
                    log::info!("service resolved: {:?}", resolved_service);
                    let domain_name = resolved_service.ty_domain;
                    let target_name = resolved_service
                        .fullname
                        .split(&format!(".{domain_name}"))
                        .next();
                    let ip = resolved_service.addresses.iter().next().map(|addr| addr);
                    let login_port: Option<u16> = resolved_service
                        .txt_properties
                        .get_property_val_str("login_port")
                        .and_then(|port| port.to_string().parse().ok());
                    if let Some(target_name) = target_name {
                        log::info!("target name: {}", target_name);
                    } else {
                        log::warn!("target name not found");
                        continue;
                    }
                    if let Some(ip) = ip {
                        log::info!("ip: {}", ip);
                    } else {
                        log::warn!("ip not found");
                        continue;
                    }
                    if let Some(login_port) = login_port {
                        log::info!("login port: {}", login_port);
                    } else {
                        log::warn!("login port not found");
                        continue;
                    }
                    let device = DiscoverDevice {
                        name: target_name.unwrap().to_string(),
                        address: ip.unwrap().to_ip_addr(),
                        login_port: login_port.unwrap(),
                    };
                    devices_arc.lock().unwrap().push(device.clone());
                    log::info!("get device: {:?}", device);
                    let _ = app.emit_to(EventTarget::app(), "found_device", device);
                }
                mdns_sd::ServiceEvent::ServiceRemoved(service_type, full_name) => {
                    log::info!("service removed: {service_type} - {full_name}")
                }
                mdns_sd::ServiceEvent::SearchStopped(service_type) => {
                    log::info!("search stopped: {service_type}")
                }
                _ => todo!(),
            }
        }
    });
    Ok(())
}

#[tauri::command]
fn set_current_device(state: State<ManagedState>, device: DiscoverDevice) -> Result<(), String> {
    let mut current_device = state.current_device.lock().unwrap();
    let old_device = current_device.take();
    if old_device.is_some() {
        // TODO: 结束连接
    }
    *current_device = Some(device);
    Ok(())
}

#[tauri::command]
fn get_devices(state: State<ManagedState>) -> Result<Vec<DiscoverDevice>, String> {
    let devices = state.devices.lock().unwrap().clone();
    Ok(devices)
}

#[tauri::command]
async fn get_language() -> Result<String, String> {
    Ok(shared_utils::lang::translate::get_current_language().to_string())
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
        .invoke_handler(tauri::generate_handler![
            start_discover_service,
            get_devices,
            get_language
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
