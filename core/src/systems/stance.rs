use crate::{ActiveStance, World};

/// Apply stance modifiers to all entities with ActiveStance.
/// This system runs in the Combat phase. It simply ensures stance
/// data is available; the actual modifier math is applied at
/// combat resolution time by querying ActiveStance.
pub fn run_stance_pulse(world: &mut World) {
    // Currently a no-op — stance modifiers are computed inline
    // during combat and AC calculations by checking ActiveStance.
    // Future: apply/reconcile ActiveEffect components for stances.
    let _ = world;
}

/// Get the stance ID for an entity, if any.
pub fn get_active_stance(world: &World, entity: crate::Entity) -> Option<String> {
    world
        .query_one::<&ActiveStance>(entity)
        .ok()
        .and_then(|mut q| q.get().and_then(|s| s.0.clone()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_active_stance_none() {
        let mut world = World::new();
        let e = world.spawn((ActiveStance(None),));
        assert_eq!(get_active_stance(&world, e), None);
    }

    #[test]
    fn test_get_active_stance_some() {
        let mut world = World::new();
        let e = world.spawn((ActiveStance(Some("defensive".to_string())),));
        assert_eq!(get_active_stance(&world, e), Some("defensive".to_string()));
    }
}
