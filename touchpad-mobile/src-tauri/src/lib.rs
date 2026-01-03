mod command;
mod emit;
mod error;
mod quic;
mod state;
use std::sync::{Arc, Mutex, OnceLock};

use tauri::AppHandle;

use crate::{
    command::{get_devices, get_language, start_connection, start_discover_service},
    quic::QuicClient,
    state::ManagedState,
};

pub static APP_HANDLE: OnceLock<AppHandle> = OnceLock::new();
pub static QUIC_CLIENT: OnceLock<Arc<Mutex<QuicClient>>> = OnceLock::new();

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
        .setup(|app| {
            // 存储到全局变量
            APP_HANDLE.set(app.handle().clone()).unwrap();
            Ok(())
        })
        .plugin(tauri_plugin_os::init())
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
            start_connection,
            get_devices,
            get_language
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
