use oxide_core::templates::DiceString;

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
        for (i, cls) in item.allowed_classes.iter().enumerate() {
            Self::add_field(table, &format!("allowed_classes[{i}]"), cls);
        }
        for (i, race) in item.allowed_races.iter().enumerate() {
            Self::add_field(table, &format!("allowed_races[{i}]"), race);
        }
        for (i, align) in item.allowed_alignments.iter().enumerate() {
            Self::add_field(table, &format!("allowed_alignments[{i}]"), align);
        }
        if let Some(ref req) = item.requires_skill {
            Self::add_field(table, "requires_skill.id", &req.id);
            Self::add_field(table, "requires_skill.level", req.level);
        }
        if let Some(ref w) = item.weapon {
            Self::add_field(table, "weapon.damage", w.damage.as_str());
            Self::add_field(table, "weapon.damage_type", &w.damage_type);
            Self::add_field(table, "weapon.speed", w.speed);
            Self::add_field(table, "weapon.range", &w.range);
        }
        if let Some(ref eq) = item.equipment {
            Self::add_field(table, "equipment.slot", &eq.slot);
        }
        if let Some(ref set) = item.set {
            Self::add_field(table, "set.id", &set.id);
            Self::add_field(table, "set.piece_type", &set.piece_type);
        }
        for (i, trigger) in item.triggers.iter().enumerate() {
            Self::add_field(table, &format!("triggers[{i}].event"), &trigger.event);
            Self::add_field(table, &format!("triggers[{i}].chance"), trigger.chance);
            Self::add_field(table, &format!("triggers[{i}].cast"), &trigger.cast);
            Self::add_field(table, &format!("triggers[{i}].target"), &trigger.target);
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
                if let Some(ref mut r) = item.requires_skill {
                    r.id = value.to_string();
                }
            }
            "requires_skill.level" => {
                if let Some(ref mut r) = item.requires_skill {
                    r.level = value.parse().map_err(|_| "invalid number")?;
                }
            }
            "weapon.damage" => {
                if let Some(ref mut w) = item.weapon {
                    w.damage = DiceString(value.to_string());
                }
            }
            "weapon.damage_type" => {
                if let Some(ref mut w) = item.weapon {
                    w.damage_type = value.to_string();
                }
            }
            "weapon.speed" => {
                if let Some(ref mut w) = item.weapon {
                    w.speed = value.parse().map_err(|_| "invalid number")?;
                }
            }
            "weapon.range" => {
                if let Some(ref mut w) = item.weapon {
                    w.range = value.to_string();
                }
            }
            "equipment.slot" => {
                if let Some(ref mut eq) = item.equipment {
                    eq.slot = value.to_string();
                }
            }
            "set.id" => {
                if let Some(ref mut s) = item.set {
                    s.id = value.to_string();
                }
            }
            "set.piece_type" => {
                if let Some(ref mut s) = item.set {
                    s.piece_type = value.to_string();
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
}
