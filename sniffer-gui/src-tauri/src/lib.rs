mod models;
mod capture;
mod aggregator;
mod cache;
mod process_connection;
mod pcap_writer;
mod platform;
mod process;
mod cmd;
mod log_layer;
mod log_ws;

extern crate sniffer;

use sniffer::utils::registry::Registry;

use std::sync::Arc;
use std::collections::HashMap;
use std::thread;
use pcap::PacketHeader;
use models::AppState;
use tracing_subscriber::fmt::time::OffsetTime;
use time::macros::{format_description, offset};
use tokio::runtime::Runtime;
use tokio::sync::broadcast;
use tracing_subscriber::EnvFilter;
use sniffer::config::Config;
use tauri::State;
use crate::capture::PacketInfo;
use crate::process::get_processes;
use crate::process_connection::get_process_connections;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, Layer};
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use std::io;
use tracing_appender::non_blocking::WorkerGuard;

use cmd::stats::get_network_stats;
use cmd::connections::{get_connections, stop_capture, get_capture_status};
use cmd::logs::get_logs;

async fn do_start_capture(state: Arc<AppState>) -> Result<(), String> {
    let (tx, rx) = tokio::sync::mpsc::channel::<PacketInfo>(10000);
    let (pcap_tx, pcap_rx) = async_channel::unbounded::<(PacketHeader, Vec<u8>)>();

    let mut pcap_writer = pcap_writer::PcapWriter::new(pcap_rx);
    let pcap_handle = thread::Builder::new()
        .name("pcap_writer".to_string())
        .spawn(move || { let _ = pcap_writer.start(); })
        .expect("failed to start pcap writer");

    let mut engine = capture::CaptureEngine::new(tx, pcap_tx);
    let state_clone = state.clone();
    thread::Builder::new()
        .name("capture".to_string())
        .spawn(move || { engine.start(state_clone); })
        .expect("failed to start capture engine");

    aggregator::Aggregator::start(rx, state).await;
    let _ = pcap_handle.join().ok();
    Ok(())
}

#[tauri::command]
async fn start_capture(state: State<'_, Arc<AppState>>) -> Result<(), String> {
    let mut running = state.running.write().await;
    if *running {
        return Err("capture already running".to_string());
    }
    *running = true;
    drop(running);

    let state_clone = state.inner().clone();
    tokio::spawn(async move {
        let _ = do_start_capture(state_clone).await;
    });
    Ok(())
}

fn init_logging(ws_tx: broadcast::Sender<String>, history: log_ws::LogHistory) -> WorkerGuard {
    let cache_dir = dirs::home_dir().map(|home| home.join(".sniffer"))
        .unwrap().to_string_lossy().into_owned();

    let file_appender = RollingFileAppender::new(Rotation::DAILY, &cache_dir, "app.log");
    let (file_writer, guard) = tracing_appender::non_blocking(file_appender);

    let time_fmt = format_description!(
        "[year]-[month]-[day] [hour]:[minute]:[second].[subsecond digits:3]"
    );
    let timer = OffsetTime::new(offset!(+8:00), time_fmt);

    let stdout_layer = tracing_subscriber::fmt::layer()
        .with_writer(io::stdout)
        .with_timer(timer.clone())
        .with_ansi(true)
        .with_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()));

    let file_layer = tracing_subscriber::fmt::layer()
        .with_timer(timer)
        .with_writer(file_writer)
        .with_ansi(false)
        .with_filter(EnvFilter::new("debug"));

    let broadcast_layer = log_layer::BroadcastLayer { tx: ws_tx, history }
        .with_filter(EnvFilter::new("debug"));

    tracing_subscriber::registry()
        .with(file_layer)
        .with(stdout_layer)
        .with(broadcast_layer)
        .init();

    guard
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

    // broadcast channel: 日志推送给 WebSocket 客户端
    let (ws_tx, _) = broadcast::channel::<String>(4096);
    let ws_tx_clone = ws_tx.clone();
    let history = log_ws::new_history();
    let history_clone = history.clone();

    let _guard = init_logging(ws_tx, history);

    // 启动 WebSocket 日志服务
    thread::Builder::new()
        .name("log_ws".to_string())
        .spawn(move || {
            let rt = Runtime::new().unwrap();
            rt.block_on(log_ws::start_log_ws(ws_tx_clone, history_clone));
        })
        .expect("failed to start log ws");

    let state_clone = state.clone();
    thread::spawn(move || {
        let config = Config::new();
        Registry::set("config", config.clone(), Some(0u64));

        let rt = Runtime::new().unwrap();
        let _result = rt.block_on(async move {
            let _ = get_processes(&state_clone).await.unwrap();
            let _ = get_process_connections(&state_clone).await.unwrap();
            *state_clone.running.write().await = true;
            let _ = do_start_capture(state_clone).await;
        });
    });

    tauri::Builder::default()
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            get_network_stats,
            get_connections,
            get_logs,
            start_capture,
            stop_capture,
            get_capture_status,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
