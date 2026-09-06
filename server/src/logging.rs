use std::collections::VecDeque;
use std::sync::{Mutex, OnceLock};
use tokio::sync::broadcast;

const MAX_LOG_BUFFER: usize = 500;
const BROADCAST_CAPACITY: usize = 512;

/// Thread-safe in-memory log buffer and broadcast hub.
pub struct ServerLogBroadcaster {
    buffer: Mutex<VecDeque<String>>,
    sender: broadcast::Sender<String>,
}

impl Default for ServerLogBroadcaster {
    fn default() -> Self {
        Self::new()
    }
}

impl ServerLogBroadcaster {
    pub fn new() -> Self {
        let (sender, _) = broadcast::channel(BROADCAST_CAPACITY);
        Self {
            buffer: Mutex::new(VecDeque::with_capacity(MAX_LOG_BUFFER)),
            sender,
        }
    }

    /// Push a new log line to the backlog buffer and broadcast to all active subscribers.
    pub fn push(&self, line: String) {
        {
            let mut buf = self.buffer.lock().unwrap_or_else(|e| e.into_inner());
            if buf.len() >= MAX_LOG_BUFFER {
                buf.pop_front();
            }
            buf.push_back(line.clone());
        }
        let _ = self.sender.send(line);
    }

    /// Subscribe to server logs, returning the current backlog and a broadcast receiver.
    pub fn subscribe(&self) -> (Vec<String>, broadcast::Receiver<String>) {
        let history = {
            let buf = self.buffer.lock().unwrap_or_else(|e| e.into_inner());
            buf.iter().cloned().collect()
        };
        (history, self.sender.subscribe())
    }
}

static LOG_BROADCASTER: OnceLock<ServerLogBroadcaster> = OnceLock::new();

pub fn get_log_broadcaster() -> &'static ServerLogBroadcaster {
    LOG_BROADCASTER.get_or_init(ServerLogBroadcaster::new)
}

/// Push a server log line to the global broadcaster.
pub fn broadcast_server_log(line: String) {
    get_log_broadcaster().push(line);
}

/// Subscribe to live server logs, receiving any existing historical buffer followed by new log events.
pub fn subscribe_server_logs() -> (Vec<String>, broadcast::Receiver<String>) {
    get_log_broadcaster().subscribe()
}
