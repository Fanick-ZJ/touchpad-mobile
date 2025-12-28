#[derive(Debug, thiserror::Error)]
pub enum ConnectionError {
    #[error("获取显示器信息失败: {0}")]
    MonitorError(String),
    #[error("网络连接失败: {0}")]
    NetworkError(String),
    #[error("数据发送失败: {0}")]
    SendError(String),
    #[error("数据接收失败: {0}")]
    ReceiveError(String),
    #[error("协议解析失败: {0}")]
    ProtocolError(String),
    #[error("设备拒绝连接: {0}")]
    Rejected(String),
    #[error("意外响应类型")]
    UnexpectedResponse,
}
