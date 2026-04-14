mod models;
mod capture;
mod aggregator;
mod cache;
mod process_connection;
mod pcap_writer;
mod platform;
mod process;

extern crate sniffer;

use sniffer::utils::registry::Registry;

use tauri::State;
use std::sync::Arc;
use std::collections::HashMap;
use std::thread;
use pcap::PacketHeader;
use models::{AppState, NetworkStats, Connection};
use tracing_subscriber::fmt::time::OffsetTime;
use time::macros::{format_description, offset};
use tokio::runtime::Runtime;
use tracing::info;
use tracing_subscriber::{EnvFilter};
use sniffer::config::Config;
use crate::capture::PacketInfo;
use crate::process::get_processes;
use crate::process_connection::get_process_connections;

#[tauri::command]
async fn get_network_stats(state: State<'_, Arc<AppState>>) -> Result<NetworkStats, String> {
    let history = state.stats_history.read().await;
    history.last()
        .cloned()
        .ok_or_else(|| "No stats available".to_string())
}

#[tauri::command]
async fn get_connections(
    state: State<'_, Arc<AppState>>,
    limit: Option<usize>,
) -> Result<Vec<Connection>, String> {
    let conns = state.connections.read().await;
    let mut list: Vec<Connection> = conns.values().cloned().into_iter()
        // .filter(|conn| {
        //     // conn.clone().process_connection.is_some()  &&
        //         conn.clone().packet_connection.is_some_and(| packet_connection|
        //         packet_connection.domain.is_some()
        //     )
        // })
        .collect();

    // 按最后活跃时间排序
    list.sort_by(|a, b| {
        let speed_a = a.upload_speed + a.download_speed;
        let speed_b = b.upload_speed + b.download_speed;
        let order = speed_b.cmp(&speed_a); // 降序
        if order.is_eq() {
            let total_a = a.bytes_sent + a.bytes_recv;
            let total_b = b.bytes_sent + b.bytes_recv;
            let order = total_b.cmp(&total_a); // 降序
            if order.is_eq() {
                return b.last_active.cmp(&a.last_active);
            }
            return order;
        }
        order
    });

    let limit = limit.unwrap_or(50);
    let result: Vec<Connection> = list.into_iter().take(limit).collect();
    info!("result len: {}", result.len());

    Ok(result)
}

#[tauri::command]
async fn start_capture(
    state: Arc<AppState>,
) -> Result<(), String> {
    // 设置运行状态
    *state.running.write().await = true;

    // 创建通道
    let (tx, rx) = tokio::sync::mpsc::channel::<PacketInfo>(10000);
    let (pcap_tx, pcap_rx) = async_channel::unbounded::<(PacketHeader, Vec<u8>)>();

    // 启动 pcap 文件写入
    let mut pcap_writer = pcap_writer::PcapWriter::new(pcap_rx);
    let pcap_handle = thread::Builder::new()
        .name("pcap_writer".to_string())
        .spawn(move || {
            let _ = pcap_writer.start();
        }).expect("failed to start pcap writer");

    // 启动抓包引擎
    let mut engine = capture::CaptureEngine::new(tx, pcap_tx);
    let state_clone = state.clone();
    thread::Builder::new()
        .name("capture".to_string())
        .spawn(move || {
            engine.start(state_clone);
        }).expect("failed to start capture engine");

    // 启动聚合器
    aggregator::Aggregator::start(rx, state).await;

    let _ = pcap_handle.join().ok();

    Ok(())
}

#[tauri::command]
async fn stop_capture(state: State<'_, Arc<AppState>>) -> Result<(), String> {
    *state.running.write().await = false;
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let state = Arc::new(AppState {
        processes: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        process_connections: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        connections: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        stats_history: Arc::new(tokio::sync::RwLock::new(Vec::new())),
        running: Arc::new(tokio::sync::RwLock::new(false)),
    });

    // 定义时间格式（年-月-日T时:分:秒.毫秒）
    let time_fmt = format_description!(
        "[year]-[month]-[day] [hour]:[minute]:[second].[subsecond digits:3]"
    );

    // 构造一个偏移 +8:00（北京时间）的时间格式器
    let timer = OffsetTime::new(offset!(+8:00), time_fmt);

    // 安装 tracing-subscriber，并用自定义时间格式
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info"))
        )
        .with_timer(timer)
        .init();

    let state_clone = state.clone();
    thread::spawn(move || {
        let config = Config::new();
        Registry::set("config", config.clone(), Some(0u64));

        // 创建运行时
        let rt = Runtime::new().unwrap();
        // 阻塞等待异步完成
        let _result = rt.block_on(async move {
            let _ = get_processes(&state_clone).await.unwrap();
            let _ = get_process_connections(&state_clone).await.unwrap();
            let _ = start_capture(state_clone).await;
        });
    });

    tauri::Builder::default()
        .manage(state)
       .invoke_handler(tauri::generate_handler![
            get_network_stats,
            get_connections,
            stop_capture,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
