use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::LazyLock,
};

use directories_next::ProjectDirs;

pub const SERVER_NAME: &str = "localhost";
pub const LOCALHOST_V4: IpAddr = IpAddr::V4(Ipv4Addr::LOCALHOST);
pub const ANY_V4: IpAddr = IpAddr::V4(Ipv4Addr::UNSPECIFIED); // 0.0.0.0 - 绑定所有网络接口
pub const CLIENT_ADDR: SocketAddr = SocketAddr::new(LOCALHOST_V4, 5000);
pub const SERVER_ADDR: SocketAddr = SocketAddr::new(LOCALHOST_V4, 5001);
pub const SERVER_STOP_CODE: &str = "||SERVER_STOP||";
pub const RECEIVE_SUCCESS: &str = "||RECEIVE_SUCCESS||";
pub const APP_HOME: LazyLock<ProjectDirs> = LazyLock::new(|| {
    directories_next::ProjectDirs::from("org", "fanick", "touchpad_mobile").unwrap()
});
