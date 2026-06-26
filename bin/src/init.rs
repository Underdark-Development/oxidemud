use oxide_core::{
    Direction, Entity, Exit, PatrolRoute, Position, Room, RoomExits, WanderBounds, World,
};

pub fn init_world() -> (World, Entity) {
    let mut world = World::new();

    let void_room = world.spawn((
        Room::new("The Void", "You are floating in a void"),
        oxide_core::VoidRoom,
    ));

    world
        .insert(void_room, (Position::new(void_room),))
        .expect("void room should exist");

    (world, void_room)
}

use oxide_core::templates::TemplateRegistry;

/// Spawn all rooms and their mobs from the given area template into the ECS world.
///
/// Each room gets [`Room`], [`Position`], [`RoomExits`], and [`SpawnKey`] components.
/// Exits are resolved from room IDs to entity references in a second pass.
/// Mob spawns defined in `RoomTemplate.content.mobs` are instantiated after
/// room entities are created.
pub fn spawn_area(
    world: &mut World,
    area: &oxide_core::templates::AreaTemplate,
    registry: &TemplateRegistry,
) {
    use std::collections::HashMap;

    let mut room_map: HashMap<&str, Entity> = HashMap::new();

    // First pass: spawn all room entities
    for (room_id, room_tpl) in &area.rooms {
        let key = format!("{}:{room_id}", area.id);
        let entity = world.spawn((
            Room::new(&room_tpl.name, &room_tpl.description).with_script(room_tpl.script.clone()),
            oxide_core::RoomFlags::default(),
            oxide_core::SpawnKey(key),
        ));
        world.insert(entity, (Position::new(entity),)).unwrap();
        if room_tpl.allow_revive {
            world
                .insert(entity, (oxide_core::RoomAllowRevive,))
                .unwrap();
        }
        room_map.insert(room_id.as_str(), entity);
    }

    // Second pass: resolve exits
    for (room_id, room_tpl) in &area.rooms {
        let room_entity = room_map[room_id.as_str()];
        let mut exits = Vec::new();

        for (dir_str, exit_tpl) in &room_tpl.exits {
            if let Some(direction) = Direction::try_from(dir_str) {
                let dest_str = exit_tpl.dest();
                let target_room_id = if let Some((_, r)) = dest_str.split_once(':') {
                    r
                } else {
                    dest_str
                };
                if let Some(&dest_entity) = room_map.get(target_room_id) {
                    let mut exit = Exit::new(direction, dest_entity);
                    if let oxide_core::ExitTemplate::Detailed {
                        door,
                        closed,
                        locked,
                        key_id,
                        ..
                    } = exit_tpl
                    {
                        if *door {
                            exit.flags |= oxide_core::EXIT_IS_DOOR;
                        }
                        if *closed {
                            exit.flags |= oxide_core::EXIT_IS_CLOSED;
                        }
                        if *locked {
                            exit.flags |= oxide_core::EXIT_IS_LOCKED;
                        }
                        exit.key_id = key_id.clone();
                    }
                    exits.push(exit);
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
                // Call MobTemplate::spawn which handles components, AI state, trainer, friendly, and equipment
                let npc = mob_tpl.spawn(world, room_entity, registry);

                // Resolve patrol route from template room IDs
                let patrol_route = if !mob_tpl.patrol_route.is_empty() {
                    let wps: Vec<Entity> = mob_tpl
                        .patrol_route
                        .iter()
                        .filter_map(|rid| room_map.get(rid.as_str()).copied())
                        .collect();
                    if wps.is_empty() {
                        None
                    } else {
                        Some(PatrolRoute(wps))
                    }
                } else {
                    None
                };

                // Resolve wander bounds from template room IDs or area
                let wander_bounds = if !mob_tpl.wander_rooms.is_empty() {
                    let rooms: Vec<Entity> = mob_tpl
                        .wander_rooms
                        .iter()
                        .filter_map(|rid| room_map.get(rid.as_str()).copied())
                        .collect();
                    if rooms.is_empty() {
                        None
                    } else {
                        Some(WanderBounds(rooms))
                    }
                } else if mob_tpl.wander_area {
                    let rooms: Vec<Entity> = area
                        .rooms
                        .keys()
                        .filter_map(|rid| room_map.get(rid.as_str()).copied())
                        .collect();
                    if rooms.is_empty() {
                        None
                    } else {
                        Some(WanderBounds(rooms))
                    }
                } else {
                    None
                };

                if let Some(route) = patrol_route {
                    world.insert(npc, (route,)).unwrap();
                }
                if let Some(bounds) = wander_bounds {
                    world.insert(npc, (bounds,)).unwrap();
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oxide_core::{Armor, Attributes, Equipment, EquipmentSlot, Npc, Weapon};
    use std::path::{Path, PathBuf};

    fn content_path() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../content")
    }

    #[test]
    fn spawn_area_applies_mob_combat_template_fields() {
        let (mut world, _) = init_world();
        let (registry, _) = oxide_core::content::load_registry(&content_path());
        let area = registry
            .get_area("starting_vale")
            .expect("starting_vale should load");

        spawn_area(&mut world, area, &registry);

        let goblin = {
            let mut q = world.query::<(&Npc, &Attributes, &Armor, &Equipment)>();
            q.iter()
                .find(|(_, (npc, _, _, _))| npc.template_id == "goblin_scavenger")
                .map(|(entity, _)| Entity::from(entity))
                .expect("goblin should spawn")
        };

        let mut attrs = world
            .query_one::<&Attributes>(goblin)
            .expect("goblin should exist");
        assert_eq!(
            attrs.get().expect("goblin should have attributes").strength,
            10
        );

        let mut armor = world
            .query_one::<&Armor>(goblin)
            .expect("goblin should exist");
        assert_eq!(armor.get().expect("goblin should have armor").base, 2);

        let weapon = {
            let mut equipment = world
                .query_one::<&Equipment>(goblin)
                .expect("goblin should exist");
            *equipment
                .get()
                .expect("goblin should have equipment")
                .equipped(&EquipmentSlot::Weapon)
                .expect("goblin should have a weapon")
        };

        assert!(world.query_one::<&Weapon>(weapon).is_ok());
    }

    #[test]
    fn actual_content_loads_all_mobs() {
        let (registry, _) = oxide_core::content::load_registry(&content_path());

        assert!(registry.get_mob("temple_acolyte").is_some());
        assert!(registry.get_mob("trainer").is_some());
        assert_eq!(registry.mobs.len(), 8);
        let errs = registry.validate();
        if !errs.is_empty() {
            println!("VALIDATION ERRORS: {:#?}", errs);
        }
        assert!(errs.is_empty());
    }

    #[test]
    fn spawn_trainer_npc_attaches_trainer_component() {
        let (mut world, _) = init_world();
        let (registry, _) = oxide_core::content::load_registry(&content_path());
        let area = registry
            .get_area("starting_vale")
            .expect("starting_vale should load");

        spawn_area(&mut world, area, &registry);

        let trainer = {
            let mut q = world.query::<(&Npc, &oxide_core::Trainer)>();
            q.iter()
                .find(|(_, (npc, _))| npc.template_id == "trainer")
                .map(|(entity, _)| Entity::from(entity))
                .expect("trainer should spawn and have Trainer component")
        };

        let mut q_trainer = world
            .query_one::<&oxide_core::Trainer>(trainer)
            .expect("trainer should exist in world");
        let t = q_trainer.get().expect("should have Trainer component");
        assert_eq!(t.trainer_types, vec!["attributes".to_string()]);
    }
}
