use crate::components::Table;
use crate::screens::entity_inspector::EntityInspectorScreen;

impl EntityInspectorScreen {
    pub(super) fn load_skills(&self, table: &mut Table) {
        let skill = match self.registry.skills.get(&self.template_id) {
            Some(s) => s,
            None => return,
        };
        Self::add_field(table, "id", &skill.id);
        Self::add_field(table, "name", &skill.name);
        Self::add_field(table, "description", &skill.description);
        Self::add_field(table, "skill_type", format!("{:?}", skill.skill_type));
        Self::add_field(table, "max_rank", skill.max_rank);
    }

    pub(super) fn update_skills(&mut self, field: &str, value: &str) -> Result<(), String> {
        let skill = self
            .registry
            .skills
            .get_mut(&self.template_id)
            .ok_or_else(|| "skill not found".to_string())?;
        match field {
            "id" => skill.id = value.to_string(),
            "name" => skill.name = value.to_string(),
            "description" => skill.description = value.to_string(),
            "max_rank" => skill.max_rank = value.parse().map_err(|_| "invalid number")?,
            _ => return Err(format!("unknown field: {field}")),
        }
        Ok(())
    }
}
