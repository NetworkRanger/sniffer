use crate::capture::PacketInfo;
use crate::models::{AppState, Connection, NetworkStats};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::select;
use tokio::sync::mpsc;
use tokio::time::{interval, Duration};
use tracing::{debug, info, trace};
use sniffer::utils::registry::Registry;
use crate::process_connection::{get_process_connections, ProcessConnection};
use sniffer::packet::Connection as PacketConnection;

pub struct Aggregator;

impl Aggregator {
    pub async fn start(mut rx: mpsc::Receiver<PacketInfo>, state: Arc<AppState>) {
        info!("Aggregator started");
        let mut ticker = interval(Duration::from_secs(1));
        let mut last_t = ticker.tick().await;
        loop {
            select! {
                Some(packet) = rx.recv() => {
                     debug!("Received packet {:?}", packet);
                     Self::update_connection(&state, packet).await;
                }
                t = ticker.tick() => {
                    let millis = t.saturating_duration_since(last_t).as_millis();
                    last_t = t;
                    debug!("Tick at {:?} {:?}", t, millis);
                    Self::calculate_stats(&state, millis as u64).await;
                }
                else => break,
            }
        }
        info!("First tick at {:?}", last_t);
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
        trace!("update_connection id: {}", id);

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let mut process_connection: Option<ProcessConnection> = None;
        if let Ok(proc_conns) = get_process_connections(&state).await {
            process_connection = proc_conns.get(id.clone().as_str()).cloned();
        }
        let packet_connection = Registry::get::<PacketConnection>(id.clone());

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
                conn.packet_connection = conn.clone().packet_connection.or(packet_connection.clone());
            })
            .or_insert(Connection {
                id,
                local_addr,
                local_port,
                remote_addr,
                remote_port,
                protocol: packet.protocol,
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
                last_active: now,
                status: "active".to_string(),
                process_connection,
                packet_connection,
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

        for (_id, conn )in conns.iter_mut() {
            total_sent += conn.bytes_sent;
            total_recv += conn.bytes_recv;
            total_packets += conn.packets_sent + conn.packets_recv;

            conn.upload_speed = (conn.bytes_sent - conn.last_bytes_sent) * 1000 / millis;
            conn.download_speed = (conn.bytes_recv - conn.last_bytes_recv) * 1000 / millis;

            conn.last_bytes_sent = conn.bytes_sent;
            conn.last_bytes_recv = conn.bytes_recv;
        }

        // 计算速率（需要历史数据，简化处理）
        let stats = NetworkStats {
            timestamp: now,
            total_bytes_sent: total_sent,
            total_bytes_recv: total_recv,
            total_packets,
            active_connections: connections.len(),
            top_connections: connections.into_iter().take(10).collect(),
            upload_speed: 0, // 实际计算需要前一次数据
            download_speed: 0,
        };

        let mut history = state.stats_history.write().await;
        history.push(stats);

        // 只保留最近 5 分钟（300 秒）的历史
        if history.len() > 300 {
            history.remove(0);
        }
    }
}
