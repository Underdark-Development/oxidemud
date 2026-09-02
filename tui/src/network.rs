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
    Shutdown { delay_mins: Option<u32> },
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
    rpc: Arc<std::sync::RwLock<Option<Arc<oxide_ws_rpc::RpcClient>>>>,
}

/// Resolve a WebSocket URL given an optional URL string, host, port, TLS setting, and default path.
pub fn resolve_ws_url(
    url: Option<&str>,
    host: &str,
    port: u16,
    tls: bool,
    default_path: &str,
) -> String {
    let target_path = if default_path.starts_with('/') {
        default_path.to_string()
    } else {
        format!("/{default_path}")
    };

    if let Some(u) = url {
        let trimmed = u.trim();
        if !trimmed.is_empty() {
            // Normalize http/https schemes to ws/wss
            let (scheme, rest) = if let Some(stripped) = trimmed.strip_prefix("http://") {
                ("ws://", stripped)
            } else if let Some(stripped) = trimmed.strip_prefix("https://") {
                ("wss://", stripped)
            } else if let Some(stripped) = trimmed.strip_prefix("ws://") {
                ("ws://", stripped)
            } else if let Some(stripped) = trimmed.strip_prefix("wss://") {
                ("wss://", stripped)
            } else {
                let default_scheme = if tls { "wss://" } else { "ws://" };
                (default_scheme, trimmed)
            };

            // Check if rest contains a path
            if let Some(idx) = rest.find('/') {
                let host_port = &rest[..idx];
                let path = &rest[idx..];
                if path == "/ws/spade" || path == "/ws/rpc" || path == "/ws/play" {
                    return format!("{scheme}{host_port}{target_path}");
                }
                return format!("{scheme}{rest}");
            } else {
                return format!("{scheme}{rest}{target_path}");
            }
        }
    }

    let scheme = if tls { "wss" } else { "ws" };
    format!("{scheme}://{host}:{port}{target_path}")
}

impl SpadeNetworkClient {
    pub fn connect(
        url: Option<&str>,
        host: &str,
        port: u16,
        tls: bool,
        api_key: Option<String>,
    ) -> Self {
        let spade_url = resolve_ws_url(url, host, port, tls, "/ws/spade");
        let rpc_url = resolve_ws_url(url, host, port, tls, "/ws/rpc");
        Self::connect_urls(spade_url, rpc_url, api_key)
    }

    pub fn connect_url(url_str: String, api_key: Option<String>) -> Self {
        let rpc_url = resolve_ws_url(Some(&url_str), "", 0, false, "/ws/rpc");
        Self::connect_urls(url_str, rpc_url, api_key)
    }

    pub fn connect_urls(url_str: String, rpc_url_str: String, api_key: Option<String>) -> Self {
        let status = Arc::new(std::sync::Mutex::new(ConnectionStatus::Connecting));
        let ping_ms = Arc::new(std::sync::Mutex::new(0));
        let (telemetry_tx, telemetry_rx) = mpsc::unbounded_channel();
        let (log_tx, log_rx) = mpsc::unbounded_channel();
        let (cmd_tx, mut cmd_rx) = mpsc::unbounded_channel::<SpadeControlCommand>();
        let shutdown_flag = Arc::new(AtomicBool::new(false));
        let rpc = Arc::new(std::sync::RwLock::new(None));

        let status_clone = status.clone();
        let ping_clone = ping_ms.clone();
        let shutdown_clone = shutdown_flag.clone();
        let rpc_clone = rpc.clone();

        tokio::spawn(async move {
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
                        *rpc_clone.write().unwrap() = None;
                        tokio::time::sleep(Duration::from_secs(3)).await;
                        continue;
                    }
                };

                // Connect to JSON-RPC alongside telemetry stream
                if let Ok(rpc_client) =
                    oxide_ws_rpc::RpcClient::connect(&rpc_url_str, api_key.as_deref()).await
                {
                    *rpc_clone.write().unwrap() = Some(Arc::new(rpc_client));
                }

                match connect_async(request).await {
                    Ok((ws_stream, _)) => {
                        *status_clone.lock().unwrap() = ConnectionStatus::Connected;
                        let (mut write, mut read) = ws_stream.split();
                        let mut last_ping_sent: Option<Instant> = None;
                        let mut ping_interval = tokio::time::interval(Duration::from_secs(3));

                        loop {
                            tokio::select! {
                                _ = ping_interval.tick() => {
                                    let ping_cmd = serde_json::to_string(&SpadeControlCommand::Ping).unwrap_or_default();
                                    last_ping_sent = Some(Instant::now());
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
                                                if let Some(sent) = last_ping_sent.take() {
                                                    let elapsed = sent.elapsed().as_millis() as u64;
                                                    *ping_clone.lock().unwrap() = elapsed;
                                                }
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
                                        Some(Ok(Message::Ping(data))) => {
                                            if write.send(Message::Pong(data)).await.is_err() {
                                                break;
                                            }
                                        }
                                        Some(Ok(Message::Close(_))) | None | Some(Err(_)) => break,
                                        _ => {}
                                    }
                                }
                            }
                        }

                        *status_clone.lock().unwrap() = ConnectionStatus::Disconnected;
                        *rpc_clone.write().unwrap() = None;
                    }
                    Err(_) => {
                        *status_clone.lock().unwrap() = ConnectionStatus::Disconnected;
                        *rpc_clone.write().unwrap() = None;
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
            rpc,
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

    pub fn rpc(&self) -> Option<Arc<oxide_ws_rpc::RpcClient>> {
        self.rpc.read().unwrap().clone()
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_ws_url_defaults() {
        let url = resolve_ws_url(None, "127.0.0.1", 8080, false, "/ws/spade");
        assert_eq!(url, "ws://127.0.0.1:8080/ws/spade");

        let tls_url = resolve_ws_url(None, "mud.example.com", 443, true, "/ws/spade");
        assert_eq!(tls_url, "wss://mud.example.com:443/ws/spade");
    }

    #[test]
    fn test_resolve_ws_url_custom_schemes() {
        let u1 = resolve_ws_url(
            Some("http://127.0.0.1:8080"),
            "localhost",
            4000,
            false,
            "/ws/spade",
        );
        assert_eq!(u1, "ws://127.0.0.1:8080/ws/spade");

        let u2 = resolve_ws_url(
            Some("https://game.io:8080/custom/ws"),
            "localhost",
            4000,
            false,
            "/ws/spade",
        );
        assert_eq!(u2, "wss://game.io:8080/custom/ws");

        let u3 = resolve_ws_url(
            Some("wss://secure.io:4000"),
            "localhost",
            4000,
            false,
            "/ws/spade",
        );
        assert_eq!(u3, "wss://secure.io:4000/ws/spade");

        let u4 = resolve_ws_url(
            Some("192.168.1.50:9000"),
            "localhost",
            4000,
            true,
            "/ws/spade",
        );
        assert_eq!(u4, "wss://192.168.1.50:9000/ws/spade");

        let u5 = resolve_ws_url(
            Some("ws://127.0.0.1:8080/ws/spade"),
            "localhost",
            4000,
            false,
            "/ws/rpc",
        );
        assert_eq!(u5, "ws://127.0.0.1:8080/ws/rpc");

        let u6 = resolve_ws_url(
            Some("wss://127.0.0.1:8080/ws/rpc"),
            "localhost",
            4000,
            false,
            "/ws/play",
        );
        assert_eq!(u6, "wss://127.0.0.1:8080/ws/play");
    }

    #[test]
    fn test_spade_control_command_serialization() {
        let cmd = SpadeControlCommand::Gecho {
            message: "Hello world".into(),
        };
        let json = serde_json::to_string(&cmd).unwrap();
        assert_eq!(
            json,
            r#"{"action":"Gecho","payload":{"message":"Hello world"}}"#
        );

        let ping = SpadeControlCommand::Ping;
        let ping_json = serde_json::to_string(&ping).unwrap();
        assert_eq!(ping_json, r#"{"action":"Ping"}"#);
    }
}
