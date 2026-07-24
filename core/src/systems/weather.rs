use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::templates::weather::{ConditionType, WeatherConfig, WeatherEffects};
use crate::Season;

/// Component attached to room entities tracking active per-room weather state.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WeatherState {
    pub base: Option<String>,
    pub modifier: Option<String>,
    #[serde(default)]
    pub effects: WeatherEffects,
}

impl WeatherState {
    pub fn new(base: Option<impl Into<String>>, modifier: Option<impl Into<String>>) -> Self {
        Self {
            base: base.map(Into::into),
            modifier: modifier.map(Into::into),
            effects: WeatherEffects::default(),
        }
    }

    pub fn is_clear(&self) -> bool {
        self.base
            .as_deref()
            .unwrap_or("clear")
            .eq_ignore_ascii_case("clear")
            && self.modifier.is_none()
    }
}

/// Parameters for resolving weather weights for a room/area.
#[derive(Debug, Clone)]
pub struct ResolutionParams<'a> {
    pub season: Season,
    pub area_no_weather: bool,
    pub area_weather_zone: Option<&'a str>,
    pub area_weather_matrix: Option<&'a HashMap<String, HashMap<String, u32>>>,
    pub room_no_weather: bool,
    pub room_exclude_weather: &'a [String],
    pub room_additional_weather: &'a HashMap<String, u32>,
}

/// Resolves effective weather condition weights for base or modifier conditions.
pub fn resolve_weather_weights(
    params: &ResolutionParams,
    config: &WeatherConfig,
    target_type: ConditionType,
) -> HashMap<String, u32> {
    // 1. If area.no_weather = true or room.no_weather = true -> Clear (empty weights)
    if params.area_no_weather || params.room_no_weather {
        return HashMap::new();
    }

    let season_key = params.season.name().to_lowercase();

    // 2. Global season availability
    let available_conditions: Vec<String> = config
        .seasons
        .get(&season_key)
        .map(|s| s.available.clone())
        .unwrap_or_default();

    // 3. Base weights from area_weather_zone reference OR area_weather_matrix
    let mut weights: HashMap<String, u32> = HashMap::new();

    if let Some(matrix) = params.area_weather_matrix {
        if let Some(season_weights) = matrix.get(&season_key) {
            weights = season_weights.clone();
        }
    } else if let Some(zone_id) = params.area_weather_zone {
        if let Some(zone_seasons) = config.zones.get(zone_id) {
            if let Some(season_weights) = zone_seasons.get(&season_key) {
                weights = season_weights.clone();
            }
        }
    }

    // Filter by season availability (if season defines available conditions)
    if !available_conditions.is_empty() {
        weights.retain(|cond, _| {
            available_conditions
                .iter()
                .any(|c| c.eq_ignore_ascii_case(cond))
        });
    }

    // 4. Room exclude_weather (remove entries)
    for exclude in params.room_exclude_weather {
        weights.retain(|k, _| !k.eq_ignore_ascii_case(exclude));
    }

    // 5. Room additional_weather (add/merge entries)
    for (add_cond, &add_weight) in params.room_additional_weather {
        weights
            .entry(add_cond.clone())
            .and_modify(|w| *w += add_weight)
            .or_insert(add_weight);
    }

    // Filter weights matching target_type (Base vs Modifier)
    weights.retain(|cond, w| {
        if *w == 0 {
            return false;
        }
        if let Some(def) = config.conditions.get(cond) {
            def.condition_type == target_type
        } else {
            // "clear" is implicitly Base
            cond.eq_ignore_ascii_case("clear") && target_type == ConditionType::Base
        }
    });

    weights
}

/// Rolls a condition ID from a map of condition -> weight using fastrand.
pub fn roll_from_weights(weights: &HashMap<String, u32>) -> Option<String> {
    let total_weight: u32 = weights.values().sum();
    if total_weight == 0 {
        return None;
    }

    let mut roll = fastrand::u32(0..total_weight);
    for (cond, &weight) in weights {
        if roll < weight {
            return Some(cond.clone());
        }
        roll -= weight;
    }

    None
}

/// Rolls base weather condition for a resolved weight map. Defaults to "clear" if None or weight sum 0.
pub fn roll_weather(weights: &HashMap<String, u32>) -> String {
    roll_from_weights(weights).unwrap_or_else(|| "clear".to_string())
}

/// Rolls modifier condition for a resolved weight map. Returns None if empty or roll fails.
pub fn roll_modifier(weights: &HashMap<String, u32>) -> Option<String> {
    roll_from_weights(weights)
}

/// Evaluates active base and modifier weather conditions in `weather_state` against `config`
/// and returns the combined active `WeatherEffects`.
pub fn get_effective_weather_effects(
    weather_state: &WeatherState,
    config: &WeatherConfig,
) -> crate::templates::weather::WeatherEffects {
    let mut combined = crate::templates::weather::WeatherEffects::default();

    if let Some(ref base_id) = weather_state.base {
        if let Some(def) = config.conditions.get(base_id) {
            combined.combine(&def.effects);
        }
    }

    if let Some(ref mod_id) = weather_state.modifier {
        if let Some(def) = config.conditions.get(mod_id) {
            combined.combine(&def.effects);
        }
    }

    combined
}

/// Helper function to retrieve the combined active `WeatherEffects` for a room entity in `world`.
pub fn get_room_weather_effects(
    world: &crate::World,
    room: crate::Entity,
    config: &WeatherConfig,
) -> crate::templates::weather::WeatherEffects {
    let weather_state = world
        .query_one::<&WeatherState>(room)
        .ok()
        .and_then(|mut q| q.get().cloned())
        .unwrap_or_default();

    get_effective_weather_effects(&weather_state, config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::templates::weather::{ConditionType, SeasonAvailability, WeatherConditionDef};

    fn make_test_config() -> WeatherConfig {
        let mut config = WeatherConfig::default();
        config.conditions.insert(
            "clear".into(),
            WeatherConditionDef {
                name: "Clear".into(),
                description: "Sky is clear.".into(),
                severity: crate::templates::weather::WeatherSeverity::Minor,
                condition_type: ConditionType::Base,
                effects: Default::default(),
            },
        );
        config.conditions.insert(
            "rain".into(),
            WeatherConditionDef {
                name: "Rain".into(),
                description: "Rain is falling.".into(),
                severity: crate::templates::weather::WeatherSeverity::Minor,
                condition_type: ConditionType::Base,
                effects: Default::default(),
            },
        );
        config.conditions.insert(
            "fog".into(),
            WeatherConditionDef {
                name: "Fog".into(),
                description: "Fog shrouds the land.".into(),
                severity: crate::templates::weather::WeatherSeverity::Minor,
                condition_type: ConditionType::Modifier,
                effects: Default::default(),
            },
        );

        config.seasons.insert(
            "spring".into(),
            SeasonAvailability {
                available: vec!["clear".into(), "rain".into(), "fog".into()],
            },
        );

        let mut temperate_spring = HashMap::new();
        temperate_spring.insert("clear".into(), 50);
        temperate_spring.insert("rain".into(), 50);
        temperate_spring.insert("fog".into(), 20);

        let mut temperate = HashMap::new();
        temperate.insert("spring".into(), temperate_spring);

        config.zones.insert("temperate".into(), temperate);
        config
    }

    #[test]
    fn test_resolution_chain_base() {
        let config = make_test_config();
        let empty_exclude = vec![];
        let empty_add = HashMap::new();

        let params = ResolutionParams {
            season: Season::Spring,
            area_no_weather: false,
            area_weather_zone: Some("temperate"),
            area_weather_matrix: None,
            room_no_weather: false,
            room_exclude_weather: &empty_exclude,
            room_additional_weather: &empty_add,
        };

        let weights = resolve_weather_weights(&params, &config, ConditionType::Base);
        assert_eq!(weights.get("clear"), Some(&50));
        assert_eq!(weights.get("rain"), Some(&50));
        assert_eq!(weights.get("fog"), None); // Fog is a modifier
    }

    #[test]
    fn test_resolution_chain_no_weather() {
        let config = make_test_config();
        let empty_exclude = vec![];
        let empty_add = HashMap::new();

        let params = ResolutionParams {
            season: Season::Spring,
            area_no_weather: true,
            area_weather_zone: Some("temperate"),
            area_weather_matrix: None,
            room_no_weather: false,
            room_exclude_weather: &empty_exclude,
            room_additional_weather: &empty_add,
        };

        let weights = resolve_weather_weights(&params, &config, ConditionType::Base);
        assert!(weights.is_empty());
        assert_eq!(roll_weather(&weights), "clear");
    }

    #[test]
    fn test_effective_weather_effects_combination() {
        let mut config = WeatherConfig::default();
        config.conditions.insert(
            "rain".into(),
            WeatherConditionDef {
                name: "Rain".into(),
                description: "Rain falls.".into(),
                severity: crate::templates::weather::WeatherSeverity::Minor,
                condition_type: ConditionType::Base,
                effects: WeatherEffects {
                    damage_fire: Some(-2),
                    damage_lightning: Some(2),
                    ..Default::default()
                },
            },
        );
        config.conditions.insert(
            "fog".into(),
            WeatherConditionDef {
                name: "Fog".into(),
                description: "Thick fog.".into(),
                severity: crate::templates::weather::WeatherSeverity::Minor,
                condition_type: ConditionType::Modifier,
                effects: WeatherEffects {
                    ranged_accuracy: Some(-2),
                    dexterity: Some(-1),
                    ..Default::default()
                },
            },
        );

        let state = WeatherState {
            base: Some("rain".into()),
            modifier: Some("fog".into()),
            effects: Default::default(),
        };

        let eff = get_effective_weather_effects(&state, &config);
        assert_eq!(eff.get_damage_modifier("fire"), -2);
        assert_eq!(eff.get_damage_modifier("lightning"), 2);
        assert_eq!(eff.ranged_accuracy, Some(-2));
        assert_eq!(eff.dexterity, Some(-1));
    }

    #[test]
    fn test_dynamic_damage_type_modifiers() {
        let mut extra = HashMap::new();
        extra.insert("damage_cold".to_string(), 4);
        extra.insert("damage_acid".to_string(), -1);

        let eff = WeatherEffects {
            damage_fire: Some(-3),
            extra_effects: extra,
            ..Default::default()
        };

        assert_eq!(eff.get_damage_modifier("fire"), -3);
        assert_eq!(eff.get_damage_modifier("cold"), 4);
        assert_eq!(eff.get_damage_modifier("acid"), -1);
        assert_eq!(eff.get_damage_modifier("slash"), 0);
    }

    #[test]
    fn test_weather_toml_deserialization() {
        let content_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("content")
            .join("weather.toml");

        if content_path.exists() {
            let toml_str =
                std::fs::read_to_string(&content_path).expect("Failed to read weather.toml");
            let config: WeatherConfig =
                toml::from_str(&toml_str).expect("Failed to parse weather.toml");

            assert!(
                !config.conditions.is_empty(),
                "Conditions should not be empty"
            );
            assert!(config.conditions.contains_key("clear"));
            assert!(config.conditions.contains_key("rain"));
            assert!(config.seasons.contains_key("spring"));
            assert!(config.zones.contains_key("temperate"));
        }
    }
}
