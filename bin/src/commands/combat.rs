use oxide_core as core;
use oxide_core::{get_pos_room, AccessLevel, World};
use oxide_server::{Command, CommandHelp, Connection, ConnectionRegistry, Server};

pub const HELP_STANCE: &str = "Usage: stance [normal|defensive|aggressive|berserk]\n  normal      Balanced offense and defense\n  defensive   Less damage, reduced offense\n  aggressive  More damage, reduced defense\n  berserk     Maximum offense, minimum defense";

pub const HELP_GROUP: &str = "Usage: group [status|invite <player>|accept|leave|disband|kick <player>|loot <mode>|formation <type>|leader <player>]\n  loot modes: freeforall, roundrobin, master\n  formations: default, line, scattered, column, wedge, shieldwall";

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
    server.register_command(Command {
        name: "group",
        aliases: &[],
        access: AccessLevel::Player,
        topic: "Combat",
        help: CommandHelp {
            short: "Manage party and formations",
            body: Some(HELP_GROUP),
        },
        handler: cmd_group,
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

pub fn cmd_group(
    world: &mut World,
    conn: &mut dyn Connection,
    _name: &str,
    args: &str,
    registry: &ConnectionRegistry,
) {
    let entity = match conn.entity() {
        Some(e) => e,
        None => {
            conn.send_line("You have no form.");
            return;
        }
    };

    let trimmed = args.trim();
    let parts: Vec<&str> = trimmed.split_whitespace().collect();

    if parts.is_empty() || parts[0].eq_ignore_ascii_case("status") {
        let gm = match world.query_one::<&core::GroupMember>(entity) {
            Ok(mut q) => q.get().copied(),
            Err(_) => None,
        };

        let group_entity = match gm {
            Some(m) => m.group_id,
            None => {
                conn.send_line("You are not in a group.");
                return;
            }
        };

        if let Ok(mut q_group) = world.query_one::<&core::Group>(group_entity) {
            if let Some(group) = q_group.get() {
                conn.send_line("--------------------------------------------------");
                conn.send_line("Group Status");
                conn.send_line(&format!("  Loot Mode: {:?}", group.loot_mode));
                conn.send_line(&format!("  Formation: {:?}", group.formation));
                conn.send_line("Members:");

                for m in &group.members {
                    let role_str = match Some(group.leader) == m.entity {
                        true => " [Leader]",
                        false => "",
                    };

                    if let Some(m_ent) = m.entity {
                        let hp_str = if let Ok(mut q_hp) = world.query_one::<&core::Health>(m_ent) {
                            q_hp.get()
                                .map(|h| format!("HP: {}/{}", h.current, h.max))
                                .unwrap_or_default()
                        } else {
                            "".to_string()
                        };

                        let mn_str = if let Ok(mut q_mn) = world.query_one::<&core::Mana>(m_ent) {
                            q_mn.get()
                                .map(|m| format!("Mana: {}/{}", m.current, m.max))
                                .unwrap_or_default()
                        } else {
                            "".to_string()
                        };

                        let st_str = if let Ok(mut q_st) = world.query_one::<&core::Stamina>(m_ent)
                        {
                            q_st.get()
                                .map(|s| format!("Stamina: {}/{}", s.current, s.max))
                                .unwrap_or_default()
                        } else {
                            "".to_string()
                        };

                        conn.send_line(&format!(
                            "  * {}{} - {}, {}, {}",
                            m.name, role_str, hp_str, mn_str, st_str
                        ));
                    } else {
                        conn.send_line(&format!("  * {} (Offline)", m.name));
                    }
                }
                conn.send_line("--------------------------------------------------");
            }
        }
        return;
    }

    let subcmd = parts[0].to_lowercase();
    match subcmd.as_str() {
        "invite" => {
            if parts.len() < 2 {
                conn.send_line("Usage: group invite <player>");
                return;
            }
            let target_name = parts[1];

            let mut target_entity = None;
            for (ent, (n_comp, _player_comp)) in
                world.query::<(&core::Name, &core::Player)>().iter()
            {
                if n_comp.as_str().eq_ignore_ascii_case(target_name) {
                    target_entity = Some(ent);
                    break;
                }
            }

            let target = match target_entity {
                Some(e) => e,
                None => {
                    conn.send_line("No player by that name is online.");
                    return;
                }
            };

            match core::handle_group_invite(world, entity, target_name) {
                Ok(msg) => {
                    conn.send_line(&msg);

                    let inviter_name = world
                        .query_one::<&core::Name>(entity)
                        .ok()
                        .and_then(|mut q| q.get().map(|n| n.as_str().to_string()))
                        .unwrap_or_else(|| "Someone".to_string());

                    if let Some(target_tx) = registry.sender(target) {
                        let _ = target_tx.send(
                            format!(
                            "{} invites you to join their group. Type 'group accept' to join.\r\n",
                            inviter_name
                        )
                            .into_bytes(),
                        );
                    }
                }
                Err(err) => {
                    conn.send_line(&err);
                }
            }
        }
        "accept" => match core::handle_group_accept(world, entity) {
            Ok((_inviter, group_entity, invitee_name)) => {
                conn.send_line("You join the group.");

                if let Ok(mut q_group) = world.query_one::<&core::Group>(group_entity) {
                    if let Some(group) = q_group.get() {
                        for m in &group.members {
                            if let Some(m_ent) = m.entity {
                                if m_ent != entity {
                                    if let Some(tx) = registry.sender(m_ent) {
                                        let _ = tx.send(
                                            format!("{} has joined the group.\r\n", invitee_name)
                                                .into_bytes(),
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
            }
            Err(err) => {
                conn.send_line(&err);
            }
        },
        "leave" => {
            let my_name = world
                .query_one::<&core::Name>(entity)
                .ok()
                .and_then(|mut q| q.get().map(|n| n.as_str().to_string()))
                .unwrap_or_else(|| "Someone".to_string());

            match core::handle_group_leave(world, entity) {
                Ok((_group_entity, remaining_active, leave_msg)) => {
                    conn.send_line(&leave_msg);

                    for member in remaining_active {
                        if let Some(tx) = registry.sender(member) {
                            let _ = tx
                                .send(format!("{} has left the group.\r\n", my_name).into_bytes());
                        }
                    }
                }
                Err(err) => {
                    conn.send_line(&err);
                }
            }
        }
        "disband" => match core::handle_group_disband(world, entity) {
            Ok((active_members, msg)) => {
                conn.send_line(&msg);
                for member in active_members {
                    if let Some(tx) = registry.sender(member) {
                        let _ = tx.send(format!("{}\r\n", msg).into_bytes());
                    }
                }
            }
            Err(err) => {
                conn.send_line(&err);
            }
        },
        "kick" => {
            if parts.len() < 2 {
                conn.send_line("Usage: group kick <player>");
                return;
            }
            let target_name = parts[1];

            match core::handle_group_kick(world, entity, target_name) {
                Ok((group_entity, kicked_entity, msg)) => {
                    conn.send_line(&msg);

                    if let Some(tx) = registry.sender(kicked_entity) {
                        let _ = tx.send(b"You have been kicked from the group.\r\n".to_vec());
                    }

                    if let Ok(mut q_group) = world.query_one::<&core::Group>(group_entity) {
                        if let Some(group) = q_group.get() {
                            for m in &group.members {
                                if let Some(m_ent) = m.entity {
                                    if let Some(tx) = registry.sender(m_ent) {
                                        let _ = tx.send(
                                            format!(
                                                "{} has been kicked from the group.\r\n",
                                                target_name
                                            )
                                            .into_bytes(),
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
                Err(err) => {
                    conn.send_line(&err);
                }
            }
        }
        "loot" => {
            if parts.len() < 2 {
                conn.send_line("Usage: group loot <freeforall|roundrobin|master>");
                return;
            }
            let mode_str = parts[1];

            match core::handle_group_loot(world, entity, mode_str) {
                Ok(mode) => {
                    let msg = format!("Loot mode changed to {:?}.\r\n", mode);

                    if let Ok(mut q_gm) = world.query_one::<&core::GroupMember>(entity) {
                        if let Some(gm) = q_gm.get() {
                            if let Ok(mut q_group) = world.query_one::<&core::Group>(gm.group_id) {
                                if let Some(group) = q_group.get() {
                                    for m in &group.members {
                                        if let Some(m_ent) = m.entity {
                                            if let Some(tx) = registry.sender(m_ent) {
                                                let _ = tx.send(msg.clone().into_bytes());
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                Err(err) => {
                    conn.send_line(&err);
                }
            }
        }
        "formation" => {
            if parts.len() < 2 {
                conn.send_line(
                    "Usage: group formation <default|line|scattered|column|wedge|shieldwall>",
                );
                return;
            }
            let form_str = parts[1];

            match core::handle_group_formation(world, entity, form_str) {
                Ok(formation) => {
                    let msg = format!("Formation changed to {:?}.\r\n", formation);

                    if let Ok(mut q_gm) = world.query_one::<&core::GroupMember>(entity) {
                        if let Some(gm) = q_gm.get() {
                            if let Ok(mut q_group) = world.query_one::<&core::Group>(gm.group_id) {
                                if let Some(group) = q_group.get() {
                                    for m in &group.members {
                                        if let Some(m_ent) = m.entity {
                                            if let Some(tx) = registry.sender(m_ent) {
                                                let _ = tx.send(msg.clone().into_bytes());
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                Err(err) => {
                    conn.send_line(&err);
                }
            }
        }
        "leader" => {
            if parts.len() < 2 {
                conn.send_line("Usage: group leader <player>");
                return;
            }
            let target_name = parts[1];

            match core::handle_group_leader(world, entity, target_name) {
                Ok((new_leader_entity, msg)) => {
                    conn.send_line(&msg);

                    if let Ok(mut q_gm) = world.query_one::<&core::GroupMember>(new_leader_entity) {
                        if let Some(gm) = q_gm.get() {
                            if let Ok(mut q_group) = world.query_one::<&core::Group>(gm.group_id) {
                                if let Some(group) = q_group.get() {
                                    for m in &group.members {
                                        if let Some(m_ent) = m.entity {
                                            if m_ent != entity {
                                                if let Some(tx) = registry.sender(m_ent) {
                                                    let _ = tx.send(
                                                        format!(
                                                            "{} is now the group leader.\r\n",
                                                            target_name
                                                        )
                                                        .into_bytes(),
                                                    );
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                Err(err) => {
                    conn.send_line(&err);
                }
            }
        }
        "say" | "tell" => {
            if parts.len() < 2 {
                conn.send_line("Usage: group say <message>");
                return;
            }
            let msg_text = parts[1..].join(" ");

            let gm = match world.query_one::<&core::GroupMember>(entity) {
                Ok(mut q) => q.get().copied(),
                Err(_) => None,
            };

            let group_entity = match gm {
                Some(m) => m.group_id,
                None => {
                    conn.send_line("You are not in a group.");
                    return;
                }
            };

            let my_name = world
                .query_one::<&core::Name>(entity)
                .ok()
                .and_then(|mut q| q.get().map(|n| n.as_str().to_string()))
                .unwrap_or_else(|| "Someone".to_string());

            let formatted = format!("[Group] {}: {}\r\n", my_name, msg_text);

            if let Ok(mut q_group) = world.query_one::<&core::Group>(group_entity) {
                if let Some(group) = q_group.get() {
                    for m in &group.members {
                        if let Some(m_ent) = m.entity {
                            if let Some(tx) = registry.sender(m_ent) {
                                let _ = tx.send(formatted.clone().into_bytes());
                            }
                        }
                    }
                }
            }
        }
        _ => {
            conn.send_line("Invalid group subcommand. Type 'help group' for help.");
        }
    }
}
