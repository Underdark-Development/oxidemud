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

        // Optionals
        Self::add_field(table, "damage", mob.damage.as_deref().unwrap_or(""));
        Self::add_field(
            table,
            "damage_type",
            mob.damage_type.as_deref().unwrap_or(""),
        );
        Self::add_field(table, "race", mob.race.as_deref().unwrap_or(""));
        Self::add_field(table, "faction", mob.faction.as_deref().unwrap_or(""));
        Self::add_field(table, "shop", mob.shop.as_deref().unwrap_or(""));

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

        // Loot entries
        if mob.loot.entries.is_empty() {
            Self::add_field(table, "loot.entries[]", "(Empty Array - Press + to add)");
        } else {
            for (i, entry) in mob.loot.entries.iter().enumerate() {
                Self::add_field(table, &format!("loot.entries[{i}].item"), &entry.item);
                Self::add_field(table, &format!("loot.entries[{i}].chance"), entry.chance);
                if let Some(ref tc) = entry.treasure_class {
                    Self::add_field(table, &format!("loot.entries[{i}].treasure_class"), tc);
                }
                if let Some(ref count) = entry.count {
                    Self::add_field(table, &format!("loot.entries[{i}].count.min"), count.min);
                    Self::add_field(table, &format!("loot.entries[{i}].count.max"), count.max);
                }
            }
        }

        // Aggro race
        if mob.aggro_race.is_empty() {
            Self::add_field(table, "aggro_race[]", "(Empty Array - Press + to add)");
        } else {
            for (i, race_id) in mob.aggro_race.iter().enumerate() {
                Self::add_field(table, &format!("aggro_race[{i}]"), race_id);
            }
        }

        // Languages
        if mob.languages.is_empty() {
            Self::add_field(table, "languages[]", "(Empty Array - Press + to add)");
        } else {
            for (i, lang) in mob.languages.iter().enumerate() {
                Self::add_field(table, &format!("languages[{i}]"), lang);
            }
        }

        // Skills
        if mob.skills.is_empty() {
            Self::add_field(table, "skills[]", "(Empty Array - Press + to add)");
        } else {
            for (i, skill) in mob.skills.iter().enumerate() {
                Self::add_field(table, &format!("skills[{i}].id"), &skill.id);
                Self::add_field(table, &format!("skills[{i}].level"), skill.level);
            }
        }

        // Trainer types
        if mob.trainer_types.is_empty() {
            Self::add_field(table, "trainer_types[]", "(Empty Array - Press + to add)");
        } else {
            for (i, trainer_type) in mob.trainer_types.iter().enumerate() {
                Self::add_field(table, &format!("trainer_types[{i}]"), trainer_type);
            }
        }

        // Scripts/hooks
        if mob.scripts.is_empty() {
            Self::add_field(table, "scripts[]", "(Empty Array - Press + to add)");
        } else {
            for (i, script) in mob.scripts.iter().enumerate() {
                Self::add_field(table, &format!("scripts[{i}].event"), &script.event);
                Self::add_field(table, &format!("scripts[{i}].script"), &script.script);
            }
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
            "damage" => {
                mob.damage = if value.is_empty() {
                    None
                } else {
                    Some(value.to_string())
                }
            }
            "damage_type" => {
                mob.damage_type = if value.is_empty() {
                    None
                } else {
                    Some(value.to_string())
                }
            }
            "race" => {
                mob.race = if value.is_empty() {
                    None
                } else {
                    Some(value.to_string())
                }
            }
            "faction" => {
                mob.faction = if value.is_empty() {
                    None
                } else {
                    Some(value.to_string())
                }
            }
            "shop" => {
                mob.shop = if value.is_empty() {
                    None
                } else {
                    Some(value.to_string())
                }
            }
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
            _ if field.starts_with("loot.entries[") => {
                let rest = field.trim_start_matches("loot.entries[");
                let (idx_str, path_rest) = rest.split_once(']').ok_or("invalid loot entry path")?;
                let idx: usize = idx_str.parse().map_err(|_| "invalid index")?;
                if idx < mob.loot.entries.len() {
                    let entry = &mut mob.loot.entries[idx];
                    if path_rest == ".item" {
                        entry.item = value.to_string();
                    } else if path_rest == ".chance" {
                        entry.chance = value.parse().map_err(|_| "invalid number")?;
                    } else if path_rest == ".treasure_class" {
                        entry.treasure_class = if value.is_empty() {
                            None
                        } else {
                            Some(value.to_string())
                        };
                    } else if path_rest == ".count.min" {
                        let min = value.parse().map_err(|_| "invalid number")?;
                        if let Some(ref mut c) = entry.count {
                            c.min = min;
                        } else {
                            entry.count = Some(oxide_core::templates::CountRange { min, max: min });
                        }
                    } else if path_rest == ".count.max" {
                        let max = value.parse().map_err(|_| "invalid number")?;
                        if let Some(ref mut c) = entry.count {
                            c.max = max;
                        } else {
                            entry.count = Some(oxide_core::templates::CountRange { min: max, max });
                        }
                    }
                }
            }
            _ if field.starts_with("aggro_race[") => {
                let idx: usize = field
                    .trim_start_matches("aggro_race[")
                    .trim_end_matches(']')
                    .parse()
                    .map_err(|_| "invalid index")?;
                if idx < mob.aggro_race.len() {
                    mob.aggro_race[idx] = value.to_string();
                }
            }
            _ if field.starts_with("languages[") => {
                let idx: usize = field
                    .trim_start_matches("languages[")
                    .trim_end_matches(']')
                    .parse()
                    .map_err(|_| "invalid index")?;
                if idx < mob.languages.len() {
                    mob.languages[idx] = value.to_string();
                }
            }
            _ if field.starts_with("trainer_types[") => {
                let idx: usize = field
                    .trim_start_matches("trainer_types[")
                    .trim_end_matches(']')
                    .parse()
                    .map_err(|_| "invalid index")?;
                if idx < mob.trainer_types.len() {
                    mob.trainer_types[idx] = value.to_string();
                }
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

    pub(super) fn add_mob_array(&mut self, prefix: &str, index: usize) -> Result<(), String> {
        let mob = self
            .registry
            .mobs
            .get_mut(&self.template_id)
            .ok_or("mob not found")?;
        match prefix {
            "loot.entries" => {
                mob.loot.entries.insert(
                    (index + 1).min(mob.loot.entries.len()),
                    oxide_core::templates::LootEntry {
                        item: "item_id".to_string(),
                        treasure_class: None,
                        count: None,
                        chance: 100,
                    },
                );
            }
            "aggro_race" => {
                mob.aggro_race
                    .insert((index + 1).min(mob.aggro_race.len()), "human".to_string());
            }
            "languages" => {
                mob.languages
                    .insert((index + 1).min(mob.languages.len()), "common".to_string());
            }
            "skills" => {
                mob.skills.insert(
                    (index + 1).min(mob.skills.len()),
                    oxide_core::templates::MobSkillEntry {
                        id: "skill_id".to_string(),
                        level: 1,
                    },
                );
            }
            "trainer_types" => {
                mob.trainer_types.insert(
                    (index + 1).min(mob.trainer_types.len()),
                    "combat".to_string(),
                );
            }
            "scripts" => {
                mob.scripts.insert(
                    (index + 1).min(mob.scripts.len()),
                    oxide_core::templates::ScriptHookEntry {
                        event: "on_spawn".to_string(),
                        script: "scripts/mobs/default.rhai".to_string(),
                    },
                );
            }
            _ => return Err(format!("unknown mob array: {prefix}")),
        }
        Ok(())
    }

    pub(super) fn remove_mob_array(&mut self, prefix: &str, index: usize) -> Result<(), String> {
        let mob = self
            .registry
            .mobs
            .get_mut(&self.template_id)
            .ok_or("mob not found")?;
        match prefix {
            "loot.entries" => {
                if index < mob.loot.entries.len() {
                    mob.loot.entries.remove(index);
                }
            }
            "aggro_race" => {
                if index < mob.aggro_race.len() {
                    mob.aggro_race.remove(index);
                }
            }
            "languages" => {
                if index < mob.languages.len() {
                    mob.languages.remove(index);
                }
            }
            "skills" => {
                if index < mob.skills.len() {
                    mob.skills.remove(index);
                }
            }
            "trainer_types" => {
                if index < mob.trainer_types.len() {
                    mob.trainer_types.remove(index);
                }
            }
            "scripts" => {
                if index < mob.scripts.len() {
                    mob.scripts.remove(index);
                }
            }
            _ => return Err(format!("unknown mob array: {prefix}")),
        }
        Ok(())
    }
}
