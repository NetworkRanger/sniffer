use std::collections::HashMap;
use std::time::{Duration, Instant};
use bytesize::ByteSize;
use pcap::{Active, Capture, PacketHeader};
use pnet::packet::ethernet::{EtherTypes, EthernetPacket};
use pnet::packet::ipv4::Ipv4Packet;
use pnet::packet::Packet;
use pnet::packet::tcp::TcpPacket;
use pnet::packet::udp::UdpPacket;
use tracing::{debug, info, trace};
use dns_parser;
use serde::{Serialize, Serializer};
use crate::config::Config;
use crate::networking::types::{quic};
use crate::networking::types::h2c::H2c;
use crate::networking::types::tls::{parse_client_hello, TlsPacket};
use crate::utils::registry::Registry;

#[allow(dead_code)]
pub struct PacketOwned {
    pub header: PacketHeader,
    pub data: Box<[u8]>,
}

fn serialize_instant<S>(_instant: &Instant, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let formatted = "instant".to_string();
    serializer.serialize_str(&formatted)
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize)]
pub struct Connection {
    pub id: String,
    pub src_ip: String,
    pub src_port: u16,
    pub dst_ip: String,
    pub dst_port: u16,
    pub protocol: String,
    pub domain: Option<String>,
    pub path: Option<String>,
    #[serde(serialize_with = "serialize_instant")]
    pub start: Instant,
    #[serde(serialize_with = "serialize_instant")]
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


#[derive(Debug, Clone)]
struct HttpPartial {
    data: Vec<u8>,
}

fn is_http_request_start(payload: &[u8]) -> bool {
    payload.starts_with(b"GET ")
        || payload.starts_with(b"POST ")
        || payload.starts_with(b"PUT ")
        || payload.starts_with(b"DELETE ")
        || payload.starts_with(b"PATCH ")
        || payload.starts_with(b"HEAD ")
        || payload.starts_with(b"OPTIONS ")
}

fn parse_http_request(payload: &[u8], connection: &mut Connection) -> Result<httparse::Status<usize>, httparse::Error> {
    let mut headers = [httparse::EMPTY_HEADER; 16];
    let mut req = httparse::Request::new(&mut headers);
    let status = req.parse(payload)?;
    trace!("status: {:?}", status);

    if status.is_complete() {
        if let Some(host) = req.headers.iter().find(|h| h.name == "Host") {
            debug!(
                "HTTP Method: {}, Host: {}, connection_id: {}",
                req.method.unwrap_or(""),
                String::from_utf8_lossy(host.value),
                connection.id
            );
            connection.domain = Some(String::from_utf8_lossy(host.value).to_string());
        }
        connection.path = req.path.map(|path| path.to_string());
        connection.protocol = "http".to_string();
    }

    Ok(status)
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
                    parse_tcp(&ip, &eth);
                    // 解析 UDP 层
                    parse_udp(&ip, &eth);
                }
            }
            _ => {

            }
        }
    }
}

pub fn parse_udp(ip: &Ipv4Packet, eth: &EthernetPacket) {
    if let Some(udp) = UdpPacket::new(ip.payload()) {
        trace!("UDP:  -> {} {}", udp.get_source(), udp.get_destination());

        let src = format!("{}:{}", ip.get_source(), udp.get_source());
        let dst = format!("{}:{}", ip.get_destination(), udp.get_destination());
        let mac = Registry::get::<String>("mac").unwrap();
        let mut connection_id = format!("udp://{}@{}", dst, src);
        if eth.get_source().to_string() == mac {
            connection_id = format!("udp://{}@{}", src, dst);
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
        } else if eth.get_destination().to_string() == mac && udp.get_destination() == 53 {
            parse_dns(payload);
        }
        stat_packet(&eth, &mut connection, payload);
    }
}

fn parse_dns(payload: &[u8]) {
    if let Ok(dns) = dns_parser::Packet::parse(payload) {
        for answer in dns.answers {
            if let dns_parser::RData::A(a) = answer.data {
                Registry::set("ip:".to_string() +  a.0.to_string().as_str(), answer.name.to_string(), None);
            }
        }
    }
}

pub fn parse_tcp(ip: &Ipv4Packet, eth: &EthernetPacket) {
    if let Some(tcp) = TcpPacket::new(ip.payload()) {
        trace!("TCP: {} -> {}", tcp.get_source(), tcp.get_destination());
        trace!("Flags: {:?}", tcp.get_flags());

        let src = format!("{}:{}", ip.get_source(), tcp.get_source());
        let dst = format!("{}:{}", ip.get_destination(), tcp.get_destination());
        let mac = Registry::get::<String>("mac").unwrap();
        let mut connection_id = format!("tcp://{}@{}", dst, src);
        if eth.get_source().to_string() == mac {
            connection_id = format!("tcp://{}@{}", src, dst);
        }
        trace!("connection_id: {}", connection_id);
        let mut connection = match Registry::get::<Connection>(&connection_id) {
            Some(connection) => connection,
            None => {
                let connection = Connection::new(connection_id.clone(), "tcp".to_string(), src, tcp.get_source(), dst, tcp.get_destination());
                Registry::set(connection_id.clone(), connection.clone(), None);
                connection
            }
        };
        let payload = tcp.payload();

        trace!("source: {:?}, mac: {:?}", eth.get_source(), mac);
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
    Registry::set(connection.id.clone(), connection.clone(), None);

    let mut millis_of_avg: u64 = 1000;
    if connection.now.elapsed() > Duration::from_millis(config.freq) {
        millis_of_avg = connection.now.elapsed().as_millis() as u64;
    } else if !connection.is_init {
        return;
    }

    let upload_sepped = ByteSize(connection.up_bytes * 1000 / millis_of_avg);
    let download_sepped = ByteSize(connection.down_bytes * 1000 / millis_of_avg);
    
    if connection.domain.is_none() {
        if let Some(domain) = Registry::get::<String>("ip:".to_string() + connection.dst_ip.clone().as_str()) {
            connection.domain = Some(domain);
        }
    }

    if (!config.has_domain || connection.domain.is_some()) && (upload_sepped.0 > 0 || download_sepped.0 > 0) {
        connection.is_init = false;
        connection.now = Instant::now();
        connection.up_bytes = 0;
        connection.down_bytes = 0;
        Registry::set(connection.id.clone(), connection.clone(), None);

        let mut domain_format = "".to_string();
        if let Some(ref domain) = connection.domain {
            domain_format += &format!("Domain: {}, ", domain);
        }
        let mut path_format = "".to_string();
        if let Some(ref path) = connection.path {
            path_format += &format!("Path: {}, ", path);
        }

        info!("protocol: {}, {}, {}{}Up: {:?}/s, Down: {:?}/s",
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
    let conn_preface = "PRI * HTTP/2.0\r\n\r\nSM\r\n\r\n".to_string();
    if is_http_request_start(payload) {   // 解析 HTTP 层
        let packet_key = "http_packet_".to_string() + connection.id.as_str();
        match parse_http_request(payload, connection) {
            Ok(status) if status.is_complete() => {
                Registry::remove(packet_key);
            }
            Ok(_) => {
                Registry::set(packet_key, HttpPartial { data: payload.to_vec() }, None);
            }
            Err(_) => {
                Registry::remove(packet_key);
            }
        }
    } else if payload.starts_with(b"\x16\x03".as_slice()) || payload.starts_with(b"\x17\x03".as_slice()) { // TLS 握手
        trace!("TLS Record: {}", hex::encode(payload));
        let len = u16::from_be_bytes(payload[3..5].try_into().unwrap()) + 5;
        trace!("TLS len: {}", len);
        if len <= payload.len() as u16 {
            let _ = parse_client_hello(payload, connection);
        } else {
            let packet_key = "tls_packet_".to_string() + connection.id.as_str();
            Registry::set(packet_key, TlsPacket{
                len,
                data: payload.to_vec(),
            }, None);
        }
    } else if payload.starts_with(conn_preface.as_bytes()){ // h2c
        let mut h2c = H2c::new();
        if let Some(headers) = h2c.parse_headers(payload[conn_preface.as_bytes().len()..].to_vec()) {
            set_h2c_connection(connection, headers);
        } else {
            let packet_key = "h2c_packet_".to_string() + connection.id.as_str();
            Registry::set(packet_key, 0, None);
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
        let packet_key = "h2c_packet_".to_string() + connection.id.as_str();
        if let Some(_) = Registry::get::<i32>(packet_key.clone()){
            let mut h2c = H2c::new();
            if let Some(headers) = h2c.parse_headers(payload.to_vec()) {
                set_h2c_connection(connection, headers);
                Registry::remove(packet_key);
            }
        }
        let packet_key = "http_packet_".to_string() + connection.id.as_str();
        if let Some(http_partial) = Registry::get::<HttpPartial>(packet_key.clone()) {
            let mut http_payload = http_partial.data;
            http_payload.extend(payload.to_vec());
            match parse_http_request(&http_payload, connection) {
                Ok(status) if status.is_complete() => {
                    Registry::remove(packet_key);
                }
                Ok(_) => {
                    Registry::set(packet_key, HttpPartial { data: http_payload }, None);
                }
                Err(_) => {
                    Registry::remove(packet_key);
                }
            }
        }
    }
}

pub fn set_h2c_connection(connection: &mut Connection, headers: HashMap<String, String>) {
    connection.protocol = "h2c".to_string();
    if let Some(domain) = headers.get(":authority"){
        connection.domain = Some(domain.to_string());
        connection.is_init = true;
    }
    if let Some(domain) = headers.get(":path"){
        connection.path = Some(domain.to_string());
        connection.is_init = true;
    }
    debug!("connection: {:?}", connection);
    Registry::set(connection.id.clone(), connection.clone(), None);
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connection_new() {
        let conn = Connection::new(
            "tcp://1.2.3.4:80@5.6.7.8:12345".to_string(),
            "tcp".to_string(),
            "1.2.3.4:80".to_string(),
            80,
            "5.6.7.8:12345".to_string(),
            12345,
        );
        assert_eq!(conn.id, "tcp://1.2.3.4:80@5.6.7.8:12345");
        assert_eq!(conn.protocol, "tcp");
        assert_eq!(conn.src_ip, "1.2.3.4:80");
        assert_eq!(conn.src_port, 80);
        assert_eq!(conn.dst_ip, "5.6.7.8:12345");
        assert_eq!(conn.dst_port, 12345);
        assert!(conn.domain.is_none());
        assert!(conn.path.is_none());
        assert_eq!(conn.up_bytes, 0);
        assert_eq!(conn.down_bytes, 0);
        assert!(conn.is_init);
    }

    #[test]
    fn test_set_h2c_connection_with_authority_and_path() {
        let mut conn = Connection::new(
            "test-id".to_string(),
            "tcp".to_string(),
            "src".to_string(),
            80,
            "dst".to_string(),
            8080,
        );
        conn.is_init = false;

        let mut headers = HashMap::new();
        headers.insert(":authority".to_string(), "example.com".to_string());
        headers.insert(":path".to_string(), "/api/v1".to_string());

        set_h2c_connection(&mut conn, headers);

        assert_eq!(conn.protocol, "h2c");
        assert_eq!(conn.domain.as_deref(), Some("example.com"));
        assert_eq!(conn.path.as_deref(), Some("/api/v1"));
        assert!(conn.is_init);
    }

    #[test]
    fn test_set_h2c_connection_without_authority() {
        let mut conn = Connection::new(
            "test-id2".to_string(),
            "tcp".to_string(),
            "src".to_string(),
            80,
            "dst".to_string(),
            8080,
        );

        let mut headers = HashMap::new();
        headers.insert(":method".to_string(), "GET".to_string());

        set_h2c_connection(&mut conn, headers);

        assert_eq!(conn.protocol, "h2c");
        assert!(conn.domain.is_none());
        assert!(conn.path.is_none());
    }

    #[test]
    fn test_set_h2c_connection_only_path() {
        let mut conn = Connection::new(
            "test-id3".to_string(),
            "tcp".to_string(),
            "src".to_string(),
            80,
            "dst".to_string(),
            8080,
        );
        conn.is_init = false;

        let mut headers = HashMap::new();
        headers.insert(":path".to_string(), "/index.html".to_string());

        set_h2c_connection(&mut conn, headers);

        assert_eq!(conn.protocol, "h2c");
        assert!(conn.domain.is_none());
        assert_eq!(conn.path.as_deref(), Some("/index.html"));
        assert!(conn.is_init);
    }

    #[test]
    fn test_parse_http_and_tls_single_packet_http() {
        let mut conn = Connection::new(
            "http-single".to_string(),
            "tcp".to_string(),
            "src".to_string(),
            80,
            "dst".to_string(),
            8080,
        );
        let payload = b"GET /hello HTTP/1.1\r\nHost: example.com\r\nUser-Agent: test\r\n\r\n";

        parse_http_and_tls(payload, &mut conn);

        assert_eq!(conn.protocol, "http");
        assert_eq!(conn.domain.as_deref(), Some("example.com"));
        assert_eq!(conn.path.as_deref(), Some("/hello"));
        assert!(Registry::get::<HttpPartial>("http_packet_http-single").is_none());
    }

    #[test]
    fn test_parse_http_and_tls_fragmented_http_request() {
        let key = "http_packet_http-fragmented";
        Registry::remove(key);
        let mut conn = Connection::new(
            "http-fragmented".to_string(),
            "tcp".to_string(),
            "src".to_string(),
            80,
            "dst".to_string(),
            8080,
        );

        parse_http_and_tls(b"GET /split HTTP/1.1\r\nHo", &mut conn);
        assert!(conn.domain.is_none());
        assert!(Registry::get::<HttpPartial>(key).is_some());

        parse_http_and_tls(b"st: example.com\r\nUser-Agent: test\r\n\r\n", &mut conn);
        assert_eq!(conn.protocol, "http");
        assert_eq!(conn.domain.as_deref(), Some("example.com"));
        assert_eq!(conn.path.as_deref(), Some("/split"));
        assert!(Registry::get::<HttpPartial>(key).is_none());
    }

    #[test]
    fn test_parse_http_and_tls_fragmented_http_request_multiple_packets() {
        let key = "http_packet_http-fragmented-multi";
        Registry::remove(key);
        let mut conn = Connection::new(
            "http-fragmented-multi".to_string(),
            "tcp".to_string(),
            "src".to_string(),
            80,
            "dst".to_string(),
            8080,
        );

        parse_http_and_tls(b"GET /multi HTTP/1.1\r\n", &mut conn);
        assert!(Registry::get::<HttpPartial>(key).is_some());
        parse_http_and_tls(b"Host: exa", &mut conn);
        assert!(Registry::get::<HttpPartial>(key).is_some());
        parse_http_and_tls(b"mple.com\r\n\r\n", &mut conn);

        assert_eq!(conn.protocol, "http");
        assert_eq!(conn.domain.as_deref(), Some("example.com"));
        assert_eq!(conn.path.as_deref(), Some("/multi"));
        assert!(Registry::get::<HttpPartial>(key).is_none());
    }

    #[test]
    fn test_parse_http_and_tls_fragmented_http_invalid_followup_clears_cache() {
        let key = "http_packet_http-invalid-followup";
        Registry::remove(key);
        let mut conn = Connection::new(
            "http-invalid-followup".to_string(),
            "tcp".to_string(),
            "src".to_string(),
            80,
            "dst".to_string(),
            8080,
        );

        parse_http_and_tls(b"GET /bad HTTP/1.1\r\nHo", &mut conn);
        assert!(Registry::get::<HttpPartial>(key).is_some());
        parse_http_and_tls(b"\x00\x01\x02", &mut conn);

        assert!(conn.domain.is_none());
        assert!(Registry::get::<HttpPartial>(key).is_none());
    }

    #[test]
    fn test_parse_http_and_tls_put_method() {
        let mut conn = Connection::new(
            "http-put".to_string(),
            "tcp".to_string(),
            "src".to_string(),
            80,
            "dst".to_string(),
            8080,
        );
        let payload = b"PUT /resource HTTP/1.1\r\nHost: put.example.com\r\n\r\n";

        parse_http_and_tls(payload, &mut conn);

        assert_eq!(conn.protocol, "http");
        assert_eq!(conn.domain.as_deref(), Some("put.example.com"));
        assert_eq!(conn.path.as_deref(), Some("/resource"));
    }

    #[test]
    fn test_parse_http_and_tls_options_method() {
        let mut conn = Connection::new(
            "http-options".to_string(),
            "tcp".to_string(),
            "src".to_string(),
            80,
            "dst".to_string(),
            8080,
        );
        let payload = b"OPTIONS * HTTP/1.1\r\nHost: options.example.com\r\n\r\n";

        parse_http_and_tls(payload, &mut conn);

        assert_eq!(conn.protocol, "http");
        assert_eq!(conn.domain.as_deref(), Some("options.example.com"));
        assert_eq!(conn.path.as_deref(), Some("*"));
    }
}
