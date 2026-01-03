use std::sync::{Arc, Mutex};

use anyhow::Result;
use mdns_sd::ResolvedService;
use rand::Rng;
use shared_utils::execute_params;
use tauri::{AppHandle, State, Window};
use tokio::net::TcpStream;
use touchpad_proto::{codec::ProtoStream, proto};
use xxhash_rust::xxh3::xxh3_64;

use crate::{
    emit,
    error::ConnectionError,
    quic::QuicClient,
    state::{DiscoverDevice, ManagedState},
    QUIC_CLIENT,
};

/// 初始化发现的服务设备
fn service_resolve_handler(resolved_service: Box<ResolvedService>) -> Option<DiscoverDevice> {
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
    let backend_port: Option<u16> = resolved_service
        .txt_properties
        .get_property_val_str("backend_port")
        .and_then(|port| port.to_string().parse().ok());

    if let Some(target_name) = target_name {
        log::info!("target name: {}", target_name);
    } else {
        log::warn!("target name not found");
        return None;
    }
    if let Some(ip) = ip {
        log::info!("ip: {}", ip);
    } else {
        log::warn!("ip not found");
        return None;
    }
    if let Some(login_port) = login_port {
        log::info!("login port: {}", login_port);
    } else {
        log::warn!("login port not found");
        return None;
    }
    if let Some(backend_port) = backend_port {
        log::info!("backend port: {}", backend_port);
    } else {
        log::warn!("backend port not found");
        return None;
    }
    let device = DiscoverDevice {
        name: target_name.unwrap().to_string(),
        full_name: resolved_service.fullname,
        address: ip.unwrap().to_ip_addr(),
        login_port: login_port.unwrap(),
        backend_port: backend_port.unwrap(),
    };
    Some(device)
}

#[tauri::command]
pub fn start_discover_service(state: State<ManagedState>) -> Result<(), String> {
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
                    let device = service_resolve_handler(resolved_service);
                    if let Some(device) = device {
                        devices_arc.lock().unwrap().push(device.clone());
                        log::info!("get device: {:?}", device);
                        let _ = emit::found_device(&device);
                    } else {
                        continue;
                    }
                }
                mdns_sd::ServiceEvent::ServiceRemoved(service_type, full_name) => {
                    let hostname = full_name
                        .split(&format!(".{service_type}"))
                        .next()
                        .unwrap_or_default();
                    for device in devices_arc.lock().unwrap().iter_mut() {
                        if device.name == hostname {
                            let _ = emit::device_offline(&full_name);
                            break;
                        }
                    }
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

async fn build_validation(
    window: &Window,
) -> Result<proto::v1::DiscoverValidation, ConnectionError> {
    let monitor = window
        .current_monitor()
        .map_err(|e| ConnectionError::MonitorError(e.to_string()))?
        .ok_or_else(|| ConnectionError::MonitorError("未找到显示器".into()))?;

    let size = monitor.size();

    Ok(proto::v1::DiscoverValidation {
        checksum: xxh3_64(shared_utils::execute_params::hash_seed().as_bytes()),
        send_ts: chrono::Utc::now().timestamp_millis() as u64,
        device_name: tauri_plugin_os::hostname(),
        random_key: rand::rng()
            .sample_iter(rand::distr::Alphanumeric)
            .take(16)
            .map(char::from)
            .collect(),
        width: size.width,
        height: size.height,
    })
}

// 连接处理逻辑分离为独立函数
async fn connect_device(
    device: DiscoverDevice,
    window: Window,
    current_device: Arc<Mutex<Option<DiscoverDevice>>>,
) -> Result<(), ConnectionError> {
    // 构建验证数据
    let validation = build_validation(&window).await?;

    // 建立 TCP 连接用于登录验证 (使用 login_port)
    let login_addr = format!("{}:{}", device.address, device.login_port);
    let stream = TcpStream::connect(&login_addr)
        .await
        .map_err(|e| ConnectionError::NetworkError(e.to_string()))?;
    let mut proto_stream = ProtoStream::from(stream);

    // 发送数据
    proto_stream
        .send_message(&validation)
        .await
        .map_err(|e| ConnectionError::SendError(e.to_string()))?;

    // 接收响应
    let response_data = proto_stream
        .receive_message()
        .await
        .map_err(|e| ConnectionError::ReceiveError(e.to_string()))?;

    // 处理业务逻辑
    match response_data {
        proto::v1::wrapper::Payload::Welcome(welcome) => {
            log::debug!("收到欢迎消息，公钥: {:?}", welcome.cert_der);
            let mut quic_client = QuicClient::new(welcome.cert_der);
            // 使用 backend_port 建立 QUIC 连接
            let backend_addr = format!("{}:{}", device.address, device.backend_port);
            log::info!("准备连接 QUIC 服务器: {}", backend_addr);
            log::info!("设备信息 - 地址: {}, login_port: {}, backend_port: {}",
                device.address, device.login_port, device.backend_port);
            if let Err(e) = quic_client.connect(&backend_addr).await {
                log::error!("QUIC 连接失败: {:?}", e);
                return Err(ConnectionError::TouchServerConnectError(e.to_string()));
            }
            QUIC_CLIENT.set(Arc::new(Mutex::new(quic_client)));
            log::info!("QUIC 连接成功");
            // 发送成功事件到前端
            emit::device_login(&device)?;
            // 使用作用域限制锁的持有时间
            {
                *current_device.lock().unwrap() = Some(device);
            }
        }
        proto::v1::wrapper::Payload::Reject(reject) => {
            return Err(ConnectionError::Rejected(format!(
                "拒绝码: {}",
                reject.reason
            )));
        }
        _ => return Err(ConnectionError::UnexpectedResponse),
    }

    Ok(())
}

// 主命令函数
#[tauri::command]
pub async fn start_connection(
    state: State<'_, ManagedState>,
    device: DiscoverDevice,
    window: Window,
) -> Result<(), String> {
    // 提前释放旧设备
    let old_device = {
        let mut current_device = state.current_device.lock().unwrap();
        current_device.take()
    };

    if let Some(old) = old_device {
        log::info!("断开旧设备: {:?}", old);
        // TODO: 实际断开逻辑
    }

    // 克隆需要的数据
    let current_device = state.current_device.clone();

    // 使用异步运行时管理任务生命周期
    tokio::spawn(async move {
        // 执行连接逻辑
        match connect_device(device, window.clone(), current_device).await {
            Ok(()) => {
                log::info!("设备连接成功");
                // 可选：发送成功事件
                let _ = emit::connect_success();
            }
            Err(e) => {
                log::error!("设备连接失败: {}", e);
                // 关键：将错误通知前端
                let _ = emit::connect_error(&e.to_string());
            }
        }
    });

    Ok(())
}

#[tauri::command]
/// 获取已发现的设备列表
pub fn get_devices(state: State<ManagedState>) -> Result<Vec<DiscoverDevice>, String> {
    let devices = state.devices.lock().unwrap().clone();
    Ok(devices)
}

#[tauri::command]
/// 获取当前语言
pub async fn get_language() -> Result<String, String> {
    Ok(shared_utils::lang::translate::get_current_language().to_string())
}
