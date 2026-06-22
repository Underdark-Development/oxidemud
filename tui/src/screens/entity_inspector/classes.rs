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
        for (i, race) in class.allowed_races.iter().enumerate() {
            Self::add_field(table, &format!("allowed_races[{i}]"), race);
        }
        for (i, align) in class.allowed_alignments.iter().enumerate() {
            Self::add_field(table, &format!("allowed_alignments[{i}]"), align);
        }
        for (i, skill) in class.auto_skills.iter().enumerate() {
            Self::add_field(table, &format!("auto_skills[{i}]"), skill);
        }
        for (i, skill) in class.skill_pool.iter().enumerate() {
            Self::add_field(table, &format!("skill_pool[{i}]"), skill);
        }
        for (i, item) in class.starting_items.iter().enumerate() {
            Self::add_field(table, &format!("starting_items[{i}]"), item);
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
            _ if field.starts_with("allowed_races[")
                || field.starts_with("allowed_alignments[")
                || field.starts_with("auto_skills[")
                || field.starts_with("skill_pool[")
                || field.starts_with("starting_items[") => {}
            _ => return Err(format!("unknown field: {field}")),
        }
        Ok(())
    }
}
