use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use tokio::net::TcpListener;
use tokio::sync::broadcast;
use tokio_tungstenite::accept_async;
use tokio_tungstenite::tungstenite::Message;
use futures_util::SinkExt;
use tracing::info;
use futures_util::StreamExt;

const HISTORY_SIZE: usize = 2000;

pub type LogHistory = Arc<Mutex<VecDeque<String>>>;

pub fn new_history() -> LogHistory {
    Arc::new(Mutex::new(VecDeque::with_capacity(HISTORY_SIZE)))
}

pub async fn start_log_ws(tx: broadcast::Sender<String>, history: LogHistory) {
    let listener = TcpListener::bind("127.0.0.1:9999").await
        .expect("failed to bind log websocket");
    info!("Log WebSocket listening on ws://127.0.0.1:9999");

    loop {
        let Ok((stream, _)) = listener.accept().await else { continue };
        let mut rx = tx.subscribe();
        let history = history.clone();
        tokio::spawn(async move {
            let Ok(ws) = accept_async(stream).await else { return };
            let (mut sink, _) = ws.split();

            // 先发送历史日志
            let snapshot: Vec<String> = history.lock().unwrap().iter().cloned().collect();
            for msg in snapshot {
                if sink.send(Message::Text(msg.into())).await.is_err() { return; }
            }

            // 再实时推送新日志
            while let Ok(msg) = rx.recv().await {
                if sink.send(Message::Text(msg.into())).await.is_err() {
                    break;
                }
            }
        });
    }
}
