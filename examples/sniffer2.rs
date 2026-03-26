use pnet_datalink::interfaces;

fn get_mac_by_name(name: &str) -> Option<String> {
    let interfaces = interfaces();

    for iface in &interfaces {
        if iface.name == name {
            return Some(iface.mac?.to_string());
        }
    }

    None
}


fn main() {
    // 通过接口名获取
    if let Some(mac) = get_mac_by_name("en0") {
        println!("eth0 MAC: {}", mac);
    }

    // 列出所有接口
    // for iface in interfaces() {
    //     println!("{}: {:?} - {:?}",
    //              iface.name,
    //              iface.mac,
    //              iface.ips.iter().map(|ip| ip.ip()).collect::<Vec<_>>()
    //     );
    // }
}