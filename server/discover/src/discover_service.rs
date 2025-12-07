use crate::device::Device;
use anyhow::{Result, anyhow};
use core_kit::codec::{dewrap, varint, wrap};
use libmdns::{Responder, Service};
use std::{
    collections::HashMap,
    net::{IpAddr, SocketAddr},
    sync::Arc,
};
use tokio::{
    io::AsyncWriteExt,
    net::{TcpListener, TcpStream},
    sync::{
        Mutex,
        oneshot::{self},
    },
};
use touchpad_proto::proto::v1::{DiscoverValidation, ErrorCode, Reject, Welcome, wrapper::Payload};
use tracing::{debug, error, info, warn};
use utils::{env, sys::get_comptuer_name, token};
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
        info!("服务端使用SEED: '{}'", self.checksum_seed);
        let seed_checksum = xxh3_64(self.checksum_seed.as_bytes());

        info!("服务端计算的校验核: {}", seed_checksum);
        info!(
            "接受到的校验核: {}, 目标校验核:{}",
            dv.checksum, seed_checksum
        );
        if dv.checksum == seed_checksum {
            let listening_device = self.listening_device.lock().await;
            if let Ok(peer_addr) = socket.peer_addr() {
                if listening_device.contains_key(&peer_addr.ip()) {
                    let reject = Reject {
                        reason: ErrorCode::RepeatedlyAddingDevices as i32,
                    };
                    let _ = socket.write(&wrap(&reject)?);
                    warn!("重复添加设备被拒绝: {}", peer_addr.ip());
                    return Err(anyhow!("Repeatedly adding devices"));
                }

                let token =
                    token::get_first_token(&peer_addr.ip(), &dv.random_key, &dv.device_name)?;
                let device = Device {
                    name: dv.device_name,
                    ip: peer_addr.ip(),
                    width: dv.width,
                    height: dv.height,
                };

                let now = chrono::Utc::now().timestamp();
                let welcome = Welcome {
                    token,
                    ts_ms: now as u64,
                };

                let response_with_prefix = varint::encode_with_length_prefix(&wrap(&welcome)?);
                let _ = socket.write(&response_with_prefix).await;
                Ok(device)
            } else {
                return Err(anyhow!("Failed to get peer address"));
            }
        } else {
            // 校验核不通过
            let reject = Reject {
                reason: ErrorCode::HelloCheckSumMismatch as i32,
            };
            let response_with_prefix = varint::encode_with_length_prefix(&wrap(&reject)?);
            let _ = socket.write(&response_with_prefix).await;
            info!(
                "🚫 已向客户端发送拒绝消息，长度: {} 字节",
                response_with_prefix.len()
            );
            return Err(anyhow!("Failed to handle client connection"));
        }
    }

    async fn handle_client_connection(
        &self,
        mut socket: TcpStream,
        addr: SocketAddr,
    ) -> Result<Device> {
        let message_bytes = match varint::read_message_with_length_prefix(&mut socket).await {
            Ok(bytes) => bytes,
            Err(e) => {
                error!("读取消息失败 {}: {}", addr, e);
                return Err(e);
            }
        };

        if let Ok(payload) = dewrap(&message_bytes) {
            // TODO: 解析校验码并返回设备信息
            match payload {
                Payload::DiscoverValidation(dv) => {
                    // 校验验证核
                    let device = self.discover_validation_handler(dv, &mut socket).await?;
                    info!("验证设备成功: {}", device.name);
                    return Ok(device);
                }
                _ => {
                    warn!("收到未知消息类型");
                    return Err(anyhow!("Received unknown payload"));
                }
            }
        } else {
            error!("解析消息数据失败");
            return Err(anyhow!("Failed to handle client connection"));
        }
    }

    /// 开启应答服务器
    pub async fn start_confirm_server(self: Arc<Self>) -> Result<()> {
        // 允许添加多个设备，调用stop函数手动停止
        let (tx, mut rx) = oneshot::channel::<()>();
        let listener = TcpListener::bind(format!("0.0.0.0:{}", self.discover_port)).await?;
        self.stop_signal.lock().await.replace(tx);
        info!("发现服务启动，端口: {}", self.discover_port);
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

    pub async fn stop(&self) -> Result<()> {
        if let Some(stop_signal) = self.stop_signal.lock().await.take() {
            let _ = stop_signal.send(());
        }
        let _ = self.mdns_service.lock().await.take();
        info!("发现服务已停止");
        Ok(())
    }

    pub async fn discover(self: Arc<Self>) -> Result<()> {
        if let Some(_) = self.mdns_service.lock().await.take() {
            return Err(anyhow!("The discover service is started!"));
        }
        let responder = if let Some(ip_list) = &self.ip_list {
            debug!("广播IP列表: {:?}", ip_list);
            Responder::new_with_ip_list(ip_list.clone())?
        } else {
            Responder::new()
        };
        let svc_type = env::get_env("MDNS_SD_META_SERVICE")
            .ok_or_else(|| anyhow!("获取服务名称环境变量失败"))?;
        info!("MDNS服务名称：{svc_type:?}");
        let server = responder.register_with_ttl(
            svc_type.into(),
            &get_comptuer_name(),
            self.discover_port,
            &[&format!("discover_port={}", self.discover_port)],
            self.ttl,
        );
        self.mdns_service.lock().await.replace(server);
        let service_clone = self.clone();
        tokio::spawn(async move {
            if let Err(e) = service_clone.start_confirm_server().await {
                error!("启动确认服务器失败: {:?}", e);
            }
        });
        Ok(())
    }
}
