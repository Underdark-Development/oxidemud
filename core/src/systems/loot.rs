use std::str::FromStr;

use crate::dice::DiceRoll;
use crate::templates::{AffixDef, LootTable, TemplateRegistry};
use crate::{
    AffixMod, AffixModifiers, AffixNames, Entity, Item, ItemSkillRequirement, Name, SetMembership,
    Weapon, WeaponHands, WeaponRange, World,
};

/// Quality tiers that determine how many affixes an item can roll.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum QualityTier {
    Common,
    Uncommon,
    Rare,
    Epic,
    Legendary,
}

impl QualityTier {
    /// Number of affixes this tier grants (prefix + suffix pairs).
    pub fn affix_count(self) -> usize {
        match self {
            QualityTier::Common => 0,
            QualityTier::Uncommon => 1,
            QualityTier::Rare => 2,
            QualityTier::Epic => 3,
            QualityTier::Legendary => 4,
        }
    }

    /// Name used in template `quality_min` filtering.
    pub fn as_str(self) -> &'static str {
        match self {
            QualityTier::Common => "common",
            QualityTier::Uncommon => "uncommon",
            QualityTier::Rare => "rare",
            QualityTier::Epic => "epic",
            QualityTier::Legendary => "legendary",
        }
    }
}

/// Result of a loot roll — entity creation is handled by the caller.
#[derive(Debug, Clone)]
pub struct ItemSpawn {
    pub template_id: String,
    pub count: u8,
    pub quality: QualityTier,
    pub prefix_ids: Vec<String>,
    pub suffix_ids: Vec<String>,
}

/// Roll loot from a mob's loot table.
pub fn roll_loot(table: &LootTable, mob_level: u8, templates: &TemplateRegistry) -> Vec<ItemSpawn> {
    let mut results = Vec::new();

    for entry in &table.entries {
        if entry.chance < 100 && fastrand::u8(0..100) >= entry.chance {
            continue;
        }

        if entry.item.is_empty() {
            continue;
        }

        // Determine count
        let count = match &entry.count {
            Some(c) => {
                let min = c.min;
                let max = c.max.max(min);
                if min == max {
                    min
                } else {
                    fastrand::u8(min..=max)
                }
            }
            None => 1,
        };

        // Roll quality based on mob level
        let quality = roll_quality(mob_level);

        // Roll affixes
        let (prefix_ids, suffix_ids) = roll_affixes(quality, &entry.item, templates);

        results.push(ItemSpawn {
            template_id: entry.item.clone(),
            count,
            quality,
            prefix_ids,
            suffix_ids,
        });
    }

    results
}

/// Roll quality tier based on mob level.
fn roll_quality(mob_level: u8) -> QualityTier {
    let roll = fastrand::u64(0..10_000);

    let (common, uncommon, rare, epic) = quality_thresholds(mob_level);

    if roll < common {
        QualityTier::Common
    } else if roll < uncommon {
        QualityTier::Uncommon
    } else if roll < rare {
        QualityTier::Rare
    } else if roll < epic {
        QualityTier::Epic
    } else {
        QualityTier::Legendary
    }
}

/// Quality thresholds per mob level (out of 10,000).
/// Higher-level mobs drop better loot more often.
fn quality_thresholds(level: u8) -> (u64, u64, u64, u64) {
    let l = level as u64;
    // Common threshold decreases with level; uncommon/rare/epic increase.
    let common = 8000u64.saturating_sub(l * 100).max(2000);
    let uncommon = common + 1500 + l * 50;
    let rare = uncommon + 300 + l * 30;
    let epic = rare + 150 + l * 15;
    (common, uncommon, rare, epic)
}

/// Roll prefix and suffix affixes for an item.
fn roll_affixes(
    quality: QualityTier,
    item_template_id: &str,
    templates: &TemplateRegistry,
) -> (Vec<String>, Vec<String>) {
    let count = quality.affix_count();
    if count == 0 {
        return (vec![], vec![]);
    }

    // Determine the equipment slot from the item template
    let slot = templates
        .get_item(item_template_id)
        .and_then(|item| item.equipment.as_ref().map(|eq| eq.slot.clone()))
        .unwrap_or_default();

    let quality_str = quality.as_str();

    // Collect eligible affixes from the pool
    let mut eligible: Vec<&AffixDef> = Vec::new();
    for affix in templates.affixes.values() {
        if (affix.slot.is_empty() || affix.slot.contains(&slot))
            && quality_meets_min(quality_str, &affix.quality_min)
        {
            eligible.push(affix);
        }
    }

    if eligible.is_empty() {
        return (vec![], vec![]);
    }

    // Separate into prefixes and suffixes
    let prefixes: Vec<&&AffixDef> = eligible
        .iter()
        .filter(|a| a.affix_type == "prefix")
        .collect();
    let suffixes: Vec<&&AffixDef> = eligible
        .iter()
        .filter(|a| a.affix_type == "suffix")
        .collect();

    // Roll success for each of the `count` slots
    let mut num_prefixes = 0;
    let mut num_suffixes = 0;
    for _ in 0..count {
        if fastrand::bool() {
            // 50% chance per affix slot to roll an affix
            if fastrand::bool() {
                // 50% prefix, 50% suffix
                num_prefixes += 1;
            } else {
                num_suffixes += 1;
            }
        }
    }

    // Adjust if one pool is too small and the other can take more
    let max_prefixes = prefixes.len();
    let max_suffixes = suffixes.len();

    if num_prefixes > max_prefixes {
        let overflow = num_prefixes - max_prefixes;
        num_prefixes = max_prefixes;
        num_suffixes = (num_suffixes + overflow).min(max_suffixes);
    }
    if num_suffixes > max_suffixes {
        let overflow = num_suffixes - max_suffixes;
        num_suffixes = max_suffixes;
        num_prefixes = (num_prefixes + overflow).min(max_prefixes);
    }

    let prefix_ids = weighted_sample(&prefixes, num_prefixes);
    let suffix_ids = weighted_sample(&suffixes, num_suffixes);

    (prefix_ids, suffix_ids)
}

/// Check if a quality tier meets the minimum requirement.
fn quality_meets_min(tier: &str, min: &str) -> bool {
    let tiers = ["common", "uncommon", "rare", "epic", "legendary"];
    let tier_idx = tiers.iter().position(|t| *t == tier).unwrap_or(0);
    let min_idx = tiers.iter().position(|t| *t == min).unwrap_or(0);
    tier_idx >= min_idx
}

/// Weighted random sample from affix candidates.
fn weighted_sample(affixes: &[&&AffixDef], count: usize) -> Vec<String> {
    use std::collections::HashMap;

    if affixes.is_empty() || count == 0 {
        return vec![];
    }

    let total_weight: u32 = affixes.iter().map(|a| a.weight).sum();
    if total_weight == 0 {
        return vec![];
    }

    let mut selected: Vec<String> = Vec::new();
    let mut used: HashMap<usize, bool> = HashMap::new();

    for _ in 0..count {
        if used.len() >= affixes.len() {
            break;
        }
        let roll = fastrand::u32(0..total_weight);
        let mut cumulative = 0u32;
        for (i, affix) in affixes.iter().enumerate() {
            if used.contains_key(&i) {
                continue;
            }
            cumulative += affix.weight;
            if roll < cumulative {
                selected.push(affix.id.clone());
                used.insert(i, true);
                break;
            }
        }
    }

    selected
}

/// Apply affix stat modifiers to an existing item entity.
/// Attaches AffixNames (for display) and AffixModifiers (resolved stat bonuses).
pub fn apply_affixes_to_item(
    world: &mut World,
    item: Entity,
    prefix_ids: &[String],
    suffix_ids: &[String],
    templates: &TemplateRegistry,
) {
    if prefix_ids.is_empty() && suffix_ids.is_empty() {
        return;
    }

    let mut names: Vec<String> = Vec::new();
    let mut mods: Vec<AffixMod> = Vec::new();

    for affix_id in prefix_ids.iter().chain(suffix_ids) {
        if let Some(affix) = templates.get_affix(affix_id) {
            names.push(affix.name.clone());

            if let (Some(stat), Some(amount)) = (&affix.stat, &affix.amount) {
                let resolved = resolve_amount(amount);
                mods.push(AffixMod {
                    stat: stat.clone(),
                    amount: resolved,
                });
            }
        }
    }

    let _ = world.insert(item, (AffixNames(names),));
    if !mods.is_empty() {
        let _ = world.insert(item, (AffixModifiers(mods),));
    }
}

/// Parse an amount string (dice notation or flat number) and return an average/rounded value.
fn resolve_amount(amount: &str) -> i32 {
    if let Ok(dice) = DiceRoll::from_str(amount) {
        dice.average_rounded()
    } else {
        amount.parse::<i32>().unwrap_or_default()
    }
}

/// Spawn a complete item entity from a loot roll result.
/// Returns the spawned entity.
pub fn spawn_loot_item(
    world: &mut World,
    spawn: &ItemSpawn,
    templates: &TemplateRegistry,
) -> Option<Entity> {
    let item_tmpl = templates.get_item(&spawn.template_id)?;

    // Core: Item + Name + ScriptParams
    let entity = world.spawn((
        Item::new(&spawn.template_id),
        Name::new(&item_tmpl.name),
        crate::ScriptParams(item_tmpl.params.clone()),
    ));

    // Weapon stats
    if let Some(wpn) = &item_tmpl.weapon {
        if let Ok(dice) = DiceRoll::from_str(wpn.damage.as_str()) {
            let damage_type = wpn
                .damage_type
                .parse()
                .unwrap_or(crate::DamageType::Bludgeon);
            let range = match wpn.range.to_lowercase().as_str() {
                "ranged" => WeaponRange::Ranged,
                "reach" => WeaponRange::Reach,
                "thrown" => WeaponRange::Thrown,
                _ => WeaponRange::Melee,
            };
            let hands = match wpn.hands.to_lowercase().as_str() {
                "twohand" | "twohanded" | "two_hand" | "two_handed" => WeaponHands::TwoHand,
                _ => WeaponHands::OneHand,
            };
            let _ = world.insert(
                entity,
                (Weapon {
                    damage_dice: dice,
                    damage_type,
                    speed: wpn.speed,
                    range,
                    hands,
                },),
            );
        }
    }

    // Set membership
    if let Some(set) = &item_tmpl.set {
        let _ = world.insert(entity, (SetMembership::from(set.clone()),));
    }

    // Item triggers
    if !item_tmpl.triggers.is_empty() {
        let _ = world.insert(entity, (crate::ItemTriggers(item_tmpl.triggers.clone()),));
    }

    // Skill requirement
    if let Some(req) = &item_tmpl.requires_skill {
        let _ = world.insert(
            entity,
            (ItemSkillRequirement {
                id: req.id.clone(),
                level: req.level,
            },),
        );
    }

    // Apply affixes (names + modifiers)
    apply_affixes_to_item(
        world,
        entity,
        &spawn.prefix_ids,
        &spawn.suffix_ids,
        templates,
    );

    Some(entity)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::templates::{AffixDef, EquipmentDef, ItemTemplate};

    fn make_templates() -> TemplateRegistry {
        let mut t = TemplateRegistry::new();
        t.items.insert(
            "test_weapon".into(),
            ItemTemplate {
                id: "test_weapon".into(),
                name: "Test Weapon".into(),
                description: ".".into(),
                item_type: "weapon".into(),
                subtype: String::new(),
                quality: "common".into(),
                level_requirement: 0,
                weight: 0.0,
                value: 0,
                flags: vec![],
                allowed_classes: vec![],
                allowed_races: vec![],
                allowed_alignments: vec![],
                requires_skill: None,
                weapon: None,
                equipment: Some(EquipmentDef {
                    slot: "weapon".into(),
                }),
                set: None,
                consumable: None,
                container: None,
                durability: None,
                triggers: vec![],
                params: std::collections::HashMap::new(),
            },
        );
        t.affixes.insert(
            "of_frost".into(),
            AffixDef {
                id: "of_frost".into(),
                name: "of Frost".into(),
                description: "Cold damage.".into(),
                affix_type: "suffix".into(),
                element: Some("cold".into()),
                amount: Some("1d6".into()),
                stat: None,
                quality_min: "uncommon".into(),
                slot: vec!["weapon".into()],
                weight: 1,
                params: std::collections::HashMap::new(),
            },
        );
        t.affixes.insert(
            "sharp".into(),
            AffixDef {
                id: "sharp".into(),
                name: "Sharp".into(),
                description: "+2 damage.".into(),
                affix_type: "prefix".into(),
                element: None,
                amount: Some("2".into()),
                stat: Some("damage".into()),
                quality_min: "common".into(),
                slot: vec!["weapon".into()],
                weight: 2,
                params: std::collections::HashMap::new(),
            },
        );
        t
    }

    #[test]
    fn test_quality_tier_ordering() {
        assert!(QualityTier::Common < QualityTier::Uncommon);
        assert!(QualityTier::Uncommon < QualityTier::Rare);
        assert!(QualityTier::Rare < QualityTier::Epic);
        assert!(QualityTier::Epic < QualityTier::Legendary);
    }

    #[test]
    fn test_quality_meets_min() {
        assert!(quality_meets_min("uncommon", "common"));
        assert!(quality_meets_min("uncommon", "uncommon"));
        assert!(!quality_meets_min("common", "uncommon"));
    }

    #[test]
    fn test_roll_quality_high_level() {
        // High-level mobs should have better quality distribution
        let q = roll_quality(50);
        // Just verify it returns a valid tier
        assert!(matches!(
            q,
            QualityTier::Common
                | QualityTier::Uncommon
                | QualityTier::Rare
                | QualityTier::Epic
                | QualityTier::Legendary
        ));
    }

    #[test]
    fn test_affix_slot_filter() {
        let templates = make_templates();
        let mut found = false;
        for _ in 0..100 {
            let (_prefixes, suffixes) =
                roll_affixes(QualityTier::Uncommon, "test_weapon", &templates);
            if !suffixes.is_empty() {
                assert_eq!(suffixes.len(), 1);
                assert!(suffixes.contains(&"of_frost".to_string()));
                found = true;
                break;
            }
        }
        assert!(found, "Should have eventually rolled a suffix");
    }

    #[test]
    fn test_roll_loot_always_drops() {
        let mut t = TemplateRegistry::new();
        t.items.insert(
            "test_item".into(),
            ItemTemplate {
                id: "test_item".into(),
                name: "Test Item".into(),
                description: ".".into(),
                item_type: "weapon".into(),
                subtype: String::new(),
                quality: "common".into(),
                level_requirement: 0,
                weight: 0.0,
                value: 0,
                flags: vec![],
                allowed_classes: vec![],
                allowed_races: vec![],
                allowed_alignments: vec![],
                requires_skill: None,
                weapon: None,
                equipment: Some(EquipmentDef {
                    slot: "weapon".into(),
                }),
                set: None,
                consumable: None,
                container: None,
                durability: None,
                triggers: vec![],
                params: std::collections::HashMap::new(),
            },
        );
        t.affixes.insert(
            "sharp".into(),
            AffixDef {
                id: "sharp".into(),
                name: "Sharp".into(),
                description: "+2 damage.".into(),
                affix_type: "prefix".into(),
                element: None,
                amount: Some("2".into()),
                stat: Some("damage".into()),
                quality_min: "common".into(),
                slot: vec!["weapon".into()],
                weight: 2,
                params: std::collections::HashMap::new(),
            },
        );

        let table = LootTable {
            entries: vec![crate::templates::LootEntry {
                item: "test_item".into(),
                treasure_class: None,
                count: None,
                chance: 100,
            }],
        };

        let spawns = roll_loot(&table, 10, &t);
        assert_eq!(spawns.len(), 1);
        assert_eq!(spawns[0].template_id, "test_item");
    }
}
