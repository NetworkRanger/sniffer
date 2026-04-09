use crate::cache::get_process_icon_by_path;
use crate::models::AppState;
use netstat2::{get_sockets_info, AddressFamilyFlags, ProtocolFlags, ProtocolSocketInfo, TcpState};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use sysinfo::{Pid, System};

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

// #[allow(dead_code)]
// #[derive(Debug, Clone, Serialize, Deserialize)]
// pub struct Connection {
//     pub id: String,
//     pub src_ip: String,
//     pub src_port: u16,
//     pub dst_ip: String,
//     pub dst_port: u16,
//     pub protocol: String,
//     pub domain: Option<String>,
//     pub path: Option<String>,
//     pub start: Instant,
//     pub now: Instant,
//     pub up_bytes: u64,
//     pub down_bytes: u64,
//     pub is_init: bool,
//     pub process_connection: Option<ProcessConnection>,
// }

// 将 netstat2 的 TCP 状态转换为字符串
fn tcp_state_to_string(state: TcpState) -> &'static str {
    match state {
        TcpState::Established => "ESTABLISHED",
        TcpState::SynSent => "SYN_SENT",
        TcpState::SynReceived => "SYN_RECV",
        TcpState::FinWait1 => "FIN_WAIT1",
        TcpState::FinWait2 => "FIN_WAIT2",
        TcpState::TimeWait => "TIME_WAIT",
        TcpState::Closed => "CLOSED",
        TcpState::CloseWait => "CLOSE_WAIT",
        TcpState::LastAck => "LAST_ACK",
        TcpState::Listen => "LISTEN",
        TcpState::Closing => "CLOSING",
        TcpState::DeleteTcb => "DELETE_TCB",
        TcpState::Unknown => "UNKNOWN",
    }
}

// ==================== 网络连接相关函数 ====================

// 获取系统网络连接列表
pub async fn get_process_connections(
    state: &Arc<AppState>,
) -> Result<HashMap<String, ProcessConnection>, String> {
    let mut connections = state.process_connections.write().await;

    // 设置地址族标志 (IPv4 和 IPv6)
    let af_flags = AddressFamilyFlags::IPV4 | AddressFamilyFlags::IPV6;
    // 设置协议标志 (TCP 和 UDP)
    let proto_flags = ProtocolFlags::TCP | ProtocolFlags::UDP;

    // 获取网络连接信息
    let sockets_info = get_sockets_info(af_flags, proto_flags).map_err(|e| e.to_string())?;

    // 创建系统信息实例以获取进程名称
    let system = System::new_all();

    // 预先构建进程信息映射表，避免重复查询
    let mut process_map: HashMap<Pid, &sysinfo::Process> =
        HashMap::new();
    for process in system.processes().values() {
        process_map.insert(process.pid(), process);
    }

    for si in sockets_info {
        match si.protocol_socket_info {
            ProtocolSocketInfo::Tcp(tcp_si) => {
                let protocol = "TCP".to_string();

                // 处理本地地址和端口
                let local_addr = tcp_si.local_addr.to_string();
                let local_port = tcp_si.local_port;

                // 处理远程地址和端口
                let remote_addr = tcp_si.remote_addr.to_string();
                let remote_port = tcp_si.remote_port;

                // 获取连接状态
                let state = tcp_state_to_string(tcp_si.state);

                // 获取 PID (如果有)
                let pid = if !si.associated_pids.is_empty() {
                    Some(si.associated_pids[0]) // 取第一个关联的PID
                } else {
                    None
                };

                // 根据 PID 获取进程信息（从预构建的映射表中获取）
                let process_info =
                    pid.and_then(|pid_val| process_map.get(&(pid_val as usize).into()));

                // 根据 PID 获取进程名称
                let process_name = if let Some(process) = process_info {
                    Some(process.name().to_string())
                } else if pid.is_some() {
                    // 如果无法获取进程信息，可能是内核进程或权限不足，显示特殊标识
                    Some("[KERNEL]".to_string())
                } else {
                    None
                };

                // 获取进程图标 - 使用统一的缓存机制
                let icon = if let Some(process) = process_info {
                    if let Some(exe_path) = process.exe() {
                        let exe_path_str = exe_path.to_string_lossy();
                        get_process_icon_by_path(&exe_path_str)
                    } else {
                        None
                    }
                } else {
                    None
                };

                // 获取进程启动时间
                let start_time = if let Some(process) = process_info {
                    Some(process.start_time())
                } else if pid.is_some() {
                    // 内核进程的时间信息可能不可用，返回0
                    Some(0)
                } else {
                    None
                };

                let id = format!(
                    "{}://{}:{}@{}:{}",
                    protocol, local_addr, local_port, remote_addr, remote_port
                );
                connections.entry(id).or_insert(ProcessConnection {
                    protocol,
                    local_addr,
                    local_port,
                    remote_addr,
                    remote_port,
                    state: state.to_string(),
                    pid,
                    process_name,
                    icon,
                    start_time,
                    fill_column: String::new(),
                });
            }
            ProtocolSocketInfo::Udp(udp_si) => {
                let protocol = "UDP".to_string();

                // 处理本地地址和端口
                let local_addr = udp_si.local_addr.to_string();
                let local_port = udp_si.local_port;

                // UDP 没有远程地址和端口的概念，通常设置为通配符
                let remote_addr = "*".to_string();
                let remote_port = 0;

                // UDP 没有连接状态，设置为 UNCONN
                let state = "UNCONN".to_string();

                // 获取 PID (如果有)
                let pid = if !si.associated_pids.is_empty() {
                    Some(si.associated_pids[0]) // 取第一个关联的PID
                } else {
                    None
                };

                // 根据 PID 获取进程信息（从预构建的映射表中获取）
                let process_info =
                    pid.and_then(|pid_val| process_map.get(&(pid_val as usize).into()));

                // 根据 PID 获取进程名称
                let process_name = if let Some(process) = process_info {
                    Some(process.name().to_string())
                } else if pid.is_some() {
                    // 如果无法获取进程信息，可能是内核进程或权限不足，显示特殊标识
                    Some("[KERNEL]".to_string())
                } else {
                    None
                };

                // 获取进程图标 - 使用统一的缓存机制
                let icon = if let Some(process) = process_info {
                    if let Some(exe_path) = process.exe() {
                        let exe_path_str = exe_path.to_string_lossy();
                        get_process_icon_by_path(&exe_path_str)
                    } else {
                        None
                    }
                } else {
                    None
                };

                // 获取进程启动时间
                let start_time = if let Some(process) = process_info {
                    Some(process.start_time())
                } else if pid.is_some() {
                    // 内核进程的时间信息可能不可用，返回0
                    Some(0)
                } else {
                    None
                };

                let id = format!(
                    "{}://{}:{}@{}:{}",
                    protocol, local_addr, local_port, remote_addr, remote_port
                );
                connections.entry(id).or_insert(ProcessConnection {
                    protocol,
                    local_addr,
                    local_port,
                    remote_addr,
                    remote_port,
                    state: state.to_string(),
                    pid,
                    process_name,
                    icon,
                    start_time,
                    fill_column: String::new(),
                });
            }
        }
    }

    Ok(connections.clone())
}
