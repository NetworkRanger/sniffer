use std::time::{Duration, Instant};
use bytesize::ByteSize;
use pcap::{Active, Capture, PacketHeader};
use pnet::packet::ethernet::{EtherTypes, EthernetPacket};
use pnet::packet::ipv4::Ipv4Packet;
pub(crate) use pnet::packet::Packet;
use pnet::packet::tcp::TcpPacket;
use pnet::packet::udp::UdpPacket;
use tracing::{debug, info, trace};
use crate::config::Config;
use crate::networking::types::quic;
use crate::networking::types::tls::{parse_client_hello, TlsPacket};
use crate::utils::registry::Registry;

#[allow(dead_code)]
pub struct PacketOwned {
    pub header: PacketHeader,
    pub data: Box<[u8]>,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct Connection {
    pub id: String,
    pub src_ip: String,
    pub src_port: u16,
    pub dst_ip: String,
    pub dst_port: u16,
    pub protocol: String,
    pub domain: Option<String>,
    pub path: Option<String>,
    pub start: Instant,
    pub now: Instant,
    pub up_bytes: u64,
    pub down_bytes: u64,
    pub is_init: bool,
}

impl Connection {
    pub fn new(id: String, protocol: String, src_ip: String, src_port: u16, dst_ip: String, dst_port: u16) -> Self {
        let now = Instant::now();
        Connection {
            id,
            src_ip,
            src_port,
            dst_ip,
            dst_port,
            protocol: protocol,
            domain: None,
            path: None,
            start: now,
            now: now,
            up_bytes: 0,
            down_bytes: 0,
            is_init: true,
        }
    }
}


pub fn packet_stream(
    mut cap: Capture<Active>,
    tx: &std::sync::mpsc::SyncSender<(Result<PacketOwned, pcap::Error>, Option<pcap::Stat>)>,
) {
    loop {
        let packet_res = cap.next_packet();
        let packet_owned = packet_res.map(|p| PacketOwned {
            header: *p.header,
            data: p.data.into(),
        });
        if tx.send((packet_owned, cap.stats().ok())).is_err() {
            return;
        }
    }
}

pub fn parse_packet(data: &[u8]) {
    // 解析以太网层
    if let Some(eth) = EthernetPacket::new(data) {
        match eth.get_ethertype() {
            EtherTypes::Ipv4 => {
                // 解析 IP 层
                if let Some(ip) = Ipv4Packet::new(eth.payload()) {
                    trace!("IPv4: {} -> {}", ip.get_source(), ip.get_destination());

                    // 解析 UDP 层
                    parse_udp(&ip, &eth);
                    // 解析 TCP 层
                    parse_tcp(&ip, &eth);
                }
            }
            _ => {

            }
        }
    }
}

fn parse_udp(ip: &Ipv4Packet, eth: &EthernetPacket) {
    if let Some(udp) = UdpPacket::new(ip.payload()) {
        trace!("UDP:  -> {} {}", udp.get_source(), udp.get_destination());

        let src = format!("{}:{}", ip.get_source(), udp.get_source());
        let dst = format!("{}:{}", ip.get_destination(), udp.get_destination());
        let mac = Registry::get::<String>("mac").unwrap();
        let mut connection_id = format!("{}->{}", dst, src);
        if eth.get_source().to_string() == mac {
            connection_id = format!("{}->{}", src, dst);
        }
        let mut connection = match Registry::get::<Connection>(&connection_id) {
            Some(connection) => connection,
            None => {
                let connection = Connection::new(connection_id.clone(), "udp".to_string(), src, udp.get_source(), dst, udp.get_destination());
                Registry::set(connection_id.clone(), connection.clone(), None);
                connection
            }
        };
        let payload = udp.payload();
        if eth.get_source().to_string() == mac {
            if payload.len() == 0 {
                return;
            }
            if payload[0] >> 4 != 0xc {
                return;
            }
            if payload[1..].starts_with(b"\xba\xba\xba\xba".as_slice()) || payload[1..].starts_with(b"\x00\x00\x00\x01".as_slice()) {
                debug!("payload {}", hex::encode(payload));
                if let Some(domain) = quic::Quic::new().parse_inital_packet(payload.to_vec()) {
                    debug!("Domain: {}", domain);
                    connection.domain = Some(domain);
                    connection.protocol = "quic".to_string();
                    Registry::set(connection_id, connection.clone(), None);
                }
            }
        }
        stat_packet(&eth, &mut connection, payload);
    }
}

fn parse_tcp(ip: &Ipv4Packet, eth: &EthernetPacket) {
    if let Some(tcp) = TcpPacket::new(ip.payload()) {
        trace!("TCP: {} -> {}", tcp.get_source(), tcp.get_destination());
        trace!("Flags: {:?}", tcp.get_flags());

        let src = format!("{}:{}", ip.get_source(), tcp.get_source());
        let dst = format!("{}:{}", ip.get_destination(), tcp.get_destination());
        let mac = Registry::get::<String>("mac").unwrap();
        let mut connection_id = format!("{}->{}", dst, src);
        if eth.get_source().to_string() == mac {
            connection_id = format!("{}->{}", src, dst);
        }
        let mut connection = match Registry::get::<Connection>(&connection_id) {
            Some(connection) => connection,
            None => {
                let connection = Connection::new(connection_id.clone(), "tcp".to_string(), src, tcp.get_source(), dst, tcp.get_destination());
                Registry::set(connection_id.clone(), connection.clone(), None);
                connection
            }
        };
        let payload = tcp.payload();

        if eth.get_source().to_string() == mac {
            parse_http_and_tls(payload, &mut connection);
            Registry::set(connection_id, connection.clone(), None);
        }

        stat_packet(&eth, &mut connection, payload,);
    }
}

fn stat_packet(eth: &EthernetPacket, connection: &mut Connection, payload: &[u8]) {
    let config = Registry::get::<Config>("config").unwrap();
    let mac = Registry::get::<String>("mac").unwrap();

    if eth.get_source().to_string() == mac {
        connection.up_bytes += payload.len() as u64;
    } else if eth.get_destination().to_string() == mac {
        connection.down_bytes += payload.len() as u64;
    }

    let mut millis_of_avg: u64 = 1000;
    if connection.now.elapsed() > Duration::from_millis(config.freq) {
        millis_of_avg = connection.now.elapsed().as_millis() as u64;
        // calc_packet(connection.clone(), config.clone(), millis_of_avg);
    } else if !connection.is_init {
        return;
        // calc_packet(connection.clone(), config.clone(), millis_of_avg);
    }

    let upload_sepped = ByteSize(connection.up_bytes * 1000 / millis_of_avg);
    let download_sepped = ByteSize(connection.down_bytes * 1000 / millis_of_avg);

    let mut domain_format = "".to_string();
    if let Some(ref domain) = connection.domain {
        domain_format += &format!("Domain: {}, ", domain);
    }
    let mut path_format = "".to_string();
    if let Some(ref path) = connection.path {
        path_format += &format!("Path: {}, ", path);
    }

    if (!config.has_domain || connection.domain != None) && (upload_sepped.0 > 0 || download_sepped.0 > 0) {
        connection.is_init = false;
        connection.now = Instant::now();
        connection.up_bytes = 0;
        connection.down_bytes = 0;
        Registry::set(connection.id.clone(), connection.clone(), None);

        info!("{}: {}, {}{}Up: {:?}/s, Down: {:?}/s",
            connection.protocol,
            connection.id,
            domain_format,
            path_format,
            upload_sepped,
            download_sepped
        );
    }
}

fn parse_http_and_tls(payload: &[u8], connection: &mut Connection) {
    if payload.starts_with(b"GET".as_slice())
        || payload.starts_with(b"POST".as_slice()) {   // 解析 HTTP 层

        // 预分配 headers 数组
        let mut headers = [httparse::EMPTY_HEADER; 16];
        let mut req = httparse::Request::new(&mut headers);

        // 解析
        let status = req.parse(payload).unwrap();
        trace!("status: {:?}", status);

        if status.is_complete() {
            let host = req
                .headers
                .iter()
                .filter(|h| h.name == "Host")
                .next()
                .unwrap();
            debug!("HTTP Method: {}, Host: {}, connection_id: {}", req.method.unwrap(), String::from_utf8_lossy(host.value), connection.id);
            connection.domain = Some(String::from_utf8_lossy(host.value).to_string());
            connection.path = req.path.and_then(|path| Some(path.to_string()));
            connection.protocol = "http".to_string();
        }
    } else if payload.starts_with(b"\x16\x03".as_slice()) || payload.starts_with(b"\x17\x03".as_slice()) { // TLS 握手
        trace!("TLS Record: {}", hex::encode(payload));
        let len = u16::from_be_bytes(payload[3..5].try_into().unwrap()) + 5;
        trace!("TLS len: {}", len);
        if len < payload.len() as u16 {
            let _ = parse_client_hello(payload, connection);
        } else {
            let packet_key = "tls_packet_".to_string() + connection.id.as_str();
            Registry::set(packet_key, TlsPacket{
                len,
                data: payload.to_vec(),
            }, None);
        }
    } else {
        trace!("Payload: {:?}", hex::encode(payload));
        let packet_key = "tls_packet_".to_string() + connection.id.as_str();
        if let Some(tls_packet) = Registry::get::<TlsPacket>(packet_key.clone()){
            let mut tls_payload = tls_packet.data;
            tls_payload.extend(payload.to_vec());
            if tls_payload.len() as u16 >= tls_packet.len {
                Registry::remove(packet_key);
                let _ = parse_client_hello(&tls_payload[..tls_payload.len()], connection);
            } else {
                Registry::set(packet_key, TlsPacket{
                    len: tls_packet.len,
                    data: tls_payload,
                }, None);
            }
        }
    }
}