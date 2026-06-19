use std::path::Path;
use std::sync::OnceLock;

use serde::Deserialize;

static CONFIG: OnceLock<ServerConfig> = OnceLock::new();

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    pub server_name: String,
    pub max_clients: u16,
    #[serde(default = "default_prompt")]
    pub default_prompt: String,
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
            server_name: "Mud".to_string(),
            max_clients: 256,
            default_prompt: default_prompt(),
        }
    } else {
        toml::from_str(&content).unwrap_or_else(|e| {
            tracing::warn!(
                "Failed to parse config at {}: {e}; using defaults",
                path.display()
            );
            ServerConfig {
                server_name: "Mud".to_string(),
                max_clients: 256,
                default_prompt: default_prompt(),
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
