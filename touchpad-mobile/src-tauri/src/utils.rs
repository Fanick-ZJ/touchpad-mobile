use mdns_sd::IfKind;

#[cfg(not(windows))]
/// 获取不支持mdns的网卡列表
pub fn enumerate_mdns_incapable_interfaces() -> Vec<IfKind> {
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
                Some(IfKind::from(interface.name.as_str()))
            } else {
                None
            }
        })
        .collect()
}

#[cfg(windows)]
pub fn enumerate_mdns_incapable_interfaces() -> Vec<IfKind> {
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
                    Some(IfKind::from(adapter.friendly_name()))
                } else {
                    None
                }
            })
            .collect()
    } else {
        vec![]
    }
}

#[cfg(test)]
mod test {
    use crate::utils::enumerate_mdns_incapable_interfaces;
    #[test]
    fn test_enumerate_mdns_incapable_interfaces() {
        let list = enumerate_mdns_incapable_interfaces();
        println!("{:?}", list);
    }
}
