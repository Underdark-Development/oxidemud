use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SpadeConfig {
    #[serde(default)]
    pub connection: ConnectionConfig,
    #[serde(default)]
    pub prefs: PrefsConfig,
    #[serde(default = "default_content_path")]
    pub content_path: String,
}

impl Default for SpadeConfig {
    fn default() -> Self {
        SpadeConfig {
            connection: ConnectionConfig::default(),
            prefs: PrefsConfig::default(),
            content_path: default_content_path(),
        }
    }
}

fn default_content_path() -> String {
    "content".to_string()
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ConnectionConfig {
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default)]
    pub username: String,
    #[serde(default)]
    pub tls: bool,
    #[serde(default)]
    pub api_key: Option<String>,
}

impl Default for ConnectionConfig {
    fn default() -> Self {
        ConnectionConfig {
            host: default_host(),
            port: default_port(),
            username: String::new(),
            tls: false,
            api_key: None,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PrefsConfig {
    #[serde(default = "default_true")]
    pub mouse: bool,
    #[serde(default = "default_scrollback")]
    pub scrollback_size: usize,
    #[serde(default = "default_true")]
    pub sidebar_open: bool,
}

impl Default for PrefsConfig {
    fn default() -> Self {
        PrefsConfig {
            mouse: true,
            scrollback_size: 5000,
            sidebar_open: true,
        }
    }
}

fn default_host() -> String {
    "localhost".to_string()
}

const fn default_port() -> u16 {
    4000
}

const fn default_true() -> bool {
    true
}

const fn default_scrollback() -> usize {
    5000
}

pub fn load_config() -> SpadeConfig {
    let config_path = dirs::config_dir().map(|p| p.join("spade").join("config.toml"));

    if let Some(path) = config_path {
        if path.exists() {
            if let Ok(content) = std::fs::read_to_string(&path) {
                if let Ok(config) = toml::from_str(&content) {
                    return config;
                }
            }
        }
    }

    SpadeConfig::default()
}

pub fn save_config(config: &SpadeConfig) -> Result<(), std::io::Error> {
    if let Some(config_dir) = dirs::config_dir() {
        let spade_dir = config_dir.join("spade");
        std::fs::create_dir_all(&spade_dir)?;
        let config_file = spade_dir.join("config.toml");
        if let Ok(toml_str) = toml::to_string_pretty(config) {
            std::fs::write(config_file, toml_str)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_serialization_roundtrip() {
        let mut config = SpadeConfig::default();
        config.connection.host = "mud.example.com".into();
        config.connection.port = 8443;
        config.connection.tls = true;
        config.connection.api_key = Some("test_secret_key_123".into());

        let toml_str = toml::to_string_pretty(&config).unwrap();
        assert!(toml_str.contains("mud.example.com"));
        assert!(toml_str.contains("8443"));
        assert!(toml_str.contains("test_secret_key_123"));

        let loaded: SpadeConfig = toml::from_str(&toml_str).unwrap();
        assert_eq!(loaded.connection.host, "mud.example.com");
        assert_eq!(loaded.connection.port, 8443);
        assert!(loaded.connection.tls);
        assert_eq!(
            loaded.connection.api_key.as_deref(),
            Some("test_secret_key_123")
        );
    }
}
