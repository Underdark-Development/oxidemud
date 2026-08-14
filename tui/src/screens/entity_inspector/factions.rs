use crate::components::Table;
use crate::screens::entity_inspector::EntityInspectorScreen;

impl EntityInspectorScreen {
    pub(super) fn load_factions(&self, table: &mut Table) {
        let faction = match self.registry.factions.get(&self.template_id) {
            Some(f) => f,
            None => return,
        };

        Self::add_field(table, "id", &faction.id);
        Self::add_field(table, "name", &faction.name);
        Self::add_field(table, "description", &faction.description);
        Self::add_field(table, "starting_standing", faction.starting_standing);
        Self::add_field(table, "min_standing", faction.min_standing);
        Self::add_field(table, "max_standing", faction.max_standing);
        Self::add_field(table, "aggro_below", faction.aggro_below);

        Self::add_array_header(table, "ranks", faction.ranks.len());
        for (i, r) in faction.ranks.iter().enumerate() {
            Self::add_array_item(
                table,
                &format!("ranks[{i}]"),
                format!("{}: threshold {}", r.name, r.threshold),
            );
        }
    }

    pub(super) fn update_factions(&mut self, field: &str, value: &str) -> Result<(), String> {
        let faction = match self.registry.factions.get_mut(&self.template_id) {
            Some(f) => f,
            None => return Err(format!("Faction template not found: {}", self.template_id)),
        };

        match field {
            "id" => {
                faction.id = value.to_string();
            }
            "name" => {
                faction.name = value.to_string();
            }
            "description" => {
                faction.description = value.to_string();
            }
            "starting_standing" => {
                faction.starting_standing = value
                    .parse()
                    .map_err(|_| "starting_standing must be an i32".to_string())?;
            }
            "min_standing" => {
                faction.min_standing = value
                    .parse()
                    .map_err(|_| "min_standing must be an i32".to_string())?;
            }
            "max_standing" => {
                faction.max_standing = value
                    .parse()
                    .map_err(|_| "max_standing must be an i32".to_string())?;
            }
            "aggro_below" => {
                faction.aggro_below = value
                    .parse()
                    .map_err(|_| "aggro_below must be an i32".to_string())?;
            }
            _ => {}
        }
        Ok(())
    }

    pub(super) fn add_faction_array(&mut self, prefix: &str, index: usize) -> Result<(), String> {
        let faction = self
            .registry
            .factions
            .get_mut(&self.template_id)
            .ok_or("faction not found")?;
        match prefix {
            "ranks" => {
                faction.ranks.insert(
                    (index + 1).min(faction.ranks.len()),
                    oxide_core::templates::FactionRank {
                        name: "New Rank".to_string(),
                        threshold: 0,
                    },
                );
            }
            _ => return Err(format!("unknown faction array: {prefix}")),
        }
        Ok(())
    }

    pub(super) fn remove_faction_array(
        &mut self,
        prefix: &str,
        index: usize,
    ) -> Result<(), String> {
        let faction = self
            .registry
            .factions
            .get_mut(&self.template_id)
            .ok_or("faction not found")?;
        match prefix {
            "ranks" => {
                if index < faction.ranks.len() {
                    faction.ranks.remove(index);
                }
            }
            _ => return Err(format!("unknown faction array: {prefix}")),
        }
        Ok(())
    }

    pub(super) fn clear_faction_array(&mut self, prefix: &str) -> Result<(), String> {
        let faction = self
            .registry
            .factions
            .get_mut(&self.template_id)
            .ok_or("faction not found")?;
        match prefix {
            "ranks" => faction.ranks.clear(),
            _ => return Err(format!("unknown faction array: {prefix}")),
        }
        Ok(())
    }
}
