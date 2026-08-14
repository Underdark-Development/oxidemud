use crate::components::Table;
use crate::screens::entity_inspector::EntityInspectorScreen;

impl EntityInspectorScreen {
    pub(super) fn load_classes(&self, table: &mut Table) {
        let class = match self.registry.classes.get(&self.template_id) {
            Some(c) => c,
            None => return,
        };
        Self::add_field(table, "id", &class.id);
        Self::add_field(table, "name", &class.name);
        Self::add_field(table, "description", &class.description);
        Self::add_field(table, "hit_die", class.hit_die);
        Self::add_field(table, "starting_skill_slots", class.starting_skill_slots);
        Self::add_field(table, "attribute_mods.str", class.attribute_mods.strength);
        Self::add_field(table, "attribute_mods.dex", class.attribute_mods.dexterity);
        Self::add_field(
            table,
            "attribute_mods.con",
            class.attribute_mods.constitution,
        );
        Self::add_field(
            table,
            "attribute_mods.int",
            class.attribute_mods.intelligence,
        );
        Self::add_field(table, "attribute_mods.wis", class.attribute_mods.wisdom);
        Self::add_field(table, "attribute_mods.cha", class.attribute_mods.charisma);

        // Allowed Races
        Self::add_array_header(table, "allowed_races", class.allowed_races.len());
        for (i, race) in class.allowed_races.iter().enumerate() {
            Self::add_array_item(table, &format!("allowed_races[{i}]"), race);
        }

        // Allowed Alignments
        Self::add_array_header(table, "allowed_alignments", class.allowed_alignments.len());
        for (i, align) in class.allowed_alignments.iter().enumerate() {
            Self::add_array_item(table, &format!("allowed_alignments[{i}]"), align);
        }

        // Auto Skills
        Self::add_array_header(table, "auto_skills", class.auto_skills.len());
        for (i, skill) in class.auto_skills.iter().enumerate() {
            Self::add_array_item(table, &format!("auto_skills[{i}]"), skill);
        }

        // Skill Pool
        Self::add_array_header(table, "skill_pool", class.skill_pool.len());
        for (i, skill) in class.skill_pool.iter().enumerate() {
            Self::add_array_item(table, &format!("skill_pool[{i}]"), skill);
        }

        // Starting Items
        Self::add_array_header(table, "starting_items", class.starting_items.len());
        for (i, item) in class.starting_items.iter().enumerate() {
            Self::add_array_item(table, &format!("starting_items[{i}]"), item);
        }

        Self::add_field(table, "starting_gold.copper", class.starting_gold.copper);
        Self::add_field(table, "starting_gold.silver", class.starting_gold.silver);
        Self::add_field(table, "starting_gold.gold", class.starting_gold.gold);
        Self::add_field(
            table,
            "starting_gold.platinum",
            class.starting_gold.platinum,
        );
    }

    pub(super) fn update_classes(&mut self, field: &str, value: &str) -> Result<(), String> {
        let cls = self
            .registry
            .classes
            .get_mut(&self.template_id)
            .ok_or_else(|| "class not found".to_string())?;
        match field {
            "id" => cls.id = value.to_string(),
            "name" => cls.name = value.to_string(),
            "description" => cls.description = value.to_string(),
            "hit_die" => cls.hit_die = value.parse().map_err(|_| "invalid number")?,
            "starting_skill_slots" => {
                cls.starting_skill_slots = value.parse().map_err(|_| "invalid number")?
            }
            "attribute_mods.str" => {
                cls.attribute_mods.strength = value.parse().map_err(|_| "invalid number")?
            }
            "attribute_mods.dex" => {
                cls.attribute_mods.dexterity = value.parse().map_err(|_| "invalid number")?
            }
            "attribute_mods.con" => {
                cls.attribute_mods.constitution = value.parse().map_err(|_| "invalid number")?
            }
            "attribute_mods.int" => {
                cls.attribute_mods.intelligence = value.parse().map_err(|_| "invalid number")?
            }
            "attribute_mods.wis" => {
                cls.attribute_mods.wisdom = value.parse().map_err(|_| "invalid number")?
            }
            "attribute_mods.cha" => {
                cls.attribute_mods.charisma = value.parse().map_err(|_| "invalid number")?
            }
            "starting_gold.copper" => {
                cls.starting_gold.copper = value.parse().map_err(|_| "invalid number")?
            }
            "starting_gold.silver" => {
                cls.starting_gold.silver = value.parse().map_err(|_| "invalid number")?
            }
            "starting_gold.gold" => {
                cls.starting_gold.gold = value.parse().map_err(|_| "invalid number")?
            }
            "starting_gold.platinum" => {
                cls.starting_gold.platinum = value.parse().map_err(|_| "invalid number")?
            }
            _ if field.starts_with("allowed_races[") => {
                let idx: usize = field
                    .trim_start_matches("allowed_races[")
                    .trim_end_matches(']')
                    .parse()
                    .map_err(|_| "invalid index")?;
                if idx < cls.allowed_races.len() {
                    cls.allowed_races[idx] = value.to_string();
                }
            }
            _ if field.starts_with("allowed_alignments[") => {
                let idx: usize = field
                    .trim_start_matches("allowed_alignments[")
                    .trim_end_matches(']')
                    .parse()
                    .map_err(|_| "invalid index")?;
                if idx < cls.allowed_alignments.len() {
                    cls.allowed_alignments[idx] = value.to_string();
                }
            }
            _ if field.starts_with("auto_skills[") => {
                let idx: usize = field
                    .trim_start_matches("auto_skills[")
                    .trim_end_matches(']')
                    .parse()
                    .map_err(|_| "invalid index")?;
                if idx < cls.auto_skills.len() {
                    cls.auto_skills[idx] = value.to_string();
                }
            }
            _ if field.starts_with("skill_pool[") => {
                let idx: usize = field
                    .trim_start_matches("skill_pool[")
                    .trim_end_matches(']')
                    .parse()
                    .map_err(|_| "invalid index")?;
                if idx < cls.skill_pool.len() {
                    cls.skill_pool[idx] = value.to_string();
                }
            }
            _ if field.starts_with("starting_items[") => {
                let idx: usize = field
                    .trim_start_matches("starting_items[")
                    .trim_end_matches(']')
                    .parse()
                    .map_err(|_| "invalid index")?;
                if idx < cls.starting_items.len() {
                    cls.starting_items[idx] = value.to_string();
                }
            }
            _ => return Err(format!("unknown field: {field}")),
        }
        Ok(())
    }

    pub(super) fn add_class_array(&mut self, prefix: &str, index: usize) -> Result<(), String> {
        let cls = self
            .registry
            .classes
            .get_mut(&self.template_id)
            .ok_or("class not found")?;
        match prefix {
            "allowed_races" => {
                cls.allowed_races.insert(
                    (index + 1).min(cls.allowed_races.len()),
                    "human".to_string(),
                );
            }
            "allowed_alignments" => {
                cls.allowed_alignments.insert(
                    (index + 1).min(cls.allowed_alignments.len()),
                    "good".to_string(),
                );
            }
            "auto_skills" => {
                cls.auto_skills.insert(
                    (index + 1).min(cls.auto_skills.len()),
                    "skill_id".to_string(),
                );
            }
            "skill_pool" => {
                cls.skill_pool.insert(
                    (index + 1).min(cls.skill_pool.len()),
                    "skill_id".to_string(),
                );
            }
            "starting_items" => {
                cls.starting_items.insert(
                    (index + 1).min(cls.starting_items.len()),
                    "item_id".to_string(),
                );
            }
            _ => return Err(format!("unknown class array: {prefix}")),
        }
        Ok(())
    }

    pub(super) fn remove_class_array(&mut self, prefix: &str, index: usize) -> Result<(), String> {
        let cls = self
            .registry
            .classes
            .get_mut(&self.template_id)
            .ok_or("class not found")?;
        match prefix {
            "allowed_races" => {
                if index < cls.allowed_races.len() {
                    cls.allowed_races.remove(index);
                }
            }
            "allowed_alignments" => {
                if index < cls.allowed_alignments.len() {
                    cls.allowed_alignments.remove(index);
                }
            }
            "auto_skills" => {
                if index < cls.auto_skills.len() {
                    cls.auto_skills.remove(index);
                }
            }
            "skill_pool" => {
                if index < cls.skill_pool.len() {
                    cls.skill_pool.remove(index);
                }
            }
            "starting_items" => {
                if index < cls.starting_items.len() {
                    cls.starting_items.remove(index);
                }
            }
            _ => return Err(format!("unknown class array: {prefix}")),
        }
        Ok(())
    }

    pub(super) fn clear_class_array(&mut self, prefix: &str) -> Result<(), String> {
        let cls = self
            .registry
            .classes
            .get_mut(&self.template_id)
            .ok_or("class not found")?;
        match prefix {
            "allowed_races" => cls.allowed_races.clear(),
            "allowed_alignments" => cls.allowed_alignments.clear(),
            "auto_skills" => cls.auto_skills.clear(),
            "skill_pool" => cls.skill_pool.clear(),
            "starting_items" => cls.starting_items.clear(),
            _ => return Err(format!("unknown class array: {prefix}")),
        }
        Ok(())
    }
}
