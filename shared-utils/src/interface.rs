use std::net::IpAddr;

use crate::interface;

#[cfg(not(windows))]
/// 获取不支持mdns的网卡列表
pub fn enumerate_mdns_incapable_interfaces() -> Vec<String> {
    use pnet::datalink;
    let interfaces = datalink::interfaces();
    interfaces
        .iter()
        .filter_map(|interface| {
            // Skip loopback outright
            if interface.is_loopback() {
                return None;
            }
            let incapable = interface.ips.is_empty()    // 没有设置ip
                || !interface.is_running()              // 网卡不在运行中
                || !interface.is_multicast()            // 网卡不支持多播
                || !interface.is_broadcast(); // 网卡不支持广播
            if incapable {
                Some(interface.name.clone())
            } else {
                None
            }
        })
        .collect()
}

#[cfg(windows)]
pub fn enumerate_mdns_incapable_interfaces() -> Vec<String> {
    use ipconfig::{IfType, OperStatus};

    if let Ok(adapters) = ipconfig::get_adapters() {
        adapters
            .iter()
            .filter_map(|adapter| {
                if adapter.if_type() == IfType::SoftwareLoopback {
                    // Skip pseudo loopback interface
                    return None;
                }
                if adapter.ip_addresses().is_empty()
                    || adapter.oper_status() != OperStatus::IfOperStatusUp
                    || (adapter.if_type() != IfType::EthernetCsmacd
                        && adapter.if_type() != IfType::Ieee80211)
                {
                    Some(adapter.friendly_name().to_string())
                } else {
                    None
                }
            })
            .collect()
    } else {
        vec![]
    }
}

#[cfg(not(windows))]
pub fn enumerate_mdns_capable_interfaces() -> Vec<String> {
    use pnet::datalink;
    let interfaces = datalink::interfaces();
    interfaces
        .iter()
        .filter_map(|interface| {
            // Skip loopback outright
            if interface.is_loopback() {
                return None;
            }
            let incapable = !interface.ips.is_empty()    // 必须设置ip
                && interface.is_running()              // 网卡在运行中
                && interface.is_multicast()            // 网卡支持多播
                && interface.is_broadcast(); // 网卡支持广播
            if incapable {
                Some(interface.name.clone())
            } else {
                None
            }
        })
        .collect()
}

#[cfg(windows)]
pub fn enumerate_mdns_capable_interfaces() -> Vec<String> {
    use ipconfig::{IfType, OperStatus};

    if let Ok(adapters) = ipconfig::get_adapters() {
        adapters
            .iter()
            .filter_map(|adapter| {
                if adapter.if_type() == IfType::SoftwareLoopback {
                    // Skip pseudo loopback interface
                    return None;
                }
                if !adapter.ip_addresses().is_empty()
                    && adapter.oper_status() == OperStatus::IfOperStatusUp
                    && (adapter.if_type() == IfType::EthernetCsmacd
                        && adapter.if_type() == IfType::Ieee80211)
                {
                    Some(adapter.friendly_name().to_string())
                } else {
                    None
                }
            })
            .collect()
    } else {
        vec![]
    }
}

/// 根据网卡名称获取其ip地址
pub fn get_ip_by_name(name: &str, prefer_ipv4: bool) -> Option<IpAddr> {
    use pnet::datalink;
    datalink::interfaces()
        .iter()
        .find(|iface| iface.name == name && iface.is_up() && !iface.is_loopback())
        .and_then(|iface| {
            let ips = &iface.ips;
            if prefer_ipv4 {
                // 优先返回IPv4
                ips.iter().find(|ip| ip.is_ipv4()).map(|ip| ip.ip())
            } else {
                // 优先返回IPv6
                ips.iter().find(|ip| ip.is_ipv6()).map(|ip| ip.ip())
            }
        })
}

#[cfg(test)]
mod test {
    use crate::interface::{
        enumerate_mdns_capable_interfaces, enumerate_mdns_incapable_interfaces, get_ip_by_name,
    };
    #[test]
    fn test_enumerate_mdns_incapable_interfaces() {
        let list = enumerate_mdns_incapable_interfaces();
        println!("{:?}", list);
    }

    #[test]
    fn test_get_ip() {
        let inter_names = enumerate_mdns_capable_interfaces();
        for name in inter_names {
            let ip = get_ip_by_name(&name, true);
            println!("interface: {} ip: {:?}", name, ip);
        }
    }
}
