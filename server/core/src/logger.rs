use tracing_appender::{
    non_blocking::WorkerGuard,
    rolling::{RollingFileAppender, Rotation},
};
use tracing_subscriber::{
    EnvFilter, Layer, Registry,
    fmt::{self, time::UtcTime},
    layer::SubscriberExt,
    util::SubscriberInitExt,
};

pub fn init_tracing() -> WorkerGuard {
    // 滚动文件 appender (按小时切割，目录logs/, 前最touchpad.log)
    let file_appender = RollingFileAppender::new(Rotation::HOURLY, "logs/", "touchpad.log");
    // 非阻塞写入
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
    // 文件层
    let file_layer = fmt::layer()
        .with_ansi(false)
        .with_timer(UtcTime::rfc_3339())
        .with_line_number(true)
        .with_writer(non_blocking)
        .with_filter(EnvFilter::new("info"));

    let console_layer = fmt::layer()
        .with_ansi(false)
        .with_timer(UtcTime::rfc_3339())
        .with_filter(EnvFilter::new("info"));

    // 5. 合并两层并注册
    Registry::default()
        .with(console_layer)
        .with(file_layer)
        .init();

    guard
}
