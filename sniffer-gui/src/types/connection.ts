// 网络连接类型定义
export interface ProcessConnection {
    protocol: string;
    local_addr: string;
    local_port: number;
    remote_addr: string;
    remote_port: number;
    state: string;
    pid: number | null;
    process_name: string | null;
    kernel_name: string | null;  // libproc pbi_comm，内核短名（≤15字符）
    icon: string | null; // Base64 encoded icon data
    start_time: number | null; // Process start time in seconds since Unix epoch
    fill_column: string; // Fill column for filling remaining space
}
export interface PacketConnection {
    id: string;
    src_ip: string;
    src_port: number;
    dst_ip: string;
    dst_port: number;
    protocol: string;
    domain: string | null;
    path: string | null;
    start: string;
    now: string;
    up_bytes: number;
    down_bytes: number;
    is_init: boolean;
}
export interface Connection {
    id: string;
    local_addr: string;
    local_port: number;
    remote_addr: string;
    remote_port: number;
    protocol: string;
    app_protocol: string;
    domain: string | null;
    path: string | null;
    bytes_sent: number;
    bytes_recv: number;
    packets_sent: number;
    packets_recv: number;
    upload_speed: number;
    download_speed: number;
    start_time: number;
    start_time_us: number;   // microseconds from pcap header (tv_sec * 1_000_000 + tv_usec)
    last_active: number;
    status: string;
    process_connection: ProcessConnection | null;
    packet_connection: PacketConnection | null;
    isNew?: boolean; // 标记是否为新链接
    hasChanged?: boolean; // 标记连接状态是否有变化
    isDelted?: boolean; // 标记是否为即将删除的连接
}

export interface ProcessGroup {
    pid: number | null;
    process_name: string | null;
    kernel_name: string | null;
    icon: string | null;
    connections: Connection[];
}

// 树形表格行：group 行或 connection 行
export interface TreeRow extends Connection {
    _isGroup?: boolean;
    children?: Connection[];
    hasChildren?: boolean;
}

// 进程详情类型定义
export interface ProcessDetails {
    pid: number;
    name: string;
    command_line: string;
    executable_path: string;
    memory_usage: number;
    cpu_usage: number;
    parent_pid: number | null;
    start_time: number;
}