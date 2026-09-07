use std::collections::{HashSet, VecDeque};

use crate::{Entity, Position, World};

use crate::components::room::EXIT_IS_DOOR;
use crate::templates::ChannelScope;

/// Returns all entities (players, NPCs, mobs, items) in the given room.
pub fn entities_in_room(world: &World, room: Entity) -> Vec<Entity> {
    world
        .query::<(&Position,)>()
        .iter()
        .map(|(raw, (pos,))| (raw, pos))
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
    let base_sd = {
        let sd = world
            .query_one::<&crate::ShortDesc>(entity)
            .ok()
            .and_then(|mut q| q.get().map(|s| s.0.clone()));
        if sd.as_ref().is_some_and(|s| !s.is_empty()) {
            sd
        } else {
            world
                .query_one::<&crate::Name>(entity)
                .ok()
                .and_then(|mut q| q.get().map(|n| n.0.clone()))
        }
    };

    let base = base_sd?;

    if let Ok(mut q) = world.query_one::<&crate::ActiveScriptEffects>(entity) {
        if let Some(active) = q.get() {
            for effect in &active.effects {
                if let Some(ref override_desc) = effect.short_desc_override {
                    return Some(override_desc.clone());
                }
            }
            let mut result = base.clone();
            for effect in &active.effects {
                if let Some(ref prefix) = effect.name_prefix {
                    if result.to_lowercase().starts_with("a ") {
                        result = format!("{}{}", prefix, &result[2..]);
                    } else if result.to_lowercase().starts_with("an ") {
                        result = format!("{}{}", prefix, &result[3..]);
                    } else {
                        result = format!("{}{}", prefix, result);
                    }
                }
                if let Some(ref suffix) = effect.name_suffix {
                    result.push_str(suffix);
                }
            }
            return Some(result);
        }
    }

    Some(base)
}

pub fn is_void_room(world: &World, room: Entity) -> bool {
    world
        .query_one::<&crate::VoidRoom>(room)
        .is_ok_and(|mut q| q.get().is_some())
}

/// Room key assigned to the always-present, content-independent Void room.
///
/// The Void is the server's disconnected holding area and spawn-of-last-resort:
/// it cannot be linked to the world, entered, or left except by imm teleport.
pub const VOID_ROOM_KEY: &str = "system:void";

/// The Void room entity, if it exists.
pub fn void_room(world: &World) -> Option<Entity> {
    world
        .query::<&crate::VoidRoom>()
        .iter()
        .next()
        .map(|(entity, _)| entity)
}

/// The Void room entity, re-created with its standard components if it is
/// somehow absent. The Void must always be available as the fallback holding
/// area, so removal is never permanent.
pub fn ensure_void_room(world: &mut World) -> Entity {
    if let Some(room) = void_room(world) {
        return room;
    }
    let room = world.spawn((
        crate::Room::new("The Void", "You are floating in a void"),
        crate::VoidRoom,
        crate::RoomKey(VOID_ROOM_KEY.to_string()),
        crate::RoomFlags(
            crate::ROOM_NO_TELEPORT_IN | crate::ROOM_NO_TELEPORT_OUT | crate::ROOM_SILENT,
        ),
    ));
    let _ = world.insert(room, (crate::Position::new(room),));
    room
}

/// Whether an entity holds staff rank (access of Immortal or higher).
///
/// Staff characters bypass all Void isolation rules.
pub fn is_staff(world: &World, entity: Entity) -> bool {
    world
        .query_one::<&crate::AccessLevel>(entity)
        .ok()
        .and_then(|mut q| q.get().copied())
        .map(|level| level >= crate::AccessLevel::Immortal)
        .unwrap_or(false)
}

/// Whether an entity is a non-staff occupant of the Void.
///
/// Isolated occupants may only use room-local communication (`say`, `emote`),
/// cannot send or receive world-facing comms, and cannot leave the Void by any
/// means other than an imm teleport.
pub fn is_void_isolated(world: &World, entity: Entity) -> bool {
    let in_void = get_pos_room(world, entity).is_some_and(|r| is_void_room(world, r));
    in_void && !is_staff(world, entity)
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

/// Collect all room entities reachable from `start_room` given a channel scope.
///
/// Propagation rules:
/// - `Room` scope — only the start room.
/// - `Adjacent(n)` — BFS up to depth n through the exit graph.
/// - `Area` — BFS unlimited depth, constrained by RoomKey area prefix.
/// - `Global` — returns empty vec (handled by caller).
///
/// Blockers: exits with `EXIT_IS_DOOR` are not traversed. Rooms with
/// `ROOM_SILENT` are not traversed *into* (but the start room is always
/// included even if silent).
pub fn collect_rooms_by_scope(
    world: &World,
    start_room: Entity,
    scope: &ChannelScope,
) -> Vec<Entity> {
    match scope {
        ChannelScope::Room => return vec![start_room],
        ChannelScope::Global => return vec![],
        _ => {}
    }

    let max_depth = match scope {
        ChannelScope::Adjacent(n) => *n,
        ChannelScope::Area => u32::MAX,
        _ => unreachable!(),
    };

    let area_prefix = if matches!(scope, ChannelScope::Area) {
        world
            .query_one::<&crate::RoomKey>(start_room)
            .ok()
            .and_then(|mut q| {
                let rk = q.get()?;
                rk.0.split_once(':').map(|(a, _)| format!("{}:", a))
            })
    } else {
        None
    };

    let start_is_silent = match world.query_one::<&crate::RoomFlags>(start_room) {
        Ok(mut q) => q.get().copied().unwrap_or_default().is_silent(),
        _ => false,
    };

    // If start room is silent, sound doesn't leave it
    if start_is_silent {
        return vec![start_room];
    }

    let mut visited = HashSet::new();
    let mut queue = VecDeque::new();
    visited.insert(start_room);
    queue.push_back((start_room, 0u32));

    while let Some((room, depth)) = queue.pop_front() {
        if depth >= max_depth {
            continue;
        }

        if let Ok(mut q) = world.query_one::<&crate::RoomExits>(room) {
            if let Some(exits) = q.get() {
                for exit in &exits.0 {
                    // Doors block sound propagation
                    if exit.flags & EXIT_IS_DOOR != 0 {
                        continue;
                    }

                    let dest = exit.dest;
                    if visited.contains(&dest) {
                        continue;
                    }

                    // Silent rooms are barriers — sound doesn't enter
                    let dest_silent = match world.query_one::<&crate::RoomFlags>(dest) {
                        Ok(mut q) => q.get().copied().unwrap_or_default().is_silent(),
                        _ => false,
                    };
                    if dest_silent {
                        continue;
                    }

                    // Area scope: check dest is in same area
                    if let Some(ref prefix) = area_prefix {
                        let dest_in_area = match world.query_one::<&crate::RoomKey>(dest) {
                            Ok(mut q) => q
                                .get()
                                .map(|rk| rk.0.starts_with(prefix.as_str()))
                                .unwrap_or(false),
                            _ => false,
                        };
                        if !dest_in_area {
                            continue;
                        }
                    }

                    visited.insert(dest);
                    queue.push_back((dest, depth + 1));
                }
            }
        }
    }

    visited.into_iter().collect()
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

    #[test]
    fn ensure_void_room_creates_standard_void() {
        let mut world = World::new();
        let room = ensure_void_room(&mut world);
        assert!(is_void_room(&world, room));
        assert_eq!(void_room(&world), Some(room));
        assert_eq!(
            world
                .query_one::<&RoomKey>(room)
                .ok()
                .and_then(|mut q| q.get().map(|k| k.0.clone())),
            Some(VOID_ROOM_KEY.to_string())
        );
        // Idempotent — second call returns the same entity.
        assert_eq!(ensure_void_room(&mut world), room);
    }

    #[test]
    fn is_staff_excludes_players_and_builders() {
        let mut world = World::new();
        let player = world.spawn((AccessLevel::Player,));
        let builder = world.spawn((AccessLevel::Builder,));
        let immortal = world.spawn((AccessLevel::Immortal,));
        let god = world.spawn((AccessLevel::God,));
        let admin = world.spawn((AccessLevel::Admin,));
        let unqualified = world.spawn(());

        assert!(!is_staff(&world, player));
        assert!(!is_staff(&world, builder));
        assert!(!is_staff(&world, unqualified));
        assert!(is_staff(&world, immortal));
        assert!(is_staff(&world, god));
        assert!(is_staff(&world, admin));
    }

    #[test]
    fn is_void_isolated_isolates_only_non_staff_in_void() {
        let mut world = World::new();
        let void = ensure_void_room(&mut world);
        let normal_room = world.spawn(());

        let lone_player = world.spawn(());

        let void_player = world.spawn((Position::new(void), AccessLevel::Player));
        let void_immortal = world.spawn((Position::new(void), AccessLevel::Immortal));
        let normal_player = world.spawn((Position::new(normal_room), AccessLevel::Player));

        assert!(is_void_isolated(&world, void_player));
        assert!(!is_void_isolated(&world, void_immortal));
        assert!(!is_void_isolated(&world, normal_player));
        assert!(!is_void_isolated(&world, lone_player));
    }
}
