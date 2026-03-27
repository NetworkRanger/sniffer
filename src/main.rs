use pcap::{Active, Capture, Device, PacketHeader};
use pnet::packet::Packet;
use pnet::packet::ethernet::{EtherTypes, EthernetPacket};
use pnet::packet::ipv4::Ipv4Packet;
use pnet::packet::tcp::TcpPacket;
use std::io::Read;
use std::thread;
use std::time::{Duration, Instant};
use tracing::{debug, info};
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast::Receiver;
use bytesize::ByteSize;

mod utils;
mod networking;
mod packet;
mod config;

use crate::utils::get_mac_by_name;
use crate::utils::registry::Registry;
use crate::config::Config;


fn main() {
    tracing_subscriber::fmt::init();
    let config = Config::new();
    Registry::set("config", config.clone());

    let device_list = Device::list().expect("device list failed");

    // 查找默认设备
    let device = device_list
        .iter()
        .filter(|d| d.flags.connection_status == pcap::ConnectionStatus::Connected)
        .next()
        .expect("no device available");
    
    info!("Using device: {}", device.name);
    let mac = get_mac_by_name(device.name.as_str()).unwrap();
    info!("Using device mac {}", get_mac_by_name(device.name.as_str()).unwrap());
    Registry::set("mac", mac.clone());

    // 配置捕获
    let cap = Capture::from_device(device.clone())
        .unwrap()
        .promisc(true) // 混杂模式
        .snaplen(65535) // 最大捕获长度
        .timeout(500) // 读取超时 ms
        .open()
        .unwrap();
    
    let (pcap_tx, pcap_rx) = std::sync::mpsc::sync_channel(10_000);
    let _ = thread::Builder::new()
        .name("thread_packet_stream".to_string())
        .spawn(move || packet::packet_stream(cap, &pcap_tx))
        .unwrap();

    let mut now = Instant::now();
    let mut up_bytes: u64 = 0;
    let mut down_bytes: u64 = 0;
    let mut packet_count = 0;
    loop {
        let (packet_res, cap_stats) = pcap_rx
            .recv_timeout(Duration::from_millis(100))
            .unwrap_or((Err(pcap::Error::TimeoutExpired), None));
       
        match packet_res {
            Err(e) => {}
            Ok(packet) => {
                if let Some(eth) = EthernetPacket::new(&packet.data) {
                    if eth.get_source().to_string() == mac {
                        up_bytes  += packet.data.len() as u64;
                    } else if eth.get_destination().to_string() == mac {
                        down_bytes += packet.data.len() as u64;
                    }
                }
                if let Some(stat) = cap_stats {
                    packet_count = stat.received;
                }
                
                if now.elapsed() > Duration::from_secs(config.freq) {
                    let sedonds_of_avg = now.elapsed().as_millis() as u64;
                    let upload_sepped = ByteSize(up_bytes * 1000 / sedonds_of_avg);
                    let download_sepped = ByteSize(down_bytes * 1000 / sedonds_of_avg);

                    info!("Total packet_count: {},  Up: {:?}/s, Down: {:?}/s",
                            packet_count,  
                            upload_sepped,
                            download_sepped
                    );
                    
                    up_bytes = 0;
                    down_bytes = 0;
                    packet_count = 0;
                    now = Instant::now();
                }
                
                packet::parse_packet(&packet.data);
            }
        }
    }
}



