use crate::{
    CombatState, Entity, Friendly, Health, Level, Npc, PatrolRoute, Position, RoomExits,
    WanderBounds, World,
};

const WANDER_PULSE_THRESHOLD: u8 = 15;

/// Per-NPC AI behavioral state machine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AiState {
    Idle,
    Wander {
        counter: u8,
    },
    Patrol {
        counter: u8,
        index: usize,
        forward: bool,
    },
    Aggro {
        hunt_target: Option<Entity>,
    },
    Combat,
    Flee {
        attempts: u8,
    },
    Return {
        home: Entity,
    },
}

/// Run one AI pulse for all NPCs with AiState.
pub fn run_ai_pulse(world: &mut World) {
    let entities: Vec<Entity> = {
        let mut q = world.query::<&Npc>();
        q.iter().map(|(raw, _)| crate::Entity::from(raw)).collect()
    };

    for entity in entities {
        let state = match world
            .query_one::<&AiState>(entity)
            .ok()
            .and_then(|mut q| q.get().cloned())
        {
            Some(s) => s,
            None => continue,
        };

        let combat_state = world
            .query_one::<&CombatState>(entity)
            .ok()
            .and_then(|mut q| q.get().cloned())
            .unwrap_or(CombatState::NotInCombat);

        let npc = match world
            .query_one::<&Npc>(entity)
            .ok()
            .and_then(|mut q| q.get().cloned())
        {
            Some(n) => n,
            None => continue,
        };

        let old_state = state.clone();
        let new_state = tick_ai(state, &combat_state, &npc, world, entity);

        if new_state != old_state {
            let _event = crate::GameEvent::AiStateChanged {
                entity,
                from: old_state.clone(),
                to: new_state.clone(),
            };
            let _ = world.insert(entity, (new_state,));
        }
    }
}

/// Formal transition function for the AI state machine.
/// Handles combat edge cases, aggro checks, movement, and state transitions.
fn tick_ai(
    state: AiState,
    combat_state: &CombatState,
    npc: &Npc,
    world: &mut World,
    entity: Entity,
) -> AiState {
    // Combat ended — transition out of combat AI states
    if matches!(state, AiState::Combat) && !combat_state.is_in_combat() {
        let current_room = world
            .query_one::<&Position>(entity)
            .ok()
            .and_then(|mut q| q.get().map(|p| p.room));
        return current_room.map_or(AiState::Idle, |room| AiState::Return { home: room });
    }

    if matches!(state, AiState::Flee { .. }) && !combat_state.is_in_combat() {
        return AiState::Idle;
    }

    // In combat — handle combat-specific logic
    if combat_state.is_in_combat() {
        // Check flee threshold when engaged
        if matches!(combat_state, CombatState::Engaged { .. }) {
            if let Some(health) = world
                .query_one::<&Health>(entity)
                .ok()
                .and_then(|mut q| q.get().cloned())
            {
                if (health.current as f32 / health.max as f32) < 0.25 {
                    try_flee(world, entity);
                    return AiState::Flee { attempts: 0 };
                }
            }
        }
        return state;
    }

    // Return: check if home is reached
    if let AiState::Return { home } = &state {
        let current_room = world
            .query_one::<&Position>(entity)
            .ok()
            .and_then(|mut q| q.get().map(|p| p.room));
        if current_room == Some(*home) {
            return AiState::Idle;
        }
        return tick_return_home(world, entity, *home);
    }

    // Normal behavior states
    match state {
        AiState::Idle => tick_idle(world, entity, npc),
        AiState::Wander { counter } => tick_wander(world, entity, npc, counter),
        AiState::Patrol {
            counter,
            index,
            forward,
        } => tick_patrol(world, entity, npc, counter, index, forward),
        AiState::Aggro { hunt_target } => tick_aggro(world, entity, npc, hunt_target),
        AiState::Combat | AiState::Flee { .. } | AiState::Return { .. } => {
            unreachable!("handled above")
        }
    }
}

/// Idle: check for aggro targets, occasionally transition to Wander.
fn tick_idle(world: &mut World, entity: Entity, npc: &Npc) -> AiState {
    if check_aggro(world, entity, npc) {
        return AiState::Combat;
    }
    match npc.ai_mode.as_str() {
        "wander" => {
            // Small chance to start wandering
            if fastrand::u8(..100) < 10 {
                AiState::Wander { counter: 0 }
            } else {
                AiState::Idle
            }
        }
        "patrol" => AiState::Patrol {
            counter: 0,
            index: 0,
            forward: true,
        },
        "aggro" | "aggressive" => AiState::Aggro { hunt_target: None },
        _ => AiState::Idle,
    }
}

/// Wander: move every WANDER_PULSE_THRESHOLD pulses, check aggro.
fn tick_wander(world: &mut World, entity: Entity, npc: &Npc, counter: u8) -> AiState {
    let new_counter = counter.wrapping_add(1);
    if new_counter >= WANDER_PULSE_THRESHOLD {
        let bounds = world
            .query_one::<&WanderBounds>(entity)
            .ok()
            .and_then(|mut q| q.get().cloned());
        do_wander_move(world, entity, bounds.as_ref());
    }
    if check_aggro(world, entity, npc) {
        return AiState::Combat;
    }
    // Occasional timer back to Idle
    if new_counter >= WANDER_PULSE_THRESHOLD && fastrand::u8(..100) < 15 {
        AiState::Idle
    } else {
        AiState::Wander {
            counter: if new_counter >= WANDER_PULSE_THRESHOLD {
                0
            } else {
                new_counter
            },
        }
    }
}

/// Patrol: follow a defined route every 6 pulses.
/// Loops if start and end waypoints are the same, ping-pongs otherwise.
/// Falls back to random wander if no PatrolRoute component is attached.
fn tick_patrol(
    world: &mut World,
    entity: Entity,
    npc: &Npc,
    counter: u8,
    index: usize,
    forward: bool,
) -> AiState {
    let new_counter = counter.wrapping_add(1);
    if new_counter >= 6 {
        // Read the patrol route if present
        let route = world
            .query_one::<&PatrolRoute>(entity)
            .ok()
            .and_then(|mut q| q.get().map(|r| r.0.clone()))
            .filter(|wps| !wps.is_empty());

        if let Some(route) = route {
            {
                let len = route.len();
                if index < len {
                    let _ = world.insert(entity, (Position::new(route[index]),));
                }

                let is_loop = len >= 2 && route[0] == route[len - 1];
                let (next_index, next_forward) = if is_loop {
                    ((index + 1) % len, true)
                } else if forward {
                    if index + 1 >= len {
                        (index.saturating_sub(1), false)
                    } else {
                        (index + 1, true)
                    }
                } else if index == 0 {
                    (1, true)
                } else {
                    (index - 1, false)
                };

                if check_aggro(world, entity, npc) {
                    return AiState::Combat;
                }
                return AiState::Patrol {
                    counter: 0,
                    index: next_index,
                    forward: next_forward,
                };
            }
        }
        // No route — fall back to random wander (respect bounds if set)
        let bounds = world
            .query_one::<&WanderBounds>(entity)
            .ok()
            .and_then(|mut q| q.get().cloned());
        do_wander_move(world, entity, bounds.as_ref());
    }
    if check_aggro(world, entity, npc) {
        return AiState::Combat;
    }
    AiState::Patrol {
        counter: if new_counter >= 6 { 0 } else { new_counter },
        index,
        forward,
    }
}

/// Aggro: actively hunt for targets.
fn tick_aggro(
    world: &mut World,
    entity: Entity,
    npc: &Npc,
    _hunt_target: Option<Entity>,
) -> AiState {
    if check_aggro(world, entity, npc) {
        return AiState::Combat;
    }
    AiState::Aggro { hunt_target: None }
}

/// Return home: move randomly toward home (no pathfinding yet).
fn tick_return_home(world: &mut World, entity: Entity, home: Entity) -> AiState {
    let bounds = world
        .query_one::<&WanderBounds>(entity)
        .ok()
        .and_then(|mut q| q.get().cloned());
    do_wander_move(world, entity, bounds.as_ref());
    AiState::Return { home }
}

/// Move to a random visible room exit, optionally filtered by wander bounds.
fn do_wander_move(world: &mut World, entity: Entity, bounds: Option<&WanderBounds>) {
    let room = match world
        .query_one::<&Position>(entity)
        .ok()
        .and_then(|mut q| q.get().map(|p| p.room))
    {
        Some(r) => r,
        None => return,
    };

    let exits = match world
        .query_one::<&RoomExits>(room)
        .ok()
        .and_then(|mut q| q.get().map(|e| e.0.clone()))
    {
        Some(e) => e,
        None => return,
    };

    let visible: Vec<_> = exits
        .iter()
        .filter(|e| {
            if e.is_hidden() {
                return false;
            }
            if let Some(b) = bounds {
                b.0.contains(&e.dest)
            } else {
                true
            }
        })
        .collect();
    if !visible.is_empty() {
        let idx = fastrand::usize(..visible.len());
        let dest = visible[idx].dest;
        let _ = world.insert(entity, (Position::new(dest),));
    }
}

/// Check for aggro targets in the same room.
/// Returns true and transitions CombatState if a target is found.
fn check_aggro(world: &mut World, entity: Entity, npc: &Npc) -> bool {
    let room = match world
        .query_one::<&Position>(entity)
        .ok()
        .and_then(|mut q| q.get().map(|p| p.room))
    {
        Some(r) => r,
        None => return false,
    };

    let targets: Vec<Entity> = {
        let mut q = world.query::<(&Position, &Health)>();
        q.iter()
            .map(|(raw, (pos, _))| (crate::Entity::from(raw), pos))
            .filter(|(e, pos)| pos.room == room && *e != entity)
            .map(|(e, _)| e)
            .collect()
    };

    for target in targets {
        // Aggro unfriendly NPCs
        if npc.aggro_mobs
            && world
                .query_one::<&Npc>(target)
                .is_ok_and(|mut q| q.get().is_some())
            && !world
                .query_one::<&Friendly>(target)
                .is_ok_and(|mut q| q.get().is_some())
        {
            let stance = crate::systems::stance::get_active_stance(world, entity);
            crate::systems::combat::transition_combat_state(
                world,
                entity,
                CombatState::Engaged {
                    target,
                    round_started: std::time::Instant::now(),
                    stance,
                },
            );
            return true;
        }

        // Aggro weak players
        if npc.aggro_players
            && world
                .query_one::<&crate::Player>(target)
                .is_ok_and(|mut q| q.get().is_some())
        {
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
                let stance = crate::systems::stance::get_active_stance(world, entity);
                crate::systems::combat::transition_combat_state(
                    world,
                    entity,
                    CombatState::Engaged {
                        target,
                        round_started: std::time::Instant::now(),
                        stance,
                    },
                );
                return true;
            }
        }
    }

    false
}

/// Attempt to flee from current combat target.
fn try_flee(world: &mut World, entity: Entity) {
    let combat_state = world
        .query_one::<&CombatState>(entity)
        .ok()
        .and_then(|mut q| q.get().cloned())
        .unwrap_or(CombatState::NotInCombat);

    if let CombatState::Engaged { target, .. } = combat_state {
        crate::systems::combat::transition_combat_state(
            world,
            entity,
            CombatState::Fleeing {
                target,
                attempts: 0,
            },
        );
    }
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
            AiState::Idle,
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

    #[test]
    fn test_ai_wander_may_move() {
        let mut world = World::new();
        let room_a = world.spawn(());
        let room_b = world.spawn(());
        // Link room_a -> room_b
        world
            .insert(
                room_a,
                (RoomExits(vec![crate::Exit::new(
                    crate::Direction::North,
                    room_b,
                )]),),
            )
            .unwrap();
        let npc = world.spawn((
            Position::new(room_a),
            Health::new(50),
            Npc::new("test_wanderer").with_ai_mode("wander"),
            Level(1),
            Name::new("Wanderer"),
            AiState::Wander {
                counter: WANDER_PULSE_THRESHOLD - 1,
            },
        ));
        // Run multiple pulses to increase chance of movement
        for _ in 0..10 {
            run_ai_pulse(&mut world);
        }
        let pos = world
            .query_one::<&Position>(npc)
            .unwrap()
            .get()
            .unwrap()
            .room;
        // May have moved to room_b or stayed — just verify no crash
        assert!(pos == room_a || pos == room_b);
    }

    #[test]
    fn test_ai_aggro_transitions_to_combat() {
        let mut world = World::new();
        let room = world.spawn(());
        let player = world.spawn((
            Position::new(room),
            Health::new(100),
            Level(1),
            crate::Player::new(1),
            Name::new("Player"),
        ));
        let mob = world.spawn((
            Position::new(room),
            Health::new(50),
            Npc::new_with_aggro("aggro_mob", 5, true, true, vec![]),
            Level(1),
            Name::new("Aggro Mob"),
            AiState::Aggro { hunt_target: None },
        ));

        run_ai_pulse(&mut world);

        // Mob should now be in Combat AI state
        let ai_state = world
            .query_one::<&AiState>(mob)
            .unwrap()
            .get()
            .cloned()
            .unwrap();
        assert!(matches!(ai_state, AiState::Combat));

        // CombatState should be Engaged against player
        let combat_state = world
            .query_one::<&CombatState>(mob)
            .unwrap()
            .get()
            .cloned()
            .unwrap();
        assert!(matches!(combat_state, CombatState::Engaged { target, .. } if target == player));
    }

    #[test]
    fn test_ai_combat_ends_returns_home() {
        let mut world = World::new();
        let room = world.spawn(());
        let mob = world.spawn((
            Position::new(room),
            Health::new(50),
            Npc::new("test_mob"),
            Level(1),
            Name::new("Mob"),
            AiState::Combat,
        ));
        // CombatState is NotInCombat — combat already ended

        run_ai_pulse(&mut world);

        let ai_state = world
            .query_one::<&AiState>(mob)
            .unwrap()
            .get()
            .cloned()
            .unwrap();
        assert!(matches!(ai_state, AiState::Return { .. }));
    }

    #[test]
    fn test_ai_flee_ends_idle() {
        let mut world = World::new();
        let room = world.spawn(());
        let mob = world.spawn((
            Position::new(room),
            Health::new(50),
            Npc::new("test_mob"),
            Level(1),
            Name::new("Mob"),
            AiState::Flee { attempts: 1 },
        ));

        run_ai_pulse(&mut world);

        let ai_state = world
            .query_one::<&AiState>(mob)
            .unwrap()
            .get()
            .cloned()
            .unwrap();
        assert!(matches!(ai_state, AiState::Idle));
    }

    #[test]
    fn test_ai_patrol_loop() {
        let mut world = World::new();
        let room_a = world.spawn(());
        let room_b = world.spawn(());
        let room_c = world.spawn(());
        let mob = world.spawn((
            Position::new(room_a),
            Health::new(50),
            Npc::new("patrol_mob").with_ai_mode("patrol"),
            Level(1),
            Name::new("Patrol"),
            PatrolRoute(vec![room_a, room_b, room_c, room_a]),
            AiState::Patrol {
                counter: 5,
                index: 0,
                forward: true,
            },
        ));

        // Moves every 6 pulses (counter wraps at 6). Run 6 pulses per move.
        // Pulse 1: counter wraps, moves to room_a (index 0), index → 1
        // Pulses 2-6: counter builds, no move
        for _ in 0..6 {
            run_ai_pulse(&mut world);
        }
        let pos = world
            .query_one::<&Position>(mob)
            .unwrap()
            .get()
            .unwrap()
            .room;
        assert_eq!(pos, room_a);
        let state = world
            .query_one::<&AiState>(mob)
            .unwrap()
            .get()
            .cloned()
            .unwrap();
        assert!(matches!(state, AiState::Patrol { index: 1, .. }));

        // Next 6 pulses: move to room_b (index 1), index → 2
        for _ in 0..6 {
            run_ai_pulse(&mut world);
        }
        let pos = world
            .query_one::<&Position>(mob)
            .unwrap()
            .get()
            .unwrap()
            .room;
        assert_eq!(pos, room_b);
        let state = world
            .query_one::<&AiState>(mob)
            .unwrap()
            .get()
            .cloned()
            .unwrap();
        assert!(matches!(state, AiState::Patrol { index: 2, .. }));

        // Next 6 pulses: move to room_c (index 2), index → 3
        for _ in 0..6 {
            run_ai_pulse(&mut world);
        }
        let pos = world
            .query_one::<&Position>(mob)
            .unwrap()
            .get()
            .unwrap()
            .room;
        assert_eq!(pos, room_c);
        let state = world
            .query_one::<&AiState>(mob)
            .unwrap()
            .get()
            .cloned()
            .unwrap();
        assert!(matches!(state, AiState::Patrol { index: 3, .. }));

        // Next 6 pulses: move to room_a (index 3, wraps to 0 — loop)
        for _ in 0..6 {
            run_ai_pulse(&mut world);
        }
        let pos = world
            .query_one::<&Position>(mob)
            .unwrap()
            .get()
            .unwrap()
            .room;
        assert_eq!(pos, room_a);
        let state = world
            .query_one::<&AiState>(mob)
            .unwrap()
            .get()
            .cloned()
            .unwrap();
        assert!(matches!(state, AiState::Patrol { index: 0, .. }));
    }

    #[test]
    fn test_ai_patrol_ping_pong() {
        let mut world = World::new();
        let room_a = world.spawn(());
        let room_b = world.spawn(());
        let room_c = world.spawn(());
        let mob = world.spawn((
            Position::new(room_a),
            Health::new(50),
            Npc::new("patrol_mob").with_ai_mode("patrol"),
            Level(1),
            Name::new("Patrol"),
            PatrolRoute(vec![room_a, room_b, room_c]),
            AiState::Patrol {
                counter: 5,
                index: 0,
                forward: true,
            },
        ));

        // 6 pulses: move to room_a (index 0), index → 1, forward=true
        for _ in 0..6 {
            run_ai_pulse(&mut world);
        }
        let pos = world
            .query_one::<&Position>(mob)
            .unwrap()
            .get()
            .unwrap()
            .room;
        assert_eq!(pos, room_a);

        // 6 pulses: move to room_b (index 1), index → 2, forward=true
        for _ in 0..6 {
            run_ai_pulse(&mut world);
        }
        let pos = world
            .query_one::<&Position>(mob)
            .unwrap()
            .get()
            .unwrap()
            .room;
        assert_eq!(pos, room_b);

        // 6 pulses: move to room_c (index 2), reverses: index → 1, forward=false
        for _ in 0..6 {
            run_ai_pulse(&mut world);
        }
        let pos = world
            .query_one::<&Position>(mob)
            .unwrap()
            .get()
            .unwrap()
            .room;
        assert_eq!(pos, room_c);
        let state = world
            .query_one::<&AiState>(mob)
            .unwrap()
            .get()
            .cloned()
            .unwrap();
        assert!(matches!(
            state,
            AiState::Patrol {
                index: 1,
                forward: false,
                ..
            }
        ));

        // 6 pulses: move to room_b (index 1, forward=false), index → 0
        for _ in 0..6 {
            run_ai_pulse(&mut world);
        }
        let pos = world
            .query_one::<&Position>(mob)
            .unwrap()
            .get()
            .unwrap()
            .room;
        assert_eq!(pos, room_b);

        // 6 pulses: move to room_a (index 0, forward=false), reverses: index → 1, forward=true
        for _ in 0..6 {
            run_ai_pulse(&mut world);
        }
        let pos = world
            .query_one::<&Position>(mob)
            .unwrap()
            .get()
            .unwrap()
            .room;
        assert_eq!(pos, room_a);
        let state = world
            .query_one::<&AiState>(mob)
            .unwrap()
            .get()
            .cloned()
            .unwrap();
        assert!(matches!(
            state,
            AiState::Patrol {
                index: 1,
                forward: true,
                ..
            }
        ));
    }

    #[test]
    fn test_ai_patrol_no_route_fallback() {
        let mut world = World::new();
        let room_a = world.spawn(());
        let room_b = world.spawn(());
        world
            .insert(
                room_a,
                (RoomExits(vec![crate::Exit::new(
                    crate::Direction::North,
                    room_b,
                )]),),
            )
            .unwrap();
        let mob = world.spawn((
            Position::new(room_a),
            Health::new(50),
            Npc::new("patrol_mob").with_ai_mode("patrol"),
            Level(1),
            Name::new("Patrol"),
            // No PatrolRoute component attached
            AiState::Patrol {
                counter: 5,
                index: 0,
                forward: true,
            },
        ));

        // Should fall back to random wander (no crash, may move)
        for _ in 0..5 {
            run_ai_pulse(&mut world);
        }
        // Just verify no crash and AiState is still Patrol
        let state = world
            .query_one::<&AiState>(mob)
            .unwrap()
            .get()
            .cloned()
            .unwrap();
        assert!(matches!(state, AiState::Patrol { .. }));
    }

    #[test]
    fn test_ai_wander_bounds() {
        let mut world = World::new();
        let room_a = world.spawn(());
        let room_b = world.spawn(());
        let room_c = world.spawn(());
        // Link room_a → room_b and room_a → room_c
        world
            .insert(
                room_a,
                (RoomExits(vec![
                    crate::Exit::new(crate::Direction::North, room_b),
                    crate::Exit::new(crate::Direction::South, room_c),
                ]),),
            )
            .unwrap();
        // Bounds only allow room_b
        let mob = world.spawn((
            Position::new(room_a),
            Health::new(50),
            Npc::new("wander_mob").with_ai_mode("wander"),
            Level(1),
            Name::new("Wanderer"),
            WanderBounds(vec![room_b]),
            AiState::Wander {
                counter: WANDER_PULSE_THRESHOLD - 1,
            },
        ));

        for _ in 0..20 {
            run_ai_pulse(&mut world);
            let pos = world
                .query_one::<&Position>(mob)
                .unwrap()
                .get()
                .unwrap()
                .room;
            // Must always be in room_a or room_b (never room_c)
            assert!(
                pos == room_a || pos == room_b,
                "Mob left bounds: was in room that is not room_a or room_b"
            );
        }
    }
}
