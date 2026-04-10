use std::panic;
use crate::models::AppState;
use pnet::datalink::{self, Channel::Ethernet};
use pnet::packet::ethernet::{EtherTypes, EthernetPacket};
use pnet::packet::ipv4::Ipv4Packet;
use pnet::packet::tcp::TcpPacket;
use pnet::packet::udp::UdpPacket;
use pnet::packet::Packet;
use std::sync::Arc;
use tracing::{debug, error, info, trace};
use sniffer::packet::{parse_tcp, parse_udp};
use sniffer::utils::registry::Registry;

pub struct CaptureEngine {
    tx: tokio::sync::mpsc::Sender<PacketInfo>,
}

#[derive(Clone, Debug)]
pub struct PacketInfo {
    pub src_ip: String,
    pub src_port: u16,
    pub dst_ip: String,
    pub dst_port: u16,
    pub protocol: String,
    pub length: u32,
    pub is_outgoing: bool, // 相对于本机
}

impl CaptureEngine {
    pub fn new(tx: tokio::sync::mpsc::Sender<PacketInfo>) -> Self {
        Self { tx }
    }

    pub fn start(&self, interface_name: String, state: Arc<AppState>) {
        let interfaces = datalink::interfaces();
        let interface = interfaces
            .into_iter()
            .find(|i| i.name == interface_name)
            .expect("Interface not found");

        let (_, mut rx) = match datalink::channel(&interface, Default::default()) {
            Ok(Ethernet(_, rx)) => ((), rx),
            Ok(_) => panic!("Unhandled channel type"),
            Err(e) => panic!("Failed to create channel: {}", e),
        };

        info!("Capture started on {}", interface_name);

        while *state.running.blocking_read() {
            match rx.next() {
                Ok(packet) => {
                    let result = panic::catch_unwind(|| {
                        if let Some(info) = parse_packet(packet) {
                            trace!("info: {:?}", info);
                            let _ = self.tx.try_send(info).unwrap(); // 发送到聚合器
                        }
                    });
                    if result.is_err() {
                        error!("packet parse error: {:?}", result);
                    }
                }
                Err(e) => eprintln!("Capture error: {}", e),
            }
        }
    }
}

fn parse_packet(packet: &[u8]) -> Option<PacketInfo> {
    let eth = EthernetPacket::new(packet)?;

    // 解析 IP 层
    if let Some(ip) = Ipv4Packet::new(eth.payload()) {
        trace!("IPv4: {} -> {}", ip.get_source(), ip.get_destination());

        // 解析 TCP 层
        parse_tcp(&ip, &eth);
        // 解析 UDP 层
        parse_udp(&ip, &eth);
    }

    match eth.get_ethertype() {
        EtherTypes::Ipv4 => {
            let ipv4 = Ipv4Packet::new(eth.payload())?;
            let src_ip = ipv4.get_source().to_string();
            let dst_ip = ipv4.get_destination().to_string();

            let mac = Registry::get::<String>("mac").unwrap();
            let is_outgoing = eth.get_source().to_string() == mac;

            match ipv4.get_next_level_protocol() {
                pnet::packet::ip::IpNextHeaderProtocols::Tcp => {
                    let tcp = TcpPacket::new(ipv4.payload())?;
                    let length = tcp.payload().len() as u32;
                    debug!("tcp payload len: {}", length);
                    if length == 0 {
                        return None
                    }
                    Some(PacketInfo {
                        src_ip,
                        src_port: tcp.get_source(),
                        dst_ip,
                        dst_port: tcp.get_destination(),
                        protocol: "TCP".to_string(),
                        length: length,
                        is_outgoing,
                    })
                }
                pnet::packet::ip::IpNextHeaderProtocols::Udp => {
                    let udp = UdpPacket::new(ipv4.payload())?;
                    let length = udp.payload().len() as u32;
                    debug!("udp payload: {}", length);
                    if length == 0 {
                        return None;
                    }
                    Some(PacketInfo {
                        src_ip,
                        src_port: udp.get_source(),
                        dst_ip,
                        dst_port: udp.get_destination(),
                        protocol: "UDP".to_string(),
                        length: length,
                        is_outgoing,
                    })
                }
                _ => None,
            }
        }
        _ => None,
    }
}
