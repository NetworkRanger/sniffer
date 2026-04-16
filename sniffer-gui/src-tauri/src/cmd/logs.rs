use serde::Serialize;
use std::io::{BufRead, BufReader};
use std::collections::VecDeque;

#[derive(Serialize, Clone)]
pub struct LogEntry {
    pub time: String,
    pub level: String,
    pub target: String,
    pub message: String,
}

#[tauri::command]
pub fn get_logs(limit: Option<usize>) -> Result<Vec<LogEntry>, String> {
    let log_dir = dirs::home_dir()
        .ok_or("no home dir")?
        .join(".sniffer");

    let log_file = std::fs::read_dir(&log_dir)
        .map_err(|e| e.to_string())?
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name().to_string_lossy().starts_with("app.log"))
        .max_by_key(|e| e.file_name());

    let path = match log_file {
        Some(f) => f.path(),
        None => return Ok(vec![]),
    };

    let limit = limit.unwrap_or(500);
    let file = std::fs::File::open(&path).map_err(|e| e.to_string())?;
    let reader = BufReader::new(file);

    // 只保留最后 limit 行
    let mut ring: VecDeque<String> = VecDeque::with_capacity(limit + 1);
    for line in reader.lines().filter_map(|l| l.ok()) {
        if ring.len() == limit {
            ring.pop_front();
        }
        ring.push_back(line);
    }

    let entries = ring.into_iter().filter_map(|line| {
        if line.len() < 25 { return None; }
        let (date_part, rest) = line.split_at(23);
        let rest = rest.trim_start();
        let (level, rest) = rest.split_once(' ')?;
        let level = level.trim().to_uppercase();
        if !["INFO","WARN","ERROR","DEBUG","TRACE"].contains(&level.as_str()) { return None; }
        let (target, message) = if let Some(pos) = rest.find(": ") {
            (rest[..pos].trim().to_string(), rest[pos+2..].trim().to_string())
        } else {
            (String::new(), rest.trim().to_string())
        };
        Some(LogEntry { time: date_part.trim().to_string(), level, target, message })
    }).collect();

    Ok(entries)
}
