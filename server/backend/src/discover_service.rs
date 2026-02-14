use anyhow::{Result, anyhow};
use mdns_sd::{ServiceDaemon, ServiceInfo};
use server_core_kit::device::Device;
use server_utils::sys::get_computer_name;
use shared_utils::execute_params;
use std::{
    collections::HashMap,
    net::{IpAddr, SocketAddr},
    process::Command,
    sync::Arc,
    time::Duration,
};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::{
        Mutex,
        oneshot::{self},
    },
};
use touchpad_proto::codec::ProtoStream;
use touchpad_proto::proto::v1::{DiscoverValidation, ErrorCode, Reject, Welcome, wrapper::Payload};
use tracing::{debug, error, info, warn};

use xxhash_rust::xxh3::xxh3_64;

type DeviceMap = Arc<Mutex<HashMap<IpAddr, Device>>>;
pub struct DiscoverService {
    /// 发现服务验证登录的端口
    login_port: u16,
    /// 发现服务的端口
    discover_port: u16,
    /// 后端服务的端口
    backend_port: u16,
    /// 用于启动mdns服务的IP
    ip: IpAddr,
    /// 校验使用的字段
    checksum_seed: String,
    /// 准备接受连接的设备
    listening_device: DeviceMap,
    /// 停止信号
    stop_signal: Arc<Mutex<Option<oneshot::Sender<()>>>>,
    /// 守护进程
    mdns_daemon: Arc<Mutex<Option<ServiceDaemon>>>,
    /// 发现服务处理句柄
    discover_handler: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
    /// 发现设后后的回调函数
    discover_callback: Option<Box<dyn Fn(&Device, Vec<&Device>) + Send + Sync>>,
    /// 登录验证成功后的公钥
    login_public_key: Vec<u8>,
}

/// 具体的发现步骤
/// 1. 根据confirm_port端口服务作为应答服务
/// 2. 启动mdns服务，在TXT中传递校验码
/// 3. 当应答服务中接受到相应格式的校验码时（protobuf格式），进行解析
/// 3.1 如果校验正确，则返回具体设备信息格式(Device)
/// 3.2 如果校验错误，记录错误日志，继续等待连接

impl<'d> DiscoverService {
    pub fn new(
        login_port: u16,
        discover_port: u16,
        backend_port: u16,
        checksum_seed: String,
        ip: IpAddr,
        login_public_key: Vec<u8>,
        discover_callback: Option<Box<dyn Fn(&Device, Vec<&Device>) + Send + Sync>>,
    ) -> Self {
        DiscoverService {
            login_port,
            backend_port,
            discover_port,
            ip,
            checksum_seed,
            listening_device: Arc::new(Mutex::new(HashMap::new())),
            stop_signal: Arc::new(Mutex::new(None)),
            mdns_daemon: Arc::new(Mutex::new(None)),
            discover_handler: Arc::new(Mutex::new(None)),
            discover_callback,
            login_public_key,
        }
    }

    /// 处理发现验证请求
    async fn discover_validation_handler(
        &self,
        dv: DiscoverValidation,
        addr: &SocketAddr,
        stream: &mut ProtoStream,
    ) -> Result<Device> {
        info!("服务端使用SEED: '{}'", self.checksum_seed);
        let seed_checksum = xxh3_64(self.checksum_seed.as_bytes());

        info!("服务端计算的校验核: {}", seed_checksum);
        info!("接受到的校验核: {}, 目标校验核:{}", dv.checksum, seed_checksum);
        if dv.checksum == seed_checksum {
            let listening_device = self.listening_device.lock().await;
            if listening_device.contains_key(&addr.ip()) {
                let reject = Reject {
                    reason: ErrorCode::RepeatedlyAddingDevices as i32,
                };
                let _ = stream.send_message(&reject);
                warn!("重复添加设备被拒绝: {}", addr.ip());
                return Err(anyhow!("Repeatedly adding devices"));
            }

            let device = Device {
                name: dv.device_name,
                ip: addr.ip(),
                width: dv.width,
                height: dv.height,
            };

            let now = chrono::Utc::now().timestamp();
            let welcome = Welcome {
                cert_der: self.login_public_key.clone(),
                ts_ms: now as u64,
            };

            let _ = stream.send_message(&welcome).await;
            Ok(device)
        } else {
            // 校验核不通过
            let reject = Reject {
                reason: ErrorCode::HelloCheckSumMismatch as i32,
            };
            let _ = stream.send_message(&reject).await;
            info!("🚫 已向客户端发送拒绝消息");
            return Err(anyhow!("Failed to handle client connection"));
        }
    }

    async fn handle_client_connection(
        &self,
        socket: TcpStream,
        addr: SocketAddr,
    ) -> Result<Device> {
        let mut proto_stream = ProtoStream::from(socket);
        let payload = match proto_stream.receive_message().await {
            Ok(bytes) => bytes,
            Err(e) => {
                error!("读取消息失败 {}: {}", addr, e);
                return Err(e);
            },
        };
        match payload {
            Payload::DiscoverValidation(dv) => {
                // 校验验证核
                if let Ok(device) = self
                    .discover_validation_handler(dv, &addr, &mut proto_stream)
                    .await
                {
                    info!("验证设备成功: {}", device.name);
                    return Ok(device);
                } else {
                    let reject = Reject { reason: 1 };
                    let _ = proto_stream.send_message(&reject).await;
                    info!("🚫 已向客户端发送拒绝消息");
                    return Err(anyhow!("Failed to handle client connection"));
                }
            },
            _ => {
                warn!("收到未知消息类型");
                return Err(anyhow!("Received unknown payload"));
            },
        }
    }

    /// 开启应答服务器
    pub async fn start_confirm_server(self: Arc<Self>) -> Result<()> {
        // 允许添加多个设备，调用stop函数手动停止
        let (tx, mut rx) = oneshot::channel::<()>();
        let listener = TcpListener::bind(format!("0.0.0.0:{}", self.login_port)).await?;
        self.stop_signal.lock().await.replace(tx);
        info!("发现服务启动，端口: {}", self.login_port);
        loop {
            tokio::select! {
                res = listener.accept() => {
                    let (socket, addr) = res?;
                    let service = self.clone();
                    tokio::spawn(async move {
                        if let  Ok(dev) = service.handle_client_connection(socket, addr).await {
                            debug!("接受连接: {}", addr);
                            let mut devices = service.listening_device.lock().await;
                            devices.insert(addr.ip(), dev);
                            if let Some(callback) = &service.discover_callback {
                                callback(
                                    devices.get(&addr.ip()).unwrap(),
                                    devices
                                        .values()
                                        .collect::<Vec<&Device>>(),
                                );
                            }
                        }
                    });
                },
                _ = &mut rx => {
                    self.stop_signal.lock().await.take();
                    info!("发现服务停止");
                    break;
                }
            }
        }
        Ok(())
    }

    pub async fn close(&self) -> Result<()> {
        // 1. 先发送停止信号，减少锁作用域
        if let Some(stop_signal) = self.stop_signal.lock().await.take() {
            if let Err(_) = stop_signal.send(()) {
                warn!("MDNS停止信号接收端已关闭");
            }
        }

        // 2. 获取daemon并立即释放锁
        let daemon_opt = { self.mdns_daemon.lock().await.take() };

        if let Some(daemon) = daemon_opt {
            // 3. 使用循环而非递归，限制重试次数
            const MAX_RETRIES: u32 = 5;
            let mut retries = 0;

            loop {
                match daemon.shutdown() {
                    Ok(_) => {
                        info!("MDNS守护进程已成功停止");
                        break;
                    },
                    Err(mdns_sd::Error::Again) if retries < MAX_RETRIES => {
                        retries += 1;
                        warn!("MDNS守护进程繁忙，重试停止 ({}/{})", retries, MAX_RETRIES);
                        tokio::time::sleep(Duration::from_millis(100)).await;
                        // continue 循环重试
                    },
                    Err(e) => {
                        error!("MDNS守护进程停止失败：{}", e);
                        return Err(e.into()); // 转换为通用Error
                    },
                }
            }
        } else {
            info!("MDNS守护进程未运行或已停止");
        }

        Ok(())
    }

    pub async fn discover(self: &Arc<Self>) -> Result<()> {
        if self.mdns_daemon.lock().await.is_some() {
            return Err(anyhow!("The discover service is started!"));
        }
        let svc_type = execute_params::mdns_server_type();
        info!("MDNS服务名称：{svc_type:?}");

        let mdns_daemon = ServiceDaemon::new().expect("Failed to create daemon");
        info!("MDNS守护进程启动");
        let host_name = self.ip.to_string() + ".local.";
        let properties = vec![
            ("login_port", self.login_port.to_string()),
            ("backend_port", self.backend_port.to_string()),
        ];
        let service = ServiceInfo::new(
            svc_type,
            &get_computer_name(),
            &host_name,
            self.ip,
            self.discover_port,
            &properties[..],
        )?;
        mdns_daemon
            .register(service)
            .expect("Failed to register our service");
        self.mdns_daemon.lock().await.replace(mdns_daemon);
        let service_clone = self.clone();
        let handler = tokio::spawn(async move {
            if let Err(e) = service_clone.start_confirm_server().await {
                error!("启动确认服务器失败: {:?}", e);
            }
        });
        self.discover_handler.lock().await.replace(handler);
        Ok(())
    }

    pub fn listening_derive(self: &Arc<Self>) -> DeviceMap {
        return self.listening_device.clone();
    }
}
