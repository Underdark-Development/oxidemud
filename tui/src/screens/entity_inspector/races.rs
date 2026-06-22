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
        for (i, cls) in race.allowed_classes.iter().enumerate() {
            Self::add_field(table, &format!("allowed_classes[{i}]"), cls);
        }
        for (i, align) in race.allowed_alignments.iter().enumerate() {
            Self::add_field(table, &format!("allowed_alignments[{i}]"), align);
        }
        for (i, ability) in race.racial_abilities.iter().enumerate() {
            Self::add_field(table, &format!("racial_abilities[{i}]"), ability);
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
            _ if field.starts_with("allowed_classes[")
                || field.starts_with("allowed_alignments[")
                || field.starts_with("racial_abilities[") => {}
            _ => return Err(format!("unknown field: {field}")),
        }
        Ok(())
    }
}
