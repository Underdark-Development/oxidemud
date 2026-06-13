use crate::{Entity, Equipment, World};

/// Evaluate item set bonuses for an entity.
/// Currently a placeholder — full set bonus evaluation
/// will be implemented alongside SetTracker component.
pub fn evaluate_set_bonuses(world: &mut World, _entity: Entity) {
    let _ = world;
}

/// Check equipped items for set membership and return
/// a map of set_id → count of equipped pieces.
pub fn get_equipped_sets(world: &World, entity: Entity) -> Vec<(String, usize)> {
    let _ = world.query_one::<&Equipment>(entity);

    // Count by template_id — currently we just return the slots
    // Set membership data comes from ItemTemplate in the registry
    // (handled at the bin/server level)
    vec![]
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
}
