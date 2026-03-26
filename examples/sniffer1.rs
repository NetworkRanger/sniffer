use pcap::{Device, Capture};
use tracing::info;
use pnet::packet::ethernet::{EthernetPacket, EtherTypes};
use pnet::packet::ipv4::Ipv4Packet;
use pnet::packet::tcp::TcpPacket;
use pnet::packet::Packet;

fn main() {
    tracing_subscriber::fmt::init();

    let device_list = Device::list().expect("device list failed");

    // 查找默认设备
    let device = device_list.iter().
        filter(|d| d.flags.connection_status == pcap::ConnectionStatus::Connected)
        .next().expect("no device available");
    info!("Using device: {}", device.name);
    
    // 配置捕获
    let mut cap = Capture::from_device(device.clone())
        .unwrap()
        .promisc(true)          // 混杂模式
        .snaplen(65535)        // 最大捕获长度
        .timeout(500)           // 读取超时 ms
        .open()
        .unwrap();
    
    // 设置 BPF 过滤器（仅 HTTP/HTTPS）
    cap.filter("port 80 or port 443", false).unwrap();
    
    // 捕获数据包
    while let Ok(packet) = cap.next_packet() {
        println!("Captured {} bytes", packet.len());
        println!("Timestamp: {:?}", packet.header.ts);
        println!("Data: {:02x?}", &packet.data[..20.min(packet.len())]);
        parse_packet(&packet.data);
    }
}


fn parse_packet(data: &[u8]) {
    // 解析以太网层
    if let Some(eth) = EthernetPacket::new(data) {
        println!("Ethernet: {} -> {}", eth.get_source(), eth.get_destination());

        match eth.get_ethertype() {
            EtherTypes::Ipv4 => {
                // 解析 IP 层
                if let Some(ip) = Ipv4Packet::new(eth.payload()) {
                    println!("IPv4: {} -> {}", ip.get_source(), ip.get_destination());

                    // 解析 TCP 层
                    if let Some(tcp) = TcpPacket::new(ip.payload()) {
                        println!("TCP: {} -> {}", tcp.get_source(), tcp.get_destination());
                        println!("Flags: {:?}", tcp.get_flags());
                    }
                }
            }
            _ => {}
        }
    }
}