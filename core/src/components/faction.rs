use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Tracks a player's standing values with various factions.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FactionStanding {
    pub standings: HashMap<String, i32>,
}

impl FactionStanding {
    pub fn new() -> Self {
        FactionStanding::default()
    }

    /// Retrieve standing for a given faction ID, defaulting to 0 if not tracked.
    pub fn standing(&self, faction_id: &str) -> i32 {
        self.standings.get(faction_id).copied().unwrap_or(0)
    }

    /// Set standing for a given faction ID.
    pub fn set_standing(&mut self, faction_id: impl Into<String>, value: i32) {
        self.standings.insert(faction_id.into(), value);
    }
}

/// Identifies an NPC/mob as a member of a faction, with their cached aggro threshold.
#[derive(Debug, Clone)]
pub struct FactionMember {
    pub faction_id: String,
    pub aggro_below: i32,
}

impl FactionMember {
    pub fn new(faction_id: impl Into<String>, aggro_below: i32) -> Self {
        FactionMember {
            faction_id: faction_id.into(),
            aggro_below,
        }
    }
}
