use std::collections::HashMap;

use crate::{CombatTarget, Entity, Friendly, Health, Level, Npc, Position, RoomExits, World};

/// Per-NPC AI state.
#[derive(Debug, Clone)]
pub struct AiState {
    pub ai_mode: String,
    pub threat_table: HashMap<Entity, i32>,
    pub wander_counter: u8,
    pub patrol_index: usize,
    pub aggro_range: u32,
    pub aggro_players: bool,
    pub aggro_race: Vec<String>,
    pub aggro_mobs: bool,
}

/// Run one AI pulse for all NPCs with AiState.
pub fn run_ai_pulse(world: &mut World) {
    let npc_list: Vec<(Entity, String)> = {
        let mut q = world.query::<(&Npc,)>();
        q.iter()
            .map(|(raw, _)| {
                let entity = crate::Entity::from(raw);
                (entity, String::new())
            })
            .collect()
    };

    for (entity, _) in npc_list {
        // Ensure AiState exists; if not, skip
        if world.query_one::<&AiState>(entity).is_err() {
            continue;
        }

        let state = match world
            .query_one::<&AiState>(entity)
            .ok()
            .and_then(|mut q| q.get().cloned())
        {
            Some(s) => s,
            None => continue,
        };

        // Skip if already in combat
        let in_combat = world.query_one::<&CombatTarget>(entity).is_ok();

        if in_combat {
            let health = world
                .query_one::<&Health>(entity)
                .ok()
                .and_then(|mut q| q.get().cloned());

            // Check flee: HP < 25%
            let should_flee = health
                .as_ref()
                .map(|h| (h.current as f32 / h.max as f32) < 0.25)
                .unwrap_or(false);

            if should_flee {
                try_flee(world, entity);
            }
            continue;
        }

        match state.ai_mode.as_str() {
            "wander" => do_wander(world, entity, state),
            "aggro" | "aggressive" => {
                do_aggro_check(world, entity, state);
            }
            "stationary" | "idle" => {
                // Just aggro check if aggressive
                do_aggro_check(world, entity, state);
            }
            "patrol" => do_patrol(world, entity, state),
            _ => {}
        }
    }
}

fn do_wander(world: &mut World, entity: Entity, mut state: AiState) {
    state.wander_counter += 1;
    // Wander every 3-5 pulses (simplified: every 4)
    if state.wander_counter < 4 {
        update_ai_state(world, entity, state);
        return;
    }
    state.wander_counter = 0;

    // Find random exit and move
    let exits = match world
        .query_one::<&RoomExits>(entity)
        .ok()
        .and_then(|mut q| q.get().map(|e| e.0.clone()))
    {
        Some(e) => e,
        None => {
            update_ai_state(world, entity, state);
            return;
        }
    };

    if exits.is_empty() {
        update_ai_state(world, entity, state);
        return;
    }

    let room = match world
        .query_one::<&Position>(entity)
        .ok()
        .and_then(|mut q| q.get().map(|p| p.room))
    {
        Some(r) => r,
        None => {
            update_ai_state(world, entity, state);
            return;
        }
    };

    // Find exits from current room
    let room_exits = match world.query_one::<&RoomExits>(room) {
        Ok(mut q) => q.get().map(|e| e.0.clone()),
        Err(_) => None,
    };

    if let Some(exits) = room_exits {
        let visible: Vec<_> = exits.iter().filter(|e| !e.is_hidden()).collect();
        if !visible.is_empty() {
            let idx = fastrand::usize(..visible.len());
            let dest = visible[idx].dest;
            let _ = world.insert(entity, (Position::new(dest),));
        }
    }

    update_ai_state(world, entity, state);
}

fn do_aggro_check(world: &mut World, entity: Entity, state: AiState) {
    let room = match world
        .query_one::<&Position>(entity)
        .ok()
        .and_then(|mut q| q.get().map(|p| p.room))
    {
        Some(r) => r,
        None => return,
    };

    // Find potential targets in the same room
    let targets: Vec<Entity> = {
        let mut q = world.query::<(&Position, &Health)>();
        q.iter()
            .map(|(raw, (pos, _health))| (crate::Entity::from(raw), pos))
            .filter(|(e, pos)| pos.room == room && *e != entity)
            .map(|(e, _)| e)
            .collect()
    };

    for target in targets {
        // Aggro unfriendly NPCs (mobs without Friendly marker)
        if state.aggro_mobs
            && world.query_one::<&Npc>(target).is_ok()
            && world.query_one::<&Friendly>(target).is_err()
        {
            let _ = world.insert(entity, (CombatTarget(target),));
            return;
        }

        // Aggro weak players
        if state.aggro_players && world.query_one::<&crate::Player>(target).is_ok() {
            let target_level = world
                .query_one::<&Level>(target)
                .ok()
                .and_then(|mut q| q.get().copied())
                .unwrap_or(Level(1));

            let npc_level = world
                .query_one::<&Level>(entity)
                .ok()
                .and_then(|mut q| q.get().copied())
                .unwrap_or(Level(1));

            if target_level.0 <= npc_level.0 + 3 {
                let _ = world.insert(entity, (CombatTarget(target),));
                return;
            }
        }
    }
}

fn do_patrol(world: &mut World, entity: Entity, mut state: AiState) {
    // Simple patrol: move to next room in a pre-defined path
    // For now, just wander as fallback
    state.wander_counter += 1;
    if state.wander_counter < 6 {
        update_ai_state(world, entity, state);
        return;
    }
    state.wander_counter = 0;
    do_wander(world, entity, state);
}

fn try_flee(world: &mut World, entity: Entity) {
    let room = match world
        .query_one::<&Position>(entity)
        .ok()
        .and_then(|mut q| q.get().map(|p| p.room))
    {
        Some(r) => r,
        None => return,
    };

    let exits = match world.query_one::<&RoomExits>(room) {
        Ok(mut q) => q.get().map(|e| e.0.clone()),
        Err(_) => None,
    };

    if let Some(exits) = exits {
        let visible: Vec<_> = exits.iter().filter(|e| !e.is_hidden()).collect();
        if !visible.is_empty() {
            let idx = fastrand::usize(..visible.len());
            let dest = visible[idx].dest;
            let _ = world.insert(entity, (Position::new(dest),));
        }
    }

    // Clear combat target
    let _ = world.remove_one::<CombatTarget>(entity);
}

fn update_ai_state(world: &mut World, entity: Entity, state: AiState) {
    let _ = world.insert(entity, (state,));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Name;

    fn npc_world() -> (World, Entity, Entity) {
        let mut world = World::new();
        let room_a = world.spawn(());
        let _room_b = world.spawn(());
        let npc = world.spawn((
            Position::new(room_a),
            Health::new(50),
            Npc::new("test_npc"),
            Level(1),
            Name::new("Test NPC"),
            AiState {
                ai_mode: "idle".to_string(),
                threat_table: HashMap::new(),
                wander_counter: 0,
                patrol_index: 0,
                aggro_range: 0,
                aggro_players: false,
                aggro_race: vec![],
                aggro_mobs: false,
            },
        ));
        (world, npc, room_a)
    }

    #[test]
    fn test_ai_pulse_no_crash() {
        let (mut world, _npc, _) = npc_world();
        run_ai_pulse(&mut world);
    }

    #[test]
    fn test_ai_idle_does_not_move() {
        let (mut world, npc, room) = npc_world();
        run_ai_pulse(&mut world);
        let pos = world
            .query_one::<&Position>(npc)
            .unwrap()
            .get()
            .unwrap()
            .room;
        assert_eq!(pos, room);
    }
}
