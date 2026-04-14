use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use sysinfo::{Pid};
use tokio::sync::RwLock;
use sniffer::packet::Connection as PacketConnection;

#[derive(Serialize, Clone, Debug)]
pub struct Connection {
    pub id: String,           // 唯一标识: src_ip:src_port-dst_ip:dst_port
    pub local_addr: String,
    pub local_port: u16,
    pub remote_addr: String,
    pub remote_port: u16,
    pub protocol: String,     // TCP/UDP/ICMP
    
    pub bytes_sent: u64,      // 本连接发送字节
    pub bytes_recv: u64,      // 本连接接收字节
    pub packets_sent: u32,
    pub packets_recv: u32,
    
    pub last_bytes_sent: u64,
    pub last_bytes_recv: u64,

    pub upload_speed: u64,
    pub download_speed: u64,
    
    pub start_time: u64,      // Unix timestamp
    pub last_active: u64,     // 最后活跃时间
    pub status: String,       // active/closed/idle
    pub process_connection: Option<ProcessConnection>,
    pub packet_connection: Option<PacketConnection>,
}

#[derive(Serialize, Clone, Debug)]
pub struct NetworkStats {
    pub timestamp: u64,
    pub total_bytes_sent: u64,
    pub total_bytes_recv: u64,
    pub total_packets: u32,
    pub active_connections: usize,
    pub top_connections: Vec<Connection>,  // 按流量排序 Top 10
    pub upload_speed: u64,    // bytes/s
    pub download_speed: u64,  // bytes/s
}

#[derive(Debug, Clone)]
#[derive(Eq, Hash, PartialEq)]
pub struct ConnectionKey {
    pub protocol: String,
    pub local_addr: String,
    pub local_port: u16,
    pub remote_addr: String,
    pub remote_port: u16,
}

#[derive(Debug, Clone)]
pub struct ProcessInfo {
    pub name: String,
    pub exe: String,
    pub start_time: u64,
}

// 应用状态
pub struct AppState {
    pub processes: Arc<RwLock<HashMap<Pid, ProcessInfo>>>,
    pub process_connections: Arc<RwLock<HashMap<ConnectionKey, ProcessConnection>>>,
    pub connections: Arc<RwLock<HashMap<String, Connection>>>,
    pub stats_history: Arc<RwLock<Vec<NetworkStats>>>,  // 最近5分钟数据
    pub running: Arc<RwLock<bool>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessConnection {
    pub protocol: String,
    pub local_addr: String,
    pub local_port: u16,
    pub remote_addr: String,
    pub remote_port: u16,
    pub state: String,
    pub pid: Option<u32>,
    pub process_name: Option<String>,
    pub icon: Option<String>,    // Base64 encoded icon data
    pub start_time: Option<u64>, // Process start time in seconds since Unix epoch
    pub fill_column: String,     // Fill column for filling remaining space
}