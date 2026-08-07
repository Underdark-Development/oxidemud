use std::collections::HashMap;
use std::time::Instant;

/// Marker component for NPC mobiles that operate as shopkeepers.
/// Attached to NPC entities spawned from mob templates with a `shop` set.
#[derive(Debug, Clone)]
pub struct Shopkeeper {
    pub shop_id: String,
}

/// Runtime stock counts (item template id → quantity on hand).
/// Counts deplete on purchase and refill on the shop's restock cadence.
#[derive(Debug, Clone, Default)]
pub struct ShopStock(pub HashMap<String, u64>);

impl ShopStock {
    pub fn new() -> Self {
        ShopStock(HashMap::new())
    }

    /// Quantity on hand for an item template id.
    pub fn count(&self, item_id: &str) -> u64 {
        self.0.get(item_id).copied().unwrap_or(0)
    }
}

/// Timestamp of the last restock for a shopkeeper.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LastRestock(pub Instant);

/// Active barter negotiation between a player and a shopkeeper.
/// Present on the player; cleared when the negotiation ends or the player
/// walks away from the keeper.
#[derive(Debug, Clone)]
pub struct PendingHaggle {
    pub shop_id: String,
    pub item_id: String,
    pub keeper: crate::Entity,
    pub asking: u64,
    pub floor: u64,
    pub rounds_used: u32,
}

/// Session-only lockout after a negotiation ends (accept, refusal, or
/// exhausted rounds). Does not block plain purchases at asking price.
#[derive(Debug, Clone)]
pub struct HaggleCooldown {
    pub shop_id: String,
    pub ready_at: Instant,
}

impl HaggleCooldown {
    /// Seconds remaining until the player may haggle again.
    pub fn remaining_secs(&self, now: Instant) -> u64 {
        self.ready_at.saturating_duration_since(now).as_secs()
    }
}
