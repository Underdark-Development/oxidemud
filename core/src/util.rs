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

pub fn get_pos_room(world: &World, entity: Entity) -> Option<Entity> {
    world
        .query_one::<&Position>(entity)
        .ok()
        .and_then(|mut q| q.get().map(|p| p.room))
}

pub fn get_room_name(world: &World, room: Entity) -> Option<String> {
    world
        .query_one::<&crate::Room>(room)
        .ok()
        .and_then(|mut q| q.get().map(|r| r.name.clone()))
}

pub fn get_room_desc(world: &World, room: Entity) -> Option<String> {
    world
        .query_one::<&crate::Room>(room)
        .ok()
        .and_then(|mut q| q.get().map(|r| r.description.clone()))
}

pub fn get_name(world: &World, entity: Entity) -> Option<crate::Name> {
    world
        .query_one::<&crate::Name>(entity)
        .ok()
        .and_then(|mut q| q.get().cloned())
}

pub fn get_entity_name(world: &World, entity: Entity) -> Option<String> {
    world
        .query_one::<&crate::Name>(entity)
        .ok()
        .and_then(|mut q| q.get().map(|n| n.as_str().to_lowercase()))
}

pub fn get_short_desc(world: &World, entity: Entity) -> Option<String> {
    let sd = world
        .query_one::<&crate::ShortDesc>(entity)
        .ok()
        .and_then(|mut q| q.get().map(|s| s.0.clone()));
    if sd.as_ref().is_some_and(|s| !s.is_empty()) {
        return sd;
    }
    world
        .query_one::<&crate::Name>(entity)
        .ok()
        .and_then(|mut q| q.get().map(|n| n.0.clone()))
}

pub fn is_void_room(world: &World, room: Entity) -> bool {
    world
        .query_one::<&crate::VoidRoom>(room)
        .is_ok_and(|mut q| q.get().is_some())
}

pub fn get_exits(world: &World, room: Entity) -> Vec<&'static str> {
    let mut exits = Vec::new();
    if let Ok(mut q) = world.query_one::<&crate::RoomExits>(room) {
        if let Some(room_exits) = q.get() {
            for exit in &room_exits.0 {
                if !exit.is_hidden() {
                    exits.push(exit.direction.short_name());
                }
            }
        }
    }
    exits
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
