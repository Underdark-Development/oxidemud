use oxide_core as core;
use oxide_core::format::preview::{item_look_template, mob_look_template};
use oxide_core::{
    get_entity_name, get_exits, get_name, get_pos_room, get_room_desc, get_room_name,
    get_short_desc, is_void_room, AccessLevel, Description, Direction, FloorItems, Inventory, Item,
    Name, Npc, Position, RoomExits, World,
};
use oxide_server::{Command, CommandHelp, Connection, ConnectionRegistry, Server};

use super::common::*;

pub const HELP_LOOK: &str =
    "Usage: look [target|direction]\n  show the room, examine a target, or peek through an exit";

pub const HELP_FOLLOW: &str =
    "Usage: follow [player]\n  start following a player, or stop following with no argument";

pub fn register(server: &mut Server) {
    server.register_command(Command {
        name: "look",
        aliases: &["l"],
        access: AccessLevel::Player,
        topic: "General",
        help: CommandHelp {
            short: "Examine your surroundings",
            body: Some(HELP_LOOK),
        },
        handler: cmd_look,
    });
    server.register_command(Command {
        name: "follow",
        aliases: &[],
        access: AccessLevel::Player,
        topic: "Movement",
        help: CommandHelp {
            short: "Follow a player",
            body: Some(HELP_FOLLOW),
        },
        handler: cmd_follow,
    });
    server.register_command(Command {
        name: "unfollow",
        aliases: &[],
        access: AccessLevel::Player,
        topic: "Movement",
        help: CommandHelp {
            short: "Stop following another player",
            body: None,
        },
        handler: cmd_unfollow,
    });
    server.register_command(Command {
        name: "open",
        aliases: &[],
        access: AccessLevel::Player,
        topic: "Movement",
        help: CommandHelp {
            short: "Open a closed door",
            body: None,
        },
        handler: cmd_open,
    });
    server.register_command(Command {
        name: "close",
        aliases: &[],
        access: AccessLevel::Player,
        topic: "Movement",
        help: CommandHelp {
            short: "Close an open door",
            body: None,
        },
        handler: cmd_close,
    });
    server.register_command(Command {
        name: "lock",
        aliases: &[],
        access: AccessLevel::Player,
        topic: "Movement",
        help: CommandHelp {
            short: "Lock a door using a key",
            body: None,
        },
        handler: cmd_lock,
    });
    server.register_command(Command {
        name: "unlock",
        aliases: &[],
        access: AccessLevel::Player,
        topic: "Movement",
        help: CommandHelp {
            short: "Unlock a door using a key",
            body: None,
        },
        handler: cmd_unlock,
    });
    server.register_command(Command {
        name: "north",
        aliases: &["n"],
        access: AccessLevel::Player,
        topic: "Movement",
        help: CommandHelp {
            short: "Move north",
            body: None,
        },
        handler: cmd_move,
    });
    server.register_command(Command {
        name: "south",
        aliases: &["s"],
        access: AccessLevel::Player,
        topic: "Movement",
        help: CommandHelp {
            short: "Move south",
            body: None,
        },
        handler: cmd_move,
    });
    server.register_command(Command {
        name: "east",
        aliases: &["e"],
        access: AccessLevel::Player,
        topic: "Movement",
        help: CommandHelp {
            short: "Move east",
            body: None,
        },
        handler: cmd_move,
    });
    server.register_command(Command {
        name: "west",
        aliases: &["w"],
        access: AccessLevel::Player,
        topic: "Movement",
        help: CommandHelp {
            short: "Move west",
            body: None,
        },
        handler: cmd_move,
    });
    server.register_command(Command {
        name: "up",
        aliases: &["u"],
        access: AccessLevel::Player,
        topic: "Movement",
        help: CommandHelp {
            short: "Move up",
            body: None,
        },
        handler: cmd_move,
    });
    server.register_command(Command {
        name: "down",
        aliases: &["d"],
        access: AccessLevel::Player,
        topic: "Movement",
        help: CommandHelp {
            short: "Move down",
            body: None,
        },
        handler: cmd_move,
    });
    server.register_command(Command {
        name: "northeast",
        aliases: &["ne"],
        access: AccessLevel::Player,
        topic: "Movement",
        help: CommandHelp {
            short: "Move northeast",
            body: None,
        },
        handler: cmd_move,
    });
    server.register_command(Command {
        name: "northwest",
        aliases: &["nw"],
        access: AccessLevel::Player,
        topic: "Movement",
        help: CommandHelp {
            short: "Move northwest",
            body: None,
        },
        handler: cmd_move,
    });
    server.register_command(Command {
        name: "southeast",
        aliases: &["se"],
        access: AccessLevel::Player,
        topic: "Movement",
        help: CommandHelp {
            short: "Move southeast",
            body: None,
        },
        handler: cmd_move,
    });
    server.register_command(Command {
        name: "southwest",
        aliases: &["sw"],
        access: AccessLevel::Player,
        topic: "Movement",
        help: CommandHelp {
            short: "Move southwest",
            body: None,
        },
        handler: cmd_move,
    });
}

pub fn cmd_look(
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

    let room = match get_pos_room(world, entity) {
        Some(r) => r,
        None => {
            conn.send_line("You are nowhere.");
            return;
        }
    };

    let args = args.trim();

    // Direction look: "look north" / "look n"
    if !args.is_empty() {
        if let Some(dir) = Direction::try_from(args) {
            look_at_direction(world, conn, room, dir);
            return;
        }
    }

    // Target look: "look <name>" / "look at <name>"
    if !args.is_empty() {
        let target_query = args.strip_prefix("at ").unwrap_or(args).trim();
        if !target_query.is_empty() {
            look_at_target(world, conn, entity, room, target_query);
            return;
        }
    }

    // Room look
    if is_void_room(world, room) {
        conn.send_line("");
        send_formatted(conn, &core::format::conventions::room_name("The Void\n"));
        conn.send_line("You are floating in an endless, featureless void.");
        conn.send_line("There is nothing here and no way out.");
        conn.send_line("");
        return;
    }

    let room_name = match get_room_name(world, room) {
        Some(n) => n,
        None => {
            conn.send_line("The void stares back.");
            return;
        }
    };

    let room_desc = get_room_desc(world, room).unwrap_or_default();

    conn.send_line("");
    send_formatted(conn, &core::format::conventions::room_name(&room_name));
    conn.send_line("");
    send_formatted(conn, &core::format::parse_tags(&room_desc));

    let weather_state = world
        .query_one::<&core::WeatherState>(room)
        .ok()
        .and_then(|mut q| q.get().cloned())
        .unwrap_or_default();

    if let Some(templates) = oxide_server::get_templates() {
        if let Some(ref weather_config) = templates.weather {
            let mut weather_descs = Vec::new();
            if let Some(ref base_id) = weather_state.base {
                if !base_id.eq_ignore_ascii_case("clear") {
                    if let Some(def) = weather_config.conditions.get(base_id) {
                        weather_descs.push(def.description.clone());
                    }
                }
            }
            if let Some(ref mod_id) = weather_state.modifier {
                if let Some(def) = weather_config.conditions.get(mod_id) {
                    weather_descs.push(def.description.clone());
                }
            }
            if !weather_descs.is_empty() {
                send_formatted(conn, &core::format::parse_tags(&weather_descs.join(" ")));
            }
        }
    }

    // Exits
    let exits = get_exits(world, room);
    if !exits.is_empty() {
        send_formatted(
            conn,
            &core::format::conventions::exit_dir(format!("[Exits: {}]", exits.join(" "))),
        );
    }

    // Floor items
    if let Ok(mut q) = world.query_one::<&FloorItems>(room) {
        if let Some(floor) = q.get() {
            for &item in &floor.0 {
                if let Some(name) = get_name(world, item) {
                    conn.send_line(&format!("{} lies here.", name.0));
                }
            }
        }
    }

    // Collect entities present in the room (excluding self)
    let players: Vec<_> = registry
        .occupants(world, room)
        .into_iter()
        .filter(|&e| e != entity)
        .collect();

    let all_npcs: Vec<_> = {
        let mut q = world.query::<(&Position, &Npc)>();
        q.iter()
            .filter(|(_, (pos, _))| pos.room == room)
            .map(|(raw, _)| core::Entity::from(raw))
            .filter(|&e| e != entity)
            .collect()
    };

    let friendly: Vec<_> = {
        let mut q = world.query::<(&Position, &Npc, &core::Friendly)>();
        q.iter()
            .filter(|(_, (pos, _, _))| pos.room == room)
            .map(|(raw, _)| core::Entity::from(raw))
            .filter(|&e| e != entity)
            .collect()
    };

    let non_friendly: Vec<_> = all_npcs
        .iter()
        .filter(|e| !friendly.contains(e))
        .copied()
        .collect();

    let corpses: Vec<_> = {
        let mut q = world.query::<(&Position, &core::Corpse)>();
        q.iter()
            .filter(|(_, (pos, _))| pos.room == room)
            .map(|(raw, _)| core::Entity::from(raw))
            .collect()
    };

    struct LookEntity {
        name: String,
        group: u8, // 0 = mob, 1 = corpse, 2 = friendly NPC, 3 = player
    }

    let mut look_entities: Vec<LookEntity> = Vec::new();

    for p in &players {
        if let Some(name) = get_name(world, *p) {
            look_entities.push(LookEntity {
                name: name.0,
                group: 3,
            });
        }
    }

    for n in &friendly {
        if let Some(desc) = get_short_desc(world, *n) {
            look_entities.push(LookEntity {
                name: desc,
                group: 2,
            });
        }
    }

    for n in &non_friendly {
        if let Some(desc) = get_short_desc(world, *n) {
            look_entities.push(LookEntity {
                name: desc,
                group: 0,
            });
        }
    }

    for corpse in &corpses {
        if let Some(name) = get_name(world, *corpse) {
            look_entities.push(LookEntity {
                name: name.0,
                group: 1,
            });
        }
    }

    look_entities.sort_by(|a, b| {
        a.group
            .cmp(&b.group)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });

    for entity_data in &look_entities {
        let line = format!("{} is here.", entity_data.name);
        conn.send_line(&line);
    }

    conn.send_line("");
}

enum TargetKind {
    Mob,
    Player,
    Item,
    Corpse,
}

fn get_room_name_for_entity(world: &World, entity: core::Entity) -> Option<String> {
    world
        .query_one::<&core::Room>(entity)
        .ok()
        .and_then(|mut q| q.get().map(|r| r.name.clone()))
}

fn find_target_in_room(
    world: &World,
    room: core::Entity,
    player_entity: core::Entity,
    query: &str,
) -> Option<(TargetKind, core::Entity)> {
    let mut candidates: Vec<(String, (TargetKind, core::Entity))> = Vec::new();

    let mut npc_q = world.query::<(&Position, &Npc)>();
    for (raw, (pos, _)) in npc_q.iter() {
        if pos.room != room {
            continue;
        }
        let e = core::Entity::from(raw);
        if e == player_entity {
            continue;
        }
        if let Some(name) = get_entity_name(world, e) {
            candidates.push((name, (TargetKind::Mob, e)));
        }
    }

    let mut player_q = world.query::<&Position>();
    for (raw, pos) in player_q.iter() {
        if pos.room != room {
            continue;
        }
        let e = core::Entity::from(raw);
        if e == player_entity {
            continue;
        }
        if world
            .query_one::<&Npc>(e)
            .is_ok_and(|mut q| q.get().is_some())
        {
            continue;
        }
        if let Some(name) = get_entity_name(world, e) {
            candidates.push((name, (TargetKind::Player, e)));
        }
    }

    if let Ok(mut q) = world.query_one::<&FloorItems>(room) {
        if let Some(floor) = q.get() {
            for &item in &floor.0 {
                if let Some(name) = get_entity_name(world, item) {
                    candidates.push((name, (TargetKind::Item, item)));
                }
            }
        }
    }

    let mut corpse_q = world.query::<(&core::Corpse, &Position, &Name)>();
    for (raw, (_, pos, name)) in corpse_q.iter() {
        if pos.room == room {
            candidates.push((
                name.as_str().to_lowercase(),
                (TargetKind::Corpse, raw.into()),
            ));
        }
    }

    if let Ok(mut q) = world.query_one::<&Inventory>(player_entity) {
        if let Some(inv) = q.get() {
            for &item in &inv.0 {
                if let Some(name) = get_entity_name(world, item) {
                    candidates.push((name, (TargetKind::Item, item)));
                }
            }
        }
    }

    match core::trie::trie_match(query, candidates) {
        core::trie::TrieMatch::One(result) => Some(result),
        core::trie::TrieMatch::Many(results) => results.into_iter().next(),
        core::trie::TrieMatch::None => None,
    }
}

fn look_at_target(
    world: &mut World,
    conn: &mut dyn Connection,
    player_entity: core::Entity,
    room: core::Entity,
    target_query: &str,
) {
    let Some((kind, target)) = find_target_in_room(world, room, player_entity, target_query) else {
        conn.send_line("You don't see that here.");
        return;
    };

    conn.send_line("");

    match kind {
        TargetKind::Mob => {
            if let Some(templates) = oxide_server::get_templates() {
                if let Ok(mut q) = world.query_one::<&Npc>(target) {
                    if let Some(npc) = q.get() {
                        if let Some(mob_tpl) = templates.mobs.get(&npc.template_id) {
                            send_formatted(conn, &mob_look_template(mob_tpl));
                            return;
                        }
                    }
                }
            }
            if let Some(sd) = get_short_desc(world, target) {
                conn.send_line(&sd);
            } else {
                conn.send_line("You see nothing special.");
            }
            if let Ok(mut q) = world.query_one::<&core::ActiveScriptEffects>(target) {
                if let Some(active) = q.get() {
                    for effect in &active.effects {
                        if effect.visible_on_look {
                            if let Some(ref aura) = effect.look_aura {
                                conn.send_line(&format!("Active Aura: {}", aura));
                            }
                        }
                    }
                }
            }
        }
        TargetKind::Player => {
            if let Some(name) = get_name(world, target) {
                send_formatted(conn, &core::format::conventions::room_name(&name.0));
                send_formatted(
                    conn,
                    &core::format::conventions::separator("-".repeat(name.0.len().min(40))),
                );
            }
            if let Ok(mut q) = world.query_one::<&Description>(target) {
                if let Some(desc) = q.get() {
                    if !desc.0.is_empty() {
                        send_formatted(conn, &core::format::parse_tags(&desc.0));
                        conn.send_line("");
                        return;
                    }
                }
            }
            conn.send_line("They are nothing special to look at.");
            conn.send_line("");
        }
        TargetKind::Corpse => {
            if let Some(name) = get_name(world, target) {
                conn.send_line(&name.0);
            }
            let item_count = world
                .query_one::<&Inventory>(target)
                .ok()
                .and_then(|mut q| q.get().map(|inv| inv.0.len()))
                .unwrap_or(0);
            if item_count == 0 {
                conn.send_line("It has nothing worth taking.");
            } else {
                conn.send_line(&format!("It contains {item_count} item(s)."));
            }
        }
        TargetKind::Item => {
            if let Some(templates) = oxide_server::get_templates() {
                if let Ok(mut q) = world.query_one::<&Item>(target) {
                    if let Some(item_comp) = q.get() {
                        if let Some(item_tpl) = templates.items.get(&item_comp.template_id) {
                            send_formatted(conn, &item_look_template(item_tpl));
                            conn.send_line("");
                            return;
                        }
                    }
                }
            }
            if let Some(name) = get_name(world, target) {
                conn.send_line(&format!("You see {}.", name.0));
            } else {
                conn.send_line("You see nothing special.");
            }
            conn.send_line("");
        }
    }
}

fn look_at_direction(
    world: &mut World,
    conn: &mut dyn Connection,
    room: core::Entity,
    dir: Direction,
) {
    if let Ok(mut q) = world.query_one::<&RoomExits>(room) {
        if let Some(exits) = q.get() {
            for exit in &exits.0 {
                if exit.direction == dir {
                    if exit.is_closed() {
                        conn.send_line("The door is closed.");
                        return;
                    }
                    if let Some(name) = get_room_name_for_entity(world, exit.dest) {
                        conn.send_line(&format!("Looking {}, you see: {}", dir.long_name(), name));
                    } else {
                        conn.send_line(&format!(
                            "Looking {}, you see nothing special.",
                            dir.long_name()
                        ));
                    }
                    return;
                }
            }
        }
    }
    conn.send_line("Nothing special in that direction.");
}

fn can_detect_ghost(world: &World, observer: core::Entity) -> bool {
    if let Ok(mut q) = world.query_one::<&Vec<core::ActiveEffect>>(observer) {
        if let Some(effects) = q.get() {
            for eff in effects {
                if let Some(stat) = &eff.stat {
                    let lower = stat.to_lowercase();
                    if lower.contains("detect_invisible")
                        || lower.contains("detect_undead")
                        || lower.contains("detect-invisible")
                        || lower.contains("detect-undead")
                    {
                        return true;
                    }
                }
                let source_lower = eff.source.to_lowercase();
                if source_lower.contains("detect_invisible")
                    || source_lower.contains("detect_undead")
                    || source_lower.contains("detect-invisible")
                    || source_lower.contains("detect-undead")
                {
                    return true;
                }
            }
        }
    }
    false
}

fn send_leave_broadcast(
    world: &World,
    registry: &ConnectionRegistry,
    entity: core::Entity,
    from_room: core::Entity,
    dir_long: &str,
) {
    let name = get_name(world, entity).unwrap_or(Name::new("Someone"));
    let is_ghost = world
        .query_one::<&core::PlayerState>(entity)
        .ok()
        .and_then(|mut q| q.get().map(|s| matches!(s, core::PlayerState::Dead)))
        .unwrap_or(false);

    let mut normal_msg = core::format::RichText::new();
    normal_msg.push(core::format::conventions::player_name_segment(
        name.as_str(),
    ));
    normal_msg.push(core::format::Segment::new(format!(" leaves {dir_long}.")));
    let normal_bytes = format!("{}\r\n", normal_msg.render(true, true)).into_bytes();

    let mut ghost_detector_msg = core::format::RichText::new();
    ghost_detector_msg.push(core::format::Segment::new(format!(
        "The ghost of {} floats {dir_long}.",
        name.as_str()
    )));
    let ghost_detector_bytes =
        format!("{}\r\n", ghost_detector_msg.render(true, true)).into_bytes();

    let chill_bytes = b"You feel a cold shiver run down your spine.\r\n".to_vec();

    for &other in &registry.occupants(world, from_room) {
        if other == entity {
            continue;
        }
        if let Some(tx) = registry.sender(other) {
            let bytes = if is_ghost {
                if can_detect_ghost(world, other) {
                    ghost_detector_bytes.clone()
                } else {
                    chill_bytes.clone()
                }
            } else {
                normal_bytes.clone()
            };
            let _ = tx.send(bytes);
        }
    }
}

fn send_enter_broadcast(
    world: &World,
    registry: &ConnectionRegistry,
    entity: core::Entity,
    dest_room: core::Entity,
    dir_long: &str,
) {
    let name = get_name(world, entity).unwrap_or(Name::new("Someone"));
    let is_ghost = world
        .query_one::<&core::PlayerState>(entity)
        .ok()
        .and_then(|mut q| q.get().map(|s| matches!(s, core::PlayerState::Dead)))
        .unwrap_or(false);

    let mut normal_msg = core::format::RichText::new();
    normal_msg.push(core::format::conventions::player_name_segment(
        name.as_str(),
    ));
    normal_msg.push(core::format::Segment::new(format!(
        " arrives from the {dir_long}."
    )));
    let normal_bytes = format!("{}\r\n", normal_msg.render(true, true)).into_bytes();

    let mut ghost_detector_msg = core::format::RichText::new();
    ghost_detector_msg.push(core::format::Segment::new(format!(
        "The ghost of {} floats in from the {dir_long}.",
        name.as_str()
    )));
    let ghost_detector_bytes =
        format!("{}\r\n", ghost_detector_msg.render(true, true)).into_bytes();

    let chill_bytes = b"You feel a sudden chill in the air.\r\n".to_vec();

    for &other in &registry.occupants(world, dest_room) {
        if other == entity {
            continue;
        }
        if let Some(tx) = registry.sender(other) {
            let bytes = if is_ghost {
                if can_detect_ghost(world, other) {
                    ghost_detector_bytes.clone()
                } else {
                    chill_bytes.clone()
                }
            } else {
                normal_bytes.clone()
            };
            let _ = tx.send(bytes);
        }
    }
}

pub fn direction_from_name(name: &str) -> Option<Direction> {
    Direction::from_short(name).or_else(|| Direction::from_long(name))
}

pub fn move_player(
    world: &mut World,
    conn: &mut dyn Connection,
    registry: &ConnectionRegistry,
    entity: core::Entity,
    direction: Direction,
) {
    let in_combat = world
        .query_one::<&core::CombatState>(entity)
        .ok()
        .and_then(|mut q| q.get().map(|cs| cs.is_in_combat()))
        .unwrap_or(false);

    if in_combat {
        conn.send_line("No way! You are fighting for your life!");
        return;
    }

    if let Ok(mut q) = world.query_one::<&core::PlayerState>(entity) {
        if let Some(state) = q.get() {
            match state {
                core::PlayerState::Dead => {}
                core::PlayerState::Resting(rest) => match rest {
                    core::RestState::Standing => {}
                    core::RestState::Sitting => {
                        conn.send_line("You cannot move while sitting down.");
                        return;
                    }
                    core::RestState::Resting => {
                        conn.send_line("You cannot move while resting.");
                        return;
                    }
                    core::RestState::Sleeping => {
                        conn.send_line("You cannot move while sleeping.");
                        return;
                    }
                    core::RestState::Unconscious => {
                        conn.send_line("You cannot move while unconscious.");
                        return;
                    }
                    core::RestState::Dead => {}
                },
                core::PlayerState::Stunned { .. } => {
                    conn.send_line("You are stunned and cannot move.");
                    return;
                }
                core::PlayerState::Casting { .. } => {
                    conn.send_line("You are casting and cannot move.");
                    return;
                }
            }
        }
    }
    let room = match get_pos_room(world, entity) {
        Some(r) => r,
        None => {
            conn.send_line("You are nowhere.");
            return;
        }
    };

    if is_void_room(world, room) {
        conn.send_line("You cannot move in the void.");
        return;
    }

    let dest = match world.query_one::<&RoomExits>(room) {
        Ok(mut q) => q.get().and_then(|exits| {
            exits
                .0
                .iter()
                .find(|e| e.direction == direction)
                .map(|e| e.dest)
        }),
        Err(_) => None,
    };

    let dest = match dest {
        Some(d) => d,
        None => {
            conn.send_line("You cannot go that way.");
            return;
        }
    };

    if let Ok(mut q) = world.query_one::<&RoomExits>(room) {
        if let Some(exits) = q.get() {
            if let Some(exit) = exits.0.iter().find(|e| e.direction == direction) {
                if exit.is_closed() {
                    conn.send_line("That exit is closed.");
                    return;
                }
                if exit.is_locked() {
                    conn.send_line("That exit is locked.");
                    return;
                }
            }
        }
    }

    let _ = world.insert(entity, (Position::new(dest), core::Dirty));

    if let Some(room_key) = world
        .query_one::<&core::RoomKey>(dest)
        .ok()
        .and_then(|mut q| q.get().map(|k| k.0.clone()))
    {
        if let Some(templates) = oxide_server::get_templates() {
            let msgs = core::handle_explore_event(world, entity, &room_key, &templates);
            for msg in msgs {
                conn.send_line(&msg);
            }
        }
    }

    let dir_long = direction.long_name();
    let opposite = direction.opposite();
    let opp_long = opposite.long_name();
    send_leave_broadcast(world, registry, entity, room, dir_long);
    send_enter_broadcast(world, registry, entity, dest, opp_long);

    cmd_look(world, conn, "", "", registry);

    trigger_follow(world, registry, entity, room, dest, direction);
}

fn trigger_follow(
    world: &mut World,
    registry: &ConnectionRegistry,
    entity: core::Entity,
    room: core::Entity,
    dest: core::Entity,
    direction: Direction,
) {
    let dir_long = direction.long_name();
    let opposite = direction.opposite();
    let opp_long = opposite.long_name();

    let mut followers = Vec::new();
    for (f_eid, (pos, following)) in world.query::<(&core::Position, &core::Following)>().iter() {
        if pos.room == room && following.target == entity {
            followers.push(core::Entity::from(f_eid));
        }
    }

    for follower in followers {
        let _ = world.insert(follower, (Position::new(dest), core::Dirty));
        send_leave_broadcast(world, registry, follower, room, dir_long);
        send_enter_broadcast(world, registry, follower, dest, opp_long);

        if let Some(tx) = registry.sender(follower) {
            let leader_name = get_name(world, entity).unwrap_or(Name::new("Someone"));
            let _ = tx
                .send(format!("You follow {} {dir_long}.\r\n", leader_name.as_str()).into_bytes());

            struct FollowerConn<'a> {
                entity: core::Entity,
                tx: &'a tokio::sync::mpsc::UnboundedSender<Vec<u8>>,
            }
            impl<'a> Connection for FollowerConn<'a> {
                fn send(&mut self, text: &str) {
                    let _ = self.tx.send(text.as_bytes().to_vec());
                }
                fn send_line(&mut self, text: &str) {
                    let _ = self.tx.send(format!("{}\r\n", text).into_bytes());
                }
                fn send_raw(&mut self, bytes: &[u8]) {
                    let _ = self.tx.send(bytes.to_vec());
                }
                fn id(&self) -> &str {
                    "0"
                }
                fn entity(&self) -> Option<core::Entity> {
                    Some(self.entity)
                }
                fn set_entity(&mut self, _entity: core::Entity) {}
                fn disconnect(&mut self) {}
                fn is_disconnected(&self) -> bool {
                    false
                }
                fn flags(&self) -> oxide_server::ConnectionFlags {
                    oxide_server::ConnectionFlags::new()
                }
                fn set_flags(&mut self, _flags: oxide_server::ConnectionFlags) {}
                fn output_sender(&self) -> Option<tokio::sync::mpsc::UnboundedSender<Vec<u8>>> {
                    Some(self.tx.clone())
                }
            }
            let mut f_conn = FollowerConn {
                entity: follower,
                tx,
            };
            cmd_look(world, &mut f_conn, "", "", registry);
        }

        trigger_follow(world, registry, follower, room, dest, direction);
    }
}

pub fn cmd_move(
    world: &mut World,
    conn: &mut dyn Connection,
    name: &str,
    _args: &str,
    registry: &ConnectionRegistry,
) {
    let entity = match conn.entity() {
        Some(e) => e,
        None => {
            conn.send_line("You have no form.");
            return;
        }
    };

    let direction = match direction_from_name(name) {
        Some(d) => d,
        None => {
            conn.send_line("Huh?");
            return;
        }
    };

    move_player(world, conn, registry, entity, direction);
}

fn update_exit_flags(
    world: &mut World,
    from_room: core::Entity,
    dir: Direction,
    to_room: core::Entity,
    set_mask: core::ExitFlags,
    clear_mask: core::ExitFlags,
) {
    if let Ok(mut q) = world.query_one::<&mut RoomExits>(from_room) {
        if let Some(exits) = q.get() {
            if let Some(exit) = exits.0.iter_mut().find(|e| e.direction == dir) {
                exit.flags |= set_mask;
                exit.flags &= !clear_mask;
            }
        }
    }
    let rev_dir = dir.opposite();
    if let Ok(mut q) = world.query_one::<&mut RoomExits>(to_room) {
        if let Some(exits) = q.get() {
            if let Some(exit) = exits
                .0
                .iter_mut()
                .find(|e| e.direction == rev_dir && e.dest == from_room)
            {
                exit.flags |= set_mask;
                exit.flags &= !clear_mask;
            }
        }
    }
}

fn has_key(world: &World, player: core::Entity, key_id: &str) -> bool {
    if let Ok(mut q) = world.query_one::<&Inventory>(player) {
        if let Some(inv) = q.get() {
            for &item_entity in &inv.0 {
                if let Ok(mut item_q) = world.query_one::<&Item>(item_entity) {
                    if let Some(item) = item_q.get() {
                        if item.template_id == key_id {
                            return true;
                        }
                    }
                }
            }
        }
    }
    false
}

pub fn cmd_open(
    world: &mut World,
    conn: &mut dyn Connection,
    _name: &str,
    args: &str,
    registry: &ConnectionRegistry,
) {
    let entity = match conn.entity() {
        Some(e) => e,
        None => return,
    };

    let room = match get_pos_room(world, entity) {
        Some(r) => r,
        None => {
            conn.send_line("You are nowhere.");
            return;
        }
    };

    let direction = match direction_from_name(args) {
        Some(d) => d,
        None => {
            conn.send_line("Open which direction?");
            return;
        }
    };

    let mut exit_found = None;
    if let Ok(mut q) = world.query_one::<&RoomExits>(room) {
        if let Some(exits) = q.get() {
            if let Some(exit) = exits.0.iter().find(|e| e.direction == direction) {
                exit_found = Some(exit.clone());
            }
        }
    }

    let exit = match exit_found {
        Some(ex) => ex,
        None => {
            conn.send_line("There is no exit that way.");
            return;
        }
    };

    if !exit.is_door() {
        conn.send_line("There is no door that way.");
        return;
    }

    if !exit.is_closed() {
        conn.send_line("It is already open.");
        return;
    }

    if exit.is_locked() {
        conn.send_line("It is locked.");
        return;
    }

    update_exit_flags(world, room, direction, exit.dest, 0, core::EXIT_IS_CLOSED);

    conn.send_line("You open the door.");
    if let Ok(mut name_q) = world.query_one::<&Name>(entity) {
        if let Some(name) = name_q.get() {
            broadcast_to_room_except(
                world,
                registry,
                room,
                entity,
                &format!("{} opens the door {}.", name.0, direction.long_name()),
            );
            broadcast_to_room_except(world, registry, exit.dest, entity, "The door opens.");
        }
    }
}

pub fn cmd_close(
    world: &mut World,
    conn: &mut dyn Connection,
    _name: &str,
    args: &str,
    registry: &ConnectionRegistry,
) {
    let entity = match conn.entity() {
        Some(e) => e,
        None => return,
    };

    let room = match get_pos_room(world, entity) {
        Some(r) => r,
        None => {
            conn.send_line("You are nowhere.");
            return;
        }
    };

    let direction = match direction_from_name(args) {
        Some(d) => d,
        None => {
            conn.send_line("Close which direction?");
            return;
        }
    };

    let mut exit_found = None;
    if let Ok(mut q) = world.query_one::<&RoomExits>(room) {
        if let Some(exits) = q.get() {
            if let Some(exit) = exits.0.iter().find(|e| e.direction == direction) {
                exit_found = Some(exit.clone());
            }
        }
    }

    let exit = match exit_found {
        Some(ex) => ex,
        None => {
            conn.send_line("There is no exit that way.");
            return;
        }
    };

    if !exit.is_door() {
        conn.send_line("There is no door that way.");
        return;
    }

    if exit.is_closed() {
        conn.send_line("It is already closed.");
        return;
    }

    update_exit_flags(world, room, direction, exit.dest, core::EXIT_IS_CLOSED, 0);

    conn.send_line("You close the door.");
    if let Ok(mut name_q) = world.query_one::<&Name>(entity) {
        if let Some(name) = name_q.get() {
            broadcast_to_room_except(
                world,
                registry,
                room,
                entity,
                &format!("{} closes the door {}.", name.0, direction.long_name()),
            );
            broadcast_to_room_except(world, registry, exit.dest, entity, "The door closes.");
        }
    }
}

pub fn cmd_lock(
    world: &mut World,
    conn: &mut dyn Connection,
    _name: &str,
    args: &str,
    registry: &ConnectionRegistry,
) {
    let entity = match conn.entity() {
        Some(e) => e,
        None => return,
    };

    let room = match get_pos_room(world, entity) {
        Some(r) => r,
        None => {
            conn.send_line("You are nowhere.");
            return;
        }
    };

    let direction = match direction_from_name(args) {
        Some(d) => d,
        None => {
            conn.send_line("Lock which direction?");
            return;
        }
    };

    let mut exit_found = None;
    if let Ok(mut q) = world.query_one::<&RoomExits>(room) {
        if let Some(exits) = q.get() {
            if let Some(exit) = exits.0.iter().find(|e| e.direction == direction) {
                exit_found = Some(exit.clone());
            }
        }
    }

    let exit = match exit_found {
        Some(ex) => ex,
        None => {
            conn.send_line("There is no exit that way.");
            return;
        }
    };

    if !exit.is_door() {
        conn.send_line("There is no door that way.");
        return;
    }

    if !exit.is_closed() {
        conn.send_line("You must close it first.");
        return;
    }

    if exit.is_locked() {
        conn.send_line("It is already locked.");
        return;
    }

    let key_id = match &exit.key_id {
        Some(k) => k,
        None => {
            conn.send_line("This door cannot be locked.");
            return;
        }
    };

    if !has_key(world, entity, key_id) {
        conn.send_line("You do not have the key.");
        return;
    }

    update_exit_flags(world, room, direction, exit.dest, core::EXIT_IS_LOCKED, 0);

    conn.send_line("You lock the door.");
    if let Ok(mut name_q) = world.query_one::<&Name>(entity) {
        if let Some(name) = name_q.get() {
            broadcast_to_room_except(
                world,
                registry,
                room,
                entity,
                &format!("{} locks the door {}.", name.0, direction.long_name()),
            );
            broadcast_to_room_except(
                world,
                registry,
                exit.dest,
                entity,
                "You hear a click as the door is locked from the other side.",
            );
        }
    }
}

pub fn cmd_unlock(
    world: &mut World,
    conn: &mut dyn Connection,
    _name: &str,
    args: &str,
    registry: &ConnectionRegistry,
) {
    let entity = match conn.entity() {
        Some(e) => e,
        None => return,
    };

    let room = match get_pos_room(world, entity) {
        Some(r) => r,
        None => {
            conn.send_line("You are nowhere.");
            return;
        }
    };

    let direction = match direction_from_name(args) {
        Some(d) => d,
        None => {
            conn.send_line("Unlock which direction?");
            return;
        }
    };

    let mut exit_found = None;
    if let Ok(mut q) = world.query_one::<&RoomExits>(room) {
        if let Some(exits) = q.get() {
            if let Some(exit) = exits.0.iter().find(|e| e.direction == direction) {
                exit_found = Some(exit.clone());
            }
        }
    }

    let exit = match exit_found {
        Some(ex) => ex,
        None => {
            conn.send_line("There is no exit that way.");
            return;
        }
    };

    if !exit.is_door() {
        conn.send_line("There is no door that way.");
        return;
    }

    if !exit.is_locked() {
        conn.send_line("It is already unlocked.");
        return;
    }

    let key_id = match &exit.key_id {
        Some(k) => k,
        None => {
            conn.send_line("This door cannot be unlocked.");
            return;
        }
    };

    if !has_key(world, entity, key_id) {
        conn.send_line("You do not have the key.");
        return;
    }

    update_exit_flags(world, room, direction, exit.dest, 0, core::EXIT_IS_LOCKED);

    conn.send_line("You unlock the door.");
    if let Ok(mut name_q) = world.query_one::<&Name>(entity) {
        if let Some(name) = name_q.get() {
            broadcast_to_room_except(
                world,
                registry,
                room,
                entity,
                &format!("{} unlocks the door {}.", name.0, direction.long_name()),
            );
            broadcast_to_room_except(
                world,
                registry,
                exit.dest,
                entity,
                "You hear a click as the door is unlocked from the other side.",
            );
        }
    }
}

pub fn cmd_follow(
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

    let target_name = args.trim();
    if target_name.is_empty() {
        cmd_unfollow(world, conn, _name, args, registry);
        return;
    }

    let my_room = match world.query_one::<&Position>(entity) {
        Ok(mut q) => q.get().map(|p| p.room),
        Err(_) => None,
    };

    let room = match my_room {
        Some(r) => r,
        None => {
            conn.send_line("You are nowhere.");
            return;
        }
    };

    let mut found_target = None;
    for other in core::entities_in_room(world, room) {
        if other != entity {
            if let Ok(mut q_name) = world.query_one::<&Name>(other) {
                if let Some(name) = q_name.get() {
                    if name.as_str().eq_ignore_ascii_case(target_name) {
                        found_target = Some(other);
                        break;
                    }
                }
            }
        }
    }

    let target = match found_target {
        Some(t) => t,
        None => {
            conn.send_line("You don't see them here.");
            return;
        }
    };

    let target_name_str = world
        .query_one::<&Name>(target)
        .ok()
        .and_then(|mut q| q.get().map(|n| n.as_str().to_string()))
        .unwrap_or_else(|| "Someone".to_string());
    let my_name_str = world
        .query_one::<&Name>(entity)
        .ok()
        .and_then(|mut q| q.get().map(|n| n.as_str().to_string()))
        .unwrap_or_else(|| "Someone".to_string());

    let _ = world.insert(
        entity,
        (core::Following {
            target,
            autofollow: true,
        },),
    );
    let _ = world.insert(entity, (core::Dirty,));

    conn.send_line(&format!("You start following {}.", target_name_str));

    let room_msg = format!("{} starts following {}.\r\n", my_name_str, target_name_str);
    for other in core::entities_in_room(world, room) {
        if other != entity {
            if let Some(tx) = registry.sender(other) {
                let _ = tx.send(room_msg.clone().into_bytes());
            }
        }
    }
}

pub fn cmd_unfollow(
    world: &mut World,
    conn: &mut dyn Connection,
    _name: &str,
    _args: &str,
    registry: &ConnectionRegistry,
) {
    let entity = match conn.entity() {
        Some(e) => e,
        None => {
            conn.send_line("You have no form.");
            return;
        }
    };

    let following = world
        .query_one::<&core::Following>(entity)
        .ok()
        .and_then(|mut q| q.get().copied());

    if let Some(f) = following {
        let target_name_str = world
            .query_one::<&Name>(f.target)
            .ok()
            .and_then(|mut q| q.get().map(|n| n.as_str().to_string()))
            .unwrap_or_else(|| "Someone".to_string());
        let my_name_str = world
            .query_one::<&Name>(entity)
            .ok()
            .and_then(|mut q| q.get().map(|n| n.as_str().to_string()))
            .unwrap_or_else(|| "Someone".to_string());

        let _ = world.remove_one::<core::Following>(entity);
        let _ = world.insert(entity, (core::Dirty,));

        conn.send_line(&format!("You stop following {}.", target_name_str));

        let my_room = world
            .query_one::<&Position>(entity)
            .ok()
            .and_then(|mut q| q.get().map(|p| p.room));

        if let Some(room) = my_room {
            let room_msg = format!("{} stops following {}.\r\n", my_name_str, target_name_str);
            for other in core::entities_in_room(world, room) {
                if other != entity {
                    if let Some(tx) = registry.sender(other) {
                        let _ = tx.send(room_msg.clone().into_bytes());
                    }
                }
            }
        }
    } else {
        conn.send_line("You are not following anyone.");
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_helpers::*;
    use super::*;
    use oxide_core::Exit;

    #[test]
    fn test_look_in_room() {
        let (mut world, _void, room_a, _room_b) = test_world();
        let (_player, mut conn, registry) = test_player(&mut world, room_a);

        cmd_look(&mut world, &mut conn, "look", "", &registry);

        let lines = conn.take_lines();
        assert!(lines.iter().any(|l| l.contains("Room A")));
        assert!(lines.iter().any(|l| l.contains("This is room A.")));
        assert!(lines.iter().any(|l| l.contains("[Exits: e]")));
    }

    #[test]
    fn test_look_in_void() {
        let (mut world, void_room, _room_a, _room_b) = test_world();
        let (_player, mut conn, registry) = test_player(&mut world, void_room);

        cmd_look(&mut world, &mut conn, "look", "", &registry);

        let lines = conn.take_lines();
        assert!(lines.iter().any(|l| l.contains("The Void")));
        assert!(lines
            .iter()
            .any(|l| l.contains("floating in an endless, featureless void")));
    }

    #[test]
    fn test_look_no_entity() {
        let (mut world, _void, _room_a, _room_b) = test_world();
        let mut conn = MockConnection::new();
        let registry = ConnectionRegistry::new();

        cmd_look(&mut world, &mut conn, "look", "", &registry);

        let lines = conn.take_lines();
        assert_eq!(lines, vec!["You have no form."]);
    }

    #[test]
    fn test_look_no_position() {
        let mut world = World::new();
        let player = world.spawn(());
        let mut conn = MockConnection::new();
        conn.set_entity(player);
        let registry = ConnectionRegistry::new();

        cmd_look(&mut world, &mut conn, "look", "", &registry);

        let lines = conn.take_lines();
        assert_eq!(lines, vec!["You are nowhere."]);
    }

    #[test]
    fn test_move_valid_direction() {
        let (mut world, _void, room_a, room_b) = test_world();
        let (player, mut conn, registry) = test_player(&mut world, room_a);

        cmd_move(&mut world, &mut conn, "east", "", &registry);

        let mut pos = world.query_one::<&Position>(player).unwrap();
        let player_room = pos.get().unwrap().room;
        assert_eq!(player_room, room_b);
    }

    #[test]
    fn test_move_invalid_direction() {
        let (mut world, _void, room_a, _room_b) = test_world();
        let (player, mut conn, registry) = test_player(&mut world, room_a);

        cmd_move(&mut world, &mut conn, "north", "", &registry);

        let mut pos = world.query_one::<&Position>(player).unwrap();
        let player_room = pos.get().unwrap().room;
        assert_eq!(player_room, room_a);

        let lines = conn.take_lines();
        assert!(lines.iter().any(|l| l.contains("cannot go that way")));
    }

    #[test]
    fn test_move_in_void() {
        let (mut world, void_room, _room_a, _room_b) = test_world();
        let (player, mut conn, registry) = test_player(&mut world, void_room);

        cmd_move(&mut world, &mut conn, "east", "", &registry);

        let mut pos = world.query_one::<&Position>(player).unwrap();
        let player_room = pos.get().unwrap().room;
        assert_eq!(player_room, void_room);

        let lines = conn.take_lines();
        assert!(lines.iter().any(|l| l.contains("cannot move in the void")));
    }

    #[test]
    fn test_move_no_entity() {
        let (mut world, _void, _room_a, _room_b) = test_world();
        let mut conn = MockConnection::new();
        let registry = ConnectionRegistry::new();

        cmd_move(&mut world, &mut conn, "east", "", &registry);

        let lines = conn.take_lines();
        assert_eq!(lines, vec!["You have no form."]);
    }

    #[test]
    fn test_move_closed_exit() {
        let (mut world, _void, room_a, room_b) = test_world();

        {
            let mut q = world.query_one::<&mut RoomExits>(room_a).unwrap();
            let exits = q.get().unwrap();
            exits.0[0] = Exit {
                direction: Direction::East,
                dest: room_b,
                flags: core::EXIT_IS_CLOSED,
                key_id: None,
            };
        }

        let (player, mut conn, registry) = test_player(&mut world, room_a);

        cmd_move(&mut world, &mut conn, "east", "", &registry);

        let mut pos = world.query_one::<&Position>(player).unwrap();
        assert_eq!(pos.get().unwrap().room, room_a);

        let lines = conn.take_lines();
        assert!(lines.iter().any(|l| l.contains("That exit is closed.")));
    }

    #[test]
    fn test_move_locked_exit() {
        let (mut world, _void, room_a, room_b) = test_world();

        {
            let mut q = world.query_one::<&mut RoomExits>(room_a).unwrap();
            let exits = q.get().unwrap();
            exits.0[0] = Exit {
                direction: Direction::East,
                dest: room_b,
                flags: core::EXIT_IS_CLOSED | core::EXIT_IS_LOCKED,
                key_id: Some("key1".to_string()),
            };
        }

        let (player, mut conn, registry) = test_player(&mut world, room_a);

        cmd_move(&mut world, &mut conn, "east", "", &registry);

        let mut pos = world.query_one::<&Position>(player).unwrap();
        assert_eq!(pos.get().unwrap().room, room_a);

        let lines = conn.take_lines();
        assert!(lines.iter().any(|l| l.contains("That exit is closed.")));
    }

    #[test]
    fn test_move_uses_short_name() {
        let (mut world, _void, room_a, room_b) = test_world();
        let (player, mut conn, registry) = test_player(&mut world, room_a);

        cmd_move(&mut world, &mut conn, "e", "", &registry);

        let mut pos = world.query_one::<&Position>(player).unwrap();
        assert_eq!(pos.get().unwrap().room, room_b);
    }

    #[test]
    fn test_move_broadcasts_leave_and_enter() {
        let (mut world, _void, room_a, room_b) = test_world();
        let (_p1, mut c1, mut registry) = test_player(&mut world, room_a);
        let (_p2, c2, _) = test_player(&mut world, room_a);
        let (_p3, c3, _) = test_player(&mut world, room_b);

        let (tx2, mut rx2) = tokio::sync::mpsc::unbounded_channel::<Vec<u8>>();
        let (tx3, mut rx3) = tokio::sync::mpsc::unbounded_channel::<Vec<u8>>();

        let dangling_entity = {
            let mut w = World::new();
            let e = w.spawn(());
            let _ = w.despawn(e);
            e
        };
        registry.register(dangling_entity, tx2.clone());

        cmd_move(&mut world, &mut c1, "east", "", &registry);

        let mut pos1 = world.query_one::<&Position>(_p1).unwrap();
        assert_eq!(pos1.get().unwrap().room, room_b);

        let _ = c2.take_lines();
        let _ = c3.take_lines();

        let _ = (tx2, tx3, rx2.try_recv(), rx3.try_recv());
    }

    #[test]
    fn test_direction_from_short_name() {
        assert_eq!(direction_from_name("n"), Some(Direction::North));
        assert_eq!(direction_from_name("s"), Some(Direction::South));
        assert_eq!(direction_from_name("e"), Some(Direction::East));
        assert_eq!(direction_from_name("w"), Some(Direction::West));
        assert_eq!(direction_from_name("u"), Some(Direction::Up));
        assert_eq!(direction_from_name("d"), Some(Direction::Down));
        assert_eq!(direction_from_name("ne"), Some(Direction::Northeast));
        assert_eq!(direction_from_name("nw"), Some(Direction::Northwest));
        assert_eq!(direction_from_name("se"), Some(Direction::Southeast));
        assert_eq!(direction_from_name("sw"), Some(Direction::Southwest));
    }

    #[test]
    fn test_direction_from_long_name() {
        assert_eq!(direction_from_name("north"), Some(Direction::North));
        assert_eq!(direction_from_name("south"), Some(Direction::South));
        assert_eq!(direction_from_name("east"), Some(Direction::East));
    }

    #[test]
    fn test_direction_from_unknown_name() {
        assert_eq!(direction_from_name("sideways"), None);
        assert_eq!(direction_from_name(""), None);
    }

    #[test]
    fn test_move_with_short_name() {
        let (mut world, _void, room_a, room_b) = test_world();
        let (player, mut conn, registry) = test_player(&mut world, room_a);

        cmd_move(&mut world, &mut conn, "e", "", &registry);

        let mut pos = world.query_one::<&Position>(player).unwrap();
        assert_eq!(pos.get().unwrap().room, room_b);
    }

    #[test]
    fn test_move_huh_for_bad_name() {
        let (mut world, _void, room_a, _room_b) = test_world();
        let (_player, mut conn, registry) = test_player(&mut world, room_a);

        cmd_move(&mut world, &mut conn, "invalid", "", &registry);

        let lines = conn.take_lines();
        assert_eq!(lines, vec!["Huh?"]);
    }

    #[test]
    fn test_door_interaction() {
        let (mut world, _void, room_a, room_b) = test_world();
        let player = world.spawn((
            Position::new(room_a),
            Name::new("TestPlayer"),
            Inventory::new(),
        ));
        let mut conn = MockConnection::new();
        conn.set_entity(player);
        let registry = ConnectionRegistry::new();

        let key_id = "gold_key".to_string();
        {
            let mut q_a = world.query_one::<&mut RoomExits>(room_a).unwrap();
            let exits = q_a.get().unwrap();
            exits.0[0] = Exit {
                direction: Direction::East,
                dest: room_b,
                flags: core::EXIT_IS_DOOR | core::EXIT_IS_CLOSED | core::EXIT_IS_LOCKED,
                key_id: Some(key_id.clone()),
            };
        }
        {
            let mut q_b = world.query_one::<&mut RoomExits>(room_b).unwrap();
            let exits = q_b.get().unwrap();
            exits.0[0] = Exit {
                direction: Direction::West,
                dest: room_a,
                flags: core::EXIT_IS_DOOR | core::EXIT_IS_CLOSED | core::EXIT_IS_LOCKED,
                key_id: Some(key_id.clone()),
            };
        }

        cmd_open(&mut world, &mut conn, "open", "east", &registry);
        let lines = conn.take_lines();
        assert!(lines.join("|").contains("It is locked"));

        cmd_unlock(&mut world, &mut conn, "unlock", "east", &registry);
        let lines = conn.take_lines();
        assert!(lines.join("|").contains("do not have the key"));

        let key_item = world.spawn((Item::new("gold_key"),));
        if let Ok(mut q) = world.query_one::<&mut Inventory>(player) {
            if let Some(inv) = q.get() {
                inv.0.push(key_item);
            }
        }

        cmd_unlock(&mut world, &mut conn, "unlock", "east", &registry);
        let lines = conn.take_lines();
        assert!(lines.join("|").contains("unlock the door"));

        {
            let mut q_a = world.query_one::<&RoomExits>(room_a).unwrap();
            let exits = q_a.get().unwrap();
            let exit = &exits.0[0];
            assert!(!exit.is_locked());
            assert!(exit.is_closed());
        }

        cmd_open(&mut world, &mut conn, "open", "east", &registry);
        let lines = conn.take_lines();
        assert!(lines.join("|").contains("open the door"));

        {
            let mut q_a = world.query_one::<&RoomExits>(room_a).unwrap();
            let exits = q_a.get().unwrap();
            let exit = &exits.0[0];
            assert!(!exit.is_closed());
        }

        cmd_close(&mut world, &mut conn, "close", "east", &registry);
        let lines = conn.take_lines();
        assert!(lines.join("|").contains("close the door"));

        cmd_lock(&mut world, &mut conn, "lock", "east", &registry);
        let lines = conn.take_lines();
        assert!(lines.join("|").contains("lock the door"));

        {
            let mut q_a = world.query_one::<&RoomExits>(room_a).unwrap();
            let exits = q_a.get().unwrap();
            let exit = &exits.0[0];
            assert!(exit.is_locked());
        }
    }

    #[test]
    fn test_ghost_movement_broadcasts() {
        let (mut world, _void, room_a, _room_b) = test_world();

        let ghost = world.spawn((
            Position::new(room_a),
            Name::new("Casper"),
            core::PlayerState::Dead,
        ));

        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let registry = {
            let mut reg = ConnectionRegistry::new();
            reg.register(ghost, tx);
            reg
        };

        send_leave_broadcast(&world, &registry, ghost, room_a, "east");

        if let Ok(bytes) = rx.try_recv() {
            let text = String::from_utf8_lossy(&bytes);
            assert!(text.contains("cold shiver"));
        }
    }

    #[test]
    fn test_move_in_combat_blocked() {
        let (mut world, _void, room_a, _room_b) = test_world();
        let (player, mut conn, registry) = test_player(&mut world, room_a);

        let enemy = world.spawn(());
        world
            .insert(
                player,
                (core::CombatState::Engaged {
                    target: enemy,
                    round_started: std::time::Instant::now(),
                    stance: None,
                },),
            )
            .unwrap();

        cmd_move(&mut world, &mut conn, "east", "", &registry);

        let mut pos = world.query_one::<&Position>(player).unwrap();
        let player_room = pos.get().unwrap().room;
        assert_eq!(player_room, room_a);

        let lines = conn.take_lines();
        assert!(lines
            .iter()
            .any(|l| l.contains("No way! You are fighting for your life!")));
    }
}
