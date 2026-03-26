use pcap::{Active, Capture, PacketHeader};
use pnet::packet::ethernet::{EtherTypes, EthernetPacket};
use pnet::packet::ipv4::Ipv4Packet;
use pnet::packet::Packet;
use pnet::packet::tcp::TcpPacket;
use tracing::{debug, info};
use crate::networking::types::tls::{parse_client_hello, TlsPacket};
use crate::utils::registry::Registry;

pub struct PacketOwned {
    pub header: PacketHeader,
    pub(crate) data: Box<[u8]>,
}


pub(crate) fn packet_stream(
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

pub(crate) fn parse_packet(data: &[u8]) {
    // 解析以太网层
    if let Some(eth) = EthernetPacket::new(data) {
        match eth.get_ethertype() {
            EtherTypes::Ipv4 => {
                // 解析 IP 层
                if let Some(ip) = Ipv4Packet::new(eth.payload()) {
                    debug!("IPv4: {} -> {}", ip.get_source(), ip.get_destination());

                    // 解析 TCP 层
                    if let Some(tcp) = TcpPacket::new(ip.payload()) {
                        debug!("TCP: {} -> {}", tcp.get_source(), tcp.get_destination());
                        debug!("Flags: {:?}", tcp.get_flags());

                        let src = format!("{}:{}", ip.get_source(), tcp.get_source());
                        let dst = format!("{}:{}", ip.get_destination(), tcp.get_destination());
                        let connection_id = format!("{}->{}", src, dst);
                        
                        let payload = tcp.payload();
                        if payload.starts_with(b"GET".as_slice())
                            || payload.starts_with(b"POST".as_slice()) {   // 解析 HTTP 层
                            debug!("Payload: {:?}", tcp.payload());

                            // 预分配 headers 数组
                            let mut headers = [httparse::EMPTY_HEADER; 16];
                            let mut req = httparse::Request::new(&mut headers);

                            // 解析
                            let status = req.parse(payload).unwrap();
                            debug!("status: {:?}", status);

                            if status.is_complete() {
                                let host = req
                                    .headers
                                    .iter()
                                    .filter(|h| h.name == "Host")
                                    .next()
                                    .unwrap();
                                info!("HTTP Method: {}, Host: {}", req.method.unwrap(), String::from_utf8_lossy(host.value));
                            }
                        } else if payload.starts_with(b"\x16\x03".as_slice()) || payload.starts_with(b"\x17\x03".as_slice()) { // TLS 握手
                            debug!("TLS Record: {}", hex::encode(payload));
                            let len = u16::from_be_bytes(payload[3..5].try_into().unwrap()) + 5;
                            debug!("TLS len: {}", len);
                            if len < payload.len() as u16 {
                                parse_client_hello(payload);
                            } else {
                                Registry::set(connection_id.clone(), TlsPacket{
                                    len,
                                    data: payload.to_vec(),
                                });
                            }
                        } else {
                            debug!("Payload: {:?}", hex::encode(payload));
                            if let Some(tls_packet) = Registry::get::<TlsPacket>(connection_id.clone()){
                                let mut tls_payload = tls_packet.data;
                                tls_payload.extend(payload.to_vec());
                                if tls_payload.len() as u16 >= tls_packet.len {
                                    Registry::remove(connection_id.clone());
                                    parse_client_hello(&tls_payload[..tls_payload.len()]);
                                } else {
                                    Registry::set(connection_id.clone(), TlsPacket{
                                        len: tls_packet.len,
                                        data: tls_payload,
                                    });
                                }
                            }

                        }
                    }
                }
            }
            _ => {

            }
        }
    }
}