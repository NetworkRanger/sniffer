use crate::cache::get_process_icon_by_path;
use crate::models::{AppState, ConnectionKey, ProcessConnection};
use netstat2::{get_sockets_info, AddressFamilyFlags, ProtocolFlags, ProtocolSocketInfo, TcpState};
use std::collections::HashMap;
use std::sync::Arc;
#[cfg(target_os = "macos")]
use crate::platform::macos::process_connection::MacOSNetMonitor;

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
pub async fn get_process_connections(
    state: &Arc<AppState>,
) -> Result<HashMap<ConnectionKey, ProcessConnection>, String> {
    #[cfg(target_os = "macos")]
    {
        get_macos_process_connections(state).await
    }
    #[cfg(not(any(target_os = "macos")))]
    {
        get_platform_process_connections(state).await
    }
}

// 获取系统网络连接列表
#[cfg(target_os = "macos")]
pub async fn get_macos_process_connections(
    state: &Arc<AppState>,
) -> Result<HashMap<ConnectionKey, ProcessConnection>, String> {
    let mut connections = state.process_connections.write().await;
    let all_connections = MacOSNetMonitor::get_all_connections();
    let process_map = state.processes.read().await;
    for proc_info in all_connections {
        for sock in &proc_info.sockets {
            let pid = proc_info.pid.clone() as u32;
            let process_info = process_map.get(&(pid as usize).into());
            let process_name = process_info.map(|p| p.name.clone());
            let kernel_name = Some(proc_info.name.clone());
            let icon = process_info.and_then(|p| get_process_icon_by_path(&p.exe));
            let start_time = process_info.map(|p| p.start_time);

            let key = ConnectionKey {
                protocol: sock.protocol.clone().to_lowercase(),
                local_addr: sock.local_addr.clone(),
                local_port: sock.local_port.clone(),
                remote_addr: sock.remote_addr.clone(),
                remote_port: sock.remote_port.clone(),
            };
            let process_connection = ProcessConnection {
                protocol: sock.protocol.clone(),
                local_addr: sock.local_addr.clone(),
                local_port: sock.local_port.clone(),
                remote_addr: sock.remote_addr.clone(),
                remote_port: sock.remote_port.clone(),
                state: sock.state.clone(),
                pid: Some(pid),
                process_name: process_name.clone(),
                kernel_name: kernel_name.clone(),
                icon: icon.clone(),
                start_time,
                fill_column: String::new(),
            };
            connections.entry(key)
                .and_modify(|c| {
                    if c.pid.is_none() {
                        c.pid = Some(pid);
                        c.process_name = process_name;
                        c.kernel_name = kernel_name;
                        c.icon = icon;
                        c.start_time = start_time;
                    }
                })
                .or_insert(process_connection);
        }
    }


    Ok(connections.clone())
}

// 获取系统网络连接列表
pub async fn get_platform_process_connections(
    state: &Arc<AppState>,
) -> Result<HashMap<ConnectionKey, ProcessConnection>, String> {
    let mut connections = state.process_connections.write().await;
    let process_map = state.processes.read().await;

    // 设置地址族标志 (IPv4 和 IPv6)
    let af_flags = AddressFamilyFlags::IPV4 | AddressFamilyFlags::IPV6;
    // 设置协议标志 (TCP 和 UDP)
    let proto_flags = ProtocolFlags::TCP | ProtocolFlags::UDP;

    // 获取网络连接信息
    let sockets_info = get_sockets_info(af_flags, proto_flags).map_err(|e| e.to_string())?;

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
                    Some(process.name.clone())
                } else if pid.is_some() {
                    // 如果无法获取进程信息，可能是内核进程或权限不足，显示特殊标识
                    Some("[KERNEL]".to_string())
                } else {
                    None
                };

                let icon = process_info.and_then(|p| get_process_icon_by_path(&p.exe));
                let start_time = process_info.map(|p| p.start_time);

                let key = ConnectionKey {
                    protocol: protocol.clone().to_lowercase(),
                    local_addr: local_addr.clone(),
                    local_port: local_port.clone(),
                    remote_addr: remote_addr.clone(),
                    remote_port: remote_port.clone(),
                };
                connections.entry(key).or_insert(ProcessConnection {
                    protocol,
                    local_addr,
                    local_port,
                    remote_addr,
                    remote_port,
                    state: state.to_string(),
                    pid,
                    process_name,
                    kernel_name: None,
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
                    Some(process.name.clone())
                } else if pid.is_some() {
                    // 如果无法获取进程信息，可能是内核进程或权限不足，显示特殊标识
                    Some("[KERNEL]".to_string())
                } else {
                    None
                };

                let icon = process_info.and_then(|p| get_process_icon_by_path(&p.exe));
                let start_time = process_info.map(|p| p.start_time);

                let key = ConnectionKey {
                    protocol: protocol.clone().to_lowercase(),
                    local_addr: local_addr.clone(),
                    local_port: local_port.clone(),
                    remote_addr: remote_addr.clone(),
                    remote_port: remote_port.clone(),
                };
                connections.entry(key).or_insert(ProcessConnection {
                    protocol,
                    local_addr,
                    local_port,
                    remote_addr,
                    remote_port,
                    state: state.to_string(),
                    pid,
                    process_name,
                    kernel_name: None,
                    icon,
                    start_time,
                    fill_column: String::new(),
                });
            }
        }
    }

    Ok(connections.clone())
}
