use crate::capture::PacketInfo;
use crate::models::{AppState, Connection, ConnectionKey, NetworkStats, ProcessConnection};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::select;
use tokio::time::{interval, Duration};
use tracing::{info};
use sniffer::utils::registry::Registry;
use crate::process_connection::{get_process_connections};
use sniffer::packet::Connection as PacketConnection;
use crate::process::get_processes;

pub struct Aggregator;

impl Aggregator {
    pub async fn start(mut rx: tokio::sync::mpsc::Receiver<PacketInfo>, state: Arc<AppState>) {
        info!("Aggregator started");
        let mut ticker = interval(Duration::from_secs(1));
        let mut process_conn_ticker = interval(Duration::from_millis(100));
        let mut process_ticker = interval(Duration::from_millis(250));
        let mut last_t = ticker.tick().await;
        loop {
            select! {
                Some(packet) = rx.recv() => {
                    Self::update_connection(&state, packet).await;
                    // Drain all buffered packets before yielding to timers
                    while let Ok(packet) = rx.try_recv() {
                        Self::update_connection(&state, packet).await;
                    }
                }
                _ = process_conn_ticker.tick() => {
                    let _ = get_process_connections(&state).await;
                }
                _ = process_ticker.tick() => {
                    let _ = get_processes(&state).await;
                }
                t = ticker.tick() => {
                    let millis = t.saturating_duration_since(last_t).as_millis();
                    last_t = t;
                    let _ = get_processes(&state).await;
                    Self::calculate_stats(&state, millis as u64).await;
                }
                else => break,
            }
        }
    }

    async fn get_process_connection_by_key(state: &Arc<AppState>, key: ConnectionKey) -> Option<ProcessConnection> {
        let process_connections = state.process_connections.read().await;
        if let Some(process_connection) = process_connections.get(&key) {
            return Some(process_connection.to_owned());
        }
        if let Some(key) = process_connections.keys().into_iter().filter(|k| {
            k.protocol == key.protocol
                && k.remote_addr == key.remote_addr
                && k.remote_port == key.remote_port
        }).next() {
            return process_connections.get(&key).cloned();
        }
        if let Some(key) = process_connections.keys().into_iter().filter(|k| {
            k.protocol == key.protocol
                && (
                k.local_addr == key.local_addr
                    || k.local_addr.starts_with("::")
            )
                && k.local_port == key.local_port
                && (k.remote_addr.starts_with("::") || k.remote_addr == "*".to_string())
                && k.remote_port == key.remote_port
        }).next() {
            return process_connections.get(&key).cloned();
        }
        if let Some(key) = process_connections.keys().into_iter().filter(|k| {
            k.protocol == key.protocol
                && (
                    k.local_addr == key.local_addr
                    || k.local_addr.starts_with("::")
                )
                && k.local_port == key.local_port
            && (k.remote_addr.starts_with("::") || k.remote_addr == "*".to_string())
            && k.remote_port == 0
        }).next() {
            return process_connections.get(&key).cloned();
        }


        if let Some(key) = process_connections.keys().into_iter().filter(|k| {
            k.protocol == key.protocol
                && (k.local_addr.starts_with("::")
                || k.local_addr == "0.0.0.0".to_string()
                || k.local_addr == "127.0.0.1".to_string()
            )
            && k.local_port == key.local_port
        }).next() {
            return process_connections.get(&key).cloned();
        }
        if let Some(key) = process_connections.keys().into_iter().filter(|k| {
            k.protocol == key.protocol && k.protocol == "udp"
                && (k.local_addr.starts_with("::")
                    || k.local_addr == "0.0.0.0".to_string()
                    || k.local_addr == "127.0.0.1".to_string()
                )
                && k.local_port == 0
        }).next() {
            return process_connections.get(&key).cloned();
        }

        None
    }

    async fn update_connection(state: &Arc<AppState>, packet: PacketInfo) {
        let ip = Registry::get::<String>("ip").unwrap();
        let mut conns = state.connections.write().await;

        let mut local_addr = packet.dst_ip.clone();
        let mut local_port = packet.dst_port;
        let mut remote_addr = packet.src_ip.clone();
        let mut remote_port = packet.src_port;
        if packet.src_ip == ip {
            local_addr = packet.src_ip;
            local_port = packet.src_port;
            remote_addr = packet.dst_ip;
            remote_port = packet.dst_port;
        }
        let id = format!(
            "{}://{}:{}@{}:{}",
            packet.protocol.to_lowercase(),
            local_addr, local_port, remote_addr, remote_port
        );
        let key = ConnectionKey {
            protocol: packet.protocol.to_lowercase(),
            local_addr: local_addr.clone(),
            local_port: local_port.clone(),
            remote_addr: remote_addr.clone(),
            remote_port: remote_port.clone(),
        };
        let process_connection = Self::get_process_connection_by_key(state, key).await;
        let packet_connection = Registry::get::<PacketConnection>(id.clone());

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        conns
            .entry(id.clone())
            .and_modify(|conn| {
                if packet.is_outgoing {
                    conn.bytes_sent += packet.length as u64;
                    conn.packets_sent += 1;
                } else {
                    conn.bytes_recv += packet.length as u64;
                    conn.packets_recv += 1;
                }
                conn.last_active = now;
                conn.status = "active".to_string();
                conn.process_connection = conn.clone().process_connection.or(process_connection.clone());
                conn.packet_connection = packet_connection.clone();
                if conn.domain.is_none() {
                    if let Some(ref pc) = packet_connection {
                        conn.domain = pc.domain.clone();
                        conn.path = pc.path.clone();
                        if pc.domain.is_some() {
                            conn.protocol = pc.protocol.clone();
                        }
                    }
                }
            })
            .or_insert_with(|| {
                let domain = packet_connection.as_ref().and_then(|pc| pc.domain.clone());
                let path = packet_connection.as_ref().and_then(|pc| pc.path.clone());
                let protocol = if domain.is_some() {
                    packet_connection.as_ref().map(|pc| pc.protocol.clone()).unwrap_or(packet.protocol)
                } else {
                    packet.protocol
                };
                Connection {
                    id,
                    local_addr,
                    local_port,
                    remote_addr,
                    remote_port,
                    protocol,
                    domain,
                    path,
                    bytes_sent: if packet.is_outgoing {
                        packet.length as u64
                    } else {
                        0
                    },
                    bytes_recv: if packet.is_outgoing {
                        0
                    } else {
                        packet.length as u64
                    },
                    packets_sent: if packet.is_outgoing { 1 } else { 0 },
                    packets_recv: if packet.is_outgoing { 0 } else { 1 },
                    last_bytes_sent: 0,
                    last_bytes_recv: 0,
                    upload_speed: 0,
                    download_speed: 0,
                    start_time: now,
                    start_time_us: packet.timestamp_us,
                    last_active: now,
                    status: "active".to_string(),
                    process_connection,
                    packet_connection,
                }
            });
    }

    async fn calculate_stats(state: &Arc<AppState>, millis: u64) {
        let millis = if millis == 0 { 1000 } else { millis };
        let mut conns = state.connections.write().await;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let mut total_sent = 0u64;
        let mut total_recv = 0u64;
        let mut total_packets = 0u32;

        // 收集连接并排序（按总流量）
        let mut connections: Vec<Connection> = conns.values().cloned().collect();
        connections.sort_by(|a, b| {
            let a_total = a.bytes_sent + a.bytes_recv;
            let b_total = b.bytes_sent + b.bytes_recv;
            b_total.cmp(&a_total) // 降序
        });

        for (_id, conn ) in conns.iter_mut() {
            total_sent += conn.bytes_sent;
            total_recv += conn.bytes_recv;
            total_packets += conn.packets_sent + conn.packets_recv;

            conn.upload_speed = (conn.bytes_sent - conn.last_bytes_sent) * 1000 / millis;
            conn.download_speed = (conn.bytes_recv - conn.last_bytes_recv) * 1000 / millis;

            conn.last_bytes_sent = conn.bytes_sent;
            conn.last_bytes_recv = conn.bytes_recv;

            // 补查未关联进程的连接
            if conn.process_connection.is_none() {
                let key = ConnectionKey {
                    protocol: conn.protocol.to_lowercase(),
                    local_addr: conn.local_addr.clone(),
                    local_port: conn.local_port,
                    remote_addr: conn.remote_addr.clone(),
                    remote_port: conn.remote_port,
                };
                conn.process_connection = Self::get_process_connection_by_key(state, key).await;
            }
        }

        // 计算速率（需要历史数据，简化处理）
        let stats = NetworkStats {
            timestamp: now,
            total_bytes_sent: total_sent,
            total_bytes_recv: total_recv,
            total_packets,
            active_connections: connections.len(),
            top_connections: connections.into_iter().take(10).collect(),
            upload_speed: total_sent * 1000 / millis,
            download_speed: total_recv * 1000 / millis,
        };

        let mut history = state.stats_history.write().await;
        history.push(stats);

        // 只保留最近 5 分钟（300 秒）的历史
        if history.len() > 300 {
            history.remove(0);
        }
    }
}
