use serde::{Deserialize, Serialize};

/// Configuration for the in-game time system, loaded from `[time]` in server.toml.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeConfig {
    /// Real-world minutes per in-game hour (default: 24).
    #[serde(default = "default_real_minutes_per_game_hour")]
    pub real_minutes_per_game_hour: u64,
    /// Game days per season (default: 30).
    #[serde(default = "default_days_per_season")]
    pub days_per_season: u32,
    /// Season on first boot (default: "spring").
    #[serde(default = "default_start_season")]
    pub start_season: String,
    /// Hour on first boot, 0–23 (default: 6).
    #[serde(default = "default_start_hour")]
    pub start_hour: u8,
}

impl Default for TimeConfig {
    fn default() -> Self {
        Self {
            real_minutes_per_game_hour: default_real_minutes_per_game_hour(),
            days_per_season: default_days_per_season(),
            start_season: default_start_season(),
            start_hour: default_start_hour(),
        }
    }
}

fn default_real_minutes_per_game_hour() -> u64 {
    24
}
fn default_days_per_season() -> u32 {
    30
}
fn default_start_season() -> String {
    "spring".to_string()
}
fn default_start_hour() -> u8 {
    6
}
