use crate::{templates::TriggerDef, Entity, Equipment, Inventory, World};

/// Evaluate item triggers for an entity when a specific event fires.
/// Scans the entity's equipment and inventory for items that have
/// `SetMembership` triggers matching the given `event`, rolls chance,
/// and returns a list of triggers that fired.
pub fn process_triggers(world: &World, entity: Entity, event: &str) -> Vec<TriggeredEffect> {
    let mut results = Vec::new();

    // Check equipment
    let equipment = world
        .query_one::<&Equipment>(entity)
        .ok()
        .and_then(|mut q| q.get().cloned());

    if let Some(ref eq) = equipment {
        for (_slot, item_entity) in &eq.slots {
            collect_item_triggers(world, *item_entity, event, &mut results);
        }
    }

    // Check inventory
    let inventory = world
        .query_one::<&Inventory>(entity)
        .ok()
        .and_then(|mut q| q.get().cloned());

    if let Some(ref inv) = inventory {
        for item_entity in &inv.0 {
            collect_item_triggers(world, *item_entity, event, &mut results);
        }
    }

    results
}

/// Collect triggers from a single item entity that match the given event.
fn collect_item_triggers(
    world: &World,
    item_entity: Entity,
    event: &str,
    results: &mut Vec<TriggeredEffect>,
) {
    // Check if the item entity has triggers stored directly
    let item_triggers = world
        .query_one::<&ItemTriggers>(item_entity)
        .ok()
        .and_then(|mut q| q.get().map(|t| t.0.clone()));

    let Some(triggers) = item_triggers else {
        return;
    };

    for trigger in &triggers {
        if trigger.event == event && fastrand::u8(0..100) < trigger.chance {
            results.push(TriggeredEffect {
                item: item_entity,
                cast: trigger.cast.clone(),
                target: trigger.target.clone(),
            });
        }
    }
}

/// A trigger that fired — the game loop/server code handles the actual effect.
#[derive(Debug, Clone)]
pub struct TriggeredEffect {
    pub item: Entity,
    pub cast: String,
    pub target: String,
}

/// Component storing trigger definitions on an item entity.
/// Populated from the item template when the item is spawned.
#[derive(Debug, Clone)]
pub struct ItemTriggers(pub Vec<TriggerDef>);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::EquipmentSlot;

    #[test]
    fn test_no_triggers_on_empty() {
        let mut world = World::new();
        let e = world.spawn((Equipment::new(),));
        let results = process_triggers(&world, e, "on_hit");
        assert!(results.is_empty());
    }

    #[test]
    fn test_trigger_fires_with_100_chance() {
        let mut world = World::new();
        let e = world.spawn((Equipment::new(),));

        let item = world.spawn((ItemTriggers(vec![TriggerDef {
            event: "on_hit".into(),
            chance: 100,
            cast: "fire_bolt".into(),
            target: "target".into(),
            script: None,
            params: std::collections::HashMap::new(),
        }]),));

        let mut q_eq = world.query_one::<&mut Equipment>(e).unwrap();
        q_eq.get().unwrap().equip(EquipmentSlot::Weapon, item);
        drop(q_eq);

        let results = process_triggers(&world, e, "on_hit");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].cast, "fire_bolt");
    }

    #[test]
    fn test_trigger_does_not_fire_for_unmatched_event() {
        let mut world = World::new();
        let e = world.spawn((Equipment::new(),));

        let item = world.spawn((ItemTriggers(vec![TriggerDef {
            event: "on_hit".into(),
            chance: 100,
            cast: "fire_bolt".into(),
            target: "target".into(),
            script: None,
            params: std::collections::HashMap::new(),
        }]),));

        let mut q_eq = world.query_one::<&mut Equipment>(e).unwrap();
        q_eq.get().unwrap().equip(EquipmentSlot::Weapon, item);
        drop(q_eq);

        let results = process_triggers(&world, e, "on_wear");
        assert!(results.is_empty());
    }
}
