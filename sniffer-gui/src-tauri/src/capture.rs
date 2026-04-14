use crate::models::AppState;
use pcap::{Capture, Device, PacketHeader};
use pnet::packet::ethernet::{EtherTypes, EthernetPacket};
use pnet::packet::ipv4::Ipv4Packet;
use pnet::packet::tcp::TcpPacket;
use pnet::packet::udp::UdpPacket;
use pnet::packet::Packet;
use sniffer::packet::{parse_tcp, parse_udp};
use sniffer::utils::get_mac_by_name;
use sniffer::utils::registry::Registry;
use std::sync::Arc;
use std::{net, panic};
use tracing::{debug, error, info, trace};

#[derive(Debug)]
pub struct CaptureEngine {
    tx: tokio::sync::mpsc::Sender<PacketInfo>,
    pcap_tx: async_channel::Sender<(PacketHeader, Vec<u8>)>,
    device: Device,
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
    pub fn new(
        tx: tokio::sync::mpsc::Sender<PacketInfo>,
        pcap_tx: async_channel::Sender<(PacketHeader, Vec<u8>)>,
    ) -> Self {
        let device_list = Device::list().expect("device list failed");
        // 查找默认设备
        let device = device_list
            .iter()
            .filter(|d| {
                d.flags.connection_status == pcap::ConnectionStatus::Connected
                    && d.addresses.len() > 0
            })
            .next()
            .expect("no device available");
        info!("Using device: {}", device.name);

        let ip = device
            .addresses
            .iter()
            .filter(|address| matches!(address.addr, net::IpAddr::V4(_)))
            .next()
            .unwrap()
            .addr
            .to_string();
        info!("ip: {}", ip);
        Registry::set("ip", ip.clone(), Some(0u64));

        let mac = get_mac_by_name(device.name.as_str()).unwrap();
        info!("Using device mac {}", mac);
        Registry::set("mac", mac.clone(), Some(0u64));

        Self {
            tx,
            pcap_tx,
            device: device.to_owned(),
        }
    }

    pub fn start(&mut self, state: Arc<AppState>) {
        // 配置捕获
        let mut cap = Capture::from_device(self.device.clone())
            .unwrap()
            .promisc(true) // 混杂模式
            .snaplen(65535) // 最大捕获长度
            .timeout(500) // 读取超时 ms
            .open()
            .unwrap();
        let _ = cap.filter("tcp or udp", true);

        info!("Capture started on {}", self.device.name);

        while *state.running.blocking_read() {
            if let Ok(packet) = cap.next_packet() {
                let runtime = tokio::runtime::Runtime::new().unwrap();
                runtime.block_on(async {
                    let _ = self.pcap_tx.send((*packet.header, packet.data.to_vec())).await;
                });
                let result = panic::catch_unwind(|| {
                    if let Some(info) = parse_packet(packet.data) {
                        trace!("info: {:?}", info);
                        let _ = self.tx.try_send(info).unwrap(); // 发送到聚合器
                    }
                });
                if result.is_err() {
                    error!("packet parse error: {:?}", result);
                }
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
                        return None;
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
