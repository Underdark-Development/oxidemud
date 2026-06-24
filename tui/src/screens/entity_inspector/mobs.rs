use crate::components::Table;
use crate::screens::entity_inspector::EntityInspectorScreen;

impl EntityInspectorScreen {
    pub(super) fn load_mobs(&self, table: &mut Table) {
        let mob = match self.registry.mobs.get(&self.template_id) {
            Some(m) => m,
            None => return,
        };
        Self::add_field(table, "id", &mob.id);
        Self::add_field(table, "name", &mob.name);
        Self::add_field(table, "description", &mob.description);
        Self::add_field(table, "level", mob.level);
        Self::add_field(table, "armor", mob.armor);
        Self::add_field(table, "size", &mob.size);
        Self::add_field(table, "xp_value", mob.xp_value);
        Self::add_field(table, "ai_mode", &mob.ai_mode);
        if !mob.patrol_route.is_empty() {
            Self::add_field(table, "patrol_route", format!("{:?}", mob.patrol_route));
        }
        if !mob.wander_rooms.is_empty() {
            Self::add_field(table, "wander_rooms", format!("{:?}", mob.wander_rooms));
        }
        if mob.wander_area {
            Self::add_field(table, "wander_area", "true");
        }
        Self::add_field(table, "aggro_range", mob.aggro_range);
        Self::add_field(table, "aggro_players", mob.aggro_players);
        Self::add_field(table, "faction_standing", mob.faction_standing);
        if let Some(ref d) = mob.damage {
            Self::add_field(table, "damage", d);
        }
        if let Some(ref dt) = mob.damage_type {
            Self::add_field(table, "damage_type", dt);
        }
        if let Some(ref race) = mob.race {
            Self::add_field(table, "race", race);
        }
        if let Some(ref faction) = mob.faction {
            Self::add_field(table, "faction", faction);
        }
        Self::add_field(table, "health.current", mob.health.current);
        Self::add_field(table, "health.max", mob.health.max);
        Self::add_field(table, "attributes.str", mob.attributes.strength);
        Self::add_field(table, "attributes.dex", mob.attributes.dexterity);
        Self::add_field(table, "attributes.con", mob.attributes.constitution);
        Self::add_field(table, "attributes.int", mob.attributes.intelligence);
        Self::add_field(table, "attributes.wis", mob.attributes.wisdom);
        Self::add_field(table, "attributes.cha", mob.attributes.charisma);
        for entry in &mob.equipment {
            Self::add_field(
                table,
                &format!("equipment.{}", entry.slot),
                &entry.template_id,
            );
        }
        for entry in &mob.loot.entries {
            Self::add_field(table, "loot.item", &entry.item);
            Self::add_field(table, "loot.chance", entry.chance);
            if let Some(ref tc) = entry.treasure_class {
                Self::add_field(table, "loot.treasure_class", tc);
            }
            if let Some(ref count) = entry.count {
                Self::add_field(table, "loot.count.min", count.min);
                Self::add_field(table, "loot.count.max", count.max);
            }
        }
        for (i, race_id) in mob.aggro_race.iter().enumerate() {
            Self::add_field(table, &format!("aggro_race[{i}]"), race_id);
        }
        for (i, lang) in mob.languages.iter().enumerate() {
            Self::add_field(table, &format!("languages[{i}]"), lang);
        }
        for (i, skill) in mob.skills.iter().enumerate() {
            Self::add_field(table, &format!("skills[{i}].id"), &skill.id);
            Self::add_field(table, &format!("skills[{i}].level"), skill.level);
        }
        for (i, trainer_type) in mob.trainer_types.iter().enumerate() {
            Self::add_field(table, &format!("trainer_types[{i}]"), trainer_type);
        }
        for (i, script) in mob.scripts.iter().enumerate() {
            Self::add_field(table, &format!("scripts[{i}].event"), &script.event);
            Self::add_field(table, &format!("scripts[{i}].script"), &script.script);
        }
    }

    pub(super) fn update_mobs(&mut self, field: &str, value: &str) -> Result<(), String> {
        let mob = self
            .registry
            .mobs
            .get_mut(&self.template_id)
            .ok_or_else(|| "mob not found".to_string())?;
        match field {
            "id" => mob.id = value.to_string(),
            "name" => mob.name = value.to_string(),
            "description" => mob.description = value.to_string(),
            "level" => mob.level = value.parse().map_err(|_| "invalid number")?,
            "armor" => mob.armor = value.parse().map_err(|_| "invalid number")?,
            "size" => mob.size = value.to_string(),
            "xp_value" => mob.xp_value = value.parse().map_err(|_| "invalid number")?,
            "ai_mode" => mob.ai_mode = value.to_string(),
            "patrol_route" => {
                mob.patrol_route = if value.is_empty() {
                    Vec::new()
                } else {
                    value
                        .trim_matches(&['[', ']', ' '][..])
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .collect()
                }
            }
            "wander_rooms" => {
                mob.wander_rooms = if value.is_empty() {
                    Vec::new()
                } else {
                    value
                        .trim_matches(&['[', ']', ' '][..])
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .collect()
                }
            }
            "wander_area" => mob.wander_area = value.parse().unwrap_or(false),
            "aggro_range" => mob.aggro_range = value.parse().map_err(|_| "invalid number")?,
            "aggro_players" => mob.aggro_players = value.parse().unwrap_or(false),
            "faction_standing" => {
                mob.faction_standing = value.parse().map_err(|_| "invalid number")?
            }
            "damage" => mob.damage = Some(value.to_string()),
            "damage_type" => mob.damage_type = Some(value.to_string()),
            "race" => mob.race = Some(value.to_string()),
            "faction" => mob.faction = Some(value.to_string()),
            "health.current" => mob.health.current = value.parse().map_err(|_| "invalid number")?,
            "health.max" => mob.health.max = value.parse().map_err(|_| "invalid number")?,
            "attributes.str" => {
                mob.attributes.strength = value.parse().map_err(|_| "invalid number")?
            }
            "attributes.dex" => {
                mob.attributes.dexterity = value.parse().map_err(|_| "invalid number")?
            }
            "attributes.con" => {
                mob.attributes.constitution = value.parse().map_err(|_| "invalid number")?
            }
            "attributes.int" => {
                mob.attributes.intelligence = value.parse().map_err(|_| "invalid number")?
            }
            "attributes.wis" => {
                mob.attributes.wisdom = value.parse().map_err(|_| "invalid number")?
            }
            "attributes.cha" => {
                mob.attributes.charisma = value.parse().map_err(|_| "invalid number")?
            }
            _ if field.starts_with("equipment.") => {
                let slot = field.trim_start_matches("equipment.");
                for entry in mob.equipment.iter_mut() {
                    if entry.slot == slot {
                        entry.template_id = value.to_string();
                        break;
                    }
                }
            }
            _ if field.starts_with("loot.") => {
                // Simple: update first loot entry
                if let Some(entry) = mob.loot.entries.first_mut() {
                    match field.trim_start_matches("loot.") {
                        "item" => entry.item = value.to_string(),
                        "chance" => entry.chance = value.parse().map_err(|_| "invalid number")?,
                        "treasure_class" => entry.treasure_class = Some(value.to_string()),
                        "count.min" => {
                            if let Some(ref mut c) = entry.count {
                                c.min = value.parse().map_err(|_| "invalid number")?;
                            }
                        }
                        "count.max" => {
                            if let Some(ref mut c) = entry.count {
                                c.max = value.parse().map_err(|_| "invalid number")?;
                            }
                        }
                        _ => return Err(format!("unknown loot field: {field}")),
                    }
                }
            }
            _ if field.starts_with("aggro_race[")
                || field.starts_with("languages[")
                || field.starts_with("trainer_types[") =>
            {
                // Skip array element edits for now
            }
            _ if field.starts_with("skills[") => {
                let rest = field.trim_start_matches("skills[");
                let (idx_str, _rest) = rest.split_once(']').ok_or("invalid skill path")?;
                let idx: usize = idx_str.parse().map_err(|_| "invalid index")?;
                if idx < mob.skills.len() {
                    let s = &mut mob.skills[idx];
                    if rest.contains(".id") {
                        s.id = value.to_string();
                    } else if rest.contains(".level") {
                        s.level = value.parse().map_err(|_| "invalid number")?;
                    }
                }
            }
            _ if field.starts_with("scripts[") => {
                let rest = field.trim_start_matches("scripts[");
                let (idx_str, _rest) = rest.split_once(']').ok_or("invalid script path")?;
                let idx: usize = idx_str.parse().map_err(|_| "invalid index")?;
                if idx < mob.scripts.len() {
                    let s = &mut mob.scripts[idx];
                    if rest.contains(".event") {
                        s.event = value.to_string();
                    } else if rest.contains(".script") {
                        s.script = value.to_string();
                    }
                }
            }
            _ => return Err(format!("unknown field: {field}")),
        }
        Ok(())
    }
}
