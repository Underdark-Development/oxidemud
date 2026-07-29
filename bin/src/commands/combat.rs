use oxide_core as core;
use oxide_core::{get_pos_room, AccessLevel, World};
use oxide_server::{Command, CommandHelp, Connection, ConnectionRegistry, Server};

pub const HELP_STANCE: &str = "Usage: stance [normal|defensive|aggressive|berserk]\n  normal      Balanced offense and defense\n  defensive   Less damage, reduced offense\n  aggressive  More damage, reduced defense\n  berserk     Maximum offense, minimum defense";

pub fn register(server: &mut Server) {
    server.register_command(Command {
        name: "kill",
        aliases: &[],
        access: AccessLevel::Player,
        topic: "Combat",
        help: CommandHelp {
            short: "Attack a target",
            body: None,
        },
        handler: cmd_kill,
    });
    server.register_command(Command {
        name: "flee",
        aliases: &[],
        access: AccessLevel::Player,
        topic: "Combat",
        help: CommandHelp {
            short: "Attempt to flee from combat",
            body: None,
        },
        handler: cmd_flee,
    });
    server.register_command(Command {
        name: "stance",
        aliases: &[],
        access: AccessLevel::Player,
        topic: "Combat",
        help: CommandHelp {
            short: "View or change combat stance",
            body: Some(HELP_STANCE),
        },
        handler: cmd_stance,
    });
}

pub fn cmd_kill(
    world: &mut World,
    conn: &mut dyn Connection,
    _name: &str,
    args: &str,
    _registry: &ConnectionRegistry,
) {
    let entity = match conn.entity() {
        Some(e) => e,
        None => {
            conn.send_line("You have no form.");
            return;
        }
    };

    if let Ok(mut q) = world.query_one::<&core::PlayerState>(entity) {
        if let Some(state) = q.get() {
            match state {
                core::PlayerState::Dead => {
                    conn.send_line("You are a ghost! You cannot attack anything.");
                    return;
                }
                core::PlayerState::Resting(rest) => match rest {
                    core::RestState::Standing | core::RestState::Sitting => {}
                    core::RestState::Resting => {
                        conn.send_line("You cannot attack while resting.");
                        return;
                    }
                    core::RestState::Sleeping => {
                        conn.send_line("You cannot attack while sleeping.");
                        return;
                    }
                    core::RestState::Unconscious => {
                        conn.send_line("You cannot attack while unconscious.");
                        return;
                    }
                    core::RestState::Dead => {
                        conn.send_line("You are a ghost! You cannot attack anything.");
                        return;
                    }
                },
                core::PlayerState::Stunned { .. } => {
                    conn.send_line("You are stunned and cannot attack.");
                    return;
                }
                core::PlayerState::Casting { .. } => {
                    conn.send_line("You are busy casting a spell.");
                    return;
                }
            }
        }
    }

    if args.trim().is_empty() {
        conn.send_line("Kill what?");
        return;
    }

    let room = match get_pos_room(world, entity) {
        Some(r) => r,
        None => {
            conn.send_line("You are nowhere.");
            return;
        }
    };

    let target = {
        let mut q = world.query::<(&core::Name, &core::Position, &core::Health)>();
        let candidates: Vec<(String, core::Entity)> = q
            .iter()
            .filter(|(raw, (_, pos, _))| {
                let e = *raw;
                pos.room == room && e != entity
            })
            .map(|(raw, (name, _, _))| (name.as_str().to_lowercase(), raw))
            .collect();
        match core::trie::trie_match(args.trim(), candidates) {
            core::trie::TrieMatch::One(e) => Some(e),
            core::trie::TrieMatch::Many(items) => items.into_iter().next(),
            core::trie::TrieMatch::None => None,
        }
    };

    let target = match target {
        Some(t) => t,
        None => {
            conn.send_line("They aren't here.");
            return;
        }
    };

    if let Ok(mut q) = world.query_one::<&core::Health>(target) {
        if q.get().is_some_and(|h| h.is_dead()) {
            conn.send_line("They are already dead.");
            return;
        }
    }

    if let Ok(mut q) = world.query_one::<&core::Player>(target) {
        if q.get().is_some() {
            conn.send_line("You cannot attack other players yet.");
            return;
        }
    }

    let attacker_stance = core::systems::stance::get_active_stance(world, entity);
    core::systems::combat::transition_combat_state(
        world,
        entity,
        core::CombatState::Engaged {
            target,
            round_started: std::time::Instant::now(),
            stance: attacker_stance,
        },
    );
    if world
        .query_one::<&core::Npc>(target)
        .is_ok_and(|mut q| q.get().is_some())
        && !world
            .query_one::<&core::Friendly>(target)
            .is_ok_and(|mut q| q.get().is_some())
    {
        let target_stance = core::systems::stance::get_active_stance(world, target);
        core::systems::combat::transition_combat_state(
            world,
            target,
            core::CombatState::Engaged {
                target: entity,
                round_started: std::time::Instant::now(),
                stance: target_stance,
            },
        );
    }
    conn.send_line("You attack!");
}

pub fn cmd_flee(
    world: &mut World,
    conn: &mut dyn Connection,
    _name: &str,
    _args: &str,
    _registry: &ConnectionRegistry,
) {
    let entity = match conn.entity() {
        Some(e) => e,
        None => {
            conn.send_line("You have no form.");
            return;
        }
    };

    let combat_state = world
        .query_one::<&core::CombatState>(entity)
        .ok()
        .and_then(|mut q| q.get().cloned())
        .unwrap_or(core::CombatState::NotInCombat);

    let target = match combat_state {
        core::CombatState::Engaged { target, .. } => target,
        core::CombatState::Fleeing { .. } => {
            conn.send_line("You are already trying to flee!");
            return;
        }
        core::CombatState::NotInCombat => {
            conn.send_line("You aren't in combat.");
            return;
        }
    };

    let room = match get_pos_room(world, entity) {
        Some(r) => r,
        None => {
            conn.send_line("You are nowhere.");
            return;
        }
    };

    let exits_exist = match world.query_one::<&core::RoomExits>(room) {
        Ok(mut q) => q.get().is_some_and(|e| {
            let visible_exits: Vec<_> = e.0.iter().filter(|x| !x.is_hidden()).collect();
            !visible_exits.is_empty()
        }),
        Err(_) => false,
    };

    if !exits_exist {
        conn.send_line("There is nowhere to flee!");
        return;
    }

    let new_state = core::CombatState::Fleeing {
        target,
        attempts: 0,
    };
    core::systems::combat::transition_combat_state(world, entity, new_state);
    conn.send_line("You attempt to flee from combat!");
}

pub fn cmd_stance(
    world: &mut World,
    conn: &mut dyn Connection,
    _name: &str,
    args: &str,
    _registry: &ConnectionRegistry,
) {
    let entity = match conn.entity() {
        Some(e) => e,
        None => {
            conn.send_line("You have no form.");
            return;
        }
    };

    let stance_name = args.trim().to_lowercase();
    if stance_name.is_empty() {
        let current = world
            .query_one::<&core::ActiveStance>(entity)
            .ok()
            .and_then(|mut q| q.get().and_then(|s| s.0.clone()))
            .unwrap_or_else(|| "normal".to_string());
        conn.send_line(&format!("Your current stance is: {current}"));
        conn.send_line("Available: normal, defensive, aggressive, berserk");
        return;
    }

    let valid = ["normal", "defensive", "aggressive", "berserk"];
    if !valid.contains(&stance_name.as_str()) {
        conn.send_line("Unknown stance. Available: normal, defensive, aggressive, berserk");
        return;
    }

    let new_stance = if stance_name == "normal" {
        None
    } else {
        Some(stance_name.clone())
    };

    let _ = world.insert(entity, (core::ActiveStance(new_stance),));
    conn.send_line(&format!("You adopt a {stance_name} stance."));
}
