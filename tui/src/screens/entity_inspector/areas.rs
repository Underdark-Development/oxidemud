use mud_core::templates::ResetInterval;

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
        if let Some(ref range) = area.level_range {
            Self::add_field(table, "level_range", format!("{}–{}", range[0], range[1]));
        }
        if let Some(ref zone) = area.weather_zone {
            Self::add_field(table, "weather_zone", zone);
        }
        if let Some(ref interval) = area.reset_interval {
            Self::add_field(table, "reset_interval_secs", interval.secs);
        }
        if let Some(ref credits) = area.credits {
            Self::add_field(table, "credits", credits);
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
                let parts: Vec<&str> = value.split(&['–', '-'][..]).collect();
                if parts.len() == 2 {
                    let lo: u8 = parts[0].trim().parse().map_err(|_| "invalid number")?;
                    let hi: u8 = parts[1].trim().parse().map_err(|_| "invalid number")?;
                    area.level_range = Some([lo, hi]);
                }
            }
            "weather_zone" => area.weather_zone = Some(value.to_string()),
            "reset_interval_secs" => {
                let secs: u64 = value.parse().map_err(|_| "invalid number")?;
                area.reset_interval = Some(ResetInterval { secs });
            }
            "credits" => area.credits = Some(value.to_string()),
            _ => return Err(format!("unknown field: {field}")),
        }
        Ok(())
    }
}
