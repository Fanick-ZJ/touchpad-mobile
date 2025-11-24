use crate::device::Device;
use anyhow::{Result, anyhow};
use core_kit::codec::{dewrap, wrap};
use libmdns::{Responder, Service};
use std::{
    collections::HashMap,
    net::{IpAddr, SocketAddr},
    sync::Arc,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::{
        Mutex,
        oneshot::{self},
    },
};
use touchpad_proto::proto::v1::{DiscoverValidation, ErrorCode, Reject, Welcome, wrapper::Payload};
use tracing::{error, info};
use utils::{sys::get_comptuer_name, token};
use xxhash_rust::xxh3::xxh3_64;

pub struct DiscoverService {
    ttl: u32,
    // mdns服务的端口
    discover_port: u16,
    // 用于启动mdns服务的IP列表
    ip_list: Option<Vec<IpAddr>>,
    // 校验使用的字段
    checksum_seed: String,
    // 准备接受连接的设备
    listening_device: Arc<Mutex<HashMap<IpAddr, Device>>>,
    stop_signal: Arc<Mutex<Option<oneshot::Sender<()>>>>,
    mdns_service: Arc<Mutex<Option<Service>>>,
    discover_callback: Option<Box<dyn Fn(&Device, Vec<&Device>) + Send + Sync>>,
}

/// 具体的发现步骤
/// 1. 根据confirm_port端口服务作为应答服务
/// 2. 启动mdns服务，在TXT中传递校验码
/// 3. 当应答服务中接受到相应格式的校验码时（protobuf格式），进行解析
/// 3.1 如果校验正确，则返回具体设备信息格式(Device)
/// 3.2 如果校验错误，记录错误日志，继续等待连接

impl<'d> DiscoverService {
    pub fn new(
        ttl: u32,
        discover_port: u16,
        checksum_seed: String,
        ip_list: Option<Vec<IpAddr>>,
        discover_callback: Option<Box<dyn Fn(&Device, Vec<&Device>) + Send + Sync>>,
    ) -> Self {
        DiscoverService {
            ttl,
            discover_port,
            ip_list,
            checksum_seed,
            listening_device: Arc::new(Mutex::new(HashMap::new())),
            stop_signal: Arc::new(Mutex::new(None)),
            mdns_service: Arc::new(Mutex::new(None)),
            discover_callback,
        }
    }

    /// 处理发现验证请求
    async fn discover_validation_handler(
        &self,
        dv: DiscoverValidation,
        socket: &mut TcpStream,
    ) -> Result<Device> {
        let seed_checksum = xxh3_64(self.checksum_seed.as_bytes());
        // 校验核通过
        if dv.checksum == seed_checksum {
            let listening_device = self.listening_device.lock().await;
            if let Ok(peer_addr) = socket.peer_addr() {
                // 重复添加设备拒绝
                if listening_device.contains_key(&peer_addr.ip()) {
                    let reject = Reject {
                        reason: ErrorCode::RepeatedlyAddingDevices as i32,
                    };
                    let _ = socket.write(&wrap(&reject)?);
                    return Err(anyhow!("Repeatedly adding devices"));
                }
                // 计算初始token
                let token =
                    token::get_first_token(&peer_addr.ip(), &dv.random_key, &dv.device_name)?;
                let device = Device {
                    name: dv.device_name,
                    ip: peer_addr.ip(),
                    width: dv.width,
                    height: dv.height,
                };
                // 添加ip对应的设备
                let now = chrono::Utc::now().timestamp();
                let welcome = Welcome {
                    token,
                    ts_ms: now as u64,
                };
                // 发送welcome
                let _ = socket.write(&wrap(&welcome)?);
                Ok(device)
            } else {
                return Err(anyhow!("Failed to get peer address"));
            }
        } else {
            // 校验核不通过
            let reject = Reject {
                reason: ErrorCode::HelloCheckSumMismatch as i32,
            };
            let _ = socket.write(&wrap(&reject)?);
            return Err(anyhow!("Failed to handle client connection"));
        }
    }

    async fn handle_client_connection(
        &self,
        mut socket: TcpStream,
        addr: SocketAddr,
    ) -> Result<Device> {
        let mut buf = vec![0; 1024];
        let mut bytes = vec![0; 1024];
        loop {
            let n = socket.read(&mut buf).await?;
            if n == 0 {
                break;
            }
            bytes.extend_from_slice(&buf[..n]);
        }
        if let Ok(payload) = dewrap(&bytes) {
            // TODO: 解析校验码并返回设备信息
            match payload {
                Payload::DiscoverValidation(dv) => {
                    // 校验验证核
                    let device = self.discover_validation_handler(dv, &mut socket).await?;
                    return Ok(device);
                }
                _ => {
                    info!("Received unknown payload");
                    return Err(anyhow!("Received unknown payload"));
                }
            }
        } else {
            error!("Failed to dewrapper data: {:?}", &bytes);
            return Err(anyhow!("Failed to handle client connection"));
        }
    }

    /// 开启应答服务器
    pub async fn start_confirm_server(self: Arc<Self>) -> Result<()> {
        // 允许添加多个设备，调用stop函数手动停止
        let (tx, mut rx) = oneshot::channel::<()>();
        let listener = TcpListener::bind(format!("0.0.0.0:{}", self.discover_port)).await?;
        self.stop_signal.lock().await.replace(tx);
        info!("Discover service started on port {}", self.discover_port);
        loop {
            tokio::select! {
                res = listener.accept() => {
                    let (socket, addr) = res?;
                    let service = self.clone();
                    tokio::spawn(async move {
                        if let  Ok(dev) = service.handle_client_connection(socket, addr).await {
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
                    info!("Discover service stopped");
                    break;
                }
            }
        }
        Ok(())
    }

    pub async fn stop(&self) -> Result<()> {
        if let Some(stop_signal) = self.stop_signal.lock().await.take() {
            if let Err(e) = stop_signal.send(()) {
                error!("Failed to send stop signal: {:?}", e);
            }
        }
        if let None = self.mdns_service.lock().await.take() {
            error!("Failed to stop mDNS service, the service is None");
        }
        info!("Discover service stopped");
        Ok(())
    }

    pub async fn discover(self: Arc<Self>) -> Result<()> {
        if let Some(_) = self.mdns_service.lock().await.take() {
            return Err(anyhow!("The discover service is started!"));
        }
        let responder = if let Some(ip_list) = &self.ip_list {
            info!("brocast ip list: {:?}", ip_list);
            Responder::new_with_ip_list(ip_list.clone())?
        } else {
            info!("brocast in auto mode");
            Responder::new()
        };
        let server = responder.register_with_ttl(
            "_touchpad._tcp".into(),
            &get_comptuer_name(),
            self.discover_port,
            &[&format!("discover_port={}", self.discover_port)],
            self.ttl,
        );
        self.mdns_service.lock().await.replace(server);
        // 启动后台进程
        tokio::spawn(async move {
            if let Err(e) = self.start_confirm_server().await {
                error!("Failed to start confirm server: {:?}", e);
            }
        });
        Ok(())
    }
}
