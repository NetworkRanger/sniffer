use std::collections::HashMap;
use std::sync::Arc;
use tauri::State;
use tracing::info;
use crate::models::{AppState, Connection, ProcessGroup};

#[tauri::command]
pub async fn get_connections(
    state: State<'_, Arc<AppState>>,
    limit: Option<usize>,
    sort_by: Option<String>,
) -> Result<Vec<Connection>, String> {
    let conns = state.connections.read().await;
    let mut list: Vec<Connection> = conns.values().cloned().collect();

    match sort_by.as_deref().unwrap_or("speed") {
        "last_active"    => list.sort_by(|a, b| b.last_active.cmp(&a.last_active)),
        "bytes_sent"     => list.sort_by(|a, b| b.bytes_sent.cmp(&a.bytes_sent)),
        "bytes_recv"     => list.sort_by(|a, b| b.bytes_recv.cmp(&a.bytes_recv)),
        "upload_speed"   => list.sort_by(|a, b| b.upload_speed.cmp(&a.upload_speed)),
        "download_speed" => list.sort_by(|a, b| b.download_speed.cmp(&a.download_speed)),
        _ => {
            list.sort_by(|a, b| {
                let speed_a = a.upload_speed + a.download_speed;
                let speed_b = b.upload_speed + b.download_speed;
                let order = speed_b.cmp(&speed_a);
                if order.is_eq() {
                    let total_a = a.bytes_sent + a.bytes_recv;
                    let total_b = b.bytes_sent + b.bytes_recv;
                    let order = total_b.cmp(&total_a);
                    if order.is_eq() {
                        return b.last_active.cmp(&a.last_active);
                    }
                    return order;
                }
                order
            });
        }
    }

    let limit = limit.unwrap_or(2000);
    let result: Vec<Connection> = list.into_iter().take(limit).collect();
    info!("result len: {}", result.len());
    Ok(result)
}

#[tauri::command]
pub async fn stop_capture(state: State<'_, Arc<AppState>>) -> Result<(), String> {
    *state.running.write().await = false;
    Ok(())
}

#[tauri::command]
pub async fn get_capture_status(state: State<'_, Arc<AppState>>) -> Result<bool, String> {
    Ok(*state.running.read().await)
}

#[tauri::command]
pub async fn get_grouped_connections(
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<ProcessGroup>, String> {
    let conns = state.connections.read().await;
    let mut groups: HashMap<Option<u32>, Vec<Connection>> = HashMap::new();

    for conn in conns.values() {
        let pid = conn.process_connection.as_ref().and_then(|pc| pc.pid);
        groups.entry(pid).or_default().push(conn.clone());
    }

    let mut result: Vec<ProcessGroup> = groups.into_iter().map(|(pid, mut connections)| {
        connections.sort_by(|a, b| {
            let ta = a.bytes_sent + a.bytes_recv;
            let tb = b.bytes_sent + b.bytes_recv;
            tb.cmp(&ta)
        });
        let first_pc = connections.iter()
            .find_map(|c| c.process_connection.as_ref());
        ProcessGroup {
            pid,
            process_name: first_pc.and_then(|pc| pc.process_name.clone()),
            kernel_name: first_pc.and_then(|pc| pc.kernel_name.clone()),
            icon: first_pc.and_then(|pc| pc.icon.clone()),
            connections,
        }
    }).collect();

    // 按组总流量降序，无 pid 的排最后
    result.sort_by(|a, b| {
        match (a.pid, b.pid) {
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (Some(_), None) => std::cmp::Ordering::Less,
            _ => {
                let ta: u64 = a.connections.iter().map(|c| c.bytes_sent + c.bytes_recv).sum();
                let tb: u64 = b.connections.iter().map(|c| c.bytes_sent + c.bytes_recv).sum();
                tb.cmp(&ta)
            }
        }
    });

    info!("grouped connections: {} groups", result.len());
    Ok(result)
}
