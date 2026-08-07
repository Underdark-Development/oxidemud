use std::collections::HashMap;
use std::time::Instant;

use crate::components::{
    Attributes, FactionMember, FactionStanding, HaggleCooldown, LastRestock, PendingHaggle,
    ShopStock, Shopkeeper,
};
use crate::templates::registry::TemplateRegistry;
use crate::templates::ItemTemplate;
use crate::templates::ShopInventoryEntry;
use crate::templates::ShopTemplate;
use crate::{Entity, World};

// ---------------------------------------------------------------------------
// Haggle tuning defaults (overridable per shop via `params`)
// ---------------------------------------------------------------------------

/// Base floor a shopkeeper will accept, as a fraction of the asking price.
pub const DEFAULT_HAGGLE_FLOOR: f64 = 0.75;
/// Maximum counter-offer rounds before the keeper refuses to deal.
pub const DEFAULT_HAGGLE_ROUNDS: u32 = 4;
/// Offers below this fraction of the asking price offend the keeper.
pub const DEFAULT_INSULT_FLOOR: f64 = 0.60;
/// Seconds after a negotiation ends before the player may haggle again.
pub const DEFAULT_HAGGLE_COOLDOWN_SECS: u64 = 300;

/// Per-shop haggle tuning resolved from a `ShopTemplate`'s `params`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShopParams {
    pub haggle_floor: f64,
    pub haggle_rounds: u32,
    pub insult_floor: f64,
    pub haggle_cooldown_secs: u64,
}

impl Default for ShopParams {
    fn default() -> Self {
        ShopParams {
            haggle_floor: DEFAULT_HAGGLE_FLOOR,
            haggle_rounds: DEFAULT_HAGGLE_ROUNDS,
            insult_floor: DEFAULT_INSULT_FLOOR,
            haggle_cooldown_secs: DEFAULT_HAGGLE_COOLDOWN_SECS,
        }
    }
}

impl ShopParams {
    pub fn from_shop(shop: &ShopTemplate) -> Self {
        ShopParams {
            haggle_floor: param_f64(&shop.params, "haggle_floor", DEFAULT_HAGGLE_FLOOR),
            haggle_rounds: param_u32(&shop.params, "haggle_rounds", DEFAULT_HAGGLE_ROUNDS),
            insult_floor: param_f64(&shop.params, "insult_floor", DEFAULT_INSULT_FLOOR),
            haggle_cooldown_secs: param_u64(
                &shop.params,
                "haggle_cooldown_secs",
                DEFAULT_HAGGLE_COOLDOWN_SECS,
            ),
        }
    }
}

fn param_f64(params: &HashMap<String, String>, key: &str, default: f64) -> f64 {
    params
        .get(key)
        .and_then(|v| v.trim().parse::<f64>().ok())
        .unwrap_or(default)
}

fn param_u32(params: &HashMap<String, String>, key: &str, default: u32) -> u32 {
    params
        .get(key)
        .and_then(|v| v.trim().parse::<u32>().ok())
        .unwrap_or(default)
}

fn param_u64(params: &HashMap<String, String>, key: &str, default: u64) -> u64 {
    params
        .get(key)
        .and_then(|v| v.trim().parse::<u64>().ok())
        .unwrap_or(default)
}

// ---------------------------------------------------------------------------
// Keeper lookup
// ---------------------------------------------------------------------------

/// Returns the shopkeeper NPC in `room`, if any.
pub fn shopkeeper_in_room(world: &World, room: Entity) -> Option<Entity> {
    crate::entities_in_room(world, room).into_iter().find(|&e| {
        world
            .query_one::<&Shopkeeper>(e)
            .ok()
            .is_some_and(|mut q| q.get().is_some())
    })
}

// ---------------------------------------------------------------------------
// Pricing
// ---------------------------------------------------------------------------

/// Base copper price for an inventory entry: per-entry `price` overrides the
/// item template's `value`.
pub fn base_price(entry: &ShopInventoryEntry, item: &ItemTemplate) -> u64 {
    if entry.price > 0 {
        entry.price
    } else {
        item.value
    }
}

/// Player purchase price for `base_cp` at this shop.
pub fn asking_price(shop: &ShopTemplate, base_cp: u64, rep_mult: f64) -> u64 {
    let mult = shop.sell_rate.max(0.0) * rep_mult.max(0.0);
    ((base_cp as f64) * mult).round().max(1.0) as u64
}

/// Player proceeds when selling an item worth `value_cp` to this shop.
pub fn sell_price(shop: &ShopTemplate, value_cp: u64, rep_mult: f64) -> u64 {
    let mult = shop.buy_rate.max(0.0) * rep_mult.max(0.0);
    ((value_cp as f64) * mult).round().max(1.0) as u64
}

/// Whether the shop will buy an item, matching on item type or subtype.
/// A shop with no `buy_types` buys nothing (classic Merc `trade-0`).
pub fn shop_buys_item(shop: &ShopTemplate, item: &ItemTemplate) -> bool {
    if shop.buy_types.is_empty() {
        return false;
    }
    let matches_type = shop
        .buy_types
        .iter()
        .any(|t| t.eq_ignore_ascii_case(&item.item_type));
    let matches_subtype = !item.subtype.is_empty()
        && shop
            .buy_types
            .iter()
            .any(|t| t.eq_ignore_ascii_case(&item.subtype));
    matches_type || matches_subtype
}

/// Price multiplier from the player's standing with the keeper's faction,
/// keyed by rank name in the shop's `price_mods`. Neutral (no faction, no
/// matching rank, or untracked standing) yields `1.0`.
pub fn reputation_multiplier(
    world: &World,
    player: Entity,
    keeper: Entity,
    shop: &ShopTemplate,
    registry: &TemplateRegistry,
) -> f64 {
    let Some(faction_id) = world
        .query_one::<&FactionMember>(keeper)
        .ok()
        .and_then(|mut q| q.get().map(|m| m.faction_id.clone()))
    else {
        return 1.0;
    };
    let standing = world
        .query_one::<&FactionStanding>(player)
        .ok()
        .and_then(|mut q| q.get().cloned())
        .map(|fs| fs.standing(&faction_id))
        .unwrap_or(0);
    let Some(faction) = registry.factions.get(&faction_id) else {
        return 1.0;
    };
    let rank = faction.get_rank(standing);
    shop.price_mods.get(&rank).copied().unwrap_or(1.0)
}

// ---------------------------------------------------------------------------
// Haggling
// ---------------------------------------------------------------------------

/// Charisma influence on haggling, in [−0.15, +0.15]. A high-charisma
/// negotiator is more persuasive and lowers the keeper's floor.
pub fn charisma_bonus(charisma: u8) -> f64 {
    ((charisma as f64 - 10.0) * 0.02).clamp(-0.15, 0.15)
}

/// Offers below this copper amount offend the keeper.
pub fn insult_threshold(asking: u64, params: &ShopParams) -> u64 {
    ((asking as f64) * params.insult_floor).round().max(1.0) as u64
}

/// The lowest price the keeper will accept without countering, derived from
/// the base floor, the player's charisma, and their reputation with the shop.
pub fn haggle_floor(asking: u64, params: &ShopParams, charisma: u8, rep_mult: f64) -> u64 {
    let cha_adj = charisma_bonus(charisma);
    // Favored customers find the keeper more flexible.
    let rep_adj = ((1.0 - rep_mult) * 0.10).clamp(0.0, 0.10);
    let floor_mult = (params.haggle_floor - cha_adj - rep_adj).clamp(0.50, 0.95);
    ((asking as f64) * floor_mult).round().max(1.0) as u64
}

/// Result of a single counter-offer evaluation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CounterOutcome {
    /// Deal accepted at the offered amount.
    Accept,
    /// Keeper counters with a new price.
    Counter(u64),
    /// Keeper refuses to deal.
    Refuse,
}

/// Evaluate a player's offer against the asking price, haggle floor, and
/// insult threshold. `rounds_used` is the number of counters already given.
pub fn evaluate_counter(
    offer: u64,
    asking: u64,
    floor: u64,
    insult: u64,
    rounds_used: u32,
    max_rounds: u32,
) -> CounterOutcome {
    if offer >= asking {
        return CounterOutcome::Accept;
    }
    if offer < insult {
        return CounterOutcome::Refuse;
    }
    if offer >= floor {
        return CounterOutcome::Accept;
    }
    if rounds_used >= max_rounds {
        return CounterOutcome::Refuse;
    }
    let counter = offer + asking.saturating_sub(offer) / 2;
    if counter <= offer {
        CounterOutcome::Refuse
    } else {
        CounterOutcome::Counter(counter)
    }
}

/// Whether the player's haggle cooldown for `shop_id` is still active.
pub fn haggle_on_cooldown(world: &World, player: Entity, shop_id: &str, now: Instant) -> bool {
    world
        .query_one::<&HaggleCooldown>(player)
        .ok()
        .and_then(|mut q| q.get().cloned())
        .is_some_and(|c| c.shop_id == shop_id && c.ready_at > now)
}

/// Whether an existing negotiation is still valid for the current shopkeeper.
pub fn pending_haggle_valid(
    world: &World,
    player: Entity,
    keeper: Entity,
    shop_id: &str,
    item_id: &str,
) -> bool {
    world
        .query_one::<&PendingHaggle>(player)
        .ok()
        .and_then(|mut q| q.get().cloned())
        .is_some_and(|p| p.keeper == keeper && p.shop_id == shop_id && p.item_id == item_id)
}

// ---------------------------------------------------------------------------
// Stock & restock
// ---------------------------------------------------------------------------

/// Roll a starting/restocked count for an inventory entry.
pub fn roll_count(count: &ShopInventoryEntry) -> u64 {
    if count.count.max <= count.count.min {
        count.count.min
    } else {
        fastrand::u64(count.count.min..=count.count.max)
    }
}

/// Initial per-item stock for a shop's inventory.
pub fn init_stock(shop: &ShopTemplate) -> HashMap<String, u64> {
    shop.inventory
        .iter()
        .map(|entry| (entry.item.clone(), roll_count(entry)))
        .collect()
}

/// Refill stock for every shopkeeper whose restock interval has elapsed.
/// Returns the number of shopkeepers restocked.
pub fn restock_shops(world: &mut World, registry: &TemplateRegistry) -> usize {
    let now = Instant::now();
    let shopkeepers: Vec<(Entity, String)> = world
        .query::<(&Shopkeeper,)>()
        .iter()
        .map(|(e, (sk,))| (e, sk.shop_id.clone()))
        .collect();

    let mut restocked = 0usize;
    for (keeper, shop_id) in shopkeepers {
        let Some(shop) = registry.shops.get(&shop_id) else {
            continue;
        };
        let Some(last) = world
            .query_one::<&LastRestock>(keeper)
            .ok()
            .and_then(|mut q| q.get().copied())
        else {
            continue;
        };
        if now.duration_since(last.0).as_secs() < shop.restock_secs {
            continue;
        }

        let mut refreshed = false;
        if let Ok(mut q) = world.query_one::<&mut ShopStock>(keeper) {
            if let Some(stock) = q.get() {
                for entry in &shop.inventory {
                    stock.0.insert(entry.item.clone(), roll_count(entry));
                }
                refreshed = true;
            }
        }
        if refreshed {
            let _ = world.insert(keeper, (LastRestock(now),));
            restocked += 1;
        }
    }
    restocked
}

/// Convenience: fetch an entity's charisma attribute (defaults to 10).
pub fn charisma_of(world: &World, entity: Entity) -> u8 {
    world
        .query_one::<&Attributes>(entity)
        .ok()
        .and_then(|mut q| q.get().cloned())
        .map(|a| a.charisma)
        .unwrap_or(10)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::{FactionStanding, Shopkeeper};
    use crate::templates::FactionDef;
    use std::collections::HashMap;

    fn shop() -> ShopTemplate {
        ShopTemplate {
            id: "general".into(),
            name: "General Store".into(),
            buy_rate: 0.5,
            sell_rate: 1.5,
            restock_secs: 300,
            inventory: vec![ShopInventoryEntry {
                item: "potion".into(),
                count: crate::templates::ShopInventoryCount { min: 2, max: 5 },
                price: 30,
            }],
            params: HashMap::new(),
            buy_types: Vec::new(),
            price_mods: HashMap::new(),
        }
    }

    fn item(item_type: &str, subtype: &str, value: u64) -> ItemTemplate {
        ItemTemplate {
            id: "i".into(),
            name: "i".into(),
            description: String::new(),
            item_type: item_type.into(),
            subtype: subtype.into(),
            value,
            ..Default::default()
        }
    }

    #[test]
    fn asking_price_rounds_and_floors_at_one() {
        let s = shop();
        assert_eq!(asking_price(&s, 30, 1.0), 45);
        assert_eq!(asking_price(&s, 1, 1.0), 2);
        assert_eq!(asking_price(&s, 0, 1.0), 1);
    }

    #[test]
    fn asking_price_applies_rep_multiplier() {
        let s = shop();
        assert_eq!(asking_price(&s, 100, 0.8), 120);
        assert_eq!(asking_price(&s, 100, 1.5), 225);
    }

    #[test]
    fn sell_price_floors_at_one() {
        let s = shop();
        assert_eq!(sell_price(&s, 100, 1.0), 50);
        assert_eq!(sell_price(&s, 1, 1.0), 1);
    }

    #[test]
    fn base_price_uses_entry_override() {
        let s = shop();
        let entry = &s.inventory[0];
        assert_eq!(base_price(entry, &item("potion", "", 5)), 30);
        let entry_no_price = ShopInventoryEntry {
            item: "x".into(),
            count: crate::templates::ShopInventoryCount { min: 1, max: 1 },
            price: 0,
        };
        assert_eq!(base_price(&entry_no_price, &item("potion", "", 7)), 7);
    }

    #[test]
    fn shop_with_no_buy_types_buys_nothing() {
        let s = shop();
        assert!(!shop_buys_item(&s, &item("potion", "", 5)));
        assert!(!shop_buys_item(&s, &item("weapon", "sword", 5)));
    }

    #[test]
    fn shop_buys_by_type_or_subtype() {
        let mut s = shop();
        s.buy_types = vec!["weapon".into(), "light".into()];
        assert!(shop_buys_item(&s, &item("weapon", "sword", 5)));
        assert!(!shop_buys_item(&s, &item("potion", "", 5)));
        assert!(!shop_buys_item(&s, &item("armor", "plate", 5)));

        s.buy_types = vec!["sword".into()];
        assert!(shop_buys_item(&s, &item("weapon", "sword", 5)));
        assert!(!shop_buys_item(&s, &item("weapon", "axe", 5)));
    }

    #[test]
    fn charisma_bonus_clamps() {
        assert_eq!(charisma_bonus(10), 0.0);
        assert_eq!(charisma_bonus(18), 0.15);
        assert_eq!(charisma_bonus(50), 0.15);
        assert_eq!(charisma_bonus(3), -0.14);
        assert!(charisma_bonus(3) >= -0.15 && charisma_bonus(3) <= 0.15);
    }

    #[test]
    fn haggle_floor_stays_within_bounds() {
        let p = ShopParams::default();
        let floor = haggle_floor(100, &p, 10, 1.0);
        assert!(floor >= 50 && floor <= 95);
        assert_eq!(floor, 75);
        let high_cha = haggle_floor(100, &p, 18, 1.0);
        assert!(high_cha < floor);
    }

    #[test]
    fn insult_threshold_from_params() {
        let p = ShopParams::default();
        assert_eq!(insult_threshold(100, &p), 60);
    }

    #[test]
    fn evaluate_counter_accepts_at_or_above_asking() {
        let r = evaluate_counter(30, 30, 20, 15, 0, 4);
        assert_eq!(r, CounterOutcome::Accept);
        let r = evaluate_counter(40, 30, 20, 15, 0, 4);
        assert_eq!(r, CounterOutcome::Accept);
    }

    #[test]
    fn evaluate_counter_accepts_above_floor() {
        let r = evaluate_counter(25, 30, 20, 15, 0, 4);
        assert_eq!(r, CounterOutcome::Accept);
    }

    #[test]
    fn evaluate_counter_insults_below_threshold() {
        let r = evaluate_counter(10, 30, 20, 15, 0, 4);
        assert_eq!(r, CounterOutcome::Refuse);
    }

    #[test]
    fn evaluate_counter_counters_between_insult_and_floor() {
        let r = evaluate_counter(17, 30, 20, 15, 0, 4);
        match r {
            CounterOutcome::Counter(c) => assert!(c > 17 && c < 30),
            other => panic!("expected counter, got {other:?}"),
        }
    }

    #[test]
    fn evaluate_counter_refuses_when_rounds_exhausted() {
        let r = evaluate_counter(17, 30, 20, 15, 4, 4);
        assert_eq!(r, CounterOutcome::Refuse);
    }

    #[test]
    fn reputation_multiplier_neutral_without_faction() {
        let mut world = World::new();
        let player = world.spawn(());
        let keeper = world.spawn((Shopkeeper {
            shop_id: "s".into(),
        },));
        let reg = TemplateRegistry::default();
        assert_eq!(
            reputation_multiplier(&world, player, keeper, &shop(), &reg),
            1.0
        );
    }

    #[test]
    fn reputation_multiplier_applies_price_mods() {
        let mut world = World::new();
        let player = world.spawn(());
        let keeper = world.spawn((
            Shopkeeper {
                shop_id: "s".into(),
            },
            FactionMember::new("guard", -500),
        ));
        let mut fs = FactionStanding::new();
        fs.set_standing("guard", 500);
        let _ = world.insert(player, (fs,));

        let mut s = shop();
        s.price_mods.insert("Friendly".into(), 0.9);
        s.price_mods.insert("Hostile".into(), 1.5);

        let mut reg = TemplateRegistry::default();
        reg.factions.insert(
            "guard".into(),
            FactionDef {
                id: "guard".into(),
                name: "Town Guard".into(),
                description: String::new(),
                starting_standing: 0,
                min_standing: -10000,
                max_standing: 10000,
                ranks: vec![
                    crate::templates::FactionRank {
                        name: "Hostile".into(),
                        threshold: -500,
                    },
                    crate::templates::FactionRank {
                        name: "Neutral".into(),
                        threshold: 0,
                    },
                    crate::templates::FactionRank {
                        name: "Friendly".into(),
                        threshold: 250,
                    },
                ],
                relationships: HashMap::new(),
                aggro_below: -500,
            },
        );

        assert_eq!(reputation_multiplier(&world, player, keeper, &s, &reg), 0.9);
    }

    #[test]
    fn init_stock_counts_within_bounds() {
        let s = shop();
        for _ in 0..20 {
            let stock = init_stock(&s);
            let count = stock["potion"];
            assert!((2..=5).contains(&count), "count {count} out of bounds");
        }
    }

    #[test]
    fn restock_shops_after_interval() {
        let mut world = World::new();
        let room = world.spawn(());
        let mut s = shop();
        s.restock_secs = 1;
        let mut reg = TemplateRegistry::default();
        reg.shops.insert("general".into(), s);

        let keeper = world.spawn((
            crate::components::Position::new(room),
            Shopkeeper {
                shop_id: "general".into(),
            },
            ShopStock(init_stock(reg.shops.get("general").unwrap())),
            LastRestock(Instant::now() - std::time::Duration::from_secs(100)),
        ));

        assert!(restock_shops(&mut world, &reg) >= 1);
        let now = Instant::now();
        let updated = world
            .query_one::<&LastRestock>(keeper)
            .ok()
            .and_then(|mut q| q.get().copied())
            .unwrap();
        assert!(now.duration_since(updated.0).as_secs() <= 1);
    }
}
