use libproc::libproc::proc_pid::{pidinfo};
use libproc::libproc::file_info::{ListFDs, ProcFDType};
use libproc::libproc::net_info::SocketFDInfo;
use libproc::bsd_info::BSDInfo;
use libproc::file_info::{pidfdinfo};
use libproc::net_info::InSIAddr;
use libproc::proc_pid::listpidinfo;
use libproc::processes::{pids_by_type, ProcFilter};

#[allow(dead_code)]
#[derive(Debug)]
pub struct ProcessNetInfo {
    pub pid: i32,
    pub name: String,
    pub sockets: Vec<SocketInfo>,
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct SocketInfo {
    pub fd: i32,
    pub protocol: String,
    pub local_addr: String,
    pub local_port: u16,
    pub remote_addr: String,
    pub remote_port: u16,
    pub state: String,
}
pub struct MacOSNetMonitor;

impl MacOSNetMonitor {
    /// 获取所有进程的网络连接
    pub fn get_all_connections() -> Vec<ProcessNetInfo> {
        let mut result = Vec::new();

        // 获取所有 PID
        let pids = pids_by_type(ProcFilter::All).unwrap_or_default();

        for pid in pids {
            if pid == 0 { continue; }

            if let Some(info) = Self::get_process_net_info(pid as i32) {
                if !info.sockets.is_empty() {
                    result.push(info);
                }
            }
        }

        result
    }

    /// 获取特定进程的网络信息
    pub fn get_process_net_info(pid: i32) -> Option<ProcessNetInfo> {
        // 获取进程基本信息
        let bsd_info = pidinfo::<BSDInfo>(pid, 0).ok()?;
        let name = String::from_utf8_lossy(&bsd_info.pbi_comm.map(|c| c as u8))
            .trim_end_matches('\0')
            .to_string();

        // 获取 FD 列表
        let fds = listpidinfo::<ListFDs>(pid, bsd_info.pbi_nfiles as usize).ok()?;

        let mut sockets = Vec::new();

        for fd_info in fds {
            // 只处理 socket
            if fd_info.proc_fdtype != ProcFDType::Socket as u32 {
                continue;
            }

            // 获取 socket 详情
            if let Some(sock) = Self::parse_socket(pid, fd_info.proc_fd) {
                sockets.push(sock);
            }
        }

        Some(ProcessNetInfo {
            pid,
            name,
            sockets,
        })
    }

    fn parse_socket(pid: i32, fd: i32) -> Option<SocketInfo> {
        let info = pidfdinfo::<SocketFDInfo>(pid, fd).ok()?;

        // 根据 kind 解析
        match info.psi.soi_kind {
            2 => Self::parse_tcp_socket(fd, &info),  // SOCKINFO_TCP
            3 => Self::parse_udp_socket(fd, &info),  // SOCKINFO_UDP
            _ => None,
        }
    }

    fn parse_tcp_socket(fd: i32, info: &SocketFDInfo) -> Option<SocketInfo> {
        let tcp = unsafe { &info.psi.soi_proto.pri_tcp };
        let ini = &tcp.tcpsi_ini;

        Some(SocketInfo {
            fd,
            protocol: "TCP".to_string(),
            local_addr: Self::format_addr(&ini.insi_laddr),
            local_port: u16::from_be(ini.insi_lport as u16),
            remote_addr: Self::format_addr(&ini.insi_faddr),
            remote_port: u16::from_be(ini.insi_fport as u16),
            state: Self::format_tcp_state(tcp.tcpsi_state),
        })
    }

    fn parse_udp_socket(fd: i32, info: &SocketFDInfo) -> Option<SocketInfo> {
        let udp = unsafe { &info.psi.soi_proto.pri_in };

        Some(SocketInfo {
            fd,
            protocol: "UDP".to_string(),
            local_addr: Self::format_addr(&udp.insi_laddr),
            local_port: u16::from_be(udp.insi_lport as u16),
            remote_addr: Self::format_addr(&udp.insi_faddr),
            remote_port: u16::from_be(udp.insi_fport as u16),
            state: "N/A".to_string(),
        })
    }

    fn format_addr(addr: &InSIAddr) -> String {
        unsafe {
            let in4 = &addr.ina_46;
            let ip = u32::from_be(in4.i46a_addr4.s_addr);  // 使用正确的字段名
            format!("{}.{}.{}.{}",
                    (ip >> 24) & 0xFF,
                    (ip >> 16) & 0xFF,
                    (ip >> 8) & 0xFF,
                    ip & 0xFF
            )
        }
    }

    fn format_tcp_state(state: i32) -> String {
        match state {
            1 => "CLOSED",
            2 => "LISTEN",
            3 => "SYN_SENT",
            4 => "SYN_RECEIVED",
            5 => "ESTABLISHED",
            6 => "CLOSE_WAIT",
            7 => "FIN_WAIT_1",
            8 => "CLOSING",
            9 => "LAST_ACK",
            10 => "FIN_WAIT_2",
            11 => "TIME_WAIT",
            _ => "UNKNOWN",
        }.to_string()
    }
}
