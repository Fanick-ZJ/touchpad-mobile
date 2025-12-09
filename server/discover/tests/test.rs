use std::{
    net::{IpAddr, Ipv4Addr},
    str::FromStr,
};

use libmdns::Responder;
use tokio::io::AsyncReadExt;

#[tokio::test]
async fn mdns_test() -> Result<(), Box<dyn std::error::Error>> {
    // 1. 起 mDNS  responder
    // 确定为本地的ipv4
    let responder =
        Responder::new_with_ip_list(vec![IpAddr::V4(Ipv4Addr::from_str("192.168.1.6")?)])?;

    // 2. 与mdns注册端口一致，客户端确认回信时，会从这边过
    tokio::spawn(async {
        let listener = tokio::net::TcpListener::bind("0.0.0.0:5412").await.unwrap();
        println!("server on :5412");
        let (mut stream, addr) = listener.accept().await.unwrap();
        let mut buff = [0; 1024];
        let n = stream.read(&mut buff).await.unwrap();
        println!("Address {} Received {} bytes", addr, n);
    });

    // 3. 注册服务：类型 _http._tcp，实例名 my-pc
    //    第二个参数是端口，第三个是 TXT 记录（可空）
    let _svc = responder.register_with_ttl(
        "_touchpad._tcp".into(), // 服务类型
        "my-pc".into(),          // 实例名（别人看到的名字）
        5412,                    // 本地端口
        &["path=/"],             // TXT 键值对，可选
        10,
    );

    println!("mDNS 服务已注册：my-pc._touchpad._tcp.local.  5412");
    println!("按 Ctrl-C 退出");
    tokio::signal::ctrl_c().await?;
    Ok(())
}
