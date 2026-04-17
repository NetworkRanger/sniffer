use tokio::sync::broadcast;
use tracing::Level;
use tracing_subscriber::Layer;
use serde_json::json;
use crate::log_ws::LogHistory;

pub struct BroadcastLayer {
    pub tx: broadcast::Sender<String>,
    pub history: LogHistory,
}

impl<S> Layer<S> for BroadcastLayer
where
    S: tracing::Subscriber,
{
    fn on_event(
        &self,
        event: &tracing::Event<'_>,
        _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        let meta = event.metadata();

        // 只保留本项目日志
        if !meta.target().contains("sniffer") {
            return;
        }
        let level = match *meta.level() {
            Level::ERROR => "ERROR",
            Level::WARN  => "WARN",
            Level::INFO  => "INFO",
            Level::DEBUG => "DEBUG",
            Level::TRACE => "TRACE",
        };

        // 收集字段
        let mut visitor = MessageVisitor(String::new());
        event.record(&mut visitor);

        let now = {
            use std::time::{SystemTime, UNIX_EPOCH};
            let secs = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis();
            // 格式化为 HH:mm:ss.SSS (简单实现)
            let ms = secs % 1000;
            let s  = (secs / 1000) % 60;
            let m  = (secs / 60000) % 60;
            let h  = (secs / 3600000) % 24;
            // 加8小时时区偏移
            let h = (h + 8) % 24;
            format!("{:02}:{:02}:{:02}.{:03}", h, m, s, ms)
        };

        let msg = json!({
            "time":    now,
            "level":   level,
            "target":  meta.target(),
            "message": visitor.0,
        })
        .to_string();

        let _ = self.tx.send(msg.clone());

        // 写入历史缓冲
        let mut h = self.history.lock().unwrap();
        if h.len() == h.capacity() { h.pop_front(); }
        h.push_back(msg);
    }
}

struct MessageVisitor(String);

impl tracing::field::Visit for MessageVisitor {
    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        if field.name() == "message" {
            self.0 = value.to_string();
        }
    }
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            self.0 = format!("{:?}", value);
        }
    }
}
