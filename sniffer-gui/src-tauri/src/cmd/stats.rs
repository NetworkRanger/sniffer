use std::sync::Arc;
use tauri::State;
use crate::models::{AppState, NetworkStats};

#[tauri::command]
pub async fn get_network_stats(state: State<'_, Arc<AppState>>) -> Result<NetworkStats, String> {
    let history = state.stats_history.read().await;
    history.last()
        .cloned()
        .ok_or_else(|| "No stats available".to_string())
}
