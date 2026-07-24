use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// WeatherConfig — top-level deserialization target for content/weather.toml
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WeatherConfig {
    #[serde(default)]
    pub conditions: HashMap<String, WeatherConditionDef>,
    #[serde(default)]
    pub seasons: HashMap<String, SeasonAvailability>,
    #[serde(default)]
    pub zones: HashMap<String, HashMap<String, HashMap<String, u32>>>,
}

// ---------------------------------------------------------------------------
// Condition definitions
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeatherConditionDef {
    pub name: String,
    pub description: String,
    #[serde(default = "default_severity")]
    pub severity: WeatherSeverity,
    #[serde(default = "default_condition_type")]
    pub condition_type: ConditionType,
    #[serde(default)]
    pub effects: WeatherEffects,
}

fn default_severity() -> WeatherSeverity {
    WeatherSeverity::Minor
}

fn default_condition_type() -> ConditionType {
    ConditionType::Base
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WeatherSeverity {
    Minor,
    Severe,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConditionType {
    Base,
    Modifier,
}

// ---------------------------------------------------------------------------
// Effects — gameplay modifiers applied while a weather condition is active
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WeatherEffects {
    #[serde(default)]
    pub damage_fire: Option<i32>,
    #[serde(default)]
    pub damage_lightning: Option<i32>,
    #[serde(default)]
    pub ranged_accuracy: Option<i32>,
    #[serde(default)]
    pub ranged_accuracy_pct: Option<i32>,
    #[serde(default)]
    pub ranged_attack: Option<i32>,
    #[serde(default)]
    pub dexterity: Option<i32>,
}

// ---------------------------------------------------------------------------
// Season availability
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SeasonAvailability {
    #[serde(default)]
    pub available: Vec<String>,
}
