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
use tracing::{debug, error, info, trace, warn};

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
    pub timestamp_us: u64, // pcap header: tv_sec * 1_000_000 + tv_usec
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
                let timestamp_us = packet.header.ts.tv_sec as u64 * 1_000_000
                    + packet.header.ts.tv_usec as u64;
                let _ = self.pcap_tx.send_blocking((*packet.header, packet.data.to_vec()));
                let result = panic::catch_unwind(|| {
                    parse_packet(packet.data, timestamp_us)
                });
                match result {
                    Ok(Some(info)) => {
                        if let Err(e) = self.tx.try_send(info) {
                            warn!("channel full, dropping packet: {}", e);
                        }
                    }
                    Err(e) => error!("packet parse error: {:?}", e),
                    _ => {}
                }
            }
        }
    }
}

fn parse_packet(packet: &[u8], timestamp_us: u64) -> Option<PacketInfo> {
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
                        timestamp_us,
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
                        timestamp_us,
                    })
                }
                _ => None,
            }
        }
        _ => None,
    }
}
