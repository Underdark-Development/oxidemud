use crate::components::Table;
use crate::screens::entity_inspector::EntityInspectorScreen;

impl EntityInspectorScreen {
    pub(super) fn load_races(&self, table: &mut Table) {
        let race = match self.registry.races.get(&self.template_id) {
            Some(r) => r,
            None => return,
        };
        Self::add_field(table, "id", &race.id);
        Self::add_field(table, "name", &race.name);
        Self::add_field(table, "description", &race.description);
        Self::add_field(table, "attributes.str", race.attributes.strength);
        Self::add_field(table, "attributes.dex", race.attributes.dexterity);
        Self::add_field(table, "attributes.con", race.attributes.constitution);
        Self::add_field(table, "attributes.int", race.attributes.intelligence);
        Self::add_field(table, "attributes.wis", race.attributes.wisdom);
        Self::add_field(table, "attributes.cha", race.attributes.charisma);

        // Allowed Classes
        Self::add_array_header(table, "allowed_classes", race.allowed_classes.len());
        for (i, cls) in race.allowed_classes.iter().enumerate() {
            Self::add_array_item(table, &format!("allowed_classes[{i}]"), cls);
        }

        // Allowed Alignments
        Self::add_array_header(table, "allowed_alignments", race.allowed_alignments.len());
        for (i, align) in race.allowed_alignments.iter().enumerate() {
            Self::add_array_item(table, &format!("allowed_alignments[{i}]"), align);
        }

        // Racial Abilities
        Self::add_array_header(table, "racial_abilities", race.racial_abilities.len());
        for (i, ability) in race.racial_abilities.iter().enumerate() {
            Self::add_array_item(table, &format!("racial_abilities[{i}]"), ability);
        }
    }

    pub(super) fn update_races(&mut self, field: &str, value: &str) -> Result<(), String> {
        let race = self
            .registry
            .races
            .get_mut(&self.template_id)
            .ok_or_else(|| "race not found".to_string())?;
        match field {
            "id" => race.id = value.to_string(),
            "name" => race.name = value.to_string(),
            "description" => race.description = value.to_string(),
            "attributes.str" => {
                race.attributes.strength = value.parse().map_err(|_| "invalid number")?
            }
            "attributes.dex" => {
                race.attributes.dexterity = value.parse().map_err(|_| "invalid number")?
            }
            "attributes.con" => {
                race.attributes.constitution = value.parse().map_err(|_| "invalid number")?
            }
            "attributes.int" => {
                race.attributes.intelligence = value.parse().map_err(|_| "invalid number")?
            }
            "attributes.wis" => {
                race.attributes.wisdom = value.parse().map_err(|_| "invalid number")?
            }
            "attributes.cha" => {
                race.attributes.charisma = value.parse().map_err(|_| "invalid number")?
            }
            _ if field.starts_with("allowed_classes[") => {
                let idx: usize = field
                    .trim_start_matches("allowed_classes[")
                    .trim_end_matches(']')
                    .parse()
                    .map_err(|_| "invalid index")?;
                if idx < race.allowed_classes.len() {
                    race.allowed_classes[idx] = value.to_string();
                }
            }
            _ if field.starts_with("allowed_alignments[") => {
                let idx: usize = field
                    .trim_start_matches("allowed_alignments[")
                    .trim_end_matches(']')
                    .parse()
                    .map_err(|_| "invalid index")?;
                if idx < race.allowed_alignments.len() {
                    race.allowed_alignments[idx] = value.to_string();
                }
            }
            _ if field.starts_with("racial_abilities[") => {
                let idx: usize = field
                    .trim_start_matches("racial_abilities[")
                    .trim_end_matches(']')
                    .parse()
                    .map_err(|_| "invalid index")?;
                if idx < race.racial_abilities.len() {
                    race.racial_abilities[idx] = value.to_string();
                }
            }
            _ => return Err(format!("unknown field: {field}")),
        }
        Ok(())
    }

    pub(super) fn add_race_array(&mut self, prefix: &str, index: usize) -> Result<(), String> {
        let race = self
            .registry
            .races
            .get_mut(&self.template_id)
            .ok_or("race not found")?;
        match prefix {
            "allowed_classes" => {
                race.allowed_classes.insert(
                    (index + 1).min(race.allowed_classes.len()),
                    "warrior".to_string(),
                );
            }
            "allowed_alignments" => {
                race.allowed_alignments.insert(
                    (index + 1).min(race.allowed_alignments.len()),
                    "good".to_string(),
                );
            }
            "racial_abilities" => {
                race.racial_abilities.insert(
                    (index + 1).min(race.racial_abilities.len()),
                    "passive_ability_id".to_string(),
                );
            }
            _ => return Err(format!("unknown race array: {prefix}")),
        }
        Ok(())
    }

    pub(super) fn remove_race_array(&mut self, prefix: &str, index: usize) -> Result<(), String> {
        let race = self
            .registry
            .races
            .get_mut(&self.template_id)
            .ok_or("race not found")?;
        match prefix {
            "allowed_classes" => {
                if index < race.allowed_classes.len() {
                    race.allowed_classes.remove(index);
                }
            }
            "allowed_alignments" => {
                if index < race.allowed_alignments.len() {
                    race.allowed_alignments.remove(index);
                }
            }
            "racial_abilities" => {
                if index < race.racial_abilities.len() {
                    race.racial_abilities.remove(index);
                }
            }
            _ => return Err(format!("unknown race array: {prefix}")),
        }
        Ok(())
    }

    pub(super) fn clear_race_array(&mut self, prefix: &str) -> Result<(), String> {
        let race = self
            .registry
            .races
            .get_mut(&self.template_id)
            .ok_or("race not found")?;
        match prefix {
            "allowed_classes" => race.allowed_classes.clear(),
            "allowed_alignments" => race.allowed_alignments.clear(),
            "racial_abilities" => race.racial_abilities.clear(),
            _ => return Err(format!("unknown race array: {prefix}")),
        }
        Ok(())
    }
}
