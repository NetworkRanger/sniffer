use std::time::{Duration, Instant};
use bytesize::ByteSize;
use pcap::{Active, Capture, PacketHeader};
use pnet::packet::ethernet::{EtherTypes, EthernetPacket};
use pnet::packet::ipv4::Ipv4Packet;
use pnet::packet::Packet;
use pnet::packet::tcp::TcpPacket;
use tracing::{debug, info, trace};
use crate::config::Config;
use crate::networking::types::tls::{parse_client_hello, TlsPacket};
use crate::utils::registry::Registry;

pub struct PacketOwned {
    pub header: PacketHeader,
    pub data: Box<[u8]>,
}

#[derive(Debug, Clone)]
pub struct Connection {
    pub id: String,
    pub src_ip: String,
    pub src_port: u16,
    pub dst_ip: String,
    pub dst_port: u16,
    pub domain: Option<String>,
    pub path: Option<String>,
    pub start: Instant,
    pub now: Instant,
    pub up_bytes: u64,
    pub down_bytes: u64,
    pub is_init: bool,
}

impl Connection {
    pub fn new(id: String, src_ip: String, src_port: u16, dst_ip: String, dst_port: u16) -> Self {
        let now = Instant::now();
        Connection {
            id,
            src_ip,
            src_port,
            dst_ip,
            dst_port,
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

                    // 解析 TCP 层
                    if let Some(tcp) = TcpPacket::new(ip.payload()) {
                        trace!("TCP: {} -> {}", tcp.get_source(), tcp.get_destination());
                        trace!("Flags: {:?}", tcp.get_flags());
                        
                        let config = Registry::get::<Config>("config").unwrap();
                        let mac = Registry::get::<String>("mac").unwrap();
                        let src = format!("{}:{}", ip.get_source(), tcp.get_source());
                        let dst = format!("{}:{}", ip.get_destination(), tcp.get_destination());
                        let connection_id = format!("{}->{}", src, dst);
                        
                        let mut connection = match Registry::get::<Connection>(&connection_id) {
                            Some(connection) => connection,
                            None => {
                                let connection = Connection::new(connection_id.clone(), src, tcp.get_source(), dst, tcp.get_destination());
                                Registry::set(connection_id.clone(), connection.clone());
                                connection
                            }
                        };
                        
                        let payload = tcp.payload();
                        parse_http_and_tls(payload, &mut connection);
                        Registry::set(connection_id, connection.clone());
                        stat_tcp(&eth, &mut connection, mac, payload, &config);
                    }
                }
            }
            _ => {

            }
        }
    }
}

fn stat_tcp(eth: &EthernetPacket, connection: &mut Connection, mac: String, payload: &[u8], config: &Config) {
    if eth.get_source().to_string() == mac {
        connection.up_bytes += payload.len() as u64;
    } else if eth.get_destination().to_string() == mac {
        connection.down_bytes += payload.len() as u64;
    }

    let mut millis_of_avg: u64 = 1000;
    if connection.now.elapsed() > Duration::from_millis(config.freq) {
        millis_of_avg = connection.now.elapsed().as_millis() as u64;
        calc_tcp(connection.clone(), config.clone(), millis_of_avg);

        connection.up_bytes = 0;
        connection.down_bytes = 0;
        connection.now = Instant::now();
    } else if connection.is_init {
        calc_tcp(connection.clone(), config.clone(), millis_of_avg);
        connection.is_init = false;
    }
    Registry::set(connection.id.clone(), connection.clone());
}

fn calc_tcp(connection: Connection, config: Config, millis_of_avg: u64) {
    let upload_sepped = ByteSize(connection.up_bytes * 1000 / millis_of_avg);
    let download_sepped = ByteSize(connection.up_bytes * 1000 / millis_of_avg);

    let mut domain_format = "".to_string();
    if let Some(ref domain) = connection.domain {
        domain_format += &format!("Domain: {}, ", domain);
    }
    let mut path_format = "".to_string();
    if let Some(ref path) = connection.path {
        path_format += &format!("Path: {}, ", path);
    }

    if (!config.has_domain || connection.domain != None) && (upload_sepped.0 > 0 || download_sepped.0 > 0) {
        info!("connection: {}, {}{}Up: {:?}/s, Down: {:?}/s",
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
        }
    } else if payload.starts_with(b"\x16\x03".as_slice()) || payload.starts_with(b"\x17\x03".as_slice()) { // TLS 握手
        trace!("TLS Record: {}", hex::encode(payload));
        let len = u16::from_be_bytes(payload[3..5].try_into().unwrap()) + 5;
        trace!("TLS len: {}", len);
        if len < payload.len() as u16 {
            parse_client_hello(payload, connection);
        } else {
            let packet_key = "tls_packet_".to_string() + connection.id.as_str();
            Registry::set(packet_key, TlsPacket{
                len,
                data: payload.to_vec(),
            });
        }
    } else {
        trace!("Payload: {:?}", hex::encode(payload));
        let packet_key = "tls_packet_".to_string() + connection.id.as_str();
        if let Some(tls_packet) = Registry::get::<TlsPacket>(packet_key.clone()){
            let mut tls_payload = tls_packet.data;
            tls_payload.extend(payload.to_vec());
            if tls_payload.len() as u16 >= tls_packet.len {
                Registry::remove(packet_key);
                parse_client_hello(&tls_payload[..tls_payload.len()], connection);
            } else {
                Registry::set(packet_key, TlsPacket{
                    len: tls_packet.len,
                    data: tls_payload,
                });
            }
        }
    }
}