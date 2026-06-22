use crate::components::Table;
use crate::screens::entity_inspector::EntityInspectorScreen;

impl EntityInspectorScreen {
    pub(super) fn load_affixes(&self, table: &mut Table) {
        let affix = match self.registry.affixes.get(&self.template_id) {
            Some(a) => a,
            None => return,
        };
        Self::add_field(table, "id", &affix.id);
        Self::add_field(table, "name", &affix.name);
        Self::add_field(table, "description", &affix.description);
        Self::add_field(table, "type", &affix.affix_type);
        Self::add_field(table, "quality_min", &affix.quality_min);
        Self::add_field(table, "weight", affix.weight);
        Self::add_field(table, "slot", affix.slot.join(", "));
        if let Some(ref el) = affix.element {
            Self::add_field(table, "element", el);
        }
        if let Some(ref amt) = affix.amount {
            Self::add_field(table, "amount", amt);
        }
        if let Some(ref stat) = affix.stat {
            Self::add_field(table, "stat", stat);
        }
    }

    pub(super) fn update_affixes(&mut self, field: &str, value: &str) -> Result<(), String> {
        let affix = self
            .registry
            .affixes
            .get_mut(&self.template_id)
            .ok_or_else(|| "affix not found".to_string())?;
        match field {
            "id" => affix.id = value.to_string(),
            "name" => affix.name = value.to_string(),
            "description" => affix.description = value.to_string(),
            "type" => affix.affix_type = value.to_string(),
            "quality_min" => affix.quality_min = value.to_string(),
            "weight" => affix.weight = value.parse().map_err(|_| "invalid number")?,
            "slot" => affix.slot = value.split(',').map(|s| s.trim().to_string()).collect(),
            "element" => affix.element = Some(value.to_string()),
            "amount" => affix.amount = Some(value.to_string()),
            "stat" => affix.stat = Some(value.to_string()),
            _ => return Err(format!("unknown field: {field}")),
        }
        Ok(())
    }
}
