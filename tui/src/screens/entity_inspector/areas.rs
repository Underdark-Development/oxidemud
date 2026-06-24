use oxide_core::templates::ResetInterval;

use crate::components::Table;
use crate::screens::entity_inspector::EntityInspectorScreen;

impl EntityInspectorScreen {
    pub(super) fn load_areas(&self, table: &mut Table) {
        let area = match self.registry.areas.get(&self.template_id) {
            Some(a) => a,
            None => return,
        };
        Self::add_field(table, "id", &area.id);
        Self::add_field(table, "name", &area.name);
        Self::add_field(table, "description", &area.description);
        Self::add_field(table, "spawn_room", &area.spawn_room);
        Self::add_field(table, "rooms", area.rooms.len());
        Self::add_field(table, "flags", area.flags.join(", "));

        let lvl = area
            .level_range
            .map(|r| format!("{}-{}", r[0], r[1]))
            .unwrap_or_default();
        Self::add_field(table, "level_range", lvl);
        Self::add_field(
            table,
            "weather_zone",
            area.weather_zone.as_deref().unwrap_or(""),
        );
        let reset = area
            .reset_interval
            .as_ref()
            .map(|r| r.secs.to_string())
            .unwrap_or_default();
        Self::add_field(table, "reset_interval_secs", reset);
        Self::add_field(table, "credits", area.credits.as_deref().unwrap_or(""));

        // Spawns
        if area.spawns.is_empty() {
            Self::add_field(table, "spawns[]", "(Empty Array - Press + to add)");
        } else {
            for (i, spawn) in area.spawns.iter().enumerate() {
                Self::add_field(table, &format!("spawns[{i}].room"), &spawn.room);
                Self::add_field(table, &format!("spawns[{i}].label"), &spawn.label);
                Self::add_field(
                    table,
                    &format!("spawns[{i}].description"),
                    &spawn.description,
                );
                Self::add_field(
                    table,
                    &format!("spawns[{i}].allowed_races"),
                    spawn.allowed_races.join(", "),
                );
                Self::add_field(
                    table,
                    &format!("spawns[{i}].allowed_classes"),
                    spawn.allowed_classes.join(", "),
                );
                Self::add_field(
                    table,
                    &format!("spawns[{i}].allowed_alignments"),
                    spawn.allowed_alignments.join(", "),
                );
            }
        }
    }

    pub(super) fn update_areas(&mut self, field: &str, value: &str) -> Result<(), String> {
        let area = self
            .registry
            .areas
            .get_mut(&self.template_id)
            .ok_or_else(|| "area not found".to_string())?;
        match field {
            "id" => area.id = value.to_string(),
            "name" => area.name = value.to_string(),
            "description" => area.description = value.to_string(),
            "spawn_room" => area.spawn_room = value.to_string(),
            "flags" => area.flags = value.split(',').map(|s| s.trim().to_string()).collect(),
            "level_range" => {
                if value.is_empty() {
                    area.level_range = None;
                } else {
                    let parts: Vec<&str> = value.split(&['–', '-'][..]).collect();
                    if parts.len() == 2 {
                        let lo: u8 = parts[0].trim().parse().map_err(|_| "invalid number")?;
                        let hi: u8 = parts[1].trim().parse().map_err(|_| "invalid number")?;
                        area.level_range = Some([lo, hi]);
                    }
                }
            }
            "weather_zone" => {
                area.weather_zone = if value.is_empty() {
                    None
                } else {
                    Some(value.to_string())
                }
            }
            "reset_interval_secs" => {
                if value.is_empty() {
                    area.reset_interval = None;
                } else {
                    let secs: u64 = value.parse().map_err(|_| "invalid number")?;
                    area.reset_interval = Some(ResetInterval { secs });
                }
            }
            "credits" => {
                area.credits = if value.is_empty() {
                    None
                } else {
                    Some(value.to_string())
                }
            }
            _ if field.starts_with("spawns[") => {
                let rest = field.trim_start_matches("spawns[");
                let (idx_str, path_rest) = rest.split_once(']').ok_or("invalid spawn path")?;
                let idx: usize = idx_str.parse().map_err(|_| "invalid index")?;
                if idx < area.spawns.len() {
                    let s = &mut area.spawns[idx];
                    if path_rest == ".room" {
                        s.room = value.to_string();
                    } else if path_rest == ".label" {
                        s.label = value.to_string();
                    } else if path_rest == ".description" {
                        s.description = value.to_string();
                    } else if path_rest == ".allowed_races" {
                        s.allowed_races = value
                            .split(',')
                            .map(|s| s.trim().to_string())
                            .filter(|s| !s.is_empty())
                            .collect();
                    } else if path_rest == ".allowed_classes" {
                        s.allowed_classes = value
                            .split(',')
                            .map(|s| s.trim().to_string())
                            .filter(|s| !s.is_empty())
                            .collect();
                    } else if path_rest == ".allowed_alignments" {
                        s.allowed_alignments = value
                            .split(',')
                            .map(|s| s.trim().to_string())
                            .filter(|s| !s.is_empty())
                            .collect();
                    }
                }
            }
            _ => return Err(format!("unknown field: {field}")),
        }
        Ok(())
    }

    pub(super) fn add_area_array(&mut self, prefix: &str, index: usize) -> Result<(), String> {
        let area = self
            .registry
            .areas
            .get_mut(&self.template_id)
            .ok_or("area not found")?;
        match prefix {
            "spawns" => {
                area.spawns.insert(
                    (index + 1).min(area.spawns.len()),
                    oxide_core::templates::SpawnEntry {
                        room: "room_id".to_string(),
                        label: "Spawn Point".to_string(),
                        description: "A spawn point.".to_string(),
                        allowed_races: vec![],
                        allowed_classes: vec![],
                        allowed_alignments: vec![],
                    },
                );
            }
            _ => return Err(format!("unknown area array: {prefix}")),
        }
        Ok(())
    }

    pub(super) fn remove_area_array(&mut self, prefix: &str, index: usize) -> Result<(), String> {
        let area = self
            .registry
            .areas
            .get_mut(&self.template_id)
            .ok_or("area not found")?;
        match prefix {
            "spawns" => {
                if index < area.spawns.len() {
                    area.spawns.remove(index);
                }
            }
            _ => return Err(format!("unknown area array: {prefix}")),
        }
        Ok(())
    }
}
