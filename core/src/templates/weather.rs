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

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
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
    #[serde(flatten, default)]
    pub extra_effects: HashMap<String, i32>,
}

impl WeatherEffects {
    pub fn get_damage_modifier(&self, damage_type: &str) -> i32 {
        let key = format!("damage_{}", damage_type.to_lowercase());
        let mut total = 0;
        if let Some(&val) = self.extra_effects.get(&key) {
            total += val;
        }
        match damage_type.to_lowercase().as_str() {
            "fire" => total += self.damage_fire.unwrap_or(0),
            "lightning" => total += self.damage_lightning.unwrap_or(0),
            _ => {}
        }
        total
    }

    pub fn combine(&mut self, other: &WeatherEffects) {
        if let Some(val) = other.damage_fire {
            *self.damage_fire.get_or_insert(0) += val;
        }
        if let Some(val) = other.damage_lightning {
            *self.damage_lightning.get_or_insert(0) += val;
        }
        if let Some(val) = other.ranged_accuracy {
            *self.ranged_accuracy.get_or_insert(0) += val;
        }
        if let Some(val) = other.ranged_accuracy_pct {
            *self.ranged_accuracy_pct.get_or_insert(0) += val;
        }
        if let Some(val) = other.ranged_attack {
            *self.ranged_attack.get_or_insert(0) += val;
        }
        if let Some(val) = other.dexterity {
            *self.dexterity.get_or_insert(0) += val;
        }
        for (k, v) in &other.extra_effects {
            *self.extra_effects.entry(k.clone()).or_insert(0) += v;
        }
    }
}

// ---------------------------------------------------------------------------
// Season availability
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SeasonAvailability {
    #[serde(default)]
    pub available: Vec<String>,
}
