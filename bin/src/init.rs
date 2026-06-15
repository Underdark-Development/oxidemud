use mud_core::templates::AreaTemplate;
use mud_core::{Direction, Entity, Exit, Position, Room, RoomExits, World};

pub fn init_world() -> (World, Entity) {
    let mut world = World::new();

    let void_room = world.spawn((
        Room::new("The Void", "You are floating in a void"),
        mud_core::VoidRoom,
    ));

    world
        .insert(void_room, (Position::new(void_room),))
        .expect("void room should exist");

    (world, void_room)
}

/// Spawn all rooms from the given area template into the ECS world.
///
/// Returns the Entity of the room designated as `spawn_room` in the template.
/// Each room gets [`Room`], [`Position`], and [`RoomExits`] components.
/// Exits are resolved from room IDs to entity references in a second pass.
pub fn spawn_area(world: &mut World, area: &AreaTemplate) -> Entity {
    use std::collections::HashMap;

    let mut room_map: HashMap<&str, Entity> = HashMap::new();

    // First pass: spawn all room entities
    for (room_id, room_tpl) in &area.rooms {
        let key = format!("{}:{room_id}", area.id);
        let entity = world.spawn((
            Room::new(&room_tpl.name, &room_tpl.description),
            mud_core::RoomFlags::default(),
            mud_core::SpawnKey(key),
        ));
        world.insert(entity, (Position::new(entity),)).unwrap();
        room_map.insert(room_id.as_str(), entity);
    }

    // Second pass: resolve exits
    for (room_id, room_tpl) in &area.rooms {
        let room_entity = room_map[room_id.as_str()];
        let mut exits = Vec::new();

        for (dir_str, dest_id) in &room_tpl.exits {
            if let Some(direction) = Direction::try_from(dir_str) {
                if let Some(&dest_entity) = room_map.get(dest_id.as_str()) {
                    exits.push(Exit::new(direction, dest_entity));
                }
            }
        }

        if !exits.is_empty() {
            exits.sort_by_key(|e| e.direction as u8);
            world.insert(room_entity, (RoomExits(exits),)).unwrap();
        }
    }

    room_map[area.spawn_room.as_str()]
}
