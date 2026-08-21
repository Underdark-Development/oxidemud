use std::path::Path;
use std::sync::OnceLock;

use serde::Deserialize;

static CONFIG: OnceLock<ServerConfig> = OnceLock::new();

#[derive(Debug, Clone, Deserialize)]
pub struct LoggingConfig {
    #[serde(default = "default_log_retention_days")]
    pub retention_days: u32,
    #[serde(default = "default_log_rotation")]
    pub rotation: String, // "daily", "hourly", "never"
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            retention_days: 5,
            rotation: "daily".to_string(),
        }
    }
}

fn default_log_retention_days() -> u32 {
    5
}

fn default_log_rotation() -> String {
    "daily".to_string()
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct TlsConfig {
    pub cert_path: Option<String>,
    pub key_path: Option<String>,
    pub acme_domain: Option<String>,
    pub acme_email: Option<String>,
    pub auto_dev_cert: Option<bool>,
    pub allow_insecure_http: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ApiConfig {
    #[serde(default = "default_api_enabled")]
    pub enabled: bool,
    #[serde(default = "default_api_bind_addr")]
    pub bind_addr: String,
    #[serde(default)]
    pub tls: TlsConfig,
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            bind_addr: "127.0.0.1:8080".to_string(),
            tls: TlsConfig::default(),
        }
    }
}

fn default_api_enabled() -> bool {
    true
}

fn default_api_bind_addr() -> String {
    "127.0.0.1:8080".to_string()
}

#[derive(Debug, Clone, Deserialize)]
pub struct WebSocketConfig {
    #[serde(default = "default_ws_enabled")]
    pub enabled: bool,
    #[serde(default = "default_ws_ping_interval_secs")]
    pub ping_interval_secs: u64,
    #[serde(default = "default_ws_max_message_size")]
    pub max_message_size_bytes: usize,
}

impl Default for WebSocketConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            ping_interval_secs: 30,
            max_message_size_bytes: 65536,
        }
    }
}

fn default_ws_enabled() -> bool {
    true
}

fn default_ws_ping_interval_secs() -> u64 {
    30
}

fn default_ws_max_message_size() -> usize {
    65536
}

#[derive(Debug, Clone, Deserialize)]
pub struct ContentConfig {
    #[serde(default = "default_content_path")]
    pub path: String,
}

impl Default for ContentConfig {
    fn default() -> Self {
        Self {
            path: default_content_path(),
        }
    }
}

fn default_content_path() -> String {
    "content".to_string()
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    pub server_name: String,
    #[serde(default)]
    pub server_url: Option<String>,
    #[serde(default)]
    pub server_version: Option<String>,
    pub max_clients: u16,
    #[serde(default = "default_prompt")]
    pub default_prompt: String,
    #[serde(default)]
    pub logging: LoggingConfig,
    #[serde(default)]
    pub content: ContentConfig,
    #[serde(default)]
    pub api: ApiConfig,
    #[serde(default)]
    pub websocket: WebSocketConfig,
    #[serde(default)]
    pub time: oxide_core::TimeConfig,
}

fn default_prompt() -> String {
    "<%hhp %hmhp> ".to_string()
}

pub fn init(path: &Path) {
    let content = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!(
                "Failed to read config at {}: {e}; using defaults",
                path.display()
            );
            String::new()
        }
    };

    let config: ServerConfig = if content.is_empty() {
        ServerConfig {
            server_name: "Oxide MUD".to_string(),
            server_url: None,
            server_version: None,
            max_clients: 256,
            default_prompt: default_prompt(),
            logging: LoggingConfig::default(),
            content: ContentConfig::default(),
            api: ApiConfig::default(),
            websocket: WebSocketConfig::default(),
            time: oxide_core::TimeConfig::default(),
        }
    } else {
        toml::from_str(&content).unwrap_or_else(|e| {
            tracing::warn!(
                "Failed to parse config at {}: {e}; using defaults",
                path.display()
            );
            ServerConfig {
                server_name: "Oxide MUD".to_string(),
                server_url: None,
                server_version: None,
                max_clients: 256,
                default_prompt: default_prompt(),
                logging: LoggingConfig::default(),
                content: ContentConfig::default(),
                api: ApiConfig::default(),
                websocket: WebSocketConfig::default(),
                time: oxide_core::TimeConfig::default(),
            }
        })
    };

    CONFIG.set(config).unwrap_or_else(|_| {
        tracing::warn!("Config already initialized");
    });
}

pub fn get() -> &'static ServerConfig {
    CONFIG.get().expect("ServerConfig not initialized")
}

/// Strictly parse a `server.toml` file for preflight validation.
///
/// Unlike `init`, this does not fall back to defaults on error and does not
/// touch the global config — it returns the parse/IO error for reporting.
pub fn validate_file(path: &Path) -> Result<(), String> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("failed to read {}: {e}", path.display()))?;
    toml::from_str::<ServerConfig>(&content)
        .map(|_| ())
        .map_err(|e| format!("failed to parse {}: {e}", path.display()))
}

/// Read just the `[content].path` value from a server.toml file without
/// initializing the global config. Used by the preflight path so it resolves
/// the content directory identically to normal startup. Returns `None` on any
/// read/parse error or if the key is absent (caller falls back to default).
pub fn content_path_from_file(path: &Path) -> Option<String> {
    let content = std::fs::read_to_string(path).ok()?;
    let config: ServerConfig = toml::from_str(&content).ok()?;
    Some(config.content.path)
}

pub fn prune_old_logs(retention_days: u32) {
    let temp_dir = std::env::temp_dir();
    let entries = match std::fs::read_dir(&temp_dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    let now = std::time::SystemTime::now();
    let max_age = std::time::Duration::from_secs(retention_days as u64 * 24 * 60 * 60);

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() {
            if let Some(filename) = path.file_name().and_then(|f| f.to_str()) {
                if (filename.starts_with("oxide_server_log_") || filename.starts_with("oxidemud_"))
                    && filename.ends_with(".log")
                {
                    if let Ok(metadata) = entry.metadata() {
                        if let Ok(modified) = metadata.modified() {
                            if let Ok(age) = now.duration_since(modified) {
                                if age >= max_age {
                                    let _ = std::fs::remove_file(&path);
                                    tracing::info!("Pruned expired server log: {}", path.display());
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;

    #[test]
    fn test_pruning_old_logs() {
        let temp_dir = std::env::temp_dir();
        let path = temp_dir.join(format!(
            "oxide_server_log_test_unit_prune_{}.log",
            fastrand::u64(..)
        ));

        // Ensure clean state
        let _ = std::fs::remove_file(&path);

        // Create the file
        let _file = File::create(&path).unwrap();
        assert!(path.exists());

        // Prune with 0 days retention (deletes immediately)
        prune_old_logs(0);

        // Verify the file was pruned
        assert!(!path.exists());
    }
}
