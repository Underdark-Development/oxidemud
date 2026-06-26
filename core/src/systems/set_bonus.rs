use std::collections::HashMap;

use crate::templates::SetDef;
use crate::{ActiveEffect, Entity, Equipment, SetMembership, SetTracker, World};
use tracing::warn;

/// Describes a change in set bonus state for an entity.
#[derive(Debug, Clone)]
pub struct SetBonusChange {
    pub set_id: String,
    pub set_name: String,
    pub old_count: u8,
    pub new_count: u8,
    pub active_tiers: Vec<u8>,
}

/// Evaluate item set bonuses for an entity.
/// Scans equipped items, counts set pieces, updates SetTracker,
/// and applies/removes ActiveEffect components based on set bonus thresholds.
/// Returns a list of set bonus changes (empty if nothing changed).
pub fn evaluate_set_bonuses(
    world: &mut World,
    entity: Entity,
    set_defs: &HashMap<String, SetDef>,
) -> Vec<SetBonusChange> {
    let equipment = match world.query_one::<&Equipment>(entity) {
        Ok(mut q) => q.get().cloned(),
        Err(_) => None,
    };

    let Some(equipment) = equipment else {
        return vec![];
    };

    // Count equipped pieces by set_id
    let mut counts: HashMap<String, u8> = HashMap::new();
    for (_slot, item_entity) in &equipment.slots {
        let membership = world
            .query_one::<&SetMembership>(*item_entity)
            .ok()
            .and_then(|mut q| q.get().cloned());
        if let Some(m) = membership {
            *counts.entry(m.set_id.clone()).or_insert(0) += 1;
        }
    }

    // Save previous SetTracker for change detection
    let old_tracker = world
        .query_one::<&SetTracker>(entity)
        .ok()
        .and_then(|mut q| q.get().cloned())
        .unwrap_or_default();

    let _ = world.remove_one::<SetTracker>(entity);
    let _ = world.insert(entity, (SetTracker(counts.clone()),));

    // Evaluate which set bonuses should be active
    let mut active_bonuses: Vec<ActiveEffect> = Vec::new();
    for (set_id, count) in &counts {
        let Some(set_def) = set_defs.get(set_id) else {
            continue;
        };

        // Build per-piece_type counts for this set
        let mut piece_counts: HashMap<String, u8> = HashMap::new();
        for (_slot, item_entity) in &equipment.slots {
            let membership = world
                .query_one::<&SetMembership>(*item_entity)
                .ok()
                .and_then(|mut q| q.get().cloned());
            if let Some(m) = membership {
                if m.set_id == *set_id {
                    *piece_counts.entry(m.piece_type).or_insert(0) += 1;
                }
            }
        }

        for bonus in &set_def.bonuses {
            if *count >= bonus.min_pieces {
                for effect in &bonus.effects {
                    let meets_conditions = bonus
                        .conditions
                        .iter()
                        .all(|c| {
                            let found = piece_counts.get(c.piece_type.as_str()).copied().unwrap_or(0);
                            if found == 0 && c.min > 0 {
                                warn!(
                                    "Set '{}' bonus requires piece_type '{}' (min {}) but 0 pieces of that type are equipped from this set",
                                    set_id, c.piece_type, c.min
                                );
                            }
                            found >= c.min
                        });
                    if meets_conditions {
                        active_bonuses.push(ActiveEffect {
                            source: format!("set:{}", set_id),
                            stat: effect.stat.clone(),
                            amount: effect.amount,
                            aura_id: effect.aura_id.clone(),
                            radius: effect.radius,
                        });
                    }
                }
            }
        }
    }

    // Keep non-set active effects
    let old_bonuses: Vec<ActiveEffect> = world
        .query_one::<&Vec<ActiveEffect>>(entity)
        .ok()
        .and_then(|mut q| q.get().cloned())
        .unwrap_or_default()
        .into_iter()
        .filter(|e| !e.source.starts_with("set:"))
        .collect();

    // Merge: keep non-set effects, add new set bonuses
    let _ = world.remove_one::<Vec<ActiveEffect>>(entity);
    let mut merged = old_bonuses;
    merged.extend(active_bonuses);
    let _ = world.insert(entity, (merged,));

    // Collect set bonus changes (for callers to react to)
    let mut changes: Vec<SetBonusChange> = Vec::new();
    let old_counts = &old_tracker.0;
    for (set_id, new_count) in &counts {
        let old_count = old_counts.get(set_id).copied().unwrap_or(0);
        if *new_count != old_count {
            let active_tiers: Vec<u8> = set_defs
                .get(set_id)
                .map(|def| {
                    def.bonuses
                        .iter()
                        .filter(|b| *new_count >= b.min_pieces)
                        .map(|b| b.min_pieces)
                        .collect()
                })
                .unwrap_or_default();
            let set_name = set_defs
                .get(set_id)
                .map(|d| d.name.clone())
                .unwrap_or_default();
            changes.push(SetBonusChange {
                set_id: set_id.clone(),
                set_name,
                old_count,
                new_count: *new_count,
                active_tiers,
            });
        }
    }
    changes
}

/// Re-evaluate set bonuses for all entities that have Equipment.
/// Useful as a maintenance pulse or after bulk equipment changes.
pub fn reconcile_all_set_bonuses(world: &mut World, set_defs: &HashMap<String, SetDef>) {
    let entities: Vec<Entity> = world
        .query::<&Equipment>()
        .into_iter()
        .map(|(e, _eq)| Entity::from(e))
        .collect();

    for entity in entities {
        evaluate_set_bonuses(world, entity, set_defs);
    }
}

/// Check equipped items for set membership and return
/// a map of set_id → count of equipped pieces.
pub fn get_equipped_sets(world: &World, entity: Entity) -> Vec<(String, usize)> {
    let equipment = world
        .query_one::<&Equipment>(entity)
        .ok()
        .and_then(|mut q| q.get().cloned());

    let Some(equipment) = equipment else {
        return vec![];
    };

    let mut counts: HashMap<String, usize> = HashMap::new();
    for (_slot, item_entity) in &equipment.slots {
        let membership = world
            .query_one::<&SetMembership>(*item_entity)
            .ok()
            .and_then(|mut q| q.get().cloned());
        if let Some(m) = membership {
            *counts.entry(m.set_id.clone()).or_insert(0) += 1;
        }
    }

    counts.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_equipped_sets_empty() {
        let mut world = World::new();
        let e = world.spawn((Equipment::new(),));
        let sets = get_equipped_sets(&world, e);
        assert!(sets.is_empty());
    }

    #[test]
    fn test_get_equipped_sets_with_membership() {
        let mut world = World::new();
        let e = world.spawn((Equipment::new(),));

        let item = world.spawn((SetMembership {
            set_id: "templar_armor".into(),
            piece_type: "torso".into(),
        },));
        world
            .query_one::<&mut Equipment>(e)
            .unwrap()
            .get()
            .unwrap()
            .equip(crate::EquipmentSlot::Torso, item);

        let sets = get_equipped_sets(&world, e);
        assert_eq!(sets.len(), 1);
        assert_eq!(sets[0], ("templar_armor".to_string(), 1));
    }

    #[test]
    fn test_set_bonuses_applied() {
        let mut world = World::new();
        let e = world.spawn((Equipment::new(), SetTracker::new()));

        let item = world.spawn((SetMembership {
            set_id: "templar_armor".into(),
            piece_type: "torso".into(),
        },));
        world
            .query_one::<&mut Equipment>(e)
            .unwrap()
            .get()
            .unwrap()
            .equip(crate::EquipmentSlot::Torso, item);

        let mut set_defs = HashMap::new();
        set_defs.insert(
            "templar_armor".to_string(),
            SetDef {
                id: "templar_armor".into(),
                name: "Templar Armor Set".into(),
                bonuses: vec![crate::templates::SetBonusEntry {
                    min_pieces: 1,
                    conditions: vec![],
                    effects: vec![crate::templates::SetEffect {
                        effect_type: "stat".into(),
                        stat: Some("constitution".into()),
                        amount: Some(2),
                        aura_id: None,
                        radius: None,
                    }],
                }],
                params: std::collections::HashMap::new(),
            },
        );

        evaluate_set_bonuses(&mut world, e, &set_defs);

        let mut q_tracker = world.query_one::<&SetTracker>(e).unwrap();
        let tracker = q_tracker.get().unwrap();
        assert_eq!(tracker.count("templar_armor"), 1);
        drop(q_tracker);

        let mut q_effects = world.query_one::<&Vec<ActiveEffect>>(e).unwrap();
        let effects = q_effects.get().unwrap();
        assert_eq!(effects.len(), 1);
        assert_eq!(effects[0].stat, Some("constitution".to_string()));
        assert_eq!(effects[0].amount, Some(2));
    }
}
