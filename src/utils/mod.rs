pub mod registry;

use pnet_datalink::interfaces;

pub fn get_mac_by_name(name: &str) -> Option<String> {
    let interfaces = interfaces();

    for iface in &interfaces {
        if iface.name == name {
            return Some(iface.mac?.to_string());
        }
    }

    None
}