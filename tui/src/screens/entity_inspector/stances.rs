use crate::components::Table;
use crate::screens::entity_inspector::EntityInspectorScreen;

impl EntityInspectorScreen {
    pub(super) fn load_stances(&self, table: &mut Table) {
        let stance = match self.registry.stances.get(&self.template_id) {
            Some(s) => s,
            None => return,
        };
        Self::add_field(table, "id", &stance.id);
        Self::add_field(table, "name", &stance.name);
        Self::add_field(table, "ac_bonus", stance.ac_bonus);
        Self::add_field(table, "attack_penalty", stance.attack_penalty);
        Self::add_field(table, "damage_bonus", stance.damage_bonus);
        Self::add_field(table, "ac_penalty", stance.ac_penalty);
        Self::add_field(table, "min_level", stance.min_level);
    }

    pub(super) fn update_stances(&mut self, field: &str, value: &str) -> Result<(), String> {
        let stance = self
            .registry
            .stances
            .get_mut(&self.template_id)
            .ok_or_else(|| "stance not found".to_string())?;
        match field {
            "id" => stance.id = value.to_string(),
            "name" => stance.name = value.to_string(),
            "ac_bonus" => stance.ac_bonus = value.parse().map_err(|_| "invalid number")?,
            "attack_penalty" => {
                stance.attack_penalty = value.parse().map_err(|_| "invalid number")?
            }
            "damage_bonus" => stance.damage_bonus = value.parse().map_err(|_| "invalid number")?,
            "ac_penalty" => stance.ac_penalty = value.parse().map_err(|_| "invalid number")?,
            "min_level" => stance.min_level = value.parse().map_err(|_| "invalid number")?,
            _ => return Err(format!("unknown field: {field}")),
        }
        Ok(())
    }
}
