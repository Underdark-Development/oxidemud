use crate::components::Table;
use crate::screens::entity_inspector::EntityInspectorScreen;

impl EntityInspectorScreen {
    pub(super) fn load_items(&self, table: &mut Table) {
        let item = match self.registry.items.get(&self.template_id) {
            Some(i) => i,
            None => return,
        };
        Self::add_field(table, "id", &item.id);
        Self::add_field(table, "name", &item.name);
        Self::add_field(table, "description", &item.description);
        Self::add_field(table, "item_type", &item.item_type);
        Self::add_field(table, "subtype", &item.subtype);
        Self::add_field(table, "quality", &item.quality);
        Self::add_field(table, "level_requirement", item.level_requirement);
        Self::add_field(table, "weight", item.weight);
        Self::add_field(table, "value", item.value);
        Self::add_field(table, "flags", item.flags.join(", "));

        // Allowed Classes
        if item.allowed_classes.is_empty() {
            Self::add_field(table, "allowed_classes[]", "(Empty Array - Press + to add)");
        } else {
            for (i, cls) in item.allowed_classes.iter().enumerate() {
                Self::add_field(table, &format!("allowed_classes[{i}]"), cls);
            }
        }

        // Allowed Races
        if item.allowed_races.is_empty() {
            Self::add_field(table, "allowed_races[]", "(Empty Array - Press + to add)");
        } else {
            for (i, race) in item.allowed_races.iter().enumerate() {
                Self::add_field(table, &format!("allowed_races[{i}]"), race);
            }
        }

        // Allowed Alignments
        if item.allowed_alignments.is_empty() {
            Self::add_field(
                table,
                "allowed_alignments[]",
                "(Empty Array - Press + to add)",
            );
        } else {
            for (i, align) in item.allowed_alignments.iter().enumerate() {
                Self::add_field(table, &format!("allowed_alignments[{i}]"), align);
            }
        }

        // Optionals: Requires Skill
        let req_id = item
            .requires_skill
            .as_ref()
            .map(|r| r.id.as_str())
            .unwrap_or("");
        let req_level = item
            .requires_skill
            .as_ref()
            .map(|r| r.level.to_string())
            .unwrap_or_default();
        Self::add_field(table, "requires_skill.id", req_id);
        Self::add_field(table, "requires_skill.level", req_level);

        // Optionals: Weapon
        let weapon_damage = item
            .weapon
            .as_ref()
            .map(|w| w.damage.as_str())
            .unwrap_or("");
        let weapon_type = item
            .weapon
            .as_ref()
            .map(|w| w.damage_type.as_str())
            .unwrap_or("");
        let weapon_speed = item
            .weapon
            .as_ref()
            .map(|w| w.speed.to_string())
            .unwrap_or_default();
        let weapon_range = item.weapon.as_ref().map(|w| w.range.as_str()).unwrap_or("");
        Self::add_field(table, "weapon.damage", weapon_damage);
        Self::add_field(table, "weapon.damage_type", weapon_type);
        Self::add_field(table, "weapon.speed", weapon_speed);
        Self::add_field(table, "weapon.range", weapon_range);

        // Optionals: Equipment
        let eq_slot = item
            .equipment
            .as_ref()
            .map(|e| e.slot.as_str())
            .unwrap_or("");
        Self::add_field(table, "equipment.slot", eq_slot);

        // Optionals: Set
        let set_id = item.set.as_ref().map(|s| s.id.as_str()).unwrap_or("");
        let set_piece = item
            .set
            .as_ref()
            .map(|s| s.piece_type.as_str())
            .unwrap_or("");
        Self::add_field(table, "set.id", set_id);
        Self::add_field(table, "set.piece_type", set_piece);

        // Triggers
        if item.triggers.is_empty() {
            Self::add_field(table, "triggers[]", "(Empty Array - Press + to add)");
        } else {
            for (i, trigger) in item.triggers.iter().enumerate() {
                Self::add_field(table, &format!("triggers[{i}].event"), &trigger.event);
                Self::add_field(table, &format!("triggers[{i}].chance"), trigger.chance);
                Self::add_field(table, &format!("triggers[{i}].cast"), &trigger.cast);
                Self::add_field(table, &format!("triggers[{i}].target"), &trigger.target);
            }
        }
    }

    pub(super) fn update_items(&mut self, field: &str, value: &str) -> Result<(), String> {
        let item = self
            .registry
            .items
            .get_mut(&self.template_id)
            .ok_or_else(|| "item not found".to_string())?;
        match field {
            "id" => item.id = value.to_string(),
            "name" => item.name = value.to_string(),
            "description" => item.description = value.to_string(),
            "item_type" => item.item_type = value.to_string(),
            "subtype" => item.subtype = value.to_string(),
            "quality" => item.quality = value.to_string(),
            "level_requirement" => {
                item.level_requirement = value.parse().map_err(|_| "invalid number")?
            }
            "weight" => item.weight = value.parse().map_err(|_| "invalid number")?,
            "value" => item.value = value.parse().map_err(|_| "invalid number")?,
            "flags" => item.flags = value.split(',').map(|s| s.trim().to_string()).collect(),
            _ if field.starts_with("allowed_classes[") => {
                let idx: usize = field
                    .trim_start_matches("allowed_classes[")
                    .trim_end_matches(']')
                    .parse()
                    .map_err(|_| "invalid index")?;
                if idx < item.allowed_classes.len() {
                    item.allowed_classes[idx] = value.to_string();
                }
            }
            _ if field.starts_with("allowed_races[") => {
                let idx: usize = field
                    .trim_start_matches("allowed_races[")
                    .trim_end_matches(']')
                    .parse()
                    .map_err(|_| "invalid index")?;
                if idx < item.allowed_races.len() {
                    item.allowed_races[idx] = value.to_string();
                }
            }
            _ if field.starts_with("allowed_alignments[") => {
                let idx: usize = field
                    .trim_start_matches("allowed_alignments[")
                    .trim_end_matches(']')
                    .parse()
                    .map_err(|_| "invalid index")?;
                if idx < item.allowed_alignments.len() {
                    item.allowed_alignments[idx] = value.to_string();
                }
            }
            "requires_skill.id" => {
                if value.is_empty() {
                    if let Some(ref mut r) = item.requires_skill {
                        r.id = String::new();
                        if r.level == 0 {
                            item.requires_skill = None;
                        }
                    }
                } else {
                    if let Some(ref mut r) = item.requires_skill {
                        r.id = value.to_string();
                    } else {
                        item.requires_skill = Some(oxide_core::templates::SkillRequirement {
                            id: value.to_string(),
                            level: 1,
                        });
                    }
                }
            }
            "requires_skill.level" => {
                if value.is_empty() {
                    if let Some(ref mut r) = item.requires_skill {
                        r.level = 0;
                        if r.id.is_empty() {
                            item.requires_skill = None;
                        }
                    }
                } else {
                    let lvl = value.parse().map_err(|_| "invalid number")?;
                    if let Some(ref mut r) = item.requires_skill {
                        r.level = lvl;
                    } else {
                        item.requires_skill = Some(oxide_core::templates::SkillRequirement {
                            id: "skill_id".to_string(),
                            level: lvl,
                        });
                    }
                }
            }
            "weapon.damage" | "weapon.damage_type" | "weapon.speed" | "weapon.range" => {
                if value.is_empty() && item.weapon.is_some() {
                    if field == "weapon.damage" {
                        item.weapon = None;
                    } else if let Some(ref mut w) = item.weapon {
                        if field == "weapon.damage_type" {
                            w.damage_type = String::new();
                        } else if field == "weapon.speed" {
                            w.speed = 1.0;
                        } else if field == "weapon.range" {
                            w.range = String::new();
                        }
                    }
                } else {
                    let mut w =
                        item.weapon
                            .take()
                            .unwrap_or_else(|| oxide_core::templates::WeaponDef {
                                damage: oxide_core::templates::DiceString("1d6".to_string()),
                                damage_type: "slashing".to_string(),
                                speed: 1.5,
                                range: "melee".to_string(),
                            });
                    match field {
                        "weapon.damage" => {
                            w.damage = oxide_core::templates::DiceString(value.to_string())
                        }
                        "weapon.damage_type" => w.damage_type = value.to_string(),
                        "weapon.speed" => w.speed = value.parse().map_err(|_| "invalid number")?,
                        "weapon.range" => w.range = value.to_string(),
                        _ => {}
                    }
                    item.weapon = Some(w);
                }
            }
            "equipment.slot" => {
                if value.is_empty() {
                    item.equipment = None;
                } else {
                    item.equipment = Some(oxide_core::templates::EquipmentDef {
                        slot: value.to_string(),
                    });
                }
            }
            "set.id" | "set.piece_type" => {
                if value.is_empty() && item.set.is_some() {
                    if field == "set.id" {
                        item.set = None;
                    }
                } else {
                    let mut s =
                        item.set
                            .take()
                            .unwrap_or_else(|| oxide_core::templates::SetMembership {
                                id: "set_id".to_string(),
                                piece_type: "chest".to_string(),
                            });
                    if field == "set.id" {
                        s.id = value.to_string();
                    } else {
                        s.piece_type = value.to_string();
                    }
                    item.set = Some(s);
                }
            }
            _ if field.starts_with("triggers[") => {
                let rest = field.trim_start_matches("triggers[");
                let (idx_str, _rest) = rest.split_once(']').ok_or("invalid trigger path")?;
                let idx: usize = idx_str.parse().map_err(|_| "invalid index")?;
                if idx < item.triggers.len() {
                    let t = &mut item.triggers[idx];
                    if rest.contains(".event") {
                        t.event = value.to_string();
                    } else if rest.contains(".chance") {
                        t.chance = value.parse().map_err(|_| "invalid number")?;
                    } else if rest.contains(".cast") {
                        t.cast = value.to_string();
                    } else if rest.contains(".target") {
                        t.target = value.to_string();
                    }
                }
            }
            _ => return Err(format!("unknown field: {field}")),
        }
        Ok(())
    }

    pub(super) fn add_item_array(&mut self, prefix: &str, index: usize) -> Result<(), String> {
        let item = self
            .registry
            .items
            .get_mut(&self.template_id)
            .ok_or("item not found")?;
        match prefix {
            "allowed_classes" => {
                item.allowed_classes.insert(
                    (index + 1).min(item.allowed_classes.len()),
                    "warrior".to_string(),
                );
            }
            "allowed_races" => {
                item.allowed_races.insert(
                    (index + 1).min(item.allowed_races.len()),
                    "human".to_string(),
                );
            }
            "allowed_alignments" => {
                item.allowed_alignments.insert(
                    (index + 1).min(item.allowed_alignments.len()),
                    "good".to_string(),
                );
            }
            "triggers" => {
                item.triggers.insert(
                    (index + 1).min(item.triggers.len()),
                    oxide_core::templates::TriggerDef {
                        event: "on_hit".to_string(),
                        chance: 10,
                        cast: "spell_id".to_string(),
                        target: "target".to_string(),
                        script: None,
                    },
                );
            }
            _ => return Err(format!("unknown item array: {prefix}")),
        }
        Ok(())
    }

    pub(super) fn remove_item_array(&mut self, prefix: &str, index: usize) -> Result<(), String> {
        let item = self
            .registry
            .items
            .get_mut(&self.template_id)
            .ok_or("item not found")?;
        match prefix {
            "allowed_classes" => {
                if index < item.allowed_classes.len() {
                    item.allowed_classes.remove(index);
                }
            }
            "allowed_races" => {
                if index < item.allowed_races.len() {
                    item.allowed_races.remove(index);
                }
            }
            "allowed_alignments" => {
                if index < item.allowed_alignments.len() {
                    item.allowed_alignments.remove(index);
                }
            }
            "triggers" => {
                if index < item.triggers.len() {
                    item.triggers.remove(index);
                }
            }
            _ => return Err(format!("unknown item array: {prefix}")),
        }
        Ok(())
    }
}
