use crate::components::Table;
use crate::screens::entity_inspector::EntityInspectorScreen;

impl EntityInspectorScreen {
    pub(super) fn load_recipes(&self, table: &mut Table) {
        let recipe = match self.registry.recipes.get(&self.template_id) {
            Some(r) => r,
            None => return,
        };

        Self::add_field(table, "id", &recipe.id);
        Self::add_field(table, "name", &recipe.name);
        Self::add_field(table, "description", &recipe.description);
        Self::add_field(table, "station", recipe.station.as_deref().unwrap_or(""));
        Self::add_field(table, "difficulty", recipe.difficulty);
        Self::add_field(table, "success_chance", recipe.success_chance);
        Self::add_field(table, "quality_scaling", recipe.quality_scaling);
        Self::add_field(table, "script", recipe.script.as_deref().unwrap_or(""));

        if let Some(ref req) = recipe.skill_requirement {
            Self::add_field(table, "skill_requirement.id", &req.id);
            Self::add_field(table, "skill_requirement.rank", req.rank);
        } else {
            Self::add_field(table, "skill_requirement.id", "");
            Self::add_field(table, "skill_requirement.rank", 0);
        }

        Self::add_array_header(table, "materials", recipe.materials.len());
        for (i, mat) in recipe.materials.iter().enumerate() {
            Self::add_array_item(
                table,
                &format!("materials[{i}]"),
                format!("{} x{}", mat.template_id, mat.quantity),
            );
        }

        Self::add_field(table, "result.template_id", &recipe.result.template_id);
        Self::add_field(table, "result.quantity", recipe.result.quantity);
    }

    pub(super) fn update_recipes(&mut self, field: &str, value: &str) -> Result<(), String> {
        let recipe = match self.registry.recipes.get_mut(&self.template_id) {
            Some(r) => r,
            None => return Err(format!("Recipe template not found: {}", self.template_id)),
        };

        match field {
            "id" => {
                recipe.id = value.to_string();
            }
            "name" => {
                recipe.name = value.to_string();
            }
            "description" => {
                recipe.description = value.to_string();
            }
            "station" => {
                recipe.station = if value.trim().is_empty() {
                    None
                } else {
                    Some(value.to_string())
                };
            }
            "difficulty" => {
                recipe.difficulty = value
                    .parse()
                    .map_err(|_| "difficulty must be a u32".to_string())?;
            }
            "success_chance" => {
                recipe.success_chance = value
                    .parse()
                    .map_err(|_| "success_chance must be a u8".to_string())?;
            }
            "quality_scaling" => {
                recipe.quality_scaling = value.parse().unwrap_or(false);
            }
            "script" => {
                recipe.script = if value.trim().is_empty() {
                    None
                } else {
                    Some(value.to_string())
                };
            }
            "result.template_id" => {
                recipe.result.template_id = value.to_string();
            }
            "result.quantity" => {
                recipe.result.quantity = value
                    .parse()
                    .map_err(|_| "result.quantity must be a u32".to_string())?;
            }
            _ => {}
        }
        Ok(())
    }

    pub(super) fn add_recipe_array(&mut self, prefix: &str, index: usize) -> Result<(), String> {
        let recipe = self
            .registry
            .recipes
            .get_mut(&self.template_id)
            .ok_or("recipe not found")?;
        match prefix {
            "materials" => {
                recipe.materials.insert(
                    (index + 1).min(recipe.materials.len()),
                    oxide_core::templates::RecipeMaterial {
                        template_id: "item_id".to_string(),
                        quantity: 1,
                    },
                );
            }
            _ => return Err(format!("unknown recipe array: {prefix}")),
        }
        Ok(())
    }

    pub(super) fn remove_recipe_array(&mut self, prefix: &str, index: usize) -> Result<(), String> {
        let recipe = self
            .registry
            .recipes
            .get_mut(&self.template_id)
            .ok_or("recipe not found")?;
        match prefix {
            "materials" => {
                if index < recipe.materials.len() {
                    recipe.materials.remove(index);
                }
            }
            _ => return Err(format!("unknown recipe array: {prefix}")),
        }
        Ok(())
    }

    pub(super) fn clear_recipe_array(&mut self, prefix: &str) -> Result<(), String> {
        let recipe = self
            .registry
            .recipes
            .get_mut(&self.template_id)
            .ok_or("recipe not found")?;
        match prefix {
            "materials" => recipe.materials.clear(),
            _ => return Err(format!("unknown recipe array: {prefix}")),
        }
        Ok(())
    }
}
