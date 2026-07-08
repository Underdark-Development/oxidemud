use serde::{Deserialize, Serialize};

/// Tracks the recipe IDs a player character has learned.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LearnedRecipes {
    pub recipes: Vec<String>,
}

impl LearnedRecipes {
    pub fn new() -> Self {
        LearnedRecipes::default()
    }

    /// Check if the player knows a recipe.
    pub fn knows(&self, recipe_id: &str) -> bool {
        self.recipes.iter().any(|r| r == recipe_id)
    }

    /// Teach the player a recipe. Returns true if it was newly learned.
    pub fn learn(&mut self, recipe_id: impl Into<String>) -> bool {
        let r = recipe_id.into();
        if !self.knows(&r) {
            self.recipes.push(r);
            true
        } else {
            false
        }
    }
}

/// Stores list of string flags/tags spawned on a room entity from the room template.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RoomTags {
    pub tags: Vec<String>,
}

impl RoomTags {
    pub fn new(tags: Vec<String>) -> Self {
        RoomTags { tags }
    }

    /// Check if a specific tag is present.
    pub fn has_tag(&self, tag: &str) -> bool {
        self.tags.iter().any(|t| t == tag)
    }
}
