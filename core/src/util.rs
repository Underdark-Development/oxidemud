use crate::{Entity, Position, World};

/// Returns all entities (players, NPCs, mobs, items) in the given room.
pub fn entities_in_room(world: &World, room: Entity) -> Vec<Entity> {
    world
        .query::<(&Position,)>()
        .iter()
        .map(|(raw, (pos,))| (Entity::from(raw), pos))
        .filter(|(_, pos)| pos.room == room)
        .map(|(entity, _)| entity)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::*;

    #[test]
    fn empty_room_returns_empty() {
        let mut world = World::new();
        let room = world.spawn(());
        let entities = entities_in_room(&world, room);
        assert!(entities.is_empty());
    }

    #[test]
    fn finds_all_occupants_in_room() {
        let mut world = World::new();
        let room = world.spawn(());
        let e1 = world.spawn((Position::new(room),));
        let e2 = world.spawn((Position::new(room),));
        let other_room = world.spawn(());
        let e3 = world.spawn((Position::new(other_room),));

        let entities = entities_in_room(&world, room);
        assert_eq!(entities.len(), 2);
        assert!(entities.contains(&e1));
        assert!(entities.contains(&e2));
        assert!(!entities.contains(&e3));
    }

    #[test]
    fn excludes_entities_without_position() {
        let mut world = World::new();
        let room = world.spawn(());
        let e1 = world.spawn((Position::new(room),));
        // entity with no Position component
        world.spawn(());

        let entities = entities_in_room(&world, room);
        assert_eq!(entities, vec![e1]);
    }
}
