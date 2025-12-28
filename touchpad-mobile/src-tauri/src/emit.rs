use tauri::Emitter;

use crate::{error::ConnectionError, state::DiscoverDevice, APP_HANDLE};

/// 设备登录验证
pub fn device_login(device: &DiscoverDevice) -> Result<(), ConnectionError> {
    let app = APP_HANDLE.get().unwrap();
    app.emit("device-login", device)
        .map_err(|e| ConnectionError::ProtocolError(e.to_string()))?;
    Ok(())
}

/// 设备连接成功
pub fn connect_success() -> Result<(), tauri::Error> {
    let app = APP_HANDLE.get().unwrap();
    app.emit("connection-success", ())?;
    Ok(())
}

/// 设备连接失败
pub fn connect_error(reason: &str) -> Result<(), tauri::Error> {
    let app = APP_HANDLE.get().unwrap();
    app.emit("connection-error", reason)?;
    Ok(())
}

/// 设备发现
pub fn found_device(device: &DiscoverDevice) -> Result<(), tauri::Error> {
    let app = APP_HANDLE.get().unwrap();
    app.emit("found-device", device)?;
    log::info!("found-device emited");
    Ok(())
}

/// 设备离线
pub fn device_offline(full_name: &str) -> Result<(), tauri::Error> {
    let app = APP_HANDLE.get().unwrap();
    app.emit("device-offline", full_name)?;
    log::info!("device-offline emited");
    Ok(())
}
