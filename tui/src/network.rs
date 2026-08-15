use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionStatus {
    Disconnected,
    Connecting,
    Connected,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OnlinePlayerInfo {
    pub name: String,
    pub level: u8,
    pub class: String,
    pub race: String,
    pub room: String,
    pub idle_secs: u64,
    pub protocol: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpadeTelemetry {
    pub status: String,
    pub uptime_secs: u64,
    pub memory_used_bytes: u64,
    pub total_memory_bytes: u64,
    pub wal_size_bytes: u64,
    pub dirty_entities: usize,
    pub pulse_drift_ms: f64,
    pub room_count: usize,
    pub mob_count: usize,
    pub item_count: usize,
    pub game_time: String,
    pub season: String,
    pub weather: String,
    pub rhai_timers: usize,
    pub players: Vec<OnlinePlayerInfo>,
    #[serde(default)]
    pub logs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action", content = "payload")]
pub enum SpadeControlCommand {
    Gecho { message: String },
    Reboot { delay_secs: Option<u64> },
    Kick { player_name: String },
    Ping,
}

pub struct SpadeNetworkClient {
    status: Arc<std::sync::Mutex<ConnectionStatus>>,
    ping_ms: Arc<std::sync::Mutex<u64>>,
    telemetry_rx: mpsc::UnboundedReceiver<SpadeTelemetry>,
    log_rx: mpsc::UnboundedReceiver<String>,
    cmd_tx: mpsc::UnboundedSender<SpadeControlCommand>,
    shutdown_flag: Arc<AtomicBool>,
}

impl SpadeNetworkClient {
    pub fn connect(host: String, port: u16, api_key: Option<String>) -> Self {
        let status = Arc::new(std::sync::Mutex::new(ConnectionStatus::Connecting));
        let ping_ms = Arc::new(std::sync::Mutex::new(0));
        let (telemetry_tx, telemetry_rx) = mpsc::unbounded_channel();
        let (log_tx, log_rx) = mpsc::unbounded_channel();
        let (cmd_tx, mut cmd_rx) = mpsc::unbounded_channel::<SpadeControlCommand>();
        let shutdown_flag = Arc::new(AtomicBool::new(false));

        let status_clone = status.clone();
        let ping_clone = ping_ms.clone();
        let shutdown_clone = shutdown_flag.clone();

        tokio::spawn(async move {
            let url_str = format!("ws://{}:{}/ws/spade", host, port);

            loop {
                if shutdown_clone.load(Ordering::Relaxed) {
                    break;
                }

                *status_clone.lock().unwrap() = ConnectionStatus::Connecting;

                let mut req_builder = http::Request::builder().uri(&url_str);

                if let Some(ref key) = api_key {
                    req_builder = req_builder.header("Authorization", format!("Bearer {}", key));
                }

                let request = match req_builder.body(()) {
                    Ok(req) => req,
                    Err(_) => {
                        *status_clone.lock().unwrap() = ConnectionStatus::Disconnected;
                        tokio::time::sleep(Duration::from_secs(3)).await;
                        continue;
                    }
                };

                match connect_async(request).await {
                    Ok((ws_stream, _)) => {
                        *status_clone.lock().unwrap() = ConnectionStatus::Connected;
                        let (mut write, mut read) = ws_stream.split();
                        let ping_start = Instant::now();

                        let mut ping_interval = tokio::time::interval(Duration::from_secs(3));

                        loop {
                            tokio::select! {
                                _ = ping_interval.tick() => {
                                    let ping_cmd = serde_json::to_string(&SpadeControlCommand::Ping).unwrap_or_default();
                                    if write.send(Message::Text(ping_cmd.into())).await.is_err() {
                                        break;
                                    }
                                }
                                Some(cmd) = cmd_rx.recv() => {
                                    if let Ok(json_str) = serde_json::to_string(&cmd) {
                                        if write.send(Message::Text(json_str.into())).await.is_err() {
                                            break;
                                        }
                                    }
                                }
                                msg = read.next() => {
                                    match msg {
                                        Some(Ok(Message::Text(txt))) => {
                                            if txt.trim() == "pong" {
                                                let elapsed = ping_start.elapsed().as_millis() as u64;
                                                *ping_clone.lock().unwrap() = elapsed;
                                                continue;
                                            }

                                            if let Ok(telemetry) = serde_json::from_str::<SpadeTelemetry>(&txt) {
                                                for log_line in &telemetry.logs {
                                                    let _ = log_tx.send(log_line.clone());
                                                }
                                                let _ = telemetry_tx.send(telemetry);
                                            } else if let Ok(val) = serde_json::from_str::<serde_json::Value>(&txt) {
                                                if let Some(log_msg) = val.get("log").and_then(|l| l.as_str()) {
                                                    let _ = log_tx.send(log_msg.to_string());
                                                }
                                            }
                                        }
                                        Some(Ok(Message::Close(_))) | None | Some(Err(_)) => break,
                                        _ => {}
                                    }
                                }
                            }
                        }

                        *status_clone.lock().unwrap() = ConnectionStatus::Disconnected;
                    }
                    Err(_) => {
                        *status_clone.lock().unwrap() = ConnectionStatus::Disconnected;
                    }
                }

                tokio::time::sleep(Duration::from_secs(3)).await;
            }
        });

        Self {
            status,
            ping_ms,
            telemetry_rx,
            log_rx,
            cmd_tx,
            shutdown_flag,
        }
    }

    pub fn status(&self) -> ConnectionStatus {
        *self.status.lock().unwrap()
    }

    pub fn ping_ms(&self) -> u64 {
        *self.ping_ms.lock().unwrap()
    }

    pub fn poll_telemetry(&mut self) -> Option<SpadeTelemetry> {
        self.telemetry_rx.try_recv().ok()
    }

    pub fn poll_log(&mut self) -> Option<String> {
        self.log_rx.try_recv().ok()
    }

    pub fn send_command(&self, cmd: SpadeControlCommand) {
        let _ = self.cmd_tx.send(cmd);
    }
}

impl Drop for SpadeNetworkClient {
    fn drop(&mut self) {
        self.shutdown_flag.store(true, Ordering::Relaxed);
    }
}
