use std::time::Instant;

use crate::{Corpse, Entity, Equipment, FloorItems, Inventory, Position, World};

/// Sweep expired corpses, transfer items to room floor.
pub fn run_corpse_pulse(world: &mut World) {
    let now = Instant::now();
    let expired: Vec<(Entity, Entity)> = {
        let mut q = world.query::<(&Corpse, &Position)>();
        q.iter()
            .filter_map(|(raw, (corpse, pos))| {
                let elapsed = now.duration_since(corpse.created_at).as_secs();
                if elapsed >= corpse.decay_secs {
                    Some((raw, pos.room))
                } else {
                    None
                }
            })
            .collect()
    };

    for (corpse, room) in expired {
        // Transfer inventory items to room floor
        let items = world
            .query_one::<&Inventory>(corpse)
            .ok()
            .and_then(|mut q| q.get().map(|inv| inv.0.clone()))
            .unwrap_or_default();

        // Transfer equipment items to room floor
        let eq_items = world
            .query_one::<&Equipment>(corpse)
            .ok()
            .and_then(|mut q| {
                q.get().map(|eq| {
                    eq.slots
                        .iter()
                        .map(|(_, item)| *item)
                        .collect::<Vec<Entity>>()
                })
            })
            .unwrap_or_default();

        let mut all_items = items;
        all_items.extend(eq_items);

        // Add items to room's FloorItems
        if !all_items.is_empty() {
            if let Ok(mut q) = world.query_one::<&mut FloorItems>(room) {
                if let Some(floor) = q.get() {
                    floor.0.extend(all_items);
                }
            }
        }

        // Despawn corpse
        let _ = world.despawn(corpse);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::LootRule;

    #[test]
    fn test_no_expired_corpses() {
        let mut world = World::new();
        let room = world.spawn((FloorItems::default(),));
        let corpse = world.spawn((
            Corpse {
                owner: None,
                owner_db_id: None,
                created_at: Instant::now(),
                decay_secs: 3600,
                lootable_by: LootRule::Public,
            },
            Position::new(room),
            Inventory::new(),
        ));
        run_corpse_pulse(&mut world);
        // Corpse should still exist
        assert!(world.query_one::<&Corpse>(corpse).is_ok());
    }
}
