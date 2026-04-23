use pcap::{Capture, Device};
use pnet::packet::ethernet::{EthernetPacket};
use std::{panic, thread};
use std::time::{Duration, Instant};
use tracing::{error, info};
use bytesize::ByteSize;
use tracing_subscriber::fmt::time::OffsetTime;
use time::macros::{format_description, offset};
use tracing_subscriber::EnvFilter;

pub mod utils;
pub mod networking;
pub mod packet;
pub mod config;
#[macro_use]
extern crate enum_primitive;

use crate::utils::get_mac_by_name;
use crate::utils::registry::Registry;
use crate::config::Config;


pub fn cli_run() {
    // 定义时间格式（年-月-日T时:分:秒.毫秒）
    let time_fmt = format_description!(
        "[year]-[month]-[day] [hour]:[minute]:[second].[subsecond digits:3]"
    );

    // 构造一个偏移 +8:00（北京时间）的时间格式器
    let timer = OffsetTime::new(offset!(+8:00), time_fmt);

    // 安装 tracing-subscriber，并用自定义时间格式
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| EnvFilter::new("info"))
        )
        .with_timer(timer)
        .init();

    let config = Config::new();
    Registry::set("config", config.clone(), Some(0u64));

    let device_list = Device::list().expect("device list failed");

    // 查找默认设备
    let device = device_list
        .iter()
        .filter(|d| {
            d.flags.connection_status == pcap::ConnectionStatus::Connected && d.addresses.len() > 0
        })
        .next()
        .expect("no device available");
    
    info!("Using device: {}", device.name);
    let mac = get_mac_by_name(device.name.as_str()).unwrap();
    info!("Using device mac {}", mac);
    Registry::set("mac", mac.clone(), Some(0u64));

    // 配置捕获
    let mut cap = Capture::from_device(device.clone())
        .unwrap()
        .promisc(true) // 混杂模式
        .snaplen(65535) // 最大捕获长度
        .timeout(500) // 读取超时 ms
        .open()
        .unwrap();
    let _ = cap.filter("tcp or udp", true);
    
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
            Err(_e) => {}
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
                
                let result = panic::catch_unwind(|| {
                    packet::parse_packet(&packet.data);
                });
                if result.is_err() {
                    error!("packet parse error: {:?}", result);
                }
            }
        }
    }
}



