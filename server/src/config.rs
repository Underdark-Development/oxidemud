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

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    pub server_name: String,
    pub max_clients: u16,
    #[serde(default = "default_prompt")]
    pub default_prompt: String,
    #[serde(default)]
    pub logging: LoggingConfig,
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
            logging: LoggingConfig::default(),
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
                logging: LoggingConfig::default(),
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
                if (filename.starts_with("mud_server_log_") || filename.starts_with("oxidemud_"))
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
        let path = temp_dir.join("mud_server_log_test_unit_prune.log");

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
