use mud_core::templates::{AreaTemplate, TemplateRegistry};
use mud_core::{
    AiState, Direction, Entity, Exit, Friendly, Health, Level, Name, Npc, Position, Race, Room,
    RoomExits, ShortDesc, World,
};

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

/// Spawn all rooms and their mobs from the given area template into the ECS world.
///
/// Returns the Entity of the room designated as `spawn_room` in the template.
/// Each room gets [`Room`], [`Position`], and [`RoomExits`] components.
/// Exits are resolved from room IDs to entity references in a second pass.
/// Mob spawns defined in `RoomTemplate.content.mobs` are instantiated after
/// room entities are created.
pub fn spawn_area(world: &mut World, area: &AreaTemplate, registry: &TemplateRegistry) -> Entity {
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

    // Third pass: spawn mobs from room content
    for (room_id, room_tpl) in &area.rooms {
        let room_entity = room_map[room_id.as_str()];

        for spawn in &room_tpl.content.mobs {
            let Some(mob_tpl) = registry.mobs.get(&spawn.template_id) else {
                tracing::warn!(
                    "Mob template '{}' not found, skipping spawn in {}/{}",
                    spawn.template_id,
                    area.id,
                    room_id
                );
                continue;
            };

            for _ in 0..spawn.count {
                let npc = world.spawn((
                    Position::new(room_entity),
                    Name::new(&mob_tpl.name),
                    Npc::new(&mob_tpl.id),
                    Health {
                        current: mob_tpl.health.current,
                        max: mob_tpl.health.max,
                    },
                    Level(mob_tpl.level),
                    AiState {
                        ai_mode: mob_tpl.ai_mode.clone(),
                        threat_table: HashMap::new(),
                        wander_counter: 0,
                        patrol_index: 0,
                        aggro_range: mob_tpl.aggro_range,
                        aggro_players: mob_tpl.aggro_players,
                        aggro_race: mob_tpl.aggro_race.clone(),
                        aggro_mobs: mob_tpl.aggro_mobs,
                    },
                ));

                // Add Race component if the mob template has one
                if let Some(ref race_id) = mob_tpl.race {
                    world.insert(npc, (Race(race_id.clone()),)).unwrap();
                }

                // Add ShortDesc component (falls back to name if not set)
                let short_desc = if mob_tpl.short_desc.is_empty() {
                    mob_tpl.name.clone()
                } else {
                    mob_tpl.short_desc.clone()
                };
                world.insert(npc, (ShortDesc(short_desc),)).unwrap();

                // Add Friendly marker if the mob template says so
                if mob_tpl.friendly {
                    world.insert(npc, (Friendly,)).unwrap();
                }
            }
        }
    }

    room_map[area.spawn_room.as_str()]
}
