use std::collections::HashMap;
use std::sync::Arc;
use sysinfo::{Pid, System};
use crate::models::{AppState, ProcessInfo};

pub async fn get_processes(state: &Arc<AppState>) -> Result<HashMap<Pid, ProcessInfo>, String> {
    let mut processes = state.processes.write().await;
    let system = System::new_all();
    for process in system.processes().values() {
        processes.insert(process.pid(), ProcessInfo{
            name: process.name().to_string(),
            exe: process.exe().map(|p| p.to_string_lossy().to_string()).unwrap_or_default(),
            start_time: process.start_time(),
        });
    }

    Ok(processes.clone())
}

