mod models;
mod capture;
mod aggregator;
mod cache;
mod process_connection;
extern crate sniffer;

use core::net;
use sniffer::utils::get_mac_by_name;
use sniffer::utils::registry::Registry;

use tauri::State;
use std::sync::Arc;
use std::collections::HashMap;
use std::thread;
use models::{AppState, NetworkStats, Connection};
use pcap::{Device};
use tracing_subscriber::fmt::time::OffsetTime;
use time::macros::{format_description, offset};
use tokio::runtime::Runtime;
use tracing::info;
use tracing_subscriber::{EnvFilter};
use sniffer::config::Config;
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
        let total_a = a.upload_speed + a.download_speed;
        let total_b = b.upload_speed + b.download_speed;
        let order = total_b.cmp(&total_a); // 降序
        if order.is_eq() {
            return b.last_active.cmp(&a.last_active);
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
    interface: String,
    state: Arc<AppState>,
) -> Result<(), String> {
    // 设置运行状态
    *state.running.write().await = true;

    // 创建通道
    let (tx, rx) = tokio::sync::mpsc::channel(10000);

    // 启动抓包引擎
    let engine = capture::CaptureEngine::new(tx);
    let state_clone = state.clone();
    thread::Builder::new()
        .name("capture".to_string())
        .spawn(move || {
            engine.start(interface, state_clone);
        }).expect("failed to start capture engine");

    // 启动聚合器
    aggregator::Aggregator::start(rx, state).await;

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
        let device_list = Device::list().expect("device list failed");
        // 查找默认设备
        let device = device_list
            .iter()
            .filter(|d| {
                d.flags.connection_status == pcap::ConnectionStatus::Connected && d.addresses.len() > 0
            })
            .next()
            .expect("no device available");

        info!("Using device: {}", device.name);

        let ip = device.addresses.iter()
            .filter(|address| {
                matches!(address.addr, net::IpAddr::V4(_))
            })
            .next().unwrap().addr.to_string();
        info!("ip: {}", ip);
        Registry::set("ip", ip.clone(), Some(0u64));

        let mac = get_mac_by_name(device.name.as_str()).unwrap();
        info!("Using device mac {}", mac);
        Registry::set("mac", mac.clone(), Some(0u64));
        let config = Config::new();
        Registry::set("config", config.clone(), Some(0u64));

        // 创建运行时
        let rt = Runtime::new().unwrap();
        // 阻塞等待异步完成
        let _result = rt.block_on(async move {
            let _ = get_process_connections(&state_clone).await.unwrap();
            let _ = start_capture(device.name.clone(), state_clone).await;
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
