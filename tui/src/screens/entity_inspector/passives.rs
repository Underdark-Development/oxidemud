use crate::components::Table;
use crate::screens::entity_inspector::EntityInspectorScreen;

impl EntityInspectorScreen {
    pub(super) fn load_passives(&self, table: &mut Table) {
        let passive = match self.registry.passives.get(&self.template_id) {
            Some(p) => p,
            None => return,
        };
        Self::add_field(table, "id", &passive.id);
        Self::add_field(table, "name", &passive.name);
        Self::add_field(table, "description", &passive.description);
        for (i, effect) in passive.effects.iter().enumerate() {
            Self::add_field(
                table,
                &format!("effects[{i}].effect_type"),
                &effect.effect_type,
            );
            Self::add_field(table, &format!("effects[{i}].target"), &effect.target);
            if let Some(ref amt) = effect.amount {
                Self::add_field(table, &format!("effects[{i}].amount"), amt);
            }
        }
    }

    pub(super) fn update_passives(&mut self, field: &str, value: &str) -> Result<(), String> {
        let passive = self
            .registry
            .passives
            .get_mut(&self.template_id)
            .ok_or_else(|| "passive not found".to_string())?;
        match field {
            "id" => passive.id = value.to_string(),
            "name" => passive.name = value.to_string(),
            "description" => passive.description = value.to_string(),
            _ if field.starts_with("effects[") => {
                let rest = field.trim_start_matches("effects[");
                let (idx_str, path_rest) = rest.split_once(']').ok_or("invalid effect path")?;
                let idx: usize = idx_str.parse().map_err(|_| "invalid index")?;
                if idx < passive.effects.len() {
                    let e = &mut passive.effects[idx];
                    if path_rest == ".effect_type" {
                        e.effect_type = value.to_string();
                    } else if path_rest == ".target" {
                        e.target = value.to_string();
                    } else if path_rest == ".amount" {
                        e.amount = Some(value.parse().map_err(|_| "invalid number")?);
                    }
                }
            }
            _ => return Err(format!("unknown field: {field}")),
        }
        Ok(())
    }
}
