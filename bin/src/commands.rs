use std::collections::HashMap;
use std::str::FromStr;

use oxide_core as core;
use oxide_core::format::preview::{item_look_template, mob_look_template};
use oxide_core::templates::{SetDef, SkillResolveError};
use oxide_core::{
    Description, Direction, FloorItems, Friendly, Inventory, Item, Name, Npc, Position, Room,
    RoomExits, ShortDesc, VoidRoom, World,
};
use oxide_server::{Connection, ConnectionFlag, ConnectionRegistry};

fn trigger_message(trigger: &core::TriggeredEffect, world: &World) -> String {
    let item_name = world
        .query_one::<&Name>(trigger.item)
        .ok()
        .and_then(|mut q| q.get().cloned())
        .map(|n| n.0)
        .unwrap_or_else(|| "something".to_owned());
    format!("Your {item_name} {}.", trigger.cast)
}

fn send_formatted(conn: &mut dyn Connection, text: &core::format::RichText) {
    let ansi = conn.flags().has(ConnectionFlag::Ansi);
    let blink = conn.flags().has(ConnectionFlag::Blink);
    let width = conn.screen_width() as usize;
    conn.send_line(&text.render_wrapped(width, ansi, blink));
}

fn get_pos_room(world: &World, entity: core::Entity) -> Option<core::Entity> {
    world
        .query_one::<&Position>(entity)
        .ok()
        .and_then(|mut q| q.get().map(|p| p.room))
}

fn get_room_name(world: &World, room: core::Entity) -> Option<String> {
    world
        .query_one::<&Room>(room)
        .ok()
        .and_then(|mut q| q.get().map(|r| r.name.clone()))
}

fn get_room_desc(world: &World, room: core::Entity) -> Option<String> {
    world
        .query_one::<&Room>(room)
        .ok()
        .and_then(|mut q| q.get().map(|r| r.description.clone()))
}

fn get_name(world: &World, entity: core::Entity) -> Option<Name> {
    world
        .query_one::<&Name>(entity)
        .ok()
        .and_then(|mut q| q.get().cloned())
}

fn get_short_desc(world: &World, entity: core::Entity) -> Option<String> {
    let sd = world
        .query_one::<&ShortDesc>(entity)
        .ok()
        .and_then(|mut q| q.get().map(|s| s.0.clone()));
    if sd.as_ref().is_some_and(|s| !s.is_empty()) {
        return sd;
    }
    world
        .query_one::<&Name>(entity)
        .ok()
        .and_then(|mut q| q.get().map(|n| n.0.clone()))
}

fn is_void_room(world: &World, room: core::Entity) -> bool {
    world
        .query_one::<&VoidRoom>(room)
        .is_ok_and(|mut q| q.get().is_some())
}

fn get_exits(world: &World, room: core::Entity) -> Vec<&'static str> {
    let mut exits = Vec::new();
    if let Ok(mut q) = world.query_one::<&RoomExits>(room) {
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

// ---------------------------------------------------------------------------
// "look at <target>" helpers
// ---------------------------------------------------------------------------

enum TargetKind {
    Mob,
    Player,
    Item,
    Corpse,
}

fn get_entity_name(world: &World, entity: core::Entity) -> Option<String> {
    world
        .query_one::<&Name>(entity)
        .ok()
        .and_then(|mut q| q.get().map(|n| n.as_str().to_lowercase()))
}

fn get_room_name_for_entity(world: &World, entity: core::Entity) -> Option<String> {
    world
        .query_one::<&Room>(entity)
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

    // Search order determines tie-breaking within the same trie tier: NPCs → players → floor → inventory
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
        let mut q = world.query::<(&Position, &Npc, &Friendly)>();
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

    // Collect display info for each entity in the room
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

    // Sort: mobs first, then corpses, friendly NPCs, and players; alphabetical within groups
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

pub fn cmd_say(
    world: &mut World,
    conn: &mut dyn Connection,
    _name: &str,
    args: &str,
    registry: &ConnectionRegistry,
) {
    if args.is_empty() {
        conn.send_line("Say what?");
        return;
    }

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

    if is_void_room(world, room) {
        conn.send_line("Your words echo in the void with no one to hear.");
        return;
    }

    let name = get_name(world, entity).unwrap_or(Name::new("Someone"));

    let is_ghost = world
        .query_one::<&core::PlayerState>(entity)
        .ok()
        .and_then(|mut q| q.get().map(|s| matches!(s, core::PlayerState::Dead)))
        .unwrap_or(false);

    let msg_fmt = if is_ghost {
        format_ghost_text(args)
    } else {
        args.to_string()
    };

    // Speaker message
    let speaker_msg = core::format::conventions::say_text(format!("You say, \"{msg_fmt}\""));
    send_formatted(conn, &speaker_msg);

    // Room broadcast
    let mut room_msg = core::format::RichText::new();
    room_msg.push(core::format::conventions::player_name_segment(
        name.as_str(),
    ));
    room_msg.push(core::format::Segment::new(format!(" says, \"{msg_fmt}\"")));

    let rendered = room_msg.render(true, true);
    let bytes = format!("{}\r\n", rendered).into_bytes();

    let occupants = registry.occupants(world, room);
    for &other in &occupants {
        if other == entity {
            continue;
        }
        if let Some(tx) = registry.sender(other) {
            let _ = tx.send(bytes.clone());
        }
    }

    if let Some(bridge) = core::scripting::get_scripting_bridge() {
        let mut targets = Vec::new();
        targets.push(room);
        targets.extend(occupants.clone());

        for (item_eid, (pos, _item)) in world.query::<(&core::Position, &core::Item)>().iter() {
            if pos.room == room {
                targets.push(core::Entity::from(item_eid));
            }
        }

        if let Ok(mut q) = world.query_one::<&core::Inventory>(entity) {
            if let Some(inv) = q.get() {
                targets.extend(inv.0.clone());
            }
        }

        if let Ok(mut q) = world.query_one::<&core::Equipment>(entity) {
            if let Some(eq) = q.get() {
                for &(_slot, item_eid) in &eq.slots {
                    targets.push(item_eid);
                }
            }
        }

        let mut unique_targets = Vec::new();
        for t in targets {
            if !unique_targets.contains(&t) {
                unique_targets.push(t);
            }
        }

        for target_eid in unique_targets {
            let _ = bridge.execute_say_hook(target_eid, entity, args, world);
        }
    }
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

    // Normal broadcast
    let mut normal_msg = core::format::RichText::new();
    normal_msg.push(core::format::conventions::player_name_segment(
        name.as_str(),
    ));
    normal_msg.push(core::format::Segment::new(format!(" leaves {dir_long}.")));
    let normal_bytes = format!("{}\r\n", normal_msg.render(true, true)).into_bytes();

    // Ghost detector broadcast
    let mut ghost_detector_msg = core::format::RichText::new();
    ghost_detector_msg.push(core::format::Segment::new(format!(
        "The ghost of {} floats {dir_long}.",
        name.as_str()
    )));
    let ghost_detector_bytes =
        format!("{}\r\n", ghost_detector_msg.render(true, true)).into_bytes();

    // Ghost non-detector broadcast (chill)
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

    // Normal broadcast
    let mut normal_msg = core::format::RichText::new();
    normal_msg.push(core::format::conventions::player_name_segment(
        name.as_str(),
    ));
    normal_msg.push(core::format::Segment::new(format!(
        " arrives from the {dir_long}."
    )));
    let normal_bytes = format!("{}\r\n", normal_msg.render(true, true)).into_bytes();

    // Ghost detector broadcast
    let mut ghost_detector_msg = core::format::RichText::new();
    ghost_detector_msg.push(core::format::Segment::new(format!(
        "The ghost of {} floats in from the {dir_long}.",
        name.as_str()
    )));
    let ghost_detector_bytes =
        format!("{}\r\n", ghost_detector_msg.render(true, true)).into_bytes();

    // Ghost non-detector broadcast (chill)
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

fn direction_from_name(name: &str) -> Option<Direction> {
    Direction::from_short(name).or_else(|| Direction::from_long(name))
}

fn move_player(
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
                core::PlayerState::Dead => {} // Ghost can move
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

    // Find the exit
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

    // Check exit flags
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

    // Move the player
    let _ = world.insert(entity, (Position::new(dest), core::Dirty));

    // Check room exploration objectives
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

    // Broadcast leave
    let dir_long = direction.long_name();
    let opposite = direction.opposite();
    let opp_long = opposite.long_name();
    send_leave_broadcast(world, registry, entity, room, dir_long);

    // Broadcast enter
    send_enter_broadcast(world, registry, entity, dest, opp_long);

    // Auto-look
    cmd_look(world, conn, "", "", registry);

    // Follower movement
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
                fn id(&self) -> u64 {
                    0
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

fn format_wide_list(items: &[String], max_width: usize) -> Vec<String> {
    if items.is_empty() {
        return Vec::new();
    }
    let max_len = items.iter().map(|s| s.len()).max().unwrap_or(0);
    let col_width = max_len + 3; // add 3 spaces of padding between columns
    let num_cols = (max_width / col_width).max(1);

    let mut lines = Vec::new();
    let mut current_line = String::new();
    for (i, item) in items.iter().enumerate() {
        if i > 0 && i % num_cols == 0 {
            lines.push(current_line);
            current_line = String::new();
        }
        current_line.push_str(&format!("{:<width$}", item, width = col_width));
    }
    if !current_line.is_empty() {
        lines.push(current_line);
    }
    lines
}

pub fn cmd_help(
    _world: &mut World,
    conn: &mut dyn Connection,
    _name: &str,
    args: &str,
    _registry: &ConnectionRegistry,
) {
    let dispatch = match oxide_server::get_commands() {
        Some(d) => d,
        None => {
            conn.send_line("Help is unavailable.");
            return;
        }
    };

    let conn_access = conn.access_level();
    let mut categories = std::collections::BTreeSet::new();
    for cmd in &dispatch.commands {
        if conn_access >= cmd.access {
            categories.insert(cmd.category);
        }
    }

    let query = args.trim();

    if !query.is_empty() {
        let query_lower = query.to_lowercase();
        // Check if query matches a category name case-insensitively
        let matched_category = categories
            .iter()
            .find(|cat| cat.to_lowercase() == query_lower);
        if let Some(cat) = matched_category {
            let mut cmds = Vec::new();
            for cmd in &dispatch.commands {
                if cmd.category == *cat && conn_access >= cmd.access {
                    let name_col = if cmd.aliases.is_empty() {
                        cmd.name.to_string()
                    } else {
                        format!("{} ({})", cmd.name, cmd.aliases.join(", "))
                    };
                    cmds.push(name_col);
                }
            }
            cmds.sort();
            let width = if conn.screen_width() > 0 {
                conn.screen_width() as usize
            } else {
                80
            };
            conn.send_line("");
            conn.send_line(&format!("Commands in Category '{cat}':"));
            conn.send_line("");
            for line in format_wide_list(&cmds, width) {
                conn.send_line(&format!("  {line}"));
            }
            conn.send_line("");
            return;
        }

        // Check if query matches a command name or alias
        if let Some(cmd) = dispatch.find(query) {
            if conn_access >= cmd.access {
                conn.send_line("");
                let header = if cmd.aliases.is_empty() {
                    cmd.name.to_string()
                } else {
                    format!("{} ({})", cmd.name, cmd.aliases.join(", "))
                };
                conn.send_line(&format!("  {header}"));
                conn.send_line("");
                for line in cmd.help_text.lines() {
                    conn.send_line(&format!("  {line}"));
                }
                conn.send_line("");
                return;
            }
        }

        conn.send_line(&format!("No help found for '{query}'."));
        return;
    }

    let cats: Vec<String> = categories.iter().map(|s| s.to_string()).collect();
    let width = if conn.screen_width() > 0 {
        conn.screen_width() as usize
    } else {
        80
    };
    conn.send_line("");
    conn.send_line("Available Help Categories  (type 'help <category>' or 'help <command>')");
    conn.send_line("");
    for line in format_wide_list(&cats, width) {
        conn.send_line(&format!("  {line}"));
    }
    conn.send_line("");
}

pub fn cmd_quit(
    world: &mut World,
    conn: &mut dyn Connection,
    _name: &str,
    _args: &str,
    _registry: &ConnectionRegistry,
) {
    if let Some(entity) = conn.entity() {
        let _ = world.insert(entity, (core::Dirty,));
    }
    conn.send_line("Goodbye!");
    conn.disconnect();
}

pub fn cmd_score(
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

    let name = world
        .query_one::<&core::Name>(entity)
        .ok()
        .and_then(|mut q| q.get().map(|n| n.to_string()))
        .unwrap_or_else(|| "Unknown".to_string());

    let level = world
        .query_one::<&core::Level>(entity)
        .ok()
        .and_then(|mut q| q.get().copied())
        .unwrap_or(core::Level(1));

    let xp = world
        .query_one::<&core::Experience>(entity)
        .ok()
        .and_then(|mut q| q.get().copied())
        .unwrap_or(core::Experience(0));

    let attrs = world
        .query_one::<&core::Attributes>(entity)
        .ok()
        .and_then(|mut q| q.get().cloned())
        .unwrap_or_default();

    let hp = world
        .query_one::<&core::Health>(entity)
        .ok()
        .and_then(|mut q| q.get().cloned())
        .unwrap_or(core::Health::new(20));

    let practice_pts = world
        .query_one::<&core::PracticePoints>(entity)
        .ok()
        .and_then(|mut q| q.get().copied())
        .unwrap_or(core::PracticePoints(0));

    let combat_stats = world
        .query_one::<&core::CombatStats>(entity)
        .ok()
        .and_then(|mut q| q.get().cloned())
        .unwrap_or_default();

    let mana = world
        .query_one::<&core::Mana>(entity)
        .ok()
        .and_then(|mut q| q.get().copied());

    let stamina = world
        .query_one::<&core::Stamina>(entity)
        .ok()
        .and_then(|mut q| q.get().copied());

    let energy = world
        .query_one::<&core::Energy>(entity)
        .ok()
        .and_then(|mut q| q.get().copied());

    let psi = world
        .query_one::<&core::Psi>(entity)
        .ok()
        .and_then(|mut q| q.get().copied());

    let deity = world
        .query_one::<&core::Deity>(entity)
        .ok()
        .and_then(|mut q| q.get().cloned())
        .map(|d| d.0.unwrap_or_else(|| "none".to_string()))
        .unwrap_or_else(|| "none".to_string());

    let age = world
        .query_one::<&core::Age>(entity)
        .ok()
        .and_then(|mut q| q.get().copied())
        .map(|a| a.0)
        .unwrap_or(20);

    let appearance = world
        .query_one::<&core::Appearance>(entity)
        .ok()
        .and_then(|mut q| q.get().cloned());

    let xp_to_next = xp.to_next_level(level.0);

    conn.send_line("");
    conn.send_line("--- Character Score ---");
    conn.send_line(&format!("  Name:            {}", name));
    conn.send_line(&format!("  Level:           {}", level.0));
    conn.send_line(&format!(
        "  Experience:      {} / {} ({} to next level)",
        xp.0,
        core::Experience::for_level(level.0 + 1),
        xp_to_next
    ));
    conn.send_line(&format!("  Practice Points: {}", practice_pts.0));
    conn.send_line(&format!("  HP:              {} / {}", hp.current, hp.max));
    conn.send_line(&format!("  Deity:           {}", deity));
    conn.send_line(&format!("  Age:             {} years", age));
    if let Some(app) = appearance {
        conn.send_line(&format!(
            "  Appearance:      {}in, {}lbs, {} build",
            app.height, app.weight, app.build
        ));
        conn.send_line(&format!(
            "                   Hair: {} ({}), Eyes: {}, Skin: {}",
            app.hair_style, app.hair_color, app.eye_color, app.skin_tone
        ));
    }

    if let Some(m) = mana {
        conn.send_line(&format!("  Mana:            {} / {}", m.current, m.max));
    }
    if let Some(s) = stamina {
        conn.send_line(&format!("  Stamina:         {} / {}", s.current, s.max));
    }
    if let Some(e) = energy {
        conn.send_line(&format!("  Energy:          {} / {}", e.current, e.max));
    }
    if let Some(p) = psi {
        conn.send_line(&format!("  Psi:             {} / {}", p.current, p.max));
    }

    let format_modifier = |val: i32| -> String {
        if val >= 0 {
            format!("+{}", val)
        } else {
            val.to_string()
        }
    };

    conn.send_line("");
    conn.send_line(&format!(
        "  BAB:             {}",
        format_modifier(combat_stats.base_attack_bonus)
    ));
    conn.send_line(&format!(
        "  Saves:           Fort: {}, Ref: {}, Will: {}",
        format_modifier(combat_stats.fort_save),
        format_modifier(combat_stats.ref_save),
        format_modifier(combat_stats.will_save)
    ));

    conn.send_line("");
    conn.send_line("  Attributes:");
    conn.send_line(&format!("    Strength:     {}", attrs.strength));
    conn.send_line(&format!("    Dexterity:    {}", attrs.dexterity));
    conn.send_line(&format!("    Intelligence: {}", attrs.intelligence));
    conn.send_line(&format!("    Wisdom:       {}", attrs.wisdom));
    conn.send_line(&format!("    Constitution: {}", attrs.constitution));
    conn.send_line(&format!("    Charisma:     {}", attrs.charisma));
    conn.send_line("");
}

pub fn cmd_motd(
    _world: &mut World,
    conn: &mut dyn Connection,
    _name: &str,
    _args: &str,
    _registry: &ConnectionRegistry,
) {
    conn.send_line("");
    conn.send_line(oxide_server::get_motd());
    conn.send_line("");
}

pub fn cmd_who(
    world: &mut World,
    conn: &mut dyn Connection,
    _name: &str,
    _args: &str,
    registry: &ConnectionRegistry,
) {
    let lines = oxide_server::login::list_who(world, registry);
    for line in &lines {
        conn.send_line(line);
    }
}

pub fn cmd_width(
    world: &mut World,
    conn: &mut dyn Connection,
    _name: &str,
    args: &str,
    _registry: &ConnectionRegistry,
) {
    let trimmed = args.trim();
    if trimmed.is_empty() {
        let current = conn.screen_width();
        if current == 0 {
            conn.send_line("Screen width: unlimited (0)");
        } else {
            conn.send_line(&format!("Screen width: {current} columns"));
        }
        conn.send_line("Usage: width <columns>  (0 = unlimited)");
        return;
    }

    let width: u16 = match trimmed.parse() {
        Ok(w) => w,
        Err(_) => {
            conn.send_line("Usage: width <columns>  (0 = unlimited)");
            return;
        }
    };

    conn.set_screen_width(width);

    // Persist to Player component and DB
    if let Some(entity) = conn.entity() {
        let mut updated = false;
        if let Ok(mut q) = world.query_one::<&mut core::Player>(entity) {
            if let Some(player) = q.get() {
                player.screen_width = width;
                updated = true;
            }
        }
        if updated {
            let _ = world.insert(entity, (core::Dirty,));
        }
    }

    if width == 0 {
        conn.send_line("Screen width set to unlimited.");
    } else {
        conn.send_line(&format!("Screen width set to {width} columns."));
    }
}

pub fn cmd_award(
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

    let amount: u64 = match args.trim().parse() {
        Ok(n) => n,
        Err(_) => {
            conn.send_line("Usage: @award <xp_amount>");
            return;
        }
    };

    let new_xp = {
        let mut q = match world.query_one::<&mut core::Experience>(entity) {
            Ok(q) => q,
            Err(_) => {
                conn.send_line("You have no experience component.");
                return;
            }
        };
        match q.get() {
            Some(xp) => {
                xp.0 = xp.0.saturating_add(amount);
                xp.0
            }
            None => {
                conn.send_line("You have no experience component.");
                return;
            }
        }
    };

    conn.send_line(&format!(
        "You gain {amount} experience points! (Total: {new_xp})"
    ));

    // Check for level-ups
    oxide_server::award_xp(world, entity);

    // Report new level
    let level = world
        .query_one::<&core::Level>(entity)
        .ok()
        .and_then(|mut q| q.get().copied())
        .unwrap_or(core::Level(1));
    let xp = world
        .query_one::<&core::Experience>(entity)
        .ok()
        .and_then(|mut q| q.get().copied())
        .unwrap_or(core::Experience(0));
    conn.send_line(&format!("You are now level {} with {} XP.", level.0, xp.0));
}

// ---------------------------------------------------------------------------
// Staff Commands (Builder, Immortal, God, Admin)
// ---------------------------------------------------------------------------

fn find_player_by_name(world: &World, name: &str) -> Option<oxide_core::Entity> {
    let name_lower = name.to_lowercase();
    let mut q = world.query::<(&core::Name, &core::Player)>();
    for (entity, (n, _)) in q.iter() {
        if n.0.to_lowercase() == name_lower {
            return Some(oxide_core::Entity::from(entity));
        }
    }
    None
}

fn find_mob_in_room(
    world: &World,
    room_entity: oxide_core::Entity,
    target_name: &str,
) -> Option<oxide_core::Entity> {
    let target_lower = target_name.to_lowercase();
    let occupants = core::util::entities_in_room(world, room_entity);
    for entity in occupants {
        if world.query_one::<&core::Player>(entity).is_ok() {
            continue; // Skip players
        }
        if let Ok(mut q) = world.query_one::<&core::Name>(entity) {
            if let Some(n) = q.get() {
                if n.0.to_lowercase().contains(&target_lower) {
                    return Some(entity);
                }
            }
        }
    }
    None
}

pub fn cmd_goto(
    world: &mut World,
    conn: &mut dyn Connection,
    _name: &str,
    args: &str,
    _registry: &ConnectionRegistry,
) {
    let entity = match conn.entity() {
        Some(e) => e,
        None => return,
    };
    let arg = args.trim();
    if arg.is_empty() {
        conn.send_line("Usage: goto <room_key_or_player_name>");
        return;
    }
    let target_room = if let Some(target_player) = find_player_by_name(world, arg) {
        world
            .query_one::<&core::Position>(target_player)
            .ok()
            .and_then(|mut q| q.get().map(|p| p.room))
    } else {
        oxide_server::get_templates().and_then(|t| t.find_room_by_key(world, arg))
    };

    let Some(dest_room) = target_room else {
        conn.send_line("No such room or player found.");
        return;
    };

    if let Ok(mut q) = world.query_one::<&mut core::Position>(entity) {
        if let Some(pos) = q.get() {
            pos.room = dest_room;
        }
    }
    conn.send_line("You teleport through space.");
    cmd_look(world, conn, "look", "", _registry);
}

pub fn cmd_at(
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
    let args = args.trim();
    let Some((target, cmd_to_run)) = args.split_once(' ') else {
        conn.send_line("Usage: at <target_room_or_player> <command>");
        return;
    };

    let target_room = if let Some(target_player) = find_player_by_name(world, target) {
        world
            .query_one::<&core::Position>(target_player)
            .ok()
            .and_then(|mut q| q.get().map(|p| p.room))
    } else {
        oxide_server::get_templates().and_then(|t| t.find_room_by_key(world, target))
    };

    let Some(dest_room) = target_room else {
        conn.send_line("No such destination found.");
        return;
    };

    let orig_room = {
        let mut q = world.query_one::<&core::Position>(entity).ok();
        q.as_mut().and_then(|q| q.get().map(|p| p.room))
    };

    if let Some(orig) = orig_room {
        if let Ok(mut q) = world.query_one::<&mut core::Position>(entity) {
            if let Some(pos) = q.get() {
                pos.room = dest_room;
            }
        }
        let dispatch = oxide_server::get_commands().unwrap();
        dispatch.execute(world, conn, cmd_to_run, registry);

        if let Ok(mut q) = world.query_one::<&mut core::Position>(entity) {
            if let Some(pos) = q.get() {
                pos.room = orig;
            }
        }
    }
}

pub fn cmd_force(
    world: &mut World,
    conn: &mut dyn Connection,
    _name: &str,
    args: &str,
    registry: &ConnectionRegistry,
) {
    let args = args.trim();
    let Some((target_name, cmd_to_run)) = args.split_once(' ') else {
        conn.send_line("Usage: force <player_or_mob> <command>");
        return;
    };

    let self_access = conn.access_level();
    let executor_entity = conn.entity();

    let target_entity = if let Some(player_ent) = find_player_by_name(world, target_name) {
        let target_access = {
            let mut q = world.query_one::<&core::AccessLevel>(player_ent).ok();
            q.as_mut()
                .and_then(|q| q.get().copied())
                .unwrap_or(core::AccessLevel::Player)
        };
        if target_access >= self_access {
            conn.send_line("You cannot force someone of equal or higher rank.");
            return;
        }
        Some(player_ent)
    } else {
        let room_entity = executor_entity.and_then(|e| {
            world
                .query_one::<&core::Position>(e)
                .ok()
                .and_then(|mut q| q.get().map(|p| p.room))
        });
        room_entity.and_then(|room| find_mob_in_room(world, room, target_name))
    };

    let Some(target) = target_entity else {
        conn.send_line("No such target found.");
        return;
    };

    struct MockConnection {
        entity: Option<oxide_core::Entity>,
        output: Vec<String>,
        access: oxide_core::AccessLevel,
    }
    impl Connection for MockConnection {
        fn send(&mut self, text: &str) {
            self.output.push(text.to_string());
        }
        fn send_line(&mut self, text: &str) {
            self.output.push(text.to_string());
        }
        fn send_raw(&mut self, _bytes: &[u8]) {}
        fn id(&self) -> u64 {
            0
        }
        fn entity(&self) -> Option<oxide_core::Entity> {
            self.entity
        }
        fn set_entity(&mut self, entity: oxide_core::Entity) {
            self.entity = Some(entity);
        }
        fn disconnect(&mut self) {}
        fn is_disconnected(&self) -> bool {
            false
        }
        fn flags(&self) -> oxide_server::ConnectionFlags {
            oxide_server::ConnectionFlags::new()
        }
        fn set_flags(&mut self, _flags: oxide_server::ConnectionFlags) {}
        fn access_level(&self) -> oxide_core::AccessLevel {
            self.access
        }
        fn set_access_level(&mut self, level: oxide_core::AccessLevel) {
            self.access = level;
        }
        fn output_sender(&self) -> Option<tokio::sync::mpsc::UnboundedSender<Vec<u8>>> {
            None
        }
    }

    let target_access = {
        let mut q = world.query_one::<&core::AccessLevel>(target).ok();
        q.as_mut()
            .and_then(|q| q.get().copied())
            .unwrap_or(core::AccessLevel::Player)
    };

    let mut mock_conn = MockConnection {
        entity: Some(target),
        output: Vec::new(),
        access: target_access,
    };

    let dispatch = oxide_server::get_commands().unwrap();
    dispatch.execute(world, &mut mock_conn, cmd_to_run, registry);

    conn.send_line(&format!("You force {target_name} to: {cmd_to_run}"));
    for line in mock_conn.output {
        conn.send_line(&format!("  [Output] {line}"));
    }
}

pub fn cmd_stat(
    world: &mut World,
    conn: &mut dyn Connection,
    _name: &str,
    args: &str,
    _registry: &ConnectionRegistry,
) {
    let arg = args.trim();
    let entity = if arg.is_empty() || arg.to_lowercase() == "room" {
        conn.entity().and_then(|e| {
            world
                .query_one::<&core::Position>(e)
                .ok()
                .and_then(|mut q| q.get().map(|p| p.room))
        })
    } else if let Some(player_ent) = find_player_by_name(world, arg) {
        Some(player_ent)
    } else {
        conn.entity()
            .and_then(|e| {
                world
                    .query_one::<&core::Position>(e)
                    .ok()
                    .and_then(|mut q| q.get().map(|p| p.room))
            })
            .and_then(|room| find_mob_in_room(world, room, arg))
    };

    let Some(ent) = entity else {
        conn.send_line("Target not found.");
        return;
    };

    conn.send_line(&format!("Entity: {} (ID: {})", ent.id(), ent.id()));

    if let Ok(mut q) = world.query_one::<&core::Name>(ent) {
        if let Some(n) = q.get() {
            conn.send_line(&format!("  Name: {}", n.0));
        }
    }
    if let Ok(mut q) = world.query_one::<&core::Attributes>(ent) {
        if let Some(a) = q.get() {
            conn.send_line(&format!(
                "  Attributes: STR:{} DEX:{} INT:{} WIS:{} CON:{} CHA:{}",
                a.strength, a.dexterity, a.intelligence, a.wisdom, a.constitution, a.charisma
            ));
        }
    }
    if let Ok(mut q) = world.query_one::<&core::Health>(ent) {
        if let Some(h) = q.get() {
            conn.send_line(&format!("  Health: {} / {}", h.current, h.max));
        }
    }
    if let Ok(mut q) = world.query_one::<&core::Level>(ent) {
        if let Some(l) = q.get() {
            conn.send_line(&format!("  Level: {}", l.0));
        }
    }
    if let Ok(mut q) = world.query_one::<&core::RoomKey>(ent) {
        if let Some(rk) = q.get() {
            conn.send_line(&format!("  RoomKey: {}", rk.0));
        }
    }
    if let Ok(mut q) = world.query_one::<&core::Position>(ent) {
        if let Some(p) = q.get() {
            conn.send_line(&format!("  Position Room ID: {}", p.room.id()));
        }
    }
}

pub fn cmd_olocate(
    world: &mut World,
    conn: &mut dyn Connection,
    _name: &str,
    args: &str,
    _registry: &ConnectionRegistry,
) {
    let query = args.trim().to_lowercase();
    if query.is_empty() {
        conn.send_line("Usage: olocate <item_name_or_template_id>");
        return;
    }

    conn.send_line(&format!("Locating items matching '{query}':"));
    let mut found = false;

    let mut q = world.query::<(&core::Item, &core::Name)>();
    for (entity, (item, name)) in q.iter() {
        if item.template_id.to_lowercase() == query || name.0.to_lowercase().contains(&query) {
            found = true;
            let mut holder_info = "Unknown location".to_string();

            let mut q_inv = world.query::<(&core::Inventory, &core::Name)>();
            for (_, (inv, owner_name)) in q_inv.iter() {
                if inv.0.contains(&oxide_core::Entity::from(entity)) {
                    holder_info = format!("In inventory of {}", owner_name.0);
                }
            }

            let mut q_eq = world.query::<(&core::Equipment, &core::Name)>();
            for (_, (eq, owner_name)) in q_eq.iter() {
                if eq
                    .slots
                    .iter()
                    .any(|(_, item_ent)| *item_ent == oxide_core::Entity::from(entity))
                {
                    holder_info = format!("Equipped on {}", owner_name.0);
                }
            }

            let mut q_floor = world.query::<(&core::FloorItems, &core::Name)>();
            for (_, (floor, room_name)) in q_floor.iter() {
                if floor.0.contains(&oxide_core::Entity::from(entity)) {
                    holder_info = format!("On floor of room '{}'", room_name.0);
                }
            }

            conn.send_line(&format!(
                "  Item: {} (ID: {}) — {}",
                name.0,
                entity.id(),
                holder_info
            ));
        }
    }

    if !found {
        conn.send_line("  No matching items found.");
    }
}

pub fn cmd_gecho(
    _world: &mut World,
    conn: &mut dyn Connection,
    _name: &str,
    args: &str,
    registry: &ConnectionRegistry,
) {
    let msg = args.trim();
    if msg.is_empty() {
        conn.send_line("Usage: gecho <message>");
        return;
    }
    tracing::warn!(executor = ?conn.entity(), "gecho run: {msg}");
    let bytes = format!("[Global Echo] {msg}\r\n").into_bytes();
    for entity in registry.connected_entities() {
        if let Some(tx) = registry.sender(entity) {
            let _ = tx.send(bytes.clone());
        }
    }
}

pub fn cmd_gtell(
    world: &mut World,
    conn: &mut dyn Connection,
    _name: &str,
    args: &str,
    registry: &ConnectionRegistry,
) {
    let msg = args.trim();
    if msg.is_empty() {
        conn.send_line("Usage: gtell <message>");
        return;
    }

    let sender_name = conn
        .entity()
        .and_then(|e| {
            world
                .query_one::<&core::Name>(e)
                .ok()
                .and_then(|mut q| q.get().map(|n| n.0.clone()))
        })
        .unwrap_or_else(|| "Someone".to_string());

    let formatted = format!("[Staff Channel] {sender_name}: {msg}\r\n").into_bytes();

    for entity in registry.connected_entities() {
        let has_staff_access = {
            let mut q = world.query_one::<&core::AccessLevel>(entity).ok();
            q.as_mut()
                .and_then(|q| q.get().copied())
                .unwrap_or(core::AccessLevel::Player)
                >= core::AccessLevel::Builder
        };
        if has_staff_access {
            if let Some(tx) = registry.sender(entity) {
                let _ = tx.send(formatted.clone());
            }
        }
    }
}

pub fn cmd_wizwho(
    world: &mut World,
    conn: &mut dyn Connection,
    _name: &str,
    _args: &str,
    registry: &ConnectionRegistry,
) {
    conn.send_line("Staff Online:");
    let mut staff = Vec::new();
    for entity in registry.connected_entities() {
        let access = {
            let mut q = world.query_one::<&core::AccessLevel>(entity).ok();
            q.as_mut()
                .and_then(|q| q.get().copied())
                .unwrap_or(core::AccessLevel::Player)
        };
        if access >= core::AccessLevel::Builder {
            let name = world
                .query_one::<&core::Name>(entity)
                .ok()
                .and_then(|mut q| q.get().map(|n| n.0.clone()))
                .unwrap_or_else(|| format!("Entity {}", entity.id()));
            staff.push((name, access));
        }
    }
    staff.sort_by_key(|s| s.1);
    staff.reverse();

    for (name, access) in staff {
        conn.send_line(&format!("  [{:?}] {}", access, name));
    }
}

pub fn cmd_wizin(
    world: &mut World,
    conn: &mut dyn Connection,
    _name: &str,
    args: &str,
    _registry: &ConnectionRegistry,
) {
    let entity = match conn.entity() {
        Some(e) => e,
        None => return,
    };
    let self_access = conn.access_level();
    let current_level = world
        .query_one::<&core::Wizin>(entity)
        .ok()
        .and_then(|mut q| q.get().map(|w| w.0))
        .unwrap_or(0);

    let target_wizin = if args.trim().is_empty() {
        if current_level > 0 {
            0
        } else {
            self_access as u8
        }
    } else {
        match args.trim().parse::<u8>() {
            Ok(n) => {
                if n > self_access as u8 {
                    conn.send_line(&format!(
                        "You cannot wizin higher than your rank level ({}).",
                        self_access as u8
                    ));
                    return;
                }
                n
            }
            Err(_) => {
                conn.send_line("Usage: wizin [level]");
                return;
            }
        }
    };

    if target_wizin == 0 {
        let _ = world.remove_one::<core::Wizin>(entity);
        conn.send_line("You are now visible to all.");
    } else {
        let _ = world.insert(entity, (core::Wizin(target_wizin),));
        conn.send_line(&format!("You are now wizin at level {}.", target_wizin));
    }
}

pub fn cmd_holylight(
    world: &mut World,
    conn: &mut dyn Connection,
    _name: &str,
    _args: &str,
    _registry: &ConnectionRegistry,
) {
    let entity = match conn.entity() {
        Some(e) => e,
        None => return,
    };

    let has_holy = world
        .query_one::<&core::HolyLight>(entity)
        .is_ok_and(|mut q| q.get().is_some());
    if has_holy {
        let _ = world.remove_one::<core::HolyLight>(entity);
        conn.send_line("Holy light disabled.");
    } else {
        let _ = world.insert(entity, (core::HolyLight,));
        conn.send_line("Holy light enabled.");
    }
}

pub fn cmd_teleport(
    world: &mut World,
    conn: &mut dyn Connection,
    _name: &str,
    args: &str,
    _registry: &ConnectionRegistry,
) {
    let args = args.trim();
    let Some((target_name, dest_name)) = args.split_once(' ') else {
        conn.send_line("Usage: @teleport <target_player> <dest_room_key_or_player>");
        return;
    };

    let Some(target) = find_player_by_name(world, target_name) else {
        conn.send_line("Target player not found.");
        return;
    };

    let dest_room = if let Some(dest_player) = find_player_by_name(world, dest_name) {
        world
            .query_one::<&core::Position>(dest_player)
            .ok()
            .and_then(|mut q| q.get().map(|p| p.room))
    } else {
        oxide_server::get_templates().and_then(|t| t.find_room_by_key(world, dest_name))
    };

    let Some(dest) = dest_room else {
        conn.send_line("Destination not found.");
        return;
    };

    if let Ok(mut q) = world.query_one::<&mut core::Position>(target) {
        if let Some(pos) = q.get() {
            pos.room = dest;
        }
    }
    conn.send_line(&format!("You teleported {target_name}."));
}

pub fn cmd_switch(
    world: &mut World,
    conn: &mut dyn Connection,
    _name: &str,
    args: &str,
    _registry: &ConnectionRegistry,
) {
    let player_ent = match conn.entity() {
        Some(e) => e,
        None => return,
    };
    let arg = args.trim();
    if arg.is_empty() {
        conn.send_line("Usage: switch <mob_name>");
        return;
    }

    let room_entity = match world
        .query_one::<&core::Position>(player_ent)
        .ok()
        .and_then(|mut q| q.get().map(|p| p.room))
    {
        Some(r) => r,
        None => {
            conn.send_line("You have no room position.");
            return;
        }
    };

    let target_mob = find_mob_in_room(world, room_entity, arg);
    let Some(mob) = target_mob else {
        conn.send_line("No such mob in this room.");
        return;
    };

    let _ = world.insert(
        mob,
        (core::Switched {
            original_entity: player_ent,
        },),
    );
    conn.set_entity(mob);
    conn.send_line("You possess the body.");
    cmd_look(world, conn, "look", "", _registry);
}

pub fn cmd_return(
    world: &mut World,
    conn: &mut dyn Connection,
    _name: &str,
    _args: &str,
    _registry: &ConnectionRegistry,
) {
    let current_ent = match conn.entity() {
        Some(e) => e,
        None => return,
    };

    let original = world
        .query_one::<&core::Switched>(current_ent)
        .ok()
        .and_then(|mut q| q.get().map(|s| s.original_entity));

    let Some(orig) = original else {
        conn.send_line("You are not switched.");
        return;
    };

    let _ = world.remove_one::<core::Switched>(current_ent);
    conn.set_entity(orig);
    conn.send_line("You return to your original form.");
    cmd_look(world, conn, "look", "", _registry);
}

pub fn cmd_desc(
    world: &mut World,
    conn: &mut dyn Connection,
    _name: &str,
    args: &str,
    _registry: &ConnectionRegistry,
) {
    let entity = match conn.entity() {
        Some(e) => e,
        None => return,
    };
    let desc = args.trim();
    if desc.is_empty() {
        conn.send_line("Usage: @desc <room description>");
        return;
    }

    let room_entity = match world
        .query_one::<&core::Position>(entity)
        .ok()
        .and_then(|mut q| q.get().map(|p| p.room))
    {
        Some(r) => r,
        None => {
            conn.send_line("You have no room position.");
            return;
        }
    };

    if let Ok(mut q) = world.query_one::<&mut core::Room>(room_entity) {
        if let Some(room) = q.get() {
            room.description = desc.to_string();
        }
    }

    if let Ok(mut q) = world.query_one::<&core::RoomKey>(room_entity) {
        if let Some(key) = q.get() {
            if let Some((area_id, room_id)) = key.0.split_once(':') {
                let area_id = area_id.to_string();
                let room_id = room_id.to_string();
                let desc_clone = desc.to_string();
                let _ = oxide_server::update_templates(move |reg| {
                    if let Some(area) = reg.areas.get_mut(&area_id) {
                        if let Some(room) = area.rooms.get_mut(&room_id) {
                            room.description = desc_clone;
                        }
                    }
                });
            }
        }
    }

    conn.send_line("Room description updated.");
}

pub fn cmd_dig(
    world: &mut World,
    conn: &mut dyn Connection,
    _name: &str,
    args: &str,
    _registry: &ConnectionRegistry,
) {
    let entity = match conn.entity() {
        Some(e) => e,
        None => return,
    };

    let args = args.trim();
    let parts: Vec<&str> = args.split_whitespace().collect();
    if parts.len() < 3 {
        conn.send_line("Usage: @dig <direction> <room_id> <room_name>");
        return;
    }

    let dir_str = parts[0];
    let room_id = parts[1];
    let room_name = parts[2..].join(" ");

    let direction = match core::Direction::try_from(dir_str) {
        Some(d) => d,
        None => {
            conn.send_line(&format!("Invalid direction: {dir_str}"));
            return;
        }
    };

    let current_room = match world
        .query_one::<&core::Position>(entity)
        .ok()
        .and_then(|mut q| q.get().map(|p| p.room))
    {
        Some(r) => r,
        None => return,
    };

    let current_area_id = {
        let mut q = world.query_one::<&core::RoomKey>(current_room).unwrap();
        let key = q.get().unwrap();
        key.0
            .split_once(':')
            .map(|(area, _)| area.to_string())
            .unwrap_or_else(|| "starting_vale".to_string())
    };

    let new_room_key = format!("{current_area_id}:{room_id}");

    let new_room = world.spawn((
        core::Room::new(&room_name, "A newly dug room."),
        core::RoomFlags::default(),
        core::RoomKey(new_room_key.clone()),
        core::ScriptParams::default(),
        core::RoomTags::new(Vec::new()),
    ));
    let _ = world.insert(new_room, (core::Position::new(new_room),));

    let opposite_dir = direction.opposite();

    let mut current_exits = world
        .query_one::<&mut core::RoomExits>(current_room)
        .ok()
        .and_then(|mut q| q.get().map(|e| e.0.clone()))
        .unwrap_or_default();
    current_exits.retain(|e| e.direction != direction);
    current_exits.push(core::Exit::new(direction, new_room));
    current_exits.sort_by_key(|e| e.direction as u8);
    let _ = world.insert(current_room, (core::RoomExits(current_exits),));

    let new_exits = vec![core::Exit::new(opposite_dir, current_room)];
    let _ = world.insert(new_room, (core::RoomExits(new_exits),));

    // Update in-memory templates
    let current_room_key = match world.query_one::<&core::RoomKey>(current_room) {
        Ok(mut q) => q.get().map(|k| k.0.clone()).unwrap_or_default(),
        Err(_) => String::new(),
    };

    if !current_room_key.is_empty() {
        let current_area_id_clone = current_area_id.clone();
        let current_room_id = current_room_key
            .split_once(':')
            .map(|(_, r)| r.to_string())
            .unwrap_or_default();
        let new_room_id = room_id.to_string();
        let room_name_clone = room_name.clone();
        let opposite_dir_str = opposite_dir.long_name().to_string();
        let dir_long_str = direction.long_name().to_string();
        let new_room_key_clone = new_room_key.clone();
        let current_room_key_clone = current_room_key.clone();

        let _ = oxide_server::update_templates(move |reg| {
            if let Some(area) = reg.areas.get_mut(&current_area_id_clone) {
                let mut exits = std::collections::HashMap::new();
                exits.insert(
                    opposite_dir_str,
                    core::templates::ExitTemplate::Simple(current_room_key_clone),
                );

                let new_room_template = core::templates::RoomTemplate {
                    id: new_room_id.clone(),
                    area: current_area_id_clone.clone(),
                    name: room_name_clone,
                    description: "A newly dug room.".to_string(),
                    exits,
                    portals: Vec::new(),
                    flags: Vec::new(),
                    content: core::templates::RoomContent {
                        mobs: Vec::new(),
                        items: Vec::new(),
                    },
                    allow_revive: false,
                    script: None,
                    params: std::collections::HashMap::new(),
                };
                area.rooms.insert(new_room_id, new_room_template);

                if let Some(current_room_template) = area.rooms.get_mut(&current_room_id) {
                    current_room_template.exits.insert(
                        dir_long_str,
                        core::templates::ExitTemplate::Simple(new_room_key_clone),
                    );
                }
            }
        });
    }

    conn.send_line(&format!(
        "You dig {dir_str} and create room '{}' ({})",
        room_name, new_room_key
    ));
}

pub fn cmd_link(
    world: &mut World,
    conn: &mut dyn Connection,
    _name: &str,
    args: &str,
    _registry: &ConnectionRegistry,
) {
    let entity = match conn.entity() {
        Some(e) => e,
        None => return,
    };
    let args = args.trim();
    let parts: Vec<&str> = args.split_whitespace().collect();
    if parts.len() < 2 {
        conn.send_line("Usage: @link <direction> <target_room_key>");
        return;
    }

    let dir_str = parts[0];
    let dest_name = parts[1];

    let direction = match core::Direction::try_from(dir_str) {
        Some(d) => d,
        None => {
            conn.send_line(&format!("Invalid direction: {dir_str}"));
            return;
        }
    };

    let current_room = match world
        .query_one::<&core::Position>(entity)
        .ok()
        .and_then(|mut q| q.get().map(|p| p.room))
    {
        Some(r) => r,
        None => return,
    };

    let target_room =
        oxide_server::get_templates().and_then(|t| t.find_room_by_key(world, dest_name));
    let Some(dest) = target_room else {
        conn.send_line("Target room not found.");
        return;
    };

    let mut current_exits = world
        .query_one::<&mut core::RoomExits>(current_room)
        .ok()
        .and_then(|mut q| q.get().map(|e| e.0.clone()))
        .unwrap_or_default();
    current_exits.retain(|e| e.direction != direction);
    current_exits.push(core::Exit::new(direction, dest));
    current_exits.sort_by_key(|e| e.direction as u8);
    let _ = world.insert(current_room, (core::RoomExits(current_exits),));

    // Update in-memory templates
    let current_room_key = match world.query_one::<&core::RoomKey>(current_room) {
        Ok(mut q) => q.get().map(|k| k.0.clone()).unwrap_or_default(),
        Err(_) => String::new(),
    };

    if !current_room_key.is_empty() {
        let (current_area_id, current_room_id) = current_room_key
            .split_once(':')
            .map(|(a, r)| (a.to_string(), r.to_string()))
            .unwrap_or_default();
        let dest_name_clone = dest_name.to_string();
        let dir_long_str = direction.long_name().to_string();

        let _ = oxide_server::update_templates(move |reg| {
            if let Some(area) = reg.areas.get_mut(&current_area_id) {
                if let Some(current_room_template) = area.rooms.get_mut(&current_room_id) {
                    current_room_template.exits.insert(
                        dir_long_str,
                        core::templates::ExitTemplate::Simple(dest_name_clone),
                    );
                }
            }
        });
    }

    conn.send_line(&format!(
        "Exit link in direction '{}' connected to '{}'.",
        dir_str, dest_name
    ));
}

pub fn cmd_unlink(
    world: &mut World,
    conn: &mut dyn Connection,
    _name: &str,
    args: &str,
    _registry: &ConnectionRegistry,
) {
    let entity = match conn.entity() {
        Some(e) => e,
        None => return,
    };
    let args = args.trim();
    if args.is_empty() {
        conn.send_line("Usage: @unlink <direction>");
        return;
    }

    let direction = match core::Direction::try_from(args) {
        Some(d) => d,
        None => {
            conn.send_line(&format!("Invalid direction: {args}"));
            return;
        }
    };

    let current_room = match world
        .query_one::<&core::Position>(entity)
        .ok()
        .and_then(|mut q| q.get().map(|p| p.room))
    {
        Some(r) => r,
        None => return,
    };

    let mut current_exits = world
        .query_one::<&mut core::RoomExits>(current_room)
        .ok()
        .and_then(|mut q| q.get().map(|e| e.0.clone()))
        .unwrap_or_default();
    let original_len = current_exits.len();
    current_exits.retain(|e| e.direction != direction);
    if current_exits.len() == original_len {
        conn.send_line(&format!("No exit found in direction '{}'.", args));
        return;
    }
    let _ = world.insert(current_room, (core::RoomExits(current_exits),));

    // Update in-memory templates
    let current_room_key = match world.query_one::<&core::RoomKey>(current_room) {
        Ok(mut q) => q.get().map(|k| k.0.clone()).unwrap_or_default(),
        Err(_) => String::new(),
    };

    if !current_room_key.is_empty() {
        let (current_area_id, current_room_id) = current_room_key
            .split_once(':')
            .map(|(a, r)| (a.to_string(), r.to_string()))
            .unwrap_or_default();
        let dir_long_str = direction.long_name().to_string();

        let _ = oxide_server::update_templates(move |reg| {
            if let Some(area) = reg.areas.get_mut(&current_area_id) {
                if let Some(current_room_template) = area.rooms.get_mut(&current_room_id) {
                    current_room_template.exits.remove(&dir_long_str);
                }
            }
        });
    }

    conn.send_line(&format!("Exit link in direction '{}' removed.", args));
}

pub fn cmd_room(
    world: &mut World,
    conn: &mut dyn Connection,
    _name: &str,
    args: &str,
    _registry: &ConnectionRegistry,
) {
    let args = args.trim();
    let parts: Vec<&str> = args.split_whitespace().collect();
    if parts.is_empty() {
        conn.send_line("Usage: @room delete [room_key]");
        return;
    }

    match parts[0] {
        "delete" => {
            let entity = match conn.entity() {
                Some(e) => e,
                None => return,
            };
            let current_room = match world
                .query_one::<&core::Position>(entity)
                .ok()
                .and_then(|mut q| q.get().map(|p| p.room))
            {
                Some(r) => r,
                None => return,
            };

            let target_room = if parts.len() > 1 {
                match oxide_server::get_templates()
                    .and_then(|t| t.find_room_by_key(world, parts[1]))
                {
                    Some(r) => r,
                    None => {
                        conn.send_line("Specified room not found.");
                        return;
                    }
                }
            } else {
                current_room
            };

            if target_room == current_room {
                conn.send_line("You cannot delete the room you are currently standing in. Move somewhere else first.");
                return;
            }

            {
                let mut q_exits = world.query::<(&mut core::RoomExits,)>();
                for (_, (exits,)) in q_exits.iter() {
                    exits.0.retain(|e| e.dest != target_room);
                }
            }

            let occupants = core::util::entities_in_room(world, target_room);
            for occ in occupants {
                if world.query_one::<&core::Player>(occ).is_ok() {
                    let void_room = {
                        let mut q_void = world.query::<(&core::VoidRoom,)>();
                        q_void
                            .iter()
                            .next()
                            .map(|(e, _)| oxide_core::Entity::from(e))
                    };
                    if let Some(vr) = void_room {
                        if let Ok(mut q_pos) = world.query_one::<&mut core::Position>(occ) {
                            if let Some(pos) = q_pos.get() {
                                pos.room = vr;
                            }
                        }
                    }
                } else {
                    let _ = world.despawn(occ);
                }
            }

            let _ = world.despawn(target_room);
            conn.send_line("Room deleted.");
        }
        _ => {
            conn.send_line("Unknown subcommand. Usage: @room delete [room_key]");
        }
    }
}

pub fn cmd_portal(
    world: &mut World,
    conn: &mut dyn Connection,
    _name: &str,
    args: &str,
    _registry: &ConnectionRegistry,
) {
    let args = args.trim();
    let parts: Vec<&str> = args.split_whitespace().collect();
    if parts.is_empty() {
        conn.send_line("Usage:");
        conn.send_line("  @portal add <target_room_key> <portal_name> [hide/show]");
        conn.send_line("  @portal remove <portal_name>");
        conn.send_line("  @portal hide <portal_name>");
        return;
    }

    let entity = match conn.entity() {
        Some(e) => e,
        None => return,
    };

    let current_room = match world
        .query_one::<&core::Position>(entity)
        .ok()
        .and_then(|mut q| q.get().map(|p| p.room))
    {
        Some(r) => r,
        None => return,
    };

    match parts[0] {
        "add" => {
            if parts.len() < 3 {
                conn.send_line("Usage: @portal add <target_room_key> <portal_name> [hide/show]");
                return;
            }
            let target_key = parts[1];
            let portal_name = parts[2];
            let hide = parts.get(3).is_some_and(|&s| s.to_lowercase() == "hide");

            let dest = match oxide_server::get_templates()
                .and_then(|t| t.find_room_by_key(world, target_key))
            {
                Some(r) => r,
                None => {
                    conn.send_line("Target room not found.");
                    return;
                }
            };

            let mut portal = core::PortalExit::new(
                portal_name,
                dest,
                format!("A shimmering portal to {target_key}."),
            );
            if hide {
                portal.flags = core::PORTAL_HIDDEN;
            }

            let mut portals = world
                .query_one::<&mut core::RoomPortals>(current_room)
                .ok()
                .and_then(|mut q| q.get().map(|p| p.0.clone()))
                .unwrap_or_default();
            portals.retain(|p| p.keyword != portal_name);
            portals.push(portal);
            let _ = world.insert(current_room, (core::RoomPortals(portals),));

            // Update template
            let current_room_key = match world.query_one::<&core::RoomKey>(current_room) {
                Ok(mut q) => q.get().map(|k| k.0.clone()).unwrap_or_default(),
                Err(_) => String::new(),
            };
            if !current_room_key.is_empty() {
                let (current_area_id, current_room_id) = current_room_key
                    .split_once(':')
                    .map(|(a, r)| (a.to_string(), r.to_string()))
                    .unwrap_or_default();
                let target_key_str = target_key.to_string();
                let portal_name_str = portal_name.to_string();
                let is_hidden = hide;
                let _ = oxide_server::update_templates(move |reg| {
                    if let Some(area) = reg.areas.get_mut(&current_area_id) {
                        if let Some(room) = area.rooms.get_mut(&current_room_id) {
                            room.portals.retain(|p| p.keyword != portal_name_str);
                            room.portals.push(core::templates::RoomPortalTemplate {
                                keyword: portal_name_str.clone(),
                                dest: target_key_str.clone(),
                                description: format!("A shimmering portal to {target_key_str}."),
                                flags: if is_hidden {
                                    vec!["hidden".to_string()]
                                } else {
                                    Vec::new()
                                },
                            });
                        }
                    }
                });
            }

            conn.send_line(&format!(
                "Portal '{}' added targeting '{}'.",
                portal_name, target_key
            ));
        }
        "remove" => {
            if parts.len() < 2 {
                conn.send_line("Usage: @portal remove <portal_name>");
                return;
            }
            let portal_name = parts[1];
            let mut portals = world
                .query_one::<&mut core::RoomPortals>(current_room)
                .ok()
                .and_then(|mut q| q.get().map(|p| p.0.clone()))
                .unwrap_or_default();
            let original_len = portals.len();
            portals.retain(|p| p.keyword != portal_name);
            if portals.len() == original_len {
                conn.send_line(&format!("Portal '{}' not found.", portal_name));
                return;
            }
            let _ = world.insert(current_room, (core::RoomPortals(portals),));

            // Update template
            let current_room_key = match world.query_one::<&core::RoomKey>(current_room) {
                Ok(mut q) => q.get().map(|k| k.0.clone()).unwrap_or_default(),
                Err(_) => String::new(),
            };
            if !current_room_key.is_empty() {
                let (current_area_id, current_room_id) = current_room_key
                    .split_once(':')
                    .map(|(a, r)| (a.to_string(), r.to_string()))
                    .unwrap_or_default();
                let portal_name_str = portal_name.to_string();
                let _ = oxide_server::update_templates(move |reg| {
                    if let Some(area) = reg.areas.get_mut(&current_area_id) {
                        if let Some(room) = area.rooms.get_mut(&current_room_id) {
                            room.portals.retain(|p| p.keyword != portal_name_str);
                        }
                    }
                });
            }

            conn.send_line(&format!("Portal '{}' removed.", portal_name));
        }
        "hide" => {
            if parts.len() < 2 {
                conn.send_line("Usage: @portal hide <portal_name>");
                return;
            }
            let portal_name = parts[1];
            let mut portals = world
                .query_one::<&mut core::RoomPortals>(current_room)
                .ok()
                .and_then(|mut q| q.get().map(|p| p.0.clone()))
                .unwrap_or_default();
            let mut updated = false;
            for p in &mut portals {
                if p.keyword == portal_name {
                    p.flags |= core::PORTAL_HIDDEN;
                    updated = true;
                }
            }
            if !updated {
                conn.send_line(&format!("Portal '{}' not found.", portal_name));
                return;
            }
            let _ = world.insert(current_room, (core::RoomPortals(portals),));

            // Update template
            let current_room_key = match world.query_one::<&core::RoomKey>(current_room) {
                Ok(mut q) => q.get().map(|k| k.0.clone()).unwrap_or_default(),
                Err(_) => String::new(),
            };
            if !current_room_key.is_empty() {
                let (current_area_id, current_room_id) = current_room_key
                    .split_once(':')
                    .map(|(a, r)| (a.to_string(), r.to_string()))
                    .unwrap_or_default();
                let portal_name_str = portal_name.to_string();
                let _ = oxide_server::update_templates(move |reg| {
                    if let Some(area) = reg.areas.get_mut(&current_area_id) {
                        if let Some(room) = area.rooms.get_mut(&current_room_id) {
                            for p in &mut room.portals {
                                if p.keyword == portal_name_str
                                    && !p.flags.contains(&"hidden".to_string())
                                {
                                    p.flags.push("hidden".to_string());
                                }
                            }
                        }
                    }
                });
            }

            conn.send_line(&format!("Portal '{}' is now hidden.", portal_name));
        }
        _ => {
            conn.send_line("Unknown portal subcommand. Use: add, remove, hide");
        }
    }
}

pub fn cmd_mob(
    world: &mut World,
    conn: &mut dyn Connection,
    _name: &str,
    args: &str,
    _registry: &ConnectionRegistry,
) {
    let args = args.trim();
    let parts: Vec<&str> = args.split_whitespace().collect();
    if parts.is_empty() {
        conn.send_line("Usage:");
        conn.send_line("  @mob add <template_id>");
        conn.send_line("  @mob remove <mob_name>");
        conn.send_line("  @mob edit <template_id> <field> <value>");
        return;
    }

    let entity = match conn.entity() {
        Some(e) => e,
        None => return,
    };
    let current_room = match world
        .query_one::<&core::Position>(entity)
        .ok()
        .and_then(|mut q| q.get().map(|p| p.room))
    {
        Some(r) => r,
        None => return,
    };

    match parts[0] {
        "add" => {
            if parts.len() < 2 {
                conn.send_line("Usage: @mob add <template_id>");
                return;
            }
            let template_id = parts[1];
            let templates = match oxide_server::get_templates() {
                Some(t) => t,
                None => return,
            };

            let mob_tpl = match templates.get_mob(template_id) {
                Some(t) => t,
                None => {
                    conn.send_line(&format!("Mob template '{}' not found.", template_id));
                    return;
                }
            };

            let mob_ent = mob_tpl.spawn(world, current_room, &templates);
            conn.send_line(&format!(
                "You spawned a {} (Entity ID: {}).",
                mob_tpl.name,
                mob_ent.id()
            ));
        }
        "remove" => {
            if parts.len() < 2 {
                conn.send_line("Usage: @mob remove <mob_name_or_entity_id>");
                return;
            }
            let target = parts[1];
            let mob_ent = if let Ok(eid) = target.parse::<u32>() {
                let occupants = core::util::entities_in_room(world, current_room);
                occupants
                    .into_iter()
                    .find(|e| e.id() == eid && world.query_one::<&core::Npc>(*e).is_ok())
            } else {
                find_mob_in_room(world, current_room, target)
            };

            let Some(m) = mob_ent else {
                conn.send_line("No matching mob found in this room.");
                return;
            };

            let mob_name = world
                .query_one::<&core::Name>(m)
                .ok()
                .and_then(|mut q| q.get().map(|n| n.0.clone()))
                .unwrap_or_else(|| "The mob".to_string());

            let _ = world.despawn(m);
            conn.send_line(&format!("Despawned {} (Entity ID: {}).", mob_name, m.id()));
        }
        "edit" => {
            if parts.len() < 4 {
                conn.send_line("Usage: @mob edit <template_id> <field> <value>");
                return;
            }
            let template_id = parts[1].to_string();
            let field = parts[2].to_lowercase();
            let value = parts[3..].join(" ");

            let template_id_clone = template_id.clone();
            let field_clone = field.clone();
            let value_clone = value.clone();

            let found = oxide_server::update_templates(move |reg| {
                if let Some(mob) = reg.mobs.get_mut(&template_id_clone) {
                    match field_clone.as_str() {
                        "name" => mob.name = value_clone,
                        "desc" | "description" => mob.description = value_clone,
                        "short" | "short_desc" => mob.short_desc = value_clone,
                        "level" => {
                            if let Ok(lvl) = value_clone.parse::<u8>() {
                                mob.level = lvl;
                            }
                        }
                        "armor" => {
                            if let Ok(arm) = value_clone.parse::<i32>() {
                                mob.armor = arm;
                            }
                        }
                        "size" => mob.size = value_clone,
                        "ai" | "ai_mode" => mob.ai_mode = value_clone,
                        "race" => mob.race = Some(value_clone),
                        "faction" => mob.faction = Some(value_clone),
                        "friendly" => {
                            if let Ok(f) = value_clone.parse::<bool>() {
                                mob.friendly = f;
                            }
                        }
                        _ => {}
                    }
                    true
                } else {
                    false
                }
            })
            .unwrap_or(false);

            if found {
                conn.send_line(&format!(
                    "Mob template '{}' field '{}' set to '{}'.",
                    template_id, field, value
                ));
            } else {
                conn.send_line(&format!("Mob template '{}' not found.", template_id));
            }
        }
        _ => {
            conn.send_line("Unknown mob subcommand. Use: add, remove, edit");
        }
    }
}

pub fn cmd_item(
    _world: &mut World,
    conn: &mut dyn Connection,
    _name: &str,
    args: &str,
    _registry: &ConnectionRegistry,
) {
    let args = args.trim();
    let parts: Vec<&str> = args.split_whitespace().collect();
    if parts.is_empty() {
        conn.send_line("Usage:");
        conn.send_line("  @item create <template_id> <name>");
        conn.send_line("  @item edit <template_id> <field> <value>");
        conn.send_line("  @item delete <template_id>");
        return;
    }

    match parts[0] {
        "create" => {
            if parts.len() < 3 {
                conn.send_line("Usage: @item create <template_id> <name>");
                return;
            }
            let template_id = parts[1].to_string();
            let name = parts[2..].join(" ");
            let template_id_clone = template_id.clone();
            let name_clone = name.clone();

            let _ = oxide_server::update_templates(move |reg| {
                reg.items.insert(
                    template_id_clone.clone(),
                    core::templates::ItemTemplate {
                        id: template_id_clone,
                        name: name_clone,
                        description: "A newly created item.".to_string(),
                        item_type: "trash".to_string(),
                        subtype: String::new(),
                        quality: "common".to_string(),
                        level_requirement: 0,
                        weight: 1.0,
                        value: 0,
                        flags: Vec::new(),
                        allowed_classes: Vec::new(),
                        allowed_races: Vec::new(),
                        allowed_alignments: Vec::new(),
                        requires_skill: None,
                        weapon: None,
                        equipment: None,
                        set: None,
                        triggers: Vec::new(),
                        params: std::collections::HashMap::new(),
                    },
                );
            });
            conn.send_line(&format!(
                "Item template '{}' created in-memory.",
                template_id
            ));
        }
        "edit" => {
            if parts.len() < 4 {
                conn.send_line("Usage: @item edit <template_id> <field> <value>");
                return;
            }
            let template_id = parts[1].to_string();
            let field = parts[2].to_lowercase();
            let value = parts[3..].join(" ");

            let template_id_clone = template_id.clone();
            let field_clone = field.clone();
            let value_clone = value.clone();

            let found = oxide_server::update_templates(move |reg| {
                if let Some(item) = reg.items.get_mut(&template_id_clone) {
                    match field_clone.as_str() {
                        "name" => item.name = value_clone,
                        "desc" | "description" => item.description = value_clone,
                        "type" | "item_type" => item.item_type = value_clone,
                        "subtype" => item.subtype = value_clone,
                        "quality" => item.quality = value_clone,
                        "level" | "level_requirement" => {
                            if let Ok(lvl) = value_clone.parse::<u8>() {
                                item.level_requirement = lvl;
                            }
                        }
                        "weight" => {
                            if let Ok(w) = value_clone.parse::<f32>() {
                                item.weight = w;
                            }
                        }
                        "value" => {
                            if let Ok(val) = value_clone.parse::<u64>() {
                                item.value = val;
                            }
                        }
                        _ => {}
                    }
                    true
                } else {
                    false
                }
            })
            .unwrap_or(false);

            if found {
                conn.send_line(&format!(
                    "Item template '{}' field '{}' set to '{}'.",
                    template_id, field, value
                ));
            } else {
                conn.send_line(&format!("Item template '{}' not found.", template_id));
            }
        }
        "delete" => {
            if parts.len() < 2 {
                conn.send_line("Usage: @item delete <template_id>");
                return;
            }
            let template_id = parts[1].to_string();
            let template_id_clone = template_id.clone();

            let found = oxide_server::update_templates(move |reg| {
                reg.items.remove(&template_id_clone).is_some()
            })
            .unwrap_or(false);

            if found {
                let path = std::path::PathBuf::from("content")
                    .join("items")
                    .join(format!("{template_id}.toml"));
                if path.exists() {
                    let _ = std::fs::remove_file(&path);
                }
                conn.send_line(&format!("Item template '{}' deleted.", template_id));
            } else {
                conn.send_line(&format!("Item template '{}' not found.", template_id));
            }
        }
        _ => {
            conn.send_line("Unknown item subcommand. Use: create, edit, delete");
        }
    }
}

pub fn cmd_validate(
    _world: &mut World,
    conn: &mut dyn Connection,
    _name: &str,
    args: &str,
    _registry: &ConnectionRegistry,
) {
    let args = args.trim();
    let templates = match oxide_server::get_templates() {
        Some(t) => t,
        None => {
            conn.send_line("Error: Template registry not initialized.");
            return;
        }
    };

    let errors = templates.validate();

    if args.is_empty() {
        if errors.is_empty() {
            conn.send_line("Validation complete: 0 errors found.");
        } else {
            conn.send_line(&format!(
                "Validation complete: {} errors found:",
                errors.len()
            ));
            for err in &errors {
                conn.send_line(&format!(
                    "  * [{}] {}: {} — {}",
                    err.template_type, err.template_id, err.field, err.message
                ));
            }
        }
    } else {
        let area_id = args;
        let filtered_errors: Vec<_> = errors
            .iter()
            .filter(|err| {
                err.template_id == area_id
                    || err.template_id.starts_with(&format!("{area_id}:"))
                    || err.template_id.starts_with(&format!("{area_id}."))
            })
            .collect();

        if filtered_errors.is_empty() {
            conn.send_line(&format!(
                "Validation for area '{}' complete: 0 errors found.",
                area_id
            ));
        } else {
            conn.send_line(&format!(
                "Validation for area '{}' complete: {} errors found:",
                area_id,
                filtered_errors.len()
            ));
            for err in filtered_errors {
                conn.send_line(&format!(
                    "  * [{}] {}: {} — {}",
                    err.template_type, err.template_id, err.field, err.message
                ));
            }
        }
    }
}

pub fn cmd_load(
    world: &mut World,
    conn: &mut dyn Connection,
    _name: &str,
    args: &str,
    _registry: &ConnectionRegistry,
) {
    let args = args.trim();
    let parts: Vec<&str> = args.split_whitespace().collect();
    if parts.len() < 2 {
        conn.send_line("Usage: @load <item|mob> <template_id>");
        return;
    }

    let load_type = parts[0];
    let template_id = parts[1];
    let templates = match oxide_server::get_templates() {
        Some(t) => t,
        None => return,
    };

    match load_type.to_lowercase().as_str() {
        "item" => {
            let item_tpl = match templates.get_item(template_id) {
                Some(t) => t,
                None => {
                    conn.send_line(&format!("Item template '{}' not found.", template_id));
                    return;
                }
            };

            let item_ent = world.spawn((
                core::Item::new(template_id),
                core::Name::new(&item_tpl.name),
            ));

            let entity = match conn.entity() {
                Some(e) => e,
                None => return,
            };

            if let Ok(mut q) = world.query_one::<&mut core::Inventory>(entity) {
                if let Some(inv) = q.get() {
                    inv.0.push(item_ent);
                    conn.send_line(&format!("Loaded '{}' into your inventory.", item_tpl.name));
                }
            } else {
                conn.send_line("You have no inventory component to receive the item.");
            }
        }
        "mob" => {
            let mob_tpl = match templates.get_mob(template_id) {
                Some(t) => t,
                None => {
                    conn.send_line(&format!("Mob template '{}' not found.", template_id));
                    return;
                }
            };

            let entity = match conn.entity() {
                Some(e) => e,
                None => return,
            };

            let current_room = match world
                .query_one::<&core::Position>(entity)
                .ok()
                .and_then(|mut q| q.get().map(|p| p.room))
            {
                Some(r) => r,
                None => return,
            };

            let mob_ent = mob_tpl.spawn(world, current_room, &templates);
            conn.send_line(&format!(
                "Loaded '{}' into the room (Entity ID: {}).",
                mob_tpl.name,
                mob_ent.id()
            ));
        }
        _ => {
            conn.send_line(
                "Invalid load type. Use: @load item <template_id> or @load mob <template_id>",
            );
        }
    }
}

pub fn cmd_area(
    world: &mut World,
    conn: &mut dyn Connection,
    _name: &str,
    args: &str,
    _registry: &ConnectionRegistry,
) {
    let args = args.trim();
    let parts: Vec<&str> = args.split_whitespace().collect();
    if parts.is_empty() {
        conn.send_line("Usage:");
        conn.send_line("  @area create <id> <name>");
        conn.send_line("  @area list");
        conn.send_line("  @area edit <id> <field> <value>");
        conn.send_line("  @area delete <id>");
        conn.send_line("  @area reset <id>");
        conn.send_line("  @area save <id>");
        return;
    }

    match parts[0] {
        "create" => {
            if parts.len() < 3 {
                conn.send_line("Usage: @area create <id> <name>");
                return;
            }
            let id = parts[1].to_string();
            let name = parts[2..].join(" ");
            let id_clone = id.clone();
            let name_clone = name.clone();

            let _ = oxide_server::update_templates(move |reg| {
                reg.areas.insert(
                    id_clone.clone(),
                    core::templates::AreaTemplate {
                        id: id_clone,
                        name: name_clone,
                        description: "A newly created area.".to_string(),
                        level_range: None,
                        flags: Vec::new(),
                        weather_zone: None,
                        reset_interval: None,
                        credits: None,
                        spawns: Vec::new(),
                        rooms: std::collections::HashMap::new(),
                    },
                );
            });
            conn.send_line(&format!("Area '{}' created in-memory.", id));
        }
        "list" => {
            conn.send_line("Areas in registry:");
            if let Some(templates) = oxide_server::get_templates() {
                for (id, area) in &templates.areas {
                    conn.send_line(&format!("  {} — {}", id, area.name));
                }
            }
        }
        "edit" => {
            if parts.len() < 4 {
                conn.send_line("Usage: @area edit <id> <field> <value>");
                return;
            }
            let id = parts[1].to_string();
            let field = parts[2].to_lowercase();
            let value = parts[3..].join(" ");

            let id_clone = id.clone();
            let field_clone = field.clone();
            let value_clone = value.clone();

            let found = oxide_server::update_templates(move |reg| {
                if let Some(area) = reg.areas.get_mut(&id_clone) {
                    match field_clone.as_str() {
                        "name" => area.name = value_clone,
                        "desc" | "description" => area.description = value_clone,
                        "credits" => area.credits = Some(value_clone),
                        "weather_zone" => area.weather_zone = Some(value_clone),
                        "level_range" => {
                            let range_parts: Vec<&str> =
                                value_clone.split(&['-', ' '][..]).collect();
                            if range_parts.len() == 2 {
                                if let (Ok(min), Ok(max)) =
                                    (range_parts[0].parse::<u8>(), range_parts[1].parse::<u8>())
                                {
                                    area.level_range = Some([min, max]);
                                }
                            }
                        }
                        "flags" | "flag" => {
                            if area.flags.contains(&value_clone) {
                                area.flags.retain(|f| f != &value_clone);
                            } else {
                                area.flags.push(value_clone);
                            }
                        }
                        _ => {}
                    }
                    true
                } else {
                    false
                }
            })
            .unwrap_or(false);

            if found {
                conn.send_line(&format!(
                    "Area '{}' field '{}' updated to '{}'.",
                    id, field, value
                ));
            } else {
                conn.send_line(&format!("Area '{}' not found.", id));
            }
        }
        "delete" => {
            if parts.len() < 2 {
                conn.send_line("Usage: @area delete <id>");
                return;
            }
            let id = parts[1].to_string();
            let id_clone = id.clone();
            let _ = oxide_server::update_templates(move |reg| {
                reg.areas.remove(&id_clone);
            });

            let path = std::path::PathBuf::from("content").join("areas").join(&id);
            if path.exists() {
                let _ = std::fs::remove_dir_all(&path);
            }

            conn.send_line(&format!("Area '{}' deleted.", id));
        }
        "reset" => {
            if parts.len() < 2 {
                conn.send_line("Usage: @area reset <id>");
                return;
            }
            let area_id = parts[1];

            let templates = match oxide_server::get_templates() {
                Some(t) => t,
                None => return,
            };

            let Some(area) = templates.areas.get(area_id) else {
                conn.send_line(&format!("Area '{}' not found.", area_id));
                return;
            };

            let mut room_entities = Vec::new();
            for (r, _) in world.query::<&core::RoomKey>().iter() {
                let ent = core::Entity::from(r);
                if let Ok(mut q) = world.query_one::<&core::RoomKey>(ent) {
                    if let Some(key) = q.get() {
                        if key.0.starts_with(&format!("{area_id}:")) {
                            room_entities.push(ent);
                        }
                    }
                }
            }

            let mut despawned_count = 0;
            let mut spawned_count = 0;

            for room_ent in room_entities {
                let occupants = core::util::entities_in_room(world, room_ent);
                for occ in occupants {
                    if world.query_one::<&core::Npc>(occ).is_ok() {
                        let _ = world.despawn(occ);
                        despawned_count += 1;
                    }
                }

                let room_key = match world.query_one::<&core::RoomKey>(room_ent) {
                    Ok(mut q) => q.get().map(|k| k.0.clone()).unwrap_or_default(),
                    Err(_) => continue,
                };
                let Some((_, room_id)) = room_key.split_once(':') else {
                    continue;
                };

                if let Some(room_tpl) = area.rooms.get(room_id) {
                    for mob_spawn in &room_tpl.content.mobs {
                        if let Some(mob_tpl) = templates.mobs.get(&mob_spawn.template_id) {
                            for _ in 0..mob_spawn.count {
                                mob_tpl.spawn(world, room_ent, &templates);
                                spawned_count += 1;
                            }
                        }
                    }
                }
            }

            conn.send_line(&format!(
                "Area '{}' reset triggered: despawned {} mobs, spawned {} mobs.",
                area_id, despawned_count, spawned_count
            ));
        }
        "save" => {
            if parts.len() < 2 {
                conn.send_line("Usage: @area save <id>");
                return;
            }
            let area_id = parts[1];

            let templates = match oxide_server::get_templates() {
                Some(t) => t,
                None => {
                    conn.send_line("Error: Template registry not initialized.");
                    return;
                }
            };

            let Some(area) = templates.areas.get(area_id) else {
                conn.send_line(&format!("Area '{}' not found.", area_id));
                return;
            };

            let area_dir = std::path::PathBuf::from("content")
                .join("areas")
                .join(area_id);
            if let Err(e) = std::fs::create_dir_all(&area_dir) {
                conn.send_line(&format!("Error: failed to create area directory: {e}"));
                return;
            }
            let rooms_dir = area_dir.join("rooms");
            if let Err(e) = std::fs::create_dir_all(&rooms_dir) {
                conn.send_line(&format!("Error: failed to create rooms directory: {e}"));
                return;
            }

            let mut area_meta = area.clone();
            area_meta.rooms.clear();

            let area_str = match toml::to_string_pretty(&area_meta) {
                Ok(s) => s,
                Err(e) => {
                    conn.send_line(&format!("Error: failed to serialize area: {e}"));
                    return;
                }
            };
            if let Err(e) = std::fs::write(area_dir.join("area.toml"), area_str) {
                conn.send_line(&format!("Error: failed to write area.toml: {e}"));
                return;
            }

            for (room_id, room_tpl) in &area.rooms {
                let room_path = rooms_dir.join(format!("{room_id}.toml"));
                let room_str = match toml::to_string_pretty(room_tpl) {
                    Ok(s) => s,
                    Err(e) => {
                        conn.send_line(&format!(
                            "Error: failed to serialize room '{room_id}': {e}"
                        ));
                        return;
                    }
                };
                if let Err(e) = std::fs::write(&room_path, room_str) {
                    conn.send_line(&format!(
                        "Error: failed to write room file for '{room_id}': {e}"
                    ));
                    return;
                }
            }

            conn.send_line(&format!(
                "Area '{}' template and {} rooms successfully saved to disk.",
                area_id,
                area.rooms.len()
            ));
        }
        _ => {
            conn.send_line("Unknown area subcommand.");
        }
    }
}

pub fn cmd_set(
    world: &mut World,
    conn: &mut dyn Connection,
    _name: &str,
    args: &str,
    _registry: &ConnectionRegistry,
) {
    let args = args.trim();
    let parts: Vec<&str> = args.split_whitespace().collect();
    if parts.len() < 3 {
        conn.send_line("Usage: @set <self|target_name> <field> <value>");
        return;
    }

    let target_name = parts[0];
    let field = parts[1];
    let value_str = parts[2];

    let executor = match conn.entity() {
        Some(e) => e,
        None => return,
    };

    let target_entity = if target_name.to_lowercase() == "self" {
        Some(executor)
    } else if target_name.to_lowercase() == "room" {
        world
            .query_one::<&core::Position>(executor)
            .ok()
            .and_then(|mut q| q.get().map(|p| p.room))
    } else if let Some(player_ent) = find_player_by_name(world, target_name) {
        Some(player_ent)
    } else {
        let room_entity = world
            .query_one::<&core::Position>(executor)
            .ok()
            .and_then(|mut q| q.get().map(|p| p.room));
        room_entity.and_then(|room| find_mob_in_room(world, room, target_name))
    };

    let Some(target) = target_entity else {
        conn.send_line("Target not found.");
        return;
    };

    let is_room = world.query_one::<&core::Room>(target).is_ok();
    let full_value = parts[2..].join(" ");

    if is_room {
        match field.to_lowercase().as_str() {
            "name" => {
                if let Ok(mut q) = world.query_one::<&mut core::Room>(target) {
                    if let Some(r) = q.get() {
                        r.name = full_value.clone();
                    }
                }
                if let Ok(mut q) = world.query_one::<&core::RoomKey>(target) {
                    if let Some(key) = q.get() {
                        if let Some((area_id, room_id)) = key.0.split_once(':') {
                            let area_id = area_id.to_string();
                            let room_id = room_id.to_string();
                            let name_clone = full_value.clone();
                            let _ = oxide_server::update_templates(move |reg| {
                                if let Some(area) = reg.areas.get_mut(&area_id) {
                                    if let Some(room) = area.rooms.get_mut(&room_id) {
                                        room.name = name_clone;
                                    }
                                }
                            });
                        }
                    }
                }
                conn.send_line(&format!("Set room name to '{}'.", full_value));
            }
            "desc" | "description" => {
                if let Ok(mut q) = world.query_one::<&mut core::Room>(target) {
                    if let Some(r) = q.get() {
                        r.description = full_value.clone();
                    }
                }
                if let Ok(mut q) = world.query_one::<&core::RoomKey>(target) {
                    if let Some(key) = q.get() {
                        if let Some((area_id, room_id)) = key.0.split_once(':') {
                            let area_id = area_id.to_string();
                            let room_id = room_id.to_string();
                            let desc_clone = full_value.clone();
                            let _ = oxide_server::update_templates(move |reg| {
                                if let Some(area) = reg.areas.get_mut(&area_id) {
                                    if let Some(room) = area.rooms.get_mut(&room_id) {
                                        room.description = desc_clone;
                                    }
                                }
                            });
                        }
                    }
                }
                conn.send_line(&format!("Set room description to '{}'.", full_value));
            }
            "flag" | "flags" => {
                let flag_name = value_str.to_lowercase();
                let bit = match flag_name.as_str() {
                    "portal_in" => Some(core::ROOM_PORTAL_IN),
                    "portal_out" => Some(core::ROOM_PORTAL_OUT),
                    "no_teleport_in" => Some(core::ROOM_NO_TELEPORT_IN),
                    "no_teleport_out" => Some(core::ROOM_NO_TELEPORT_OUT),
                    _ => None,
                };
                if let Some(b) = bit {
                    let mut added = false;
                    if let Ok(mut q) = world.query_one::<&mut core::RoomFlags>(target) {
                        if let Some(flags) = q.get() {
                            if flags.0 & b != 0 {
                                flags.0 &= !b;
                            } else {
                                flags.0 |= b;
                                added = true;
                            }
                        }
                    }
                    if let Ok(mut q_key) = world.query_one::<&core::RoomKey>(target) {
                        if let Some(key) = q_key.get() {
                            if let Some((area_id, room_id)) = key.0.split_once(':') {
                                let area_id = area_id.to_string();
                                let room_id = room_id.to_string();
                                let flag_clone = flag_name.clone();
                                let add = added;
                                let _ = oxide_server::update_templates(move |reg| {
                                    if let Some(area) = reg.areas.get_mut(&area_id) {
                                        if let Some(room) = area.rooms.get_mut(&room_id) {
                                            if add {
                                                if !room.flags.contains(&flag_clone) {
                                                    room.flags.push(flag_clone);
                                                }
                                            } else {
                                                room.flags.retain(|f| f != &flag_clone);
                                            }
                                        }
                                    }
                                });
                            }
                        }
                    }
                    if added {
                        conn.send_line(&format!("Added room flag '{}'.", flag_name));
                    } else {
                        conn.send_line(&format!("Removed room flag '{}'.", flag_name));
                    }
                } else {
                    conn.send_line("Invalid room flag. Supported: portal_in, portal_out, no_teleport_in, no_teleport_out");
                }
            }
            "tag" | "tags" => {
                let tag_name = value_str.to_lowercase();
                let mut added = false;
                if let Ok(mut q) = world.query_one::<&mut core::RoomTags>(target) {
                    if let Some(tags) = q.get() {
                        if tags.tags.contains(&tag_name) {
                            tags.tags.retain(|t| t != &tag_name);
                        } else {
                            tags.tags.push(tag_name.clone());
                            added = true;
                        }
                    }
                }
                if let Ok(mut q_key) = world.query_one::<&core::RoomKey>(target) {
                    if let Some(key) = q_key.get() {
                        if let Some((area_id, room_id)) = key.0.split_once(':') {
                            let area_id = area_id.to_string();
                            let room_id = room_id.to_string();
                            let tag_clone = tag_name.clone();
                            let add = added;
                            let _ = oxide_server::update_templates(move |reg| {
                                if let Some(area) = reg.areas.get_mut(&area_id) {
                                    if let Some(room) = area.rooms.get_mut(&room_id) {
                                        if add {
                                            if !room.flags.contains(&tag_clone) {
                                                room.flags.push(tag_clone);
                                            }
                                        } else {
                                            room.flags.retain(|f| f != &tag_clone);
                                        }
                                    }
                                }
                            });
                        }
                    }
                }
                if added {
                    conn.send_line(&format!("Added room tag '{}'.", tag_name));
                } else {
                    conn.send_line(&format!("Removed room tag '{}'.", tag_name));
                }
            }
            _ => {
                conn.send_line("Unsupported room field. Supported: name, desc, flag, tag");
            }
        }
        return;
    }

    match field.to_lowercase().as_str() {
        "hp" | "health" => {
            if let Ok(val) = value_str.parse::<i32>() {
                if let Ok(mut q) = world.query_one::<&mut core::Health>(target) {
                    if let Some(h) = q.get() {
                        h.current = val;
                        conn.send_line(&format!("Set Health to {val}."));
                    }
                }
            } else {
                conn.send_line("Value must be an integer.");
            }
        }
        "max_hp" | "max_health" => {
            if let Ok(val) = value_str.parse::<i32>() {
                if let Ok(mut q) = world.query_one::<&mut core::Health>(target) {
                    if let Some(h) = q.get() {
                        h.max = val;
                        conn.send_line(&format!("Set Max Health to {val}."));
                    }
                }
            } else {
                conn.send_line("Value must be an integer.");
            }
        }
        "level" => {
            if let Ok(val) = value_str.parse::<u8>() {
                if let Ok(mut q) = world.query_one::<&mut core::Level>(target) {
                    if let Some(l) = q.get() {
                        l.0 = val;
                        conn.send_line(&format!("Set Level to {val}."));
                    }
                }
            } else {
                conn.send_line("Value must be an integer (0-255).");
            }
        }
        "strength" | "str" => {
            if let Ok(val) = value_str.parse::<u8>() {
                if let Ok(mut q) = world.query_one::<&mut core::Attributes>(target) {
                    if let Some(a) = q.get() {
                        a.strength = val;
                        conn.send_line(&format!("Set Strength to {val}."));
                    }
                }
            }
        }
        "dexterity" | "dex" => {
            if let Ok(val) = value_str.parse::<u8>() {
                if let Ok(mut q) = world.query_one::<&mut core::Attributes>(target) {
                    if let Some(a) = q.get() {
                        a.dexterity = val;
                        conn.send_line(&format!("Set Dexterity to {val}."));
                    }
                }
            }
        }
        "intelligence" | "int" => {
            if let Ok(val) = value_str.parse::<u8>() {
                if let Ok(mut q) = world.query_one::<&mut core::Attributes>(target) {
                    if let Some(a) = q.get() {
                        a.intelligence = val;
                        conn.send_line(&format!("Set Intelligence to {val}."));
                    }
                }
            }
        }
        "wisdom" | "wis" => {
            if let Ok(val) = value_str.parse::<u8>() {
                if let Ok(mut q) = world.query_one::<&mut core::Attributes>(target) {
                    if let Some(a) = q.get() {
                        a.wisdom = val;
                        conn.send_line(&format!("Set Wisdom to {val}."));
                    }
                }
            }
        }
        "constitution" | "con" => {
            if let Ok(val) = value_str.parse::<u8>() {
                if let Ok(mut q) = world.query_one::<&mut core::Attributes>(target) {
                    if let Some(a) = q.get() {
                        a.constitution = val;
                        conn.send_line(&format!("Set Constitution to {val}."));
                    }
                }
            }
        }
        "charisma" | "cha" => {
            if let Ok(val) = value_str.parse::<u8>() {
                if let Ok(mut q) = world.query_one::<&mut core::Attributes>(target) {
                    if let Some(a) = q.get() {
                        a.charisma = val;
                        conn.send_line(&format!("Set Charisma to {val}."));
                    }
                }
            }
        }
        _ => {
            conn.send_line(&format!(
                "Setting field '{}' is not supported via this command.",
                field
            ));
        }
    }
}

// ---------------------------------------------------------------------------
// Phase 3 — Combat commands
// ---------------------------------------------------------------------------

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

    // Find target in the same room by name (only entities with Health are valid combat targets)
    let target = {
        let mut q = world.query::<(&core::Name, &core::Position, &core::Health)>();
        let candidates: Vec<(String, core::Entity)> = q
            .iter()
            .filter(|(raw, (_, pos, _))| {
                let e = core::Entity::from(*raw);
                pos.room == room && e != entity
            })
            .map(|(raw, (name, _, _))| (name.as_str().to_lowercase(), core::Entity::from(raw)))
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

    // Verify target is alive
    if let Ok(mut q) = world.query_one::<&core::Health>(target) {
        if q.get().is_some_and(|h| h.is_dead()) {
            conn.send_line("They are already dead.");
            return;
        }
    }

    // Check target isn't already a player in combat with someone
    // (no PvP flag check yet — simplified)
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

// ---------------------------------------------------------------------------
// Inventory / Equipment commands
// ---------------------------------------------------------------------------

pub fn cmd_inventory(
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

    let inv = match world.query_one::<&core::Inventory>(entity) {
        Ok(mut q) => q.get().map(|inv| inv.0.clone()).unwrap_or_default(),
        Err(_) => {
            conn.send_line("You are carrying nothing.");
            return;
        }
    };

    if inv.is_empty() {
        conn.send_line("You are carrying nothing.");
        return;
    }

    conn.send_line("");
    conn.send_line("You are carrying:");

    for (i, item) in inv.iter().enumerate() {
        let name = world
            .query_one::<&core::Name>(*item)
            .ok()
            .and_then(|mut q| q.get().map(|n| n.to_string()))
            .unwrap_or_else(|| format!("item_{}", item.id()));

        conn.send_line(&format!("  {}. {name}", i + 1));
    }
}

pub fn cmd_equipment(
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

    let eq = match world.query_one::<&core::Equipment>(entity) {
        Ok(mut q) => q.get().map(|eq| eq.slots.clone()).unwrap_or_default(),
        Err(_) => {
            conn.send_line("You have no equipment.");
            return;
        }
    };

    if eq.is_empty() {
        conn.send_line("You are not wearing anything.");
        return;
    }

    conn.send_line("");
    conn.send_line("Equipment:");

    for (slot, item) in &eq {
        let name = world
            .query_one::<&core::Name>(*item)
            .ok()
            .and_then(|mut q| q.get().map(|n| n.to_string()))
            .unwrap_or_else(|| format!("item_{}", item.id()));

        conn.send_line(&format!("  {:?}: {name}", slot));
    }
}

fn find_item_in_inventory(
    world: &World,
    entity: core::Entity,
    query: &str,
) -> Option<core::Entity> {
    let inv = world
        .query_one::<&core::Inventory>(entity)
        .ok()?
        .get()?
        .0
        .clone();

    // Try bare numeric index first (1-based, for inventory navigation)
    if let Ok(idx) = query.parse::<usize>() {
        if idx > 0 && idx <= inv.len() {
            return Some(inv[idx - 1]);
        }
    }

    let candidates: Vec<(String, core::Entity)> = inv
        .iter()
        .filter_map(|&item| get_entity_name(world, item).map(|name| (name, item)))
        .collect();

    match core::trie::trie_match(query, candidates) {
        core::trie::TrieMatch::One(e) => Some(e),
        core::trie::TrieMatch::Many(items) => items.into_iter().next(),
        core::trie::TrieMatch::None => None,
    }
}

/// Re-evaluate set bonuses for an entity after equipment changes.
/// Returns a message to send to the player (if any).
fn evaluate_equipment_sets(world: &mut World, entity: core::Entity) -> Option<String> {
    if let Some(templates) = oxide_server::get_templates() {
        let set_defs: HashMap<String, SetDef> = templates.sets.clone();
        let changes = core::systems::set_bonus::evaluate_set_bonuses(world, entity, &set_defs);
        if changes.is_empty() {
            return None;
        }
        let mut msgs: Vec<String> = Vec::new();
        for change in &changes {
            if change.new_count > change.old_count {
                let tier_str = change
                    .active_tiers
                    .iter()
                    .map(|t| t.to_string())
                    .collect::<Vec<_>>()
                    .join(", ");
                if tier_str.is_empty() {
                    msgs.push(format!(
                        "Your {} pieces now number {}.",
                        change.set_name, change.new_count
                    ));
                } else {
                    msgs.push(format!(
                        "Your {} set grants tier bonuses at {} piece(s)!",
                        change.set_name, tier_str
                    ));
                }
            }
        }
        if msgs.is_empty() {
            None
        } else {
            Some(msgs.join(" "))
        }
    } else {
        None
    }
}

pub fn cmd_wear(
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

    let is_ghost = world
        .query_one::<&core::PlayerState>(entity)
        .ok()
        .and_then(|mut q| q.get().map(|s| matches!(s, core::PlayerState::Dead)))
        .unwrap_or(false);

    if is_ghost {
        conn.send_line("You are a ghost! You cannot wear items.");
        return;
    }

    let item_name = args.trim();
    if item_name.is_empty() {
        conn.send_line("Wear what?");
        return;
    }

    let item = match find_item_in_inventory(world, entity, item_name) {
        Some(i) => i,
        None => {
            conn.send_line("You don't have that item.");
            return;
        }
    };

    // Check skill requirement before equipping
    if let Some(req) = world
        .query_one::<&core::ItemSkillRequirement>(item)
        .ok()
        .and_then(|mut q| q.get().cloned())
    {
        let has_skill = world
            .query_one::<&core::LearnedSkills>(entity)
            .ok()
            .and_then(|mut q| q.get().map(|s| s.rank(&req.id) >= req.level))
            .unwrap_or(false);
        if !has_skill {
            conn.send_line("You lack the skill to use that.");
            return;
        }
    }

    let slot = if let Some(templates) = oxide_server::get_templates() {
        if let Ok(mut q) = world.query_one::<&core::Item>(item) {
            if let Some(item_comp) = q.get() {
                if let Some(item_tmpl) = templates.items.get(&item_comp.template_id) {
                    if let Some(eq_def) = &item_tmpl.equipment {
                        use std::str::FromStr;
                        core::EquipmentSlot::from_str(&eq_def.slot)
                            .unwrap_or(core::EquipmentSlot::Torso)
                    } else {
                        conn.send_line("You can't wear that.");
                        return;
                    }
                } else {
                    conn.send_line("You can't wear that.");
                    return;
                }
            } else {
                conn.send_line("You can't wear that.");
                return;
            }
        } else {
            conn.send_line("You can't wear that.");
            return;
        }
    } else {
        conn.send_line("Server error: templates unavailable.");
        return;
    };

    if slot == core::EquipmentSlot::Shield {
        let wielding_two_handed = world
            .query_one::<&core::Equipment>(entity)
            .ok()
            .and_then(|mut q| {
                q.get().and_then(|eq| {
                    eq.equipped(&core::EquipmentSlot::Weapon)
                        .and_then(|w_entity| world.query_one::<&core::Weapon>(*w_entity).ok())
                        .and_then(|mut w_query| w_query.get().map(|w| w.is_two_handed()))
                })
            })
            .unwrap_or(false);

        if wielding_two_handed {
            conn.send_line(
                "You are wielding a two-handed weapon and cannot use a shield/off-hand.",
            );
            return;
        }
    }

    // Remove from inventory
    if let Ok(mut q) = world.query_one::<&mut core::Inventory>(entity) {
        if let Some(inv) = q.get() {
            inv.0.retain(|e| *e != item);
        }
    }

    // Equip
    if let Ok(mut q) = world.query_one::<&mut core::Equipment>(entity) {
        if let Some(eq) = q.get() {
            // Unequip existing item in same slot
            if let Some(old) = eq.unequip(&slot) {
                // Put old item back in inventory
                if let Ok(mut iq) = world.query_one::<&mut core::Inventory>(entity) {
                    if let Some(inv) = iq.get() {
                        inv.0.push(old);
                    }
                }
            }
            eq.equip(slot, item);
            conn.send_line("You wear it.");
        }
    }

    if let Some(msg) = evaluate_equipment_sets(world, entity) {
        conn.send_line(&msg);
    }

    // Process on_wear triggers
    for trigger in core::systems::trigger::process_triggers(world, entity, "on_wear") {
        conn.send_line(&trigger_message(&trigger, world));
    }
}

pub fn cmd_wield(
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

    let is_ghost = world
        .query_one::<&core::PlayerState>(entity)
        .ok()
        .and_then(|mut q| q.get().map(|s| matches!(s, core::PlayerState::Dead)))
        .unwrap_or(false);

    if is_ghost {
        conn.send_line("You are a ghost! You cannot wield items.");
        return;
    }

    let item_name = args.trim();
    if item_name.is_empty() {
        conn.send_line("Wield what?");
        return;
    }

    let item = match find_item_in_inventory(world, entity, item_name) {
        Some(i) => i,
        None => {
            conn.send_line("You don't have that item.");
            return;
        }
    };

    // Check skill requirement before equipping
    if let Some(req) = world
        .query_one::<&core::ItemSkillRequirement>(item)
        .ok()
        .and_then(|mut q| q.get().cloned())
    {
        let has_skill = world
            .query_one::<&core::LearnedSkills>(entity)
            .ok()
            .and_then(|mut q| q.get().map(|s| s.rank(&req.id) >= req.level))
            .unwrap_or(false);
        if !has_skill {
            conn.send_line("You lack the skill to use that.");
            return;
        }
    }

    // Check if it's a weapon
    let has_weapon = world
        .query_one::<&core::Weapon>(item)
        .is_ok_and(|mut q| q.get().is_some());
    if !has_weapon {
        conn.send_line("You can't wield that.");
        return;
    }

    // Remove from inventory
    if let Ok(mut q) = world.query_one::<&mut core::Inventory>(entity) {
        if let Some(inv) = q.get() {
            inv.0.retain(|e| *e != item);
        }
    }

    let is_two_handed = world
        .query_one::<&core::Weapon>(item)
        .ok()
        .and_then(|mut q| q.get().map(|w| w.is_two_handed()))
        .unwrap_or(false);

    // Equip to Weapon slot
    if let Ok(mut q) = world.query_one::<&mut core::Equipment>(entity) {
        if let Some(eq) = q.get() {
            if let Some(old) = eq.unequip(&core::EquipmentSlot::Weapon) {
                if let Ok(mut iq) = world.query_one::<&mut core::Inventory>(entity) {
                    if let Some(inv) = iq.get() {
                        inv.0.push(old);
                    }
                }
            }

            if is_two_handed {
                if let Some(old_shield) = eq.unequip(&core::EquipmentSlot::Shield) {
                    if let Ok(mut name_q) = world.query_one::<&core::Name>(old_shield) {
                        if let Some(sname) = name_q.get() {
                            conn.send_line(&format!(
                                "You unequip {} to wield the two-handed weapon.",
                                sname.0
                            ));
                        }
                    }
                    if let Ok(mut iq) = world.query_one::<&mut core::Inventory>(entity) {
                        if let Some(inv) = iq.get() {
                            inv.0.push(old_shield);
                        }
                    }
                }
            }

            eq.equip(core::EquipmentSlot::Weapon, item);
            conn.send_line("You wield it.");
        }
    }

    if let Some(msg) = evaluate_equipment_sets(world, entity) {
        conn.send_line(&msg);
    }

    // Process on_wear triggers (wielding also counts as "wearing")
    for trigger in core::systems::trigger::process_triggers(world, entity, "on_wear") {
        conn.send_line(&trigger_message(&trigger, world));
    }
}

pub fn cmd_remove(
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

    let is_ghost = world
        .query_one::<&core::PlayerState>(entity)
        .ok()
        .and_then(|mut q| q.get().map(|s| matches!(s, core::PlayerState::Dead)))
        .unwrap_or(false);

    if is_ghost {
        conn.send_line("You are a ghost! You cannot equip or remove items.");
        return;
    }

    let slot_name = args.trim().to_lowercase();
    if slot_name.is_empty() {
        conn.send_line("Remove what?");
        return;
    }

    let slot = match core::EquipmentSlot::from_str(&slot_name).ok() {
        Some(s) => s,
        None => {
            conn.send_line("Unknown slot. Try: head, neck, torso, arms, hands, finger, legs, feet, weapon, shield, back, waist.");
            return;
        }
    };

    if let Ok(mut q) = world.query_one::<&mut core::Equipment>(entity) {
        if let Some(eq) = q.get() {
            if let Some(item) = eq.unequip(&slot) {
                if let Ok(mut iq) = world.query_one::<&mut core::Inventory>(entity) {
                    if let Some(inv) = iq.get() {
                        inv.0.push(item);
                    }
                }
                conn.send_line("You remove it.");
            } else {
                conn.send_line("You aren't wearing anything there.");
            }
        }
    }

    if let Some(msg) = evaluate_equipment_sets(world, entity) {
        conn.send_line(&msg);
    }

    // Process on_remove triggers
    for trigger in core::systems::trigger::process_triggers(world, entity, "on_remove") {
        conn.send_line(&trigger_message(&trigger, world));
    }
}

pub fn cmd_examine(
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

    let item_name = args.trim();
    if item_name.is_empty() {
        conn.send_line("Examine what?");
        return;
    }

    // Search inventory first, then room floor
    let item = find_item_in_inventory(world, entity, item_name).or_else(|| {
        let room = get_pos_room(world, entity)?;
        let mut floor_q = world.query_one::<&core::FloorItems>(room).ok()?;
        let floor = floor_q.get()?;
        if let Ok(idx) = item_name.parse::<usize>() {
            if idx > 0 && idx <= floor.0.len() {
                return Some(floor.0[idx - 1]);
            }
        }
        let candidates: Vec<(String, core::Entity)> = floor
            .0
            .iter()
            .filter_map(|&item| get_entity_name(world, item).map(|name| (name, item)))
            .collect();
        match core::trie::trie_match(item_name, candidates) {
            core::trie::TrieMatch::One(e) => Some(e),
            core::trie::TrieMatch::Many(items) => items.into_iter().next(),
            core::trie::TrieMatch::None => None,
        }
    });

    let item = match item {
        Some(i) => i,
        None => {
            conn.send_line("You don't see that here.");
            return;
        }
    };

    conn.send_line("");

    let name = world
        .query_one::<&core::Name>(item)
        .ok()
        .and_then(|mut q| q.get().map(|n| n.to_string()))
        .unwrap_or_else(|| "Unknown item".to_string());

    // Check for affix names and build display name
    let display_name = if let Ok(mut q) = world.query_one::<&core::AffixNames>(item) {
        if let Some(affixes) = q.get() {
            let quality = affixes.0.first().map(|s| s.as_str()).unwrap_or("");
            if !quality.is_empty() && quality != "Common" && affixes.0.len() <= 1 {
                format!("[{quality}] {name}")
            } else if !affixes.0.is_empty() {
                let rest: Vec<&str> = affixes.0.iter().map(|s| s.as_str()).collect();
                format!("[{}] {name}", rest.join(" "))
            } else {
                name.clone()
            }
        } else {
            name.clone()
        }
    } else {
        name.clone()
    };

    conn.send_line(&format!("--- {display_name} ---"));

    // Affix details
    if let Ok(mut q) = world.query_one::<&core::AffixNames>(item) {
        if let Some(affixes) = q.get() {
            for affix_name in &affixes.0 {
                conn.send_line(&format!("  ~ {affix_name}"));
            }
        }
    }

    // Affix modifiers
    if let Ok(mut q) = world.query_one::<&core::AffixModifiers>(item) {
        if let Some(mods) = q.get() {
            for m in &mods.0 {
                conn.send_line(&format!("  * +{} {}", m.amount, m.stat));
            }
        }
    }

    // Item info
    if let Ok(mut q) = world.query_one::<&core::Item>(item) {
        if let Some(item_comp) = q.get() {
            conn.send_line(&format!("Template: {}", item_comp.template_id));
        }
    }

    // Weapon info
    if let Ok(mut q) = world.query_one::<&core::Weapon>(item) {
        if let Some(wep) = q.get() {
            conn.send_line(&format!(
                "Weapon: {} {:?} damage, Range: {:?}",
                wep.damage_dice, wep.damage_type, wep.range
            ));
        }
    }

    // Armor info
    if let Ok(mut q) = world.query_one::<&core::Armor>(item) {
        if let Some(armor) = q.get() {
            conn.send_line(&format!("Armor: base {} bonus {}", armor.base, armor.bonus));
        }
    }

    // Durability
    if let Ok(mut q) = world.query_one::<&core::Durability>(item) {
        if let Some(dur) = q.get() {
            conn.send_line(&format!("Durability: {}/{}", dur.current, dur.max));
        }
    }
}

// ---------------------------------------------------------------------------
// Item manipulation commands
// ---------------------------------------------------------------------------

pub fn cmd_get(
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

    let is_ghost = world
        .query_one::<&core::PlayerState>(entity)
        .ok()
        .and_then(|mut q| q.get().map(|s| matches!(s, core::PlayerState::Dead)))
        .unwrap_or(false);

    if is_ghost {
        conn.send_line("You are a ghost! You cannot pick up items.");
        return;
    }

    let item_name = args.trim();
    if item_name.is_empty() {
        conn.send_line("Get what?");
        return;
    }

    let room = match get_pos_room(world, entity) {
        Some(r) => r,
        None => {
            conn.send_line("You are nowhere.");
            return;
        }
    };

    // Find item on the floor
    let item = {
        let mut q = world.query_one::<&mut core::FloorItems>(room);
        match q.as_mut().ok().and_then(|q| q.get()) {
            Some(floor) => {
                if let Ok(n) = item_name.parse::<usize>() {
                    if n > 0 && n <= floor.0.len() {
                        Some(floor.0.remove(n - 1))
                    } else {
                        None
                    }
                } else {
                    let candidates: Vec<(String, core::Entity)> = floor
                        .0
                        .iter()
                        .filter_map(|&e| get_entity_name(world, e).map(|name| (name, e)))
                        .collect();
                    let matched = match core::trie::trie_match(item_name, candidates) {
                        core::trie::TrieMatch::One(e) => Some(e),
                        core::trie::TrieMatch::Many(items) => items.into_iter().next(),
                        core::trie::TrieMatch::None => None,
                    };
                    matched.and_then(|m| {
                        let idx = floor.0.iter().position(|&e| e == m)?;
                        Some(floor.0.remove(idx))
                    })
                }
            }
            None => None,
        }
    };

    let item = match item {
        Some(i) => i,
        None => {
            conn.send_line("You don't see that here.");
            return;
        }
    };

    // Add to inventory
    if let Ok(mut q) = world.query_one::<&mut core::Inventory>(entity) {
        if let Some(inv) = q.get() {
            inv.0.push(item);
            conn.send_line("You pick it up.");
        }
    }

    if let Some(templates) = oxide_server::get_templates() {
        let msgs = core::reconcile_gather_objectives(world, entity, &templates);
        for msg in msgs {
            conn.send_line(&msg);
        }
    }
}

pub fn cmd_drop(
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

    let item_name = args.trim();
    if item_name.is_empty() {
        conn.send_line("Drop what?");
        return;
    }

    let item = match find_item_in_inventory(world, entity, item_name) {
        Some(i) => i,
        None => {
            conn.send_line("You don't have that item.");
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

    // Remove from inventory
    if let Ok(mut q) = world.query_one::<&mut core::Inventory>(entity) {
        if let Some(inv) = q.get() {
            inv.0.retain(|e| *e != item);
        }
    }

    // Add to room floor
    if let Ok(mut q) = world.query_one::<&mut core::FloorItems>(room) {
        if let Some(floor) = q.get() {
            floor.0.push(item);
            conn.send_line("You drop it.");
        }
    }

    if let Some(templates) = oxide_server::get_templates() {
        let msgs = core::reconcile_gather_objectives(world, entity, &templates);
        for msg in msgs {
            conn.send_line(&msg);
        }
    }
}

pub fn cmd_put(
    _world: &mut World,
    conn: &mut dyn Connection,
    _name: &str,
    _args: &str,
    _registry: &ConnectionRegistry,
) {
    // Simplified placeholder — no container support yet
    conn.send_line("Containers are not yet implemented.");
}

pub fn cmd_give(
    _world: &mut World,
    conn: &mut dyn Connection,
    _name: &str,
    _args: &str,
    _registry: &ConnectionRegistry,
) {
    // Simplified placeholder
    conn.send_line("Giving items is not yet implemented.");
}

pub fn cmd_loot(
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

    let corpse_name = args.trim();
    if corpse_name.is_empty() {
        conn.send_line("Loot what?");
        return;
    }

    let room = match get_pos_room(world, entity) {
        Some(r) => r,
        None => {
            conn.send_line("You are nowhere.");
            return;
        }
    };

    // Find corpse in room
    let corpse = {
        let mut q = world.query::<(&core::Corpse, &core::Position, &core::Name)>();
        let candidates: Vec<(String, core::Entity)> = q
            .iter()
            .filter(|(_, (_, pos, _))| pos.room == room)
            .map(|(raw, (_, _, name))| (name.as_str().to_lowercase(), core::Entity::from(raw)))
            .collect();
        match core::trie::trie_match(corpse_name, candidates) {
            core::trie::TrieMatch::One(e) => Some(e),
            core::trie::TrieMatch::Many(items) => items.into_iter().next(),
            core::trie::TrieMatch::None => None,
        }
    };

    let corpse = match corpse {
        Some(c) => c,
        None => {
            conn.send_line("You don't see a corpse here.");
            return;
        }
    };

    // Check loot rules
    let player_db_id = world
        .query_one::<&core::DbId>(entity)
        .ok()
        .and_then(|mut q| q.get().map(|d| d.0));

    if let Ok(mut q) = world.query_one::<&core::Corpse>(corpse) {
        if let Some(c_comp) = q.get() {
            match c_comp.lootable_by {
                core::LootRule::Public => {}
                core::LootRule::OwnerOnly | core::LootRule::Faction => {
                    let is_owner = c_comp.owner == Some(entity)
                        || (player_db_id.is_some() && c_comp.owner_db_id == player_db_id);
                    if !is_owner {
                        conn.send_line("This corpse does not belong to you.");
                        return;
                    }
                }
                core::LootRule::GroupOnly => {
                    let is_owner = c_comp.owner == Some(entity)
                        || (player_db_id.is_some() && c_comp.owner_db_id == player_db_id);
                    if !is_owner {
                        let mut in_same_group = false;
                        if let Ok(mut q_gm) = world.query_one::<&core::GroupMember>(entity) {
                            if let Some(gm) = q_gm.get() {
                                let group_entity = gm.group_id;
                                if let Ok(mut q_group) =
                                    world.query_one::<&core::Group>(group_entity)
                                {
                                    if let Some(group) = q_group.get() {
                                        in_same_group = group.members.iter().any(|m| {
                                            (c_comp.owner.is_some() && m.entity == c_comp.owner)
                                                || (c_comp.owner_db_id.is_some()
                                                    && Some(m.db_id) == c_comp.owner_db_id)
                                        });
                                    }
                                }
                            }
                        }
                        if !in_same_group {
                            conn.send_line("This corpse does not belong to you or your group.");
                            return;
                        }
                    }
                }
            }
        }
    }

    // Transfer items from corpse inventory to player inventory
    let items = world
        .query_one::<&core::Inventory>(corpse)
        .ok()
        .and_then(|mut q| q.get().map(|inv| inv.0.clone()))
        .unwrap_or_default();

    if items.is_empty() {
        conn.send_line("The corpse has nothing.");
        return;
    }

    if let Ok(mut q) = world.query_one::<&mut core::Inventory>(entity) {
        if let Some(inv) = q.get() {
            let count = items.len();
            inv.0.extend(items);
            conn.send_line(&format!("You loot {count} item(s) from the corpse."));
        }
    }
}

// ---------------------------------------------------------------------------
// Stance command
// ---------------------------------------------------------------------------

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
        // Show current stance
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

// ---------------------------------------------------------------------------
// Practice command
// ---------------------------------------------------------------------------

fn max_rank_for_level(level: u8) -> u16 {
    // Max any skill rank = level * 5 + 5 (level 1 = 10, level 10 = 55, level 50 = 255)
    (level as u16 * 5) + 5
}

pub fn cmd_practice(
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

    let level = world
        .query_one::<&core::Level>(entity)
        .ok()
        .and_then(|mut q| q.get().copied())
        .unwrap_or(core::Level(1));

    let args = args.trim();

    // `practice` with no args — show status
    if args.is_empty() {
        let skills = world
            .query_one::<&core::LearnedSkills>(entity)
            .ok()
            .and_then(|mut q| q.get().cloned())
            .unwrap_or_default();

        let practice_pts = world
            .query_one::<&core::PracticePoints>(entity)
            .ok()
            .and_then(|mut q| q.get().copied())
            .unwrap_or(core::PracticePoints(0));

        conn.send_line("");
        conn.send_line("--- Skill Practice ---");
        conn.send_line(&format!("Practice points: {}", practice_pts.0));
        conn.send_line(&format!(
            "Max rank per skill at level {}: {}",
            level.0,
            max_rank_for_level(level.0)
        ));
        conn.send_line("");

        if skills.skills.is_empty() {
            conn.send_line("You have no skills yet.");
        } else {
            let mut skills_vec: Vec<_> = skills.skills.iter().collect();
            skills_vec.sort_by(|a, b| a.0.cmp(b.0));

            conn.send_line("Known skills:");
            for (skill_id, rank) in &skills_vec {
                conn.send_line(&format!("  {skill_id}: rank {rank} (1 point to practice)"));
            }
        }

        conn.send_line("");
        return;
    }

    // `practice list` — show all available skills from templates
    if args == "list" {
        conn.send_line("");
        conn.send_line("Available skills:");
        conn.send_line("  Skills are granted through race/class selection.");
        conn.send_line("  Use 'practice <skill>' to increase a known skill's rank.");
        conn.send_line("");
        return;
    }

    // `practice <skill>` — practice a specific skill
    let skill_id = match resolve_skill_name_for_practicing(args, world, entity) {
        Ok(id) => id,
        Err(msg) => {
            conn.send_line(&msg);
            return;
        }
    };

    let mut skills = match world.query_one::<&mut core::LearnedSkills>(entity) {
        Ok(mut q) => match q.get() {
            Some(s) => s.clone(),
            None => {
                conn.send_line("You have no skills component.");
                return;
            }
        },
        Err(_) => {
            conn.send_line("You have no skills component.");
            return;
        }
    };

    let mut practice_pts = match world.query_one::<&mut core::PracticePoints>(entity) {
        Ok(mut q) => match q.get() {
            Some(p) => *p,
            None => {
                conn.send_line("You have no practice points.");
                return;
            }
        },
        Err(_) => {
            conn.send_line("You have no practice points.");
            return;
        }
    };

    // Check if skill is known
    let current_rank = skills.rank(&skill_id);
    if current_rank == 0 {
        conn.send_line(&format!("You don't know the skill '{skill_id}'."));
        return;
    }

    // Proximity check: room and trainer presence
    let room = match get_pos_room(world, entity) {
        Some(r) => r,
        None => {
            conn.send_line("You can't do that here. Seek out a trainer.");
            return;
        }
    };

    let room_entities = core::util::entities_in_room(world, room);
    let mut has_trainer = false;
    for e in &room_entities {
        if let Ok(mut q) = world.query_one::<&core::Trainer>(*e) {
            if q.get().is_some() {
                has_trainer = true;
                break;
            }
        }
    }

    if !has_trainer {
        conn.send_line("You can't do that here. Seek out a trainer.");
        return;
    }

    // Retrieve skill type to verify if any trainer teaches it
    let templates = oxide_server::get_templates();
    let skill_def = templates.as_ref().and_then(|t| t.get_skill(&skill_id));
    let skill_type_str = skill_def
        .map(|def| format!("{:?}", def.skill_type).to_lowercase())
        .unwrap_or_else(|| "combat".to_string());

    let mut can_train_skill = false;
    for e in room_entities {
        if let Ok(mut q) = world.query_one::<&core::Trainer>(e) {
            if let Some(t) = q.get() {
                if t.can_train(&skill_type_str) {
                    can_train_skill = true;
                    break;
                }
            }
        }
    }

    if !can_train_skill {
        conn.send_line("You can't practice that here.");
        return;
    }

    // Check skill cap by level
    let max_rank = max_rank_for_level(level.0);
    if current_rank >= max_rank {
        conn.send_line(&format!(
            "You cannot practice '{skill_id}' beyond rank {max_rank} at your level."
        ));
        return;
    }

    // Check cost
    let cost = 1;
    if practice_pts.0 < cost {
        conn.send_line(&format!(
            "Practicing '{skill_id}' costs {cost} point(s), but you only have {}.",
            practice_pts.0
        ));
        return;
    }

    // Apply practicing
    practice_pts.0 -= cost;
    let new_rank = current_rank + 1;
    skills.set_rank(&skill_id, new_rank);
    let remaining = practice_pts.0;

    let _ = world.insert(entity, (skills, practice_pts, core::Dirty));
    conn.send_line(&format!(
        "You practice '{skill_id}' to rank {new_rank}. ({remaining} point(s) remaining)",
    ));
}

/// Resolve a skill name (exact or partial) for the `practice` command.
/// Falls back to exact match when the template registry is unavailable.
fn resolve_skill_name_for_practicing(
    input: &str,
    world: &World,
    entity: core::Entity,
) -> Result<String, String> {
    let templates = match oxide_server::get_templates() {
        Some(t) => t,
        None => return Ok(input.to_lowercase()),
    };

    let raw = input.to_lowercase();
    match templates.resolve_skill(&raw, None) {
        Ok(id) => {
            // Verify the player knows this skill before we return the resolved ID.
            // If they don't know it, fall back to raw input so the caller's
            // "You don't know the skill" message uses what the player typed.
            let known = world
                .query_one::<&core::LearnedSkills>(entity)
                .ok()
                .and_then(|mut q| q.get().cloned())
                .is_some_and(|s| s.has(&id));
            if known {
                Ok(id)
            } else {
                Ok(raw)
            }
        }
        Err(SkillResolveError::NotFound) => Ok(raw),
        Err(SkillResolveError::Multiple(candidates)) => {
            let names: Vec<String> = candidates
                .into_iter()
                .map(|(id, name)| format!("{id} ({name})"))
                .collect();
            Err(format!("Which skill did you mean? {}", names.join(", ")))
        }
    }
}

// ---------------------------------------------------------------------------
// Train command
// ---------------------------------------------------------------------------

pub fn cmd_train(
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

    let args = args.trim();

    let mut attrs = match world.query_one::<&mut core::Attributes>(entity) {
        Ok(mut q) => match q.get() {
            Some(a) => a.clone(),
            None => {
                conn.send_line("You have no attributes component.");
                return;
            }
        },
        Err(_) => {
            conn.send_line("You have no attributes component.");
            return;
        }
    };

    let mut practice_pts = match world.query_one::<&mut core::PracticePoints>(entity) {
        Ok(mut q) => match q.get() {
            Some(p) => *p,
            None => {
                conn.send_line("You have no practice points.");
                return;
            }
        },
        Err(_) => {
            conn.send_line("You have no practice points.");
            return;
        }
    };

    let class_id = world
        .query_one::<&core::Class>(entity)
        .ok()
        .and_then(|mut q| q.get().map(|c| c.0.clone()));

    let templates = oxide_server::get_templates();

    // Helper closure to calculate cost for a given attribute name (lowercase)
    let get_attr_cost = |attr_name: &str| -> u32 {
        if let (Some(c_id), Some(t)) = (&class_id, &templates) {
            if let Some(class_template) = t.get_class(c_id) {
                let c = &class_template.attribute_mods;
                let mods = [
                    c.strength,
                    c.dexterity,
                    c.intelligence,
                    c.wisdom,
                    c.constitution,
                    c.charisma,
                ];
                let max_val = mods.into_iter().max().unwrap_or(0);
                let is_prime = match attr_name {
                    "strength" => c.strength == max_val,
                    "dexterity" => c.dexterity == max_val,
                    "intelligence" => c.intelligence == max_val,
                    "wisdom" => c.wisdom == max_val,
                    "constitution" => c.constitution == max_val,
                    "charisma" => c.charisma == max_val,
                    _ => false,
                };
                if is_prime {
                    return 3;
                }
            }
        }
        5
    };

    // `train` with no args — show status
    if args.is_empty() {
        conn.send_line("");
        conn.send_line("--- Attribute Training ---");
        conn.send_line(&format!("Practice points: {}", practice_pts.0));
        conn.send_line("");
        conn.send_line("Attributes:");
        conn.send_line(&format!(
            "  Strength:     {} (cost: {} pts)",
            attrs.strength,
            get_attr_cost("strength")
        ));
        conn.send_line(&format!(
            "  Dexterity:    {} (cost: {} pts)",
            attrs.dexterity,
            get_attr_cost("dexterity")
        ));
        conn.send_line(&format!(
            "  Intelligence: {} (cost: {} pts)",
            attrs.intelligence,
            get_attr_cost("intelligence")
        ));
        conn.send_line(&format!(
            "  Wisdom:       {} (cost: {} pts)",
            attrs.wisdom,
            get_attr_cost("wisdom")
        ));
        conn.send_line(&format!(
            "  Constitution: {} (cost: {} pts)",
            attrs.constitution,
            get_attr_cost("constitution")
        ));
        conn.send_line(&format!(
            "  Charisma:     {} (cost: {} pts)",
            attrs.charisma,
            get_attr_cost("charisma")
        ));
        conn.send_line("");
        return;
    }

    // `train <attribute>` — raise attribute
    let input = args.to_lowercase();
    let (target_attr, attr_name_cap) = match input.as_str() {
        "str" | "strength" => ("strength", "Strength"),
        "dex" | "dexterity" => ("dexterity", "Dexterity"),
        "int" | "intelligence" => ("intelligence", "Intelligence"),
        "wis" | "wisdom" => ("wisdom", "Wisdom"),
        "con" | "constitution" => ("constitution", "Constitution"),
        "cha" | "charisma" => ("charisma", "Charisma"),
        _ => {
            conn.send_line("Invalid attribute. Choose from: Strength, Dexterity, Intelligence, Wisdom, Constitution, Charisma.");
            return;
        }
    };

    // Proximity check: room and trainer presence
    let room = match get_pos_room(world, entity) {
        Some(r) => r,
        None => {
            conn.send_line("You can't do that here. Seek out a trainer.");
            return;
        }
    };

    let room_entities = core::util::entities_in_room(world, room);
    let mut has_trainer = false;
    for e in &room_entities {
        if let Ok(mut q) = world.query_one::<&core::Trainer>(*e) {
            if q.get().is_some() {
                has_trainer = true;
                break;
            }
        }
    }

    if !has_trainer {
        conn.send_line("You can't do that here. Seek out a trainer.");
        return;
    }

    let mut can_train_attr = false;
    for e in room_entities {
        if let Ok(mut q) = world.query_one::<&core::Trainer>(e) {
            if let Some(t) = q.get() {
                if t.can_train("attributes") {
                    can_train_attr = true;
                    break;
                }
            }
        }
    }

    if !can_train_attr {
        conn.send_line("You can't train that here.");
        return;
    }

    // Check bounds
    let current_val = match target_attr {
        "strength" => attrs.strength,
        "dexterity" => attrs.dexterity,
        "intelligence" => attrs.intelligence,
        "wisdom" => attrs.wisdom,
        "constitution" => attrs.constitution,
        "charisma" => attrs.charisma,
        _ => unreachable!(),
    };

    if current_val >= core::Attributes::MAX {
        conn.send_line(&format!(
            "Your {} is already at the maximum of {}.",
            attr_name_cap,
            core::Attributes::MAX
        ));
        return;
    }

    // Determine cost
    let cost = get_attr_cost(target_attr);
    if practice_pts.0 < cost {
        conn.send_line(&format!(
            "Training {} costs {} practice points, but you only have {}.",
            attr_name_cap, cost, practice_pts.0
        ));
        return;
    }

    // Apply training
    practice_pts.0 -= cost;
    let new_val = current_val + 1;
    match target_attr {
        "strength" => attrs.strength = new_val,
        "dexterity" => attrs.dexterity = new_val,
        "intelligence" => attrs.intelligence = new_val,
        "wisdom" => attrs.wisdom = new_val,
        "constitution" => attrs.constitution = new_val,
        "charisma" => attrs.charisma = new_val,
        _ => unreachable!(),
    }

    let _ = world.insert(entity, (attrs, practice_pts, core::Dirty));
    conn.send_line(&format!(
        "You train {} to {}. ({} practice points remaining)",
        attr_name_cap, new_val, practice_pts.0
    ));
}

// ---------------------------------------------------------------------------
// Prompt command
// ---------------------------------------------------------------------------

pub fn cmd_prompt(
    world: &mut World,
    conn: &mut dyn Connection,
    _name: &str,
    args: &str,
    _registry: &ConnectionRegistry,
) {
    let trimmed = args.trim();
    if trimmed.is_empty() {
        let msg = conn
            .entity()
            .and_then(|e| {
                world
                    .query_one::<&oxide_core::Player>(e)
                    .ok()
                    .and_then(|mut q| q.get().cloned())
                    .map(|p| match &p.prompt {
                        Some(t) => format!("Current prompt: {t}"),
                        None => format!(
                            "Using default prompt: {}",
                            oxide_server::config::get().default_prompt
                        ),
                    })
            })
            .unwrap_or_else(|| {
                format!(
                    "Using default prompt: {}",
                    oxide_server::config::get().default_prompt
                )
            });
        conn.send_line(&msg);
        conn.send_line("Usage: prompt <template>");
        conn.send_line("See 'help prompt' for available variables.");
        conn.send_line("Type 'prompt reset' to revert to the server default.");
        return;
    }

    if let Some(entity) = conn.entity() {
        let updated = {
            let mut q = match world.query_one::<&mut oxide_core::Player>(entity) {
                Ok(q) => q,
                Err(_) => return conn.send_line("You can't change your prompt right now."),
            };
            match q.get() {
                Some(player) => {
                    if trimmed == "reset" {
                        tracing::debug!(entity = ?entity, old_prompt = ?player.prompt, "cmd_prompt: resetting to None");
                        player.prompt = None;
                        true
                    } else {
                        tracing::debug!(entity = ?entity, new_prompt = %trimmed, "cmd_prompt: setting custom prompt");
                        player.prompt = Some(trimmed.to_string());
                        true
                    }
                }
                None => false,
            }
        };
        if updated {
            let _ = world.insert(entity, (oxide_core::Dirty,));
            if trimmed == "reset" {
                conn.send_line("Prompt reset to default.");
            } else {
                conn.send_line("Prompt updated.");
            }
            return;
        }
    }
    conn.send_line("You can't change your prompt right now.");
}

fn room_has_shrine_for_deity(world: &World, player: core::Entity, deity_id: &str) -> bool {
    let Some(room) = get_pos_room(world, player) else {
        return false;
    };
    let Ok(mut q) = world.query_one::<&FloorItems>(room) else {
        return false;
    };
    let Some(floor_items) = q.get() else {
        return false;
    };
    for &item_entity in &floor_items.0 {
        if let Ok(mut item_q) = world.query_one::<(&Name, &Item)>(item_entity) {
            if let Some((name, item)) = item_q.get() {
                let name_lower = name.0.to_lowercase();
                if name_lower.contains("shrine") && name_lower.contains(&deity_id.to_lowercase()) {
                    return true;
                }
                if item.template_id.to_lowercase().contains("shrine")
                    && item
                        .template_id
                        .to_lowercase()
                        .contains(&deity_id.to_lowercase())
                {
                    return true;
                }
            }
        }
    }
    false
}

pub fn cmd_pray(
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

    let target = args.trim();
    let player_deity = world
        .query_one::<&core::Deity>(entity)
        .ok()
        .and_then(|mut q| q.get().cloned())
        .and_then(|d| d.0);

    let templates = match oxide_server::get_templates() {
        Some(t) => t,
        None => {
            conn.send_line("Server error: templates unavailable.");
            return;
        }
    };

    let deity_id = if target.is_empty() {
        match player_deity {
            Some(d) => d,
            None => {
                conn.send_line("You do not follow a deity. Who do you wish to pray to?");
                return;
            }
        }
    } else {
        let resolved = templates
            .deities
            .keys()
            .find(|k| k.to_lowercase() == target.to_lowercase())
            .cloned();
        let deity_id = match resolved {
            Some(d) => d,
            None => {
                conn.send_line(&format!("There is no deity named '{}'.", target));
                return;
            }
        };

        if Some(&deity_id) != player_deity.as_ref() {
            let room_has_shrine = room_has_shrine_for_deity(world, entity, &deity_id);
            if !room_has_shrine {
                conn.send_line(&format!(
                    "You do not follow {}, and there is no shrine to them here.",
                    deity_id
                ));
                return;
            }
        }
        deity_id
    };

    let deity_tmpl = match templates.deities.get(&deity_id) {
        Some(d) => d,
        None => {
            conn.send_line(&format!(
                "The deity '{}' does not exist in our archives.",
                deity_id
            ));
            return;
        }
    };

    let effect = match &deity_tmpl.prayer_effect {
        Some(e) => e,
        None => {
            conn.send_line(&format!(
                "You pray to {}, but feel no response.",
                deity_tmpl.name
            ));
            return;
        }
    };

    // Check cooldown
    let now = std::time::Instant::now();
    let on_cooldown = if let Ok(mut q) = world.query_one::<&core::PrayerCooldown>(entity) {
        if let Some(cooldown) = q.get() {
            let elapsed = now.duration_since(cooldown.last_prayed).as_secs();
            if elapsed < effect.cooldown_secs {
                let wait = effect.cooldown_secs - elapsed;
                conn.send_line(&format!("Your prayers to {} have been answered too recently. You must wait {} more seconds.", deity_tmpl.name, wait));
                true
            } else {
                false
            }
        } else {
            false
        }
    } else {
        false
    };

    if on_cooldown {
        return;
    }

    // Update cooldown
    let _ = world.remove_one::<core::PrayerCooldown>(entity);
    let _ = world.insert(entity, (core::PrayerCooldown { last_prayed: now },));

    conn.send_line(&format!(
        "You bow your head and pray to {}.",
        deity_tmpl.name
    ));
    conn.send_line(&format!("You feel a response: {}", effect.description));

    // Apply heal / mana / stamina restoration based on template values
    if deity_id == "solaris" {
        if let Ok(mut q) = world.query_one::<&mut core::Health>(entity) {
            if let Some(h) = q.get() {
                let restore = h.max / 4;
                h.current = (h.current + restore).min(h.max);
                conn.send_line("Your wounds are knit by Solaris' light.");
            }
        }
        if let Ok(mut q) = world.query_one::<&mut core::Stamina>(entity) {
            if let Some(s) = q.get() {
                let restore = s.max / 2;
                s.current = (s.current + restore).min(s.max);
                conn.send_line("You feel a surge of solar stamina.");
            }
        }
    } else if deity_id == "luna" {
        if let Ok(mut q) = world.query_one::<&mut core::Mana>(entity) {
            if let Some(m) = q.get() {
                let restore = m.max / 3;
                m.current = (m.current + restore).min(m.max);
                conn.send_line("Moonlight replenishes your magical energy.");
            }
        }
    } else if deity_id == "astra" {
        if let Ok(mut q) = world.query_one::<&mut core::Mana>(entity) {
            if let Some(m) = q.get() {
                let restore = m.max / 4;
                m.current = (m.current + restore).min(m.max);
                conn.send_line("Starlight clears your mind and replenishes your mana.");
            }
        }
        if let Ok(mut q) = world.query_one::<&mut core::Stamina>(entity) {
            if let Some(s) = q.get() {
                let restore = s.max / 4;
                s.current = (s.current + restore).min(s.max);
                conn.send_line("Starlight fills your limbs with light energy.");
            }
        }
    } else if deity_id == "kronos" {
        if let Ok(mut q) = world.query_one::<&mut core::Stamina>(entity) {
            if let Some(s) = q.get() {
                let restore = s.max / 2;
                s.current = (s.current + restore).min(s.max);
                conn.send_line("You feel a moment of stillness, and your stamina is restored.");
            }
        }
    } else if deity_id == "vulgath" {
        if let Ok(mut q) = world.query_one::<&mut core::Health>(entity) {
            if let Some(h) = q.get() {
                let restore = h.max / 6;
                h.current = (h.current + restore).min(h.max);
                conn.send_line("Sinuous shadows envelope you, drawing away pain.");
            }
        }
        if let Ok(mut q) = world.query_one::<&mut core::Mana>(entity) {
            if let Some(m) = q.get() {
                let restore = m.max / 6;
                m.current = (m.current + restore).min(m.max);
                conn.send_line("Sinuous shadows seep into your mind, replenishing your mana.");
            }
        }
    } else if deity_id == "karrgath" {
        if let Ok(mut q) = world.query_one::<&mut core::Health>(entity) {
            if let Some(h) = q.get() {
                let restore = h.max / 8;
                h.current = (h.current + restore).min(h.max);
                conn.send_line("A surge of iron fury knit your minor wounds.");
            }
        }
        if let Ok(mut q) = world.query_one::<&mut core::Stamina>(entity) {
            if let Some(s) = q.get() {
                let restore = s.max / 3;
                s.current = (s.current + restore).min(s.max);
                conn.send_line("A surge of war-fury restores your stamina.");
            }
        }
    }

    // Apply the active effect
    let active_effect = core::ActiveEffect {
        source: format!("prayer:{}", deity_id),
        stat: Some(effect.buff_id.clone()),
        amount: Some(1),
        aura_id: None,
        radius: None,
    };
    let mut effects = world
        .query_one::<&Vec<core::ActiveEffect>>(entity)
        .ok()
        .and_then(|mut q| q.get().cloned())
        .unwrap_or_default();
    effects.retain(|e| !e.source.starts_with("prayer:"));
    effects.push(active_effect);
    let _ = world.remove_one::<Vec<core::ActiveEffect>>(entity);
    let _ = world.insert(entity, (effects,));

    // Mark entity as dirty so progress persists
    let _ = world.insert(entity, (core::Dirty,));
}

fn format_ghost_text(text: &str) -> String {
    let mut out = String::new();
    let mut use_cyan = true;
    for c in text.chars() {
        if c.is_whitespace() {
            out.push(c);
        } else {
            if use_cyan {
                out.push_str(&format!("{{cyan}}{c}"));
            } else {
                out.push_str(&format!("{{brightblue}}{c}"));
            }
            use_cyan = !use_cyan;
        }
    }
    out.push_str("{/}");
    out
}

fn broadcast_to_room_except(
    world: &World,
    registry: &ConnectionRegistry,
    room: core::Entity,
    except: core::Entity,
    message: &str,
) {
    let bytes = format!("{}\r\n", message).into_bytes();
    for &other in &registry.occupants(world, room) {
        if other == except {
            continue;
        }
        if let Some(tx) = registry.sender(other) {
            let _ = tx.send(bytes.clone());
        }
    }
}

fn send_to_online_player(registry: &ConnectionRegistry, entity: core::Entity, message: &str) {
    if let Some(tx) = registry.sender(entity) {
        let text = core::format::parse_tags(message);
        let rendered = text.render(true, true);
        let _ = tx.send(format!("{}\r\n", rendered).into_bytes());
    }
}

fn send_to_conn(conn: &mut dyn Connection, message: &str) {
    let text = core::format::parse_tags(message);
    send_formatted(conn, &text);
}

pub fn cmd_sit(
    world: &mut World,
    conn: &mut dyn Connection,
    _name: &str,
    _args: &str,
    registry: &ConnectionRegistry,
) {
    let entity = match conn.entity() {
        Some(e) => e,
        None => return,
    };
    let current_state = world
        .query_one::<&core::PlayerState>(entity)
        .ok()
        .and_then(|mut q| q.get().cloned())
        .unwrap_or_default();

    match current_state {
        core::PlayerState::Dead => {
            conn.send_line("You are a ghost! Ghosts do not sit down.");
        }
        core::PlayerState::Resting(rest) => match rest {
            core::RestState::Sitting => {
                conn.send_line("You are already sitting.");
            }
            core::RestState::Standing | core::RestState::Resting => {
                let next_state = core::PlayerState::Resting(core::RestState::Sitting);
                let _ = world.insert(entity, (next_state, core::Dirty));
                conn.send_line("You sit down.");
                if let Ok(mut name_q) = world.query_one::<&core::Name>(entity) {
                    if let Some(name) = name_q.get() {
                        if let Some(room) = get_pos_room(world, entity) {
                            broadcast_to_room_except(
                                world,
                                registry,
                                room,
                                entity,
                                &format!("{} sits down.", name.0),
                            );
                        }
                    }
                }
            }
            core::RestState::Sleeping => {
                conn.send_line("You must wake up first.");
            }
            core::RestState::Unconscious => {
                conn.send_line("You are unconscious.");
            }
            core::RestState::Dead => {
                conn.send_line("You are a ghost! Ghosts do not sit down.");
            }
        },
        core::PlayerState::Stunned { .. } => {
            conn.send_line("You are stunned and cannot move.");
        }
        core::PlayerState::Casting { .. } => {
            conn.send_line("You are too busy casting to sit down.");
        }
    }
}

pub fn cmd_rest(
    world: &mut World,
    conn: &mut dyn Connection,
    _name: &str,
    _args: &str,
    registry: &ConnectionRegistry,
) {
    let entity = match conn.entity() {
        Some(e) => e,
        None => return,
    };
    let current_state = world
        .query_one::<&core::PlayerState>(entity)
        .ok()
        .and_then(|mut q| q.get().cloned())
        .unwrap_or_default();

    match current_state {
        core::PlayerState::Dead => {
            conn.send_line("You are a ghost! Ghosts do not rest.");
        }
        core::PlayerState::Resting(rest) => match rest {
            core::RestState::Resting => {
                conn.send_line("You are already resting.");
            }
            core::RestState::Standing | core::RestState::Sitting | core::RestState::Sleeping => {
                let next_state = core::PlayerState::Resting(core::RestState::Resting);
                let _ = world.insert(entity, (next_state, core::Dirty));
                conn.send_line("You lean back and rest.");
                if let Ok(mut name_q) = world.query_one::<&core::Name>(entity) {
                    if let Some(name) = name_q.get() {
                        if let Some(room) = get_pos_room(world, entity) {
                            broadcast_to_room_except(
                                world,
                                registry,
                                room,
                                entity,
                                &format!("{} rests.", name.0),
                            );
                        }
                    }
                }
            }
            core::RestState::Unconscious => {
                conn.send_line("You are unconscious.");
            }
            core::RestState::Dead => {
                conn.send_line("You are a ghost! Ghosts do not rest.");
            }
        },
        core::PlayerState::Stunned { .. } => {
            conn.send_line("You are stunned and cannot rest.");
        }
        core::PlayerState::Casting { .. } => {
            conn.send_line("You are too busy casting to rest.");
        }
    }
}

pub fn cmd_sleep(
    world: &mut World,
    conn: &mut dyn Connection,
    _name: &str,
    _args: &str,
    registry: &ConnectionRegistry,
) {
    let entity = match conn.entity() {
        Some(e) => e,
        None => return,
    };
    let current_state = world
        .query_one::<&core::PlayerState>(entity)
        .ok()
        .and_then(|mut q| q.get().cloned())
        .unwrap_or_default();

    match current_state {
        core::PlayerState::Dead => {
            conn.send_line("You are a ghost! Ghosts do not sleep.");
        }
        core::PlayerState::Resting(rest) => match rest {
            core::RestState::Sleeping => {
                conn.send_line("You are already sleeping.");
            }
            core::RestState::Standing | core::RestState::Sitting | core::RestState::Resting => {
                let next_state = core::PlayerState::Resting(core::RestState::Sleeping);
                let _ = world.insert(entity, (next_state, core::Dirty));
                conn.send_line("You lie down and go to sleep.");
                if let Ok(mut name_q) = world.query_one::<&core::Name>(entity) {
                    if let Some(name) = name_q.get() {
                        if let Some(room) = get_pos_room(world, entity) {
                            broadcast_to_room_except(
                                world,
                                registry,
                                room,
                                entity,
                                &format!("{} goes to sleep.", name.0),
                            );
                        }
                    }
                }
            }
            core::RestState::Unconscious => {
                conn.send_line("You are unconscious.");
            }
            core::RestState::Dead => {
                conn.send_line("You are a ghost! Ghosts do not sleep.");
            }
        },
        core::PlayerState::Stunned { .. } => {
            conn.send_line("You are stunned and cannot sleep.");
        }
        core::PlayerState::Casting { .. } => {
            conn.send_line("You are too busy casting to sleep.");
        }
    }
}

pub fn cmd_wake(
    world: &mut World,
    conn: &mut dyn Connection,
    _name: &str,
    _args: &str,
    registry: &ConnectionRegistry,
) {
    let entity = match conn.entity() {
        Some(e) => e,
        None => return,
    };
    let current_state = world
        .query_one::<&core::PlayerState>(entity)
        .ok()
        .and_then(|mut q| q.get().cloned())
        .unwrap_or_default();

    match current_state {
        core::PlayerState::Dead => {
            conn.send_line("You are a ghost! You cannot wake up.");
        }
        core::PlayerState::Resting(rest) => match rest {
            core::RestState::Sleeping => {
                let next_state = core::PlayerState::Resting(core::RestState::Resting);
                let _ = world.insert(entity, (next_state, core::Dirty));
                conn.send_line("You wake up.");
                if let Ok(mut name_q) = world.query_one::<&core::Name>(entity) {
                    if let Some(name) = name_q.get() {
                        if let Some(room) = get_pos_room(world, entity) {
                            broadcast_to_room_except(
                                world,
                                registry,
                                room,
                                entity,
                                &format!("{} wakes up.", name.0),
                            );
                        }
                    }
                }
            }
            core::RestState::Standing | core::RestState::Sitting | core::RestState::Resting => {
                conn.send_line("You are already awake.");
            }
            core::RestState::Unconscious => {
                conn.send_line("You are unconscious.");
            }
            core::RestState::Dead => {
                conn.send_line("You are a ghost! You cannot wake up.");
            }
        },
        core::PlayerState::Stunned { .. } => {
            conn.send_line("You are stunned and cannot wake up.");
        }
        core::PlayerState::Casting { .. } => {
            conn.send_line("You are already awake.");
        }
    }
}

pub fn cmd_stand(
    world: &mut World,
    conn: &mut dyn Connection,
    _name: &str,
    _args: &str,
    registry: &ConnectionRegistry,
) {
    let entity = match conn.entity() {
        Some(e) => e,
        None => return,
    };
    let current_state = world
        .query_one::<&core::PlayerState>(entity)
        .ok()
        .and_then(|mut q| q.get().cloned())
        .unwrap_or_default();

    match current_state {
        core::PlayerState::Dead => {
            conn.send_line("You are a ghost! Ghosts stand in ethereal form.");
        }
        core::PlayerState::Resting(rest) => match rest {
            core::RestState::Standing => {
                conn.send_line("You are already standing.");
            }
            core::RestState::Sitting | core::RestState::Resting | core::RestState::Sleeping => {
                let next_state = core::PlayerState::Resting(core::RestState::Standing);
                let _ = world.insert(entity, (next_state, core::Dirty));
                conn.send_line("You stand up.");
                if let Ok(mut name_q) = world.query_one::<&core::Name>(entity) {
                    if let Some(name) = name_q.get() {
                        if let Some(room) = get_pos_room(world, entity) {
                            broadcast_to_room_except(
                                world,
                                registry,
                                room,
                                entity,
                                &format!("{} stands up.", name.0),
                            );
                        }
                    }
                }
            }
            core::RestState::Unconscious => {
                conn.send_line("You are unconscious.");
            }
            core::RestState::Dead => {
                conn.send_line("You are a ghost! Ghosts stand in ethereal form.");
            }
        },
        core::PlayerState::Stunned { .. } => {
            conn.send_line("You are stunned and cannot stand up.");
        }
        core::PlayerState::Casting { .. } => {
            conn.send_line("You stand up (you were already awake).");
        }
    }
}

fn find_online_player(
    world: &World,
    registry: &ConnectionRegistry,
    name: &str,
) -> Option<core::Entity> {
    let lower_name = name.to_lowercase();
    let candidates: Vec<core::Entity> = registry
        .connected_entities()
        .into_iter()
        .filter(|&e| {
            if let Some(n) = get_name(world, e) {
                n.0.to_lowercase().starts_with(&lower_name)
            } else {
                false
            }
        })
        .collect();

    if let Some(&exact) = candidates.iter().find(|&&e| {
        if let Some(n) = get_name(world, e) {
            n.0.to_lowercase() == lower_name
        } else {
            false
        }
    }) {
        return Some(exact);
    }

    if candidates.len() == 1 {
        Some(candidates[0])
    } else {
        None
    }
}

pub fn cmd_tell(
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

    let is_ghost = world
        .query_one::<&core::PlayerState>(entity)
        .ok()
        .and_then(|mut q| q.get().map(|s| matches!(s, core::PlayerState::Dead)))
        .unwrap_or(false);

    let (target_name, message) = match args.split_once(' ') {
        Some((t, m)) if !t.is_empty() && !m.is_empty() => (t, m),
        _ => {
            conn.send_line("Tell whom what?");
            return;
        }
    };

    let target_entity = match find_online_player(world, registry, target_name) {
        Some(te) => te,
        None => {
            conn.send_line("No one by that name is here.");
            return;
        }
    };

    if target_entity == entity {
        conn.send_line("You talk to yourself.");
        return;
    }

    let sender_name = get_name(world, entity).unwrap_or(Name::new("Someone")).0;
    let target_name_real = get_name(world, target_entity)
        .unwrap_or(Name::new("Someone"))
        .0;

    let msg_fmt = if is_ghost {
        format_ghost_text(message)
    } else {
        message.to_string()
    };

    let _ = world.insert(target_entity, (core::LastMessenger(entity), core::Dirty));

    send_to_online_player(
        registry,
        target_entity,
        &format!("{} tells you, \"{}\"", sender_name, msg_fmt),
    );

    send_to_conn(
        conn,
        &format!("You tell {}, \"{}\"", target_name_real, msg_fmt),
    );
}

pub fn cmd_reply(
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

    if args.is_empty() {
        conn.send_line("Reply what?");
        return;
    }

    let is_ghost = world
        .query_one::<&core::PlayerState>(entity)
        .ok()
        .and_then(|mut q| q.get().map(|s| matches!(s, core::PlayerState::Dead)))
        .unwrap_or(false);

    let target_entity = match world.query_one::<&core::LastMessenger>(entity) {
        Ok(mut q) => q.get().map(|lm| lm.0),
        Err(_) => None,
    };

    let target_entity = match target_entity {
        Some(te) => te,
        None => {
            conn.send_line("No one to reply to.");
            return;
        }
    };

    if !registry.is_connected(target_entity) {
        conn.send_line("They are no longer online.");
        return;
    }

    let sender_name = get_name(world, entity).unwrap_or(Name::new("Someone")).0;
    let target_name_real = get_name(world, target_entity)
        .unwrap_or(Name::new("Someone"))
        .0;

    let msg_fmt = if is_ghost {
        format_ghost_text(args)
    } else {
        args.to_string()
    };

    let _ = world.insert(target_entity, (core::LastMessenger(entity), core::Dirty));

    send_to_online_player(
        registry,
        target_entity,
        &format!("{} tells you, \"{}\"", sender_name, msg_fmt),
    );

    send_to_conn(
        conn,
        &format!("You tell {}, \"{}\"", target_name_real, msg_fmt),
    );
}

pub fn cmd_shout(
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

    if args.is_empty() {
        conn.send_line("Shout what?");
        return;
    }

    let is_ghost = world
        .query_one::<&core::PlayerState>(entity)
        .ok()
        .and_then(|mut q| q.get().map(|s| matches!(s, core::PlayerState::Dead)))
        .unwrap_or(false);

    let room = match get_pos_room(world, entity) {
        Some(r) => r,
        None => {
            conn.send_line("You are nowhere.");
            return;
        }
    };

    let area_id = match world.query_one::<&core::RoomKey>(room) {
        Ok(mut q) => q
            .get()
            .map(|sk| sk.0.split_once(':').unwrap().0.to_string()),
        Err(_) => None,
    };

    let area_id = match area_id {
        Some(a) => a,
        None => {
            conn.send_line("You cannot shout here.");
            return;
        }
    };

    let sender_name = get_name(world, entity).unwrap_or(Name::new("Someone")).0;
    let msg_fmt = if is_ghost {
        format_ghost_text(args)
    } else {
        args.to_string()
    };

    let target_msg = format!("{} shouts, \"{}\"", sender_name, msg_fmt);
    let target_msg_parsed = core::format::parse_tags(&target_msg);
    let target_msg_rendered = target_msg_parsed.render(true, true);
    let target_msg_bytes = format!("{}\r\n", target_msg_rendered).into_bytes();

    for &other in &registry.connected_entities() {
        if other == entity {
            continue;
        }
        if let Some(other_room) = get_pos_room(world, other) {
            if let Ok(mut q) = world.query_one::<&core::RoomKey>(other_room) {
                if let Some(sk) = q.get() {
                    if sk.0.starts_with(&area_id) {
                        if let Some(tx) = registry.sender(other) {
                            let _ = tx.send(target_msg_bytes.clone());
                        }
                    }
                }
            }
        }
    }

    send_to_conn(conn, &format!("You shout, \"{}\"", msg_fmt));
}

pub fn cmd_whisper(
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

    let is_ghost = world
        .query_one::<&core::PlayerState>(entity)
        .ok()
        .and_then(|mut q| q.get().map(|s| matches!(s, core::PlayerState::Dead)))
        .unwrap_or(false);

    let (target_name, message) = match args.split_once(' ') {
        Some((t, m)) if !t.is_empty() && !m.is_empty() => (t, m),
        _ => {
            conn.send_line("Whisper to whom what?");
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

    let target_entity = {
        let occupants = registry.occupants(world, room);
        let lower_target = target_name.to_lowercase();
        let candidates: Vec<core::Entity> = occupants
            .into_iter()
            .filter(|&e| {
                if let Some(n) = get_name(world, e) {
                    n.0.to_lowercase().starts_with(&lower_target)
                } else {
                    false
                }
            })
            .collect();

        if let Some(&exact) = candidates.iter().find(|&&e| {
            if let Some(n) = get_name(world, e) {
                n.0.to_lowercase() == lower_target
            } else {
                false
            }
        }) {
            Some(exact)
        } else if candidates.len() == 1 {
            Some(candidates[0])
        } else {
            None
        }
    };

    let target_entity = match target_entity {
        Some(te) => te,
        None => {
            conn.send_line("No one by that name is here.");
            return;
        }
    };

    if target_entity == entity {
        conn.send_line("You whisper to yourself.");
        return;
    }

    let sender_name = get_name(world, entity).unwrap_or(Name::new("Someone")).0;
    let target_name_real = get_name(world, target_entity)
        .unwrap_or(Name::new("Someone"))
        .0;

    let msg_fmt = if is_ghost {
        format_ghost_text(message)
    } else {
        message.to_string()
    };

    send_to_online_player(
        registry,
        target_entity,
        &format!("{} whispers to you, \"{}\"", sender_name, msg_fmt),
    );

    send_to_conn(
        conn,
        &format!("You whisper to {}, \"{}\"", target_name_real, msg_fmt),
    );
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
    if let Ok(mut q) = world.query_one::<&core::Inventory>(player) {
        if let Some(inv) = q.get() {
            for &item_entity in &inv.0 {
                if let Ok(mut item_q) = world.query_one::<&core::Item>(item_entity) {
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
    if let Ok(mut name_q) = world.query_one::<&core::Name>(entity) {
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
    if let Ok(mut name_q) = world.query_one::<&core::Name>(entity) {
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
    if let Ok(mut name_q) = world.query_one::<&core::Name>(entity) {
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
    if let Ok(mut name_q) = world.query_one::<&core::Name>(entity) {
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

pub fn cmd_die(
    world: &mut World,
    conn: &mut dyn Connection,
    _name: &str,
    _args: &str,
    registry: &ConnectionRegistry,
) {
    let entity = match conn.entity() {
        Some(e) => e,
        None => return,
    };

    let mut is_unconscious = false;
    if let Ok(mut q) = world.query_one::<&core::Health>(entity) {
        if let Some(hp) = q.get() {
            if hp.current <= 0 {
                is_unconscious = true;
            }
        }
    }
    if !is_unconscious {
        if let Ok(mut q) = world.query_one::<&core::PlayerState>(entity) {
            if let Some(core::PlayerState::Resting(core::RestState::Unconscious)) = q.get() {
                is_unconscious = true;
            }
        }
    }

    if !is_unconscious {
        conn.send_line("You can only choose to die when you are unconscious.");
        return;
    }

    let name = world
        .query_one::<&core::Name>(entity)
        .ok()
        .and_then(|mut q| q.get().map(|n| n.0.clone()))
        .unwrap_or_else(|| "Someone".to_string());

    let room = get_pos_room(world, entity);

    core::systems::combat::handle_death(world, entity);

    conn.send_line(
        "You choose to submit to death...\r\nAlas, you are dead! You are a ghost now...",
    );

    oxide_server::prompt::send_player_prompt(world, entity, registry);

    if let Some(r) = room {
        broadcast_to_room_except(
            world,
            registry,
            r,
            entity,
            &format!("{} has died.\r\n{} is dead! R.I.P.", name, name),
        );
    }
}

pub fn cmd_reclaim(
    world: &mut World,
    conn: &mut dyn Connection,
    _name: &str,
    _args: &str,
    registry: &ConnectionRegistry,
) {
    let entity = match conn.entity() {
        Some(e) => e,
        None => return,
    };

    let is_ghost = world
        .query_one::<&core::PlayerState>(entity)
        .ok()
        .and_then(|mut q| q.get().map(|s| matches!(s, core::PlayerState::Dead)))
        .unwrap_or(false);

    if !is_ghost {
        conn.send_line("You are already alive.");
        return;
    }

    let player_db_id = world
        .query_one::<&core::DbId>(entity)
        .ok()
        .and_then(|mut q| q.get().map(|d| d.0));

    let corpse_entity = if let Some(room) = get_pos_room(world, entity) {
        let mut found = None;
        let mut to_update = None;
        {
            let mut q = world.query::<(&core::Corpse, &core::Position)>();
            for (raw, (corpse, pos)) in q.iter() {
                if pos.room == room {
                    let is_owner = corpse.owner == Some(entity)
                        || (player_db_id.is_some() && corpse.owner_db_id == player_db_id);
                    if is_owner {
                        found = Some(core::Entity::from(raw));
                        if corpse.owner != Some(entity) {
                            to_update = Some(core::Entity::from(raw));
                        }
                        break;
                    }
                }
            }
        }
        if let Some(c_entity) = to_update {
            if let Ok(mut q) = world.query_one::<&mut core::Corpse>(c_entity) {
                if let Some(corpse) = q.get() {
                    corpse.owner = Some(entity);
                }
            }
        }
        found
    } else {
        None
    };

    let corpse_entity = match corpse_entity {
        Some(c) => c,
        None => {
            conn.send_line("Your corpse is not in this room. You cannot reclaim your body here.");
            return;
        }
    };

    let corpse_eq = world
        .query_one::<&core::Equipment>(corpse_entity)
        .ok()
        .and_then(|mut q| q.get().map(|eq| eq.slots.clone()))
        .unwrap_or_default();

    if let Ok(mut q) = world.query_one::<&mut core::Equipment>(entity) {
        if let Some(player_eq) = q.get() {
            player_eq.slots = corpse_eq;
        }
    }

    let corpse_inv = world
        .query_one::<&core::Inventory>(corpse_entity)
        .ok()
        .and_then(|mut q| q.get().map(|inv| inv.0.clone()))
        .unwrap_or_default();

    if let Ok(mut q) = world.query_one::<&mut core::Inventory>(entity) {
        if let Some(player_inv) = q.get() {
            player_inv.0 = corpse_inv;
        }
    }

    if let Ok(mut q) = world.query_one::<&mut core::Health>(entity) {
        if let Some(hp) = q.get() {
            hp.current = hp.max;
        }
    }

    let _ = world.insert(
        entity,
        (
            core::PlayerState::Resting(core::RestState::Standing),
            core::Dirty,
        ),
    );

    let _ = world.despawn(corpse_entity);

    conn.send_line("You reclaim your body and return to the land of the living!");
    if let Ok(mut name_q) = world.query_one::<&core::Name>(entity) {
        if let Some(name) = name_q.get() {
            if let Some(room) = get_pos_room(world, entity) {
                broadcast_to_room_except(
                    world,
                    registry,
                    room,
                    entity,
                    &format!("{} returns to life!", name.0),
                );
            }
        }
    }
}

pub fn cmd_revive(
    world: &mut World,
    conn: &mut dyn Connection,
    name: &str,
    args: &str,
    registry: &ConnectionRegistry,
) {
    let entity = match conn.entity() {
        Some(e) => e,
        None => return,
    };

    let is_ghost = world
        .query_one::<&core::PlayerState>(entity)
        .ok()
        .and_then(|mut q| q.get().map(|s| matches!(s, core::PlayerState::Dead)))
        .unwrap_or(false);

    if !is_ghost {
        conn.send_line("You are already alive.");
        return;
    }

    let room = match get_pos_room(world, entity) {
        Some(r) => r,
        None => {
            conn.send_line("You are nowhere.");
            return;
        }
    };

    let player_db_id = world
        .query_one::<&core::DbId>(entity)
        .ok()
        .and_then(|mut q| q.get().map(|d| d.0));

    let corpse_entity = {
        let mut found = None;
        let mut q = world.query::<(&core::Corpse, &core::Position)>();
        for (raw, (corpse, pos)) in q.iter() {
            if pos.room == room {
                let is_owner = corpse.owner == Some(entity)
                    || (player_db_id.is_some() && corpse.owner_db_id == player_db_id);
                if is_owner {
                    found = Some(core::Entity::from(raw));
                    break;
                }
            }
        }
        found
    };

    if corpse_entity.is_some() {
        cmd_reclaim(world, conn, name, args, registry);
        return;
    }

    let can_revive = world.query_one::<&core::RoomAllowRevive>(room).is_ok();

    if !can_revive {
        conn.send_line("You cannot revive here. You must find your corpse or pray at a temple.");
        return;
    }

    if let Ok(mut q) = world.query_one::<&mut core::Health>(entity) {
        if let Some(hp) = q.get() {
            hp.current = hp.max;
        }
    }

    let _ = world.insert(
        entity,
        (
            core::PlayerState::Resting(core::RestState::Standing),
            core::Dirty,
        ),
    );

    conn.send_line("You pray at the altar and are restored to life! (Your equipment remains with your corpse.)");
    if let Ok(mut name_q) = world.query_one::<&core::Name>(entity) {
        if let Some(name) = name_q.get() {
            broadcast_to_room_except(
                world,
                registry,
                room,
                entity,
                &format!("{} returns to life!", name.0),
            );
        }
    }
}

pub fn cmd_toggle(
    world: &mut World,
    conn: &mut dyn Connection,
    _name: &str,
    args: &str,
    _registry: &ConnectionRegistry,
) {
    let entity = match conn.entity() {
        Some(e) => e,
        None => return,
    };

    let arg = args.trim().to_lowercase();
    if arg == "resurrect" || arg == "res" {
        let mut current_val = false;
        if let Ok(mut q) = world.query_one::<&mut core::Player>(entity) {
            if let Some(player) = q.get() {
                player.no_resurrect = !player.no_resurrect;
                current_val = player.no_resurrect;
            }
        }
        let _ = world.insert(entity, (core::Dirty,));
        if current_val {
            conn.send_line("You will now prevent unwanted resurrections (no_resurrect is ON).");
        } else {
            conn.send_line("You now allow resurrections (no_resurrect is OFF).");
        }
    } else {
        conn.send_line("Toggle options: resurrect");
    }
}

pub fn cmd_time(
    _world: &mut World,
    conn: &mut dyn Connection,
    _name: &str,
    _args: &str,
    _registry: &ConnectionRegistry,
) {
    conn.send_line("It is currently 10:00 AM in the morning.");
}

pub fn cmd_weather(
    _world: &mut World,
    conn: &mut dyn Connection,
    _name: &str,
    _args: &str,
    _registry: &ConnectionRegistry,
) {
    conn.send_line("The sky is clear and a gentle breeze blows from the east.");
}

pub fn cmd_quest(
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

    let templates = match oxide_server::get_templates() {
        Some(t) => t,
        None => {
            conn.send_line("Error: Template registry not loaded.");
            return;
        }
    };

    let parts: Vec<&str> = args.split_whitespace().collect();
    if parts.is_empty() {
        list_quests(world, conn, entity, &templates);
        return;
    }

    match parts[0] {
        "list" => {
            list_quests(world, conn, entity, &templates);
        }
        "show" | "info" => {
            if parts.len() < 2 {
                conn.send_line("Usage: quest show <quest_id>");
                return;
            }
            show_quest(world, conn, entity, parts[1], &templates);
        }
        "accept" => {
            if parts.len() < 2 {
                conn.send_line("Usage: quest accept <quest_id>");
                return;
            }
            accept_quest_command(world, conn, entity, parts[1], &templates);
        }
        "complete" | "turnin" => {
            if parts.len() < 2 {
                conn.send_line("Usage: quest complete <quest_id>");
                return;
            }
            complete_quest_command(world, conn, entity, parts[1], &templates);
        }
        "abandon" => {
            if parts.len() < 2 {
                conn.send_line("Usage: quest abandon <quest_id>");
                return;
            }
            abandon_quest_command(world, conn, entity, parts[1]);
        }
        _ => {
            conn.send_line("Unknown quest subcommand. Try: list, show, accept, complete, abandon");
        }
    }
}

fn list_quests(
    world: &World,
    conn: &mut dyn Connection,
    player: core::Entity,
    templates: &core::templates::TemplateRegistry,
) {
    let mut q_log = match world.query_one::<&core::QuestLog>(player) {
        Ok(q) => q,
        Err(_) => {
            conn.send_line("You have no quest log.");
            return;
        }
    };
    let quest_log = match q_log.get() {
        Some(log) => log,
        None => {
            conn.send_line("You have no quest log.");
            return;
        }
    };

    if quest_log.active.is_empty() && quest_log.completed.is_empty() {
        conn.send_line("You have no quests.");
        return;
    }

    if !quest_log.active.is_empty() {
        conn.send_line("Active Quests:");
        for (quest_id, progress) in &quest_log.active {
            if let Some(quest_def) = templates.quests.get(quest_id) {
                let all_done = progress.objectives.iter().all(|o| o.completed);
                let status = if all_done { " (Ready to turn in)" } else { "" };
                conn.send_line(&format!(
                    "  {} - {}{}",
                    quest_def.id, quest_def.name, status
                ));
            } else {
                conn.send_line(&format!("  {} (Unknown Quest definition)", quest_id));
            }
        }
    }

    if !quest_log.completed.is_empty() {
        conn.send_line("Completed Quests:");
        for quest_id in &quest_log.completed {
            if let Some(quest_def) = templates.quests.get(quest_id) {
                conn.send_line(&format!("  {} - {}", quest_def.id, quest_def.name));
            } else {
                conn.send_line(&format!("  {}", quest_id));
            }
        }
    }
}

fn show_quest(
    world: &World,
    conn: &mut dyn Connection,
    player: core::Entity,
    quest_id: &str,
    templates: &core::templates::TemplateRegistry,
) {
    let mut q_log = match world.query_one::<&core::QuestLog>(player) {
        Ok(q) => q,
        Err(_) => {
            conn.send_line("You have no quest log.");
            return;
        }
    };
    let quest_log = match q_log.get() {
        Some(log) => log,
        None => {
            conn.send_line("You have no quest log.");
            return;
        }
    };

    let progress = match quest_log.active.get(quest_id) {
        Some(p) => p,
        None => {
            if quest_log.completed.contains(quest_id) {
                if let Some(quest_def) = templates.quests.get(quest_id) {
                    conn.send_line(&format!(
                        "Quest: {}\r\nStatus: Completed\r\nDescription: {}",
                        quest_def.name, quest_def.description
                    ));
                } else {
                    conn.send_line(&format!("Quest '{}' (Completed)", quest_id));
                }
            } else {
                conn.send_line("You are not on that quest.");
            }
            return;
        }
    };

    let quest_def = match templates.quests.get(quest_id) {
        Some(d) => d,
        None => {
            conn.send_line("Error: Quest definition not found.");
            return;
        }
    };

    conn.send_line(&format!("Quest: {}", quest_def.name));
    conn.send_line(&format!("Description: {}", quest_def.description));
    conn.send_line("Objectives:");
    for (objective, obj_progress) in quest_def.objectives.iter().zip(&progress.objectives) {
        let status = if obj_progress.completed { "[x]" } else { "[ ]" };
        match objective {
            core::templates::QuestObjective::Kill { mob, count } => {
                let mob_name = templates
                    .mobs
                    .get(mob)
                    .map(|m| m.name.as_str())
                    .unwrap_or(mob);
                conn.send_line(&format!(
                    "  {} Kill {}: {}/{}",
                    status, mob_name, obj_progress.current, count
                ));
            }
            core::templates::QuestObjective::Gather { item, count } => {
                let item_name = templates
                    .items
                    .get(item)
                    .map(|i| i.name.as_str())
                    .unwrap_or(item);
                conn.send_line(&format!(
                    "  {} Gather {}: {}/{}",
                    status, item_name, obj_progress.current, count
                ));
            }
            core::templates::QuestObjective::Deliver { item, npc } => {
                let item_name = templates
                    .items
                    .get(item)
                    .map(|i| i.name.as_str())
                    .unwrap_or(item);
                let npc_name = templates
                    .mobs
                    .get(npc)
                    .map(|m| m.name.as_str())
                    .unwrap_or(npc);
                conn.send_line(&format!(
                    "  {} Deliver {} to {}",
                    status, item_name, npc_name
                ));
            }
            core::templates::QuestObjective::Explore { room } => {
                conn.send_line(&format!("  {} Explore room: {}", status, room));
            }
            core::templates::QuestObjective::Talk { npc } => {
                let npc_name = templates
                    .mobs
                    .get(npc)
                    .map(|m| m.name.as_str())
                    .unwrap_or(npc);
                conn.send_line(&format!("  {} Talk to {}", status, npc_name));
            }
        }
    }
}

fn accept_quest_command(
    world: &mut World,
    conn: &mut dyn Connection,
    player: core::Entity,
    quest_id: &str,
    templates: &core::templates::TemplateRegistry,
) {
    let quest_def = match templates.quests.get(quest_id) {
        Some(qd) => qd,
        None => {
            conn.send_line(&format!("Quest '{}' does not exist.", quest_id));
            return;
        }
    };

    let room = match get_pos_room(world, player) {
        Some(r) => r,
        None => {
            conn.send_line("You are nowhere.");
            return;
        }
    };

    if let Some(ref giver_id) = quest_def.giver_npc {
        let occupants = core::util::entities_in_room(world, room);
        let mut giver_present = false;
        for occupant in occupants {
            if let Ok(mut q_npc) = world.query_one::<&core::Npc>(occupant) {
                if let Some(npc) = q_npc.get() {
                    if npc.template_id == *giver_id {
                        giver_present = true;
                        break;
                    }
                }
            }
        }
        if !giver_present {
            let giver_name = templates
                .mobs
                .get(giver_id)
                .map(|m| m.name.as_str())
                .unwrap_or(giver_id);
            conn.send_line(&format!(
                "You must be near {} to accept this quest.",
                giver_name
            ));
            return;
        }
    }

    match core::accept_quest(world, player, quest_id, templates) {
        Ok(msgs) => {
            for msg in msgs {
                conn.send_line(&msg);
            }
        }
        Err(err) => {
            conn.send_line(&err);
        }
    }
}

fn complete_quest_command(
    world: &mut World,
    conn: &mut dyn Connection,
    player: core::Entity,
    quest_id: &str,
    templates: &core::templates::TemplateRegistry,
) {
    let quest_def = match templates.quests.get(quest_id) {
        Some(qd) => qd,
        None => {
            conn.send_line(&format!("Quest '{}' does not exist.", quest_id));
            return;
        }
    };

    let room = match get_pos_room(world, player) {
        Some(r) => r,
        None => {
            conn.send_line("You are nowhere.");
            return;
        }
    };

    if let Some(ref turn_in_id) = quest_def.turn_in_npc {
        let occupants = core::util::entities_in_room(world, room);
        let mut turn_in_present = false;
        for occupant in occupants {
            if let Ok(mut q_npc) = world.query_one::<&core::Npc>(occupant) {
                if let Some(npc) = q_npc.get() {
                    if npc.template_id == *turn_in_id {
                        turn_in_present = true;
                        break;
                    }
                }
            }
        }
        if !turn_in_present {
            let turn_in_name = templates
                .mobs
                .get(turn_in_id)
                .map(|m| m.name.as_str())
                .unwrap_or(turn_in_id);
            conn.send_line(&format!(
                "You must be near {} to complete this quest.",
                turn_in_name
            ));
            return;
        }
    }

    match core::complete_quest(world, player, quest_id, templates) {
        Ok(msgs) => {
            for msg in msgs {
                conn.send_line(&msg);
            }
            let level_up_msgs = oxide_server::award_xp(world, player);
            for msg in level_up_msgs {
                conn.send_line(&msg);
            }
        }
        Err(err) => {
            conn.send_line(&err);
        }
    }
}

fn abandon_quest_command(
    world: &mut World,
    conn: &mut dyn Connection,
    player: core::Entity,
    quest_id: &str,
) {
    match core::abandon_quest(world, player, quest_id) {
        Ok(msgs) => {
            for msg in msgs {
                conn.send_line(&msg);
            }
        }
        Err(err) => {
            conn.send_line(&err);
        }
    }
}

pub fn cmd_use(
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

    let templates = match oxide_server::get_templates() {
        Some(t) => t,
        None => {
            conn.send_line("Error: Template registry not loaded.");
            return;
        }
    };

    let mut parts = args.split_whitespace();
    let skill_input = match parts.next() {
        Some(s) => s,
        None => {
            conn.send_line("Use what skill?");
            return;
        }
    };

    let target_arg = parts.next();

    let skill_id = match templates.resolve_skill(skill_input, None) {
        Ok(id) => id,
        Err(core::templates::SkillResolveError::NotFound) => {
            conn.send_line(&format!(
                "You don't know any skill named '{}'.",
                skill_input
            ));
            return;
        }
        Err(core::templates::SkillResolveError::Multiple(candidates)) => {
            let names: Vec<&str> = candidates.iter().map(|(id, _)| id.as_str()).collect();
            conn.send_line(&format!("Which skill did you mean? {}", names.join(", ")));
            return;
        }
    };

    let skill_def = match templates.skills.get(&skill_id) {
        Some(def) => def,
        None => {
            conn.send_line("Error: Skill definition not found in registry.");
            return;
        }
    };

    let target_entity = if let Some(target_name) = target_arg {
        let room = match get_pos_room(world, entity) {
            Some(r) => r,
            None => {
                conn.send_line("You are nowhere.");
                return;
            }
        };

        let occupants = core::util::entities_in_room(world, room);
        let mut matched = None;
        for occupant in occupants {
            if occupant == entity {
                continue;
            }
            if let Ok(mut q_name) = world.query_one::<&core::Name>(occupant) {
                if let Some(name) = q_name.get() {
                    if name
                        .0
                        .to_lowercase()
                        .starts_with(&target_name.to_lowercase())
                    {
                        matched = Some(occupant);
                        break;
                    }
                }
            }
        }
        if matched.is_none() {
            conn.send_line(&format!("You don't see '{}' here.", target_name));
            return;
        }
        matched
    } else {
        None
    };

    if let Err(err) = core::can_use_skill(world, entity, skill_def, target_entity) {
        conn.send_line(&err);
        return;
    }

    let _ = core::deduct_resource_cost(world, entity, &skill_def.cost);

    if skill_def.cooldown_secs > 0 {
        let mut cd_comp = world
            .query_one::<&core::SkillCooldowns>(entity)
            .ok()
            .and_then(|mut q| q.get().cloned())
            .unwrap_or_default();
        cd_comp
            .cooldowns
            .insert(skill_def.id.clone(), skill_def.cooldown_secs);
        let _ = world.insert(entity, (cd_comp, core::Dirty));
    }

    conn.send_line(&format!("You use {}!", skill_def.name));

    if let Ok(mut q_name) = world.query_one::<&core::Name>(entity) {
        if let Some(name) = q_name.get() {
            if let Some(room) = get_pos_room(world, entity) {
                let msg = format!("{} uses {}!", name.0, skill_def.name);
                broadcast_to_room_except(world, registry, room, entity, &msg);
            }
        }
    }

    if let Some(ref effect) = skill_def.effect {
        let msgs = core::apply_skill_effect(
            world,
            entity,
            target_entity,
            effect,
            &skill_def.name,
            &templates,
        );
        for msg in msgs {
            conn.send_line(&msg);
        }
    }
}

pub fn cmd_cast(
    world: &mut World,
    conn: &mut dyn Connection,
    name: &str,
    args: &str,
    registry: &ConnectionRegistry,
) {
    let templates = match oxide_server::get_templates() {
        Some(t) => t,
        None => {
            conn.send_line("Error: Template registry not loaded.");
            return;
        }
    };

    let mut parts = args.split_whitespace();
    let skill_input = match parts.next() {
        Some(s) => s,
        None => {
            conn.send_line("Cast what spell?");
            return;
        }
    };

    let skill_id = match templates.resolve_skill(skill_input, None) {
        Ok(id) => id,
        Err(core::templates::SkillResolveError::NotFound) => {
            conn.send_line(&format!(
                "You don't know any spell named '{}'.",
                skill_input
            ));
            return;
        }
        Err(core::templates::SkillResolveError::Multiple(candidates)) => {
            let names: Vec<&str> = candidates.iter().map(|(id, _)| id.as_str()).collect();
            conn.send_line(&format!("Which spell did you mean? {}", names.join(", ")));
            return;
        }
    };

    let skill_def = match templates.skills.get(&skill_id) {
        Some(def) => def,
        None => {
            conn.send_line("Error: Spell definition not found in registry.");
            return;
        }
    };

    if !matches!(skill_def.skill_type, core::SkillType::Magic) {
        conn.send_line("That is not a magic spell! Use 'use' instead.");
        return;
    }

    cmd_use(world, conn, name, args, registry);
}

pub fn cmd_faction(
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

    let templates = match oxide_server::get_templates() {
        Some(t) => t,
        None => {
            conn.send_line("Error: Template registry not loaded.");
            return;
        }
    };

    let fs = match world.query_one::<&core::FactionStanding>(entity) {
        Ok(mut q) => q.get().cloned().unwrap_or_default(),
        Err(_) => core::FactionStanding::new(),
    };

    if fs.standings.is_empty() {
        conn.send_line("You have no faction standings.");
        return;
    }

    conn.send_line("Your faction standings:");
    conn.send_line("------------------------------------------------");
    for (faction_id, standing) in &fs.standings {
        let faction_name = templates
            .factions
            .get(faction_id)
            .map(|f| f.name.as_str())
            .unwrap_or(faction_id);

        let rank = templates
            .factions
            .get(faction_id)
            .map(|f| f.get_rank(*standing))
            .unwrap_or_else(|| "Neutral".to_string());

        conn.send_line(&format!(
            "{:<24} : {:>6} ({})",
            faction_name, standing, rank
        ));
    }
}

pub fn cmd_recipes(
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

    let templates = match oxide_server::get_templates() {
        Some(t) => t,
        None => {
            conn.send_line("Error: Template registry not loaded.");
            return;
        }
    };

    let lr = match world.query_one::<&core::LearnedRecipes>(entity) {
        Ok(mut q) => q.get().cloned().unwrap_or_default(),
        Err(_) => core::LearnedRecipes::new(),
    };

    if lr.recipes.is_empty() {
        conn.send_line("You do not know any crafting recipes.");
        return;
    }

    conn.send_line("You know the following recipes:");
    conn.send_line("------------------------------------------------");
    for recipe_id in &lr.recipes {
        if let Some(recipe) = templates.recipes.get(recipe_id) {
            conn.send_line(&format!(
                "{} (Difficulty {}, Success Chance {}%):",
                recipe.name, recipe.difficulty, recipe.success_chance
            ));
            conn.send_line(&format!("  Description: {}", recipe.description));
            if let Some(ref station) = recipe.station {
                conn.send_line(&format!("  Station: {}", station.replace("station:", "")));
            }
            if let Some(ref skill_req) = recipe.skill_requirement {
                conn.send_line(&format!(
                    "  Skill: {} (Rank {})",
                    skill_req.id, skill_req.rank
                ));
            }
            conn.send_line("  Materials:");
            for material in &recipe.materials {
                let mat_name = templates
                    .items
                    .get(&material.template_id)
                    .map(|i| i.name.as_str())
                    .unwrap_or(&material.template_id);
                conn.send_line(&format!("    - {} x {}", mat_name, material.quantity));
            }
            let res_name = templates
                .items
                .get(&recipe.result.template_id)
                .map(|i| i.name.as_str())
                .unwrap_or(&recipe.result.template_id);
            conn.send_line("  Result:");
            conn.send_line(&format!("    - {} x {}", res_name, recipe.result.quantity));
            conn.send_line("");
        }
    }
}

pub fn cmd_craft(
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

    let templates = match oxide_server::get_templates() {
        Some(t) => t,
        None => {
            conn.send_line("Error: Template registry not loaded.");
            return;
        }
    };

    let recipe_input = args.trim();
    if recipe_input.is_empty() {
        conn.send_line("Craft what recipe?");
        return;
    }

    let recipe_input_lower = recipe_input.to_lowercase();
    let lr = match world.query_one::<&core::LearnedRecipes>(entity) {
        Ok(mut q) => q.get().cloned().unwrap_or_default(),
        Err(_) => core::LearnedRecipes::new(),
    };

    let mut candidates = Vec::new();
    for recipe_id in &lr.recipes {
        if let Some(recipe) = templates.recipes.get(recipe_id) {
            if recipe.name.to_lowercase().starts_with(&recipe_input_lower)
                || recipe_id.to_lowercase().starts_with(&recipe_input_lower)
            {
                candidates.push((recipe_id.clone(), recipe.name.clone()));
            }
        }
    }

    if candidates.is_empty() {
        conn.send_line(&format!(
            "You don't know any recipe matching '{}'.",
            recipe_input
        ));
        return;
    }

    if candidates.len() > 1 {
        let names: Vec<String> = candidates.iter().map(|(_, name)| name.clone()).collect();
        conn.send_line(&format!("Which recipe did you mean? {}", names.join(", ")));
        return;
    }

    let (recipe_id, _) = &candidates[0];
    match core::craft_recipe(world, entity, recipe_id, &templates) {
        Ok(msg) => conn.send_line(&msg),
        Err(err) => conn.send_line(&format!("Error: {}", err)),
    }
}

pub fn cmd_advance(
    world: &mut World,
    conn: &mut dyn Connection,
    _name: &str,
    args: &str,
    _registry: &ConnectionRegistry,
) {
    let player = match conn.entity() {
        Some(p) => p,
        None => return,
    };
    let class_id = args.trim();
    if class_id.is_empty() {
        conn.send_line("Usage: @advance <class_id>");
        return;
    }
    match oxide_server::advance_player_class(world, player, class_id) {
        Ok(msgs) => {
            for m in msgs {
                conn.send_line(&m);
            }
        }
        Err(e) => conn.send_line(&format!("Error: {}", e)),
    }
}

pub fn cmd_multi_class(
    world: &mut World,
    conn: &mut dyn Connection,
    _name: &str,
    args: &str,
    _registry: &ConnectionRegistry,
) {
    let player = match conn.entity() {
        Some(p) => p,
        None => return,
    };
    let class_id = args.trim();
    if class_id.is_empty() {
        conn.send_line("Usage: @multi_class <class_id>");
        return;
    }

    let templates = match oxide_server::get_templates() {
        Some(t) => t,
        None => {
            conn.send_line("Error: Template registry not loaded.");
            return;
        }
    };

    let class_template = match templates.get_class(class_id) {
        Some(c) => c,
        None => {
            conn.send_line(&format!("Error: Class '{}' not found.", class_id));
            return;
        }
    };

    if class_template.prestige {
        conn.send_line("Error: That is a prestige class. Use '@prestige' instead.");
        return;
    }

    let mut mc_info = match world.query_one::<&mut core::MultiClassInfo>(player) {
        Ok(mut q) => q.get().cloned().unwrap_or_default(),
        Err(_) => {
            conn.send_line("Error: You do not have class information.");
            return;
        }
    };

    if mc_info.has_class(class_id) {
        conn.send_line(&format!(
            "Error: You already have the class '{}'. Use '@advance' to level it up.",
            class_id
        ));
        return;
    }

    // Check race constraints
    if !class_template.allowed_races.is_empty() {
        let race = world
            .query_one::<&core::Race>(player)
            .ok()
            .and_then(|mut q| q.get().map(|r| r.0.clone()))
            .unwrap_or_default();
        if !class_template
            .allowed_races
            .iter()
            .any(|r| r.to_lowercase() == race.to_lowercase())
        {
            conn.send_line(&format!(
                "Error: Your race '{}' is not allowed for this class.",
                race
            ));
            return;
        }
    }

    // Check alignment constraints
    if !class_template.allowed_alignments.is_empty() {
        let align = world
            .query_one::<&core::Alignment>(player)
            .ok()
            .and_then(|mut q| q.get().map(|a| a.0.clone()))
            .unwrap_or_default();
        if !class_template
            .allowed_alignments
            .iter()
            .any(|a| a.to_lowercase() == align.to_lowercase())
        {
            conn.send_line(&format!(
                "Error: Your alignment '{}' is not allowed for this class.",
                align
            ));
            return;
        }
    }

    // Must have a pending level up to multi-class
    let current_level = mc_info.total_level();
    let next_level = current_level + 1;
    let xp = world
        .query_one::<&core::Experience>(player)
        .ok()
        .and_then(|mut q| q.get().map(|x| x.0))
        .unwrap_or(0);
    let threshold = core::Experience::for_level(next_level);

    if xp < threshold {
        conn.send_line(&format!(
            "Error: You need {} XP to level up/multi-class, but you only have {}.",
            threshold, xp
        ));
        return;
    }

    // Add new class at level 1, non-favored
    mc_info.add_class(class_id.to_string(), 1, false);

    // HP gain: hit die + CON mod
    let attrs = world
        .query_one::<&core::Attributes>(player)
        .ok()
        .and_then(|mut q| q.get().cloned())
        .unwrap_or_default();
    let con_mod = (attrs.constitution as i32 - 10) / 2;
    let hp_gain = (class_template.hit_die as i32 + con_mod).max(1);

    if let Ok(mut q) = world.query_one::<&mut core::Health>(player) {
        if let Some(health) = q.get() {
            health.max += hp_gain;
            health.current = health.max;
        }
    }

    // Update level and experience
    if let Ok(mut q) = world.query_one::<&mut core::Level>(player) {
        if let Some(level) = q.get() {
            level.0 = next_level;
        }
    }

    if let Ok(mut q) = world.query_one::<&mut core::Experience>(player) {
        if let Some(xp_comp) = q.get() {
            xp_comp.0 = xp.saturating_sub(threshold);
            let db_arc = oxide_server::get_db();
            let db = db_arc.as_ref().and_then(|d| d.try_lock().ok());
            let conn_db = db.as_ref().map(|g| g.conn());
            if let Some(conn_db) = conn_db {
                if let Ok(mut q_db) = world.query_one::<&core::DbId>(player) {
                    if let Some(db_id) = q_db.get() {
                        let _ =
                            oxide_data::save_level_component(conn_db, db_id.0, next_level as i64);
                        let _ = oxide_data::save_experience_component(
                            conn_db,
                            db_id.0,
                            xp_comp.0 as i64,
                        );
                    }
                }
            }
        }
    }

    // Recalculate Mana pool
    if let Ok(mut q) = world.query_one::<&mut core::Mana>(player) {
        if let Some(mana) = q.get() {
            let formula_mana = core::Mana::from_formula(
                next_level as u16,
                attrs.intelligence as u16,
                attrs.wisdom as u16,
            );
            mana.max = formula_mana.max;
            mana.current = mana.max;
        }
    }

    // Recalculate Stamina pool
    if let Ok(mut q) = world.query_one::<&mut core::Stamina>(player) {
        if let Some(stamina) = q.get() {
            let formula_stamina = core::Stamina::from_formula(
                next_level as u16,
                attrs.strength as u16,
                attrs.dexterity as u16,
            );
            stamina.max = formula_stamina.max;
            stamina.current = stamina.max;
        }
    }

    let new_combat_stats = core::calculate_multiclass_combat_stats(&mc_info, &templates);
    let _ = world.insert(player, (new_combat_stats,));

    let _ = world.insert(player, (mc_info,));
    let _ = world.insert(player, (core::Dirty,));

    conn.send_line(&format!(
        "You successfully multi-classed into {}!",
        class_template.name
    ));
    conn.send_line(&format!(
        "You are now level {} ({} level 1).",
        next_level, class_template.name
    ));
    conn.send_line(&format!("You gained {} max HP.", hp_gain));
}

pub fn cmd_prestige(
    world: &mut World,
    conn: &mut dyn Connection,
    _name: &str,
    args: &str,
    _registry: &ConnectionRegistry,
) {
    let player = match conn.entity() {
        Some(p) => p,
        None => return,
    };
    let class_id = args.trim();
    if class_id.is_empty() {
        conn.send_line("Usage: @prestige <prestige_class_id>");
        return;
    }

    let templates = match oxide_server::get_templates() {
        Some(t) => t,
        None => {
            conn.send_line("Error: Template registry not loaded.");
            return;
        }
    };

    let class_template = match templates.get_class(class_id) {
        Some(c) => c,
        None => {
            conn.send_line(&format!("Error: Prestige Class '{}' not found.", class_id));
            return;
        }
    };

    if !class_template.prestige {
        conn.send_line("Error: That is not a prestige class. Use '@multi_class' instead.");
        return;
    }

    if let Some(ref gate) = class_template.prestige_gate {
        if let Err(gate_err) = core::satisfies_prestige_gate(world, player, gate, &templates) {
            conn.send_line(&format!(
                "Error: You do not satisfy requirements for prestige class '{}': {}",
                class_template.name, gate_err
            ));
            return;
        }
    }

    let mut mc_info = match world.query_one::<&mut core::MultiClassInfo>(player) {
        Ok(mut q) => q.get().cloned().unwrap_or_default(),
        Err(_) => {
            conn.send_line("Error: You do not have class information.");
            return;
        }
    };

    if mc_info.has_class(class_id) {
        conn.send_line(&format!(
            "Error: You already have prestige class '{}'. Use '@advance' to level it up.",
            class_id
        ));
        return;
    }

    let current_level = mc_info.total_level();
    let next_level = current_level + 1;
    let xp = world
        .query_one::<&core::Experience>(player)
        .ok()
        .and_then(|mut q| q.get().map(|x| x.0))
        .unwrap_or(0);
    let threshold = core::Experience::for_level(next_level);

    if xp < threshold {
        conn.send_line(&format!(
            "Error: You need {} XP to level up/adopt prestige class, but you only have {}.",
            threshold, xp
        ));
        return;
    }

    mc_info.add_class(class_id.to_string(), 1, false);

    let attrs = world
        .query_one::<&core::Attributes>(player)
        .ok()
        .and_then(|mut q| q.get().cloned())
        .unwrap_or_default();
    let con_mod = (attrs.constitution as i32 - 10) / 2;
    let hp_gain = (class_template.hit_die as i32 + con_mod).max(1);

    if let Ok(mut q) = world.query_one::<&mut core::Health>(player) {
        if let Some(health) = q.get() {
            health.max += hp_gain;
            health.current = health.max;
        }
    }

    if let Ok(mut q) = world.query_one::<&mut core::Level>(player) {
        if let Some(level) = q.get() {
            level.0 = next_level;
        }
    }

    if let Ok(mut q) = world.query_one::<&mut core::Experience>(player) {
        if let Some(xp_comp) = q.get() {
            xp_comp.0 = xp.saturating_sub(threshold);
            let db_arc = oxide_server::get_db();
            let db = db_arc.as_ref().and_then(|d| d.try_lock().ok());
            let conn_db = db.as_ref().map(|g| g.conn());
            if let Some(conn_db) = conn_db {
                if let Ok(mut q_db) = world.query_one::<&core::DbId>(player) {
                    if let Some(db_id) = q_db.get() {
                        let _ =
                            oxide_data::save_level_component(conn_db, db_id.0, next_level as i64);
                        let _ = oxide_data::save_experience_component(
                            conn_db,
                            db_id.0,
                            xp_comp.0 as i64,
                        );
                    }
                }
            }
        }
    }

    if let Ok(mut q) = world.query_one::<&mut core::Mana>(player) {
        if let Some(mana) = q.get() {
            let formula_mana = core::Mana::from_formula(
                next_level as u16,
                attrs.intelligence as u16,
                attrs.wisdom as u16,
            );
            mana.max = formula_mana.max;
            mana.current = mana.max;
        }
    }

    if let Ok(mut q) = world.query_one::<&mut core::Stamina>(player) {
        if let Some(stamina) = q.get() {
            let formula_stamina = core::Stamina::from_formula(
                next_level as u16,
                attrs.strength as u16,
                attrs.dexterity as u16,
            );
            stamina.max = formula_stamina.max;
            stamina.current = stamina.max;
        }
    }

    let new_combat_stats = core::calculate_multiclass_combat_stats(&mc_info, &templates);
    let _ = world.insert(player, (new_combat_stats,));

    let _ = world.insert(player, (mc_info,));
    let _ = world.insert(player, (core::Dirty,));

    conn.send_line(&format!(
        "You successfully unlocked prestige class {}!",
        class_template.name
    ));
    conn.send_line(&format!(
        "You are now level {} ({} level 1).",
        next_level, class_template.name
    ));
    conn.send_line(&format!("You gained {} max HP.", hp_gain));
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
        // Show status
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

            // Find target player first to invite
            let mut target_entity = None;
            for (ent, (n_comp, _player_comp)) in
                world.query::<(&core::Name, &core::Player)>().iter()
            {
                if n_comp.as_str().eq_ignore_ascii_case(target_name) {
                    target_entity = Some(core::Entity::from(ent));
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

                    // Notify target
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
        "accept" => {
            match core::handle_group_accept(world, entity) {
                Ok((_inviter, group_entity, invitee_name)) => {
                    conn.send_line("You join the group.");

                    // Notify group
                    if let Ok(mut q_group) = world.query_one::<&core::Group>(group_entity) {
                        if let Some(group) = q_group.get() {
                            for m in &group.members {
                                if let Some(m_ent) = m.entity {
                                    if m_ent != entity {
                                        if let Some(tx) = registry.sender(m_ent) {
                                            let _ = tx.send(
                                                format!(
                                                    "{} has joined the group.\r\n",
                                                    invitee_name
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
                Err(err) => {
                    conn.send_line(&err);
                }
            }
        }
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

    let my_room = match world.query_one::<&core::Position>(entity) {
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
            if let Ok(mut q_name) = world.query_one::<&core::Name>(other) {
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
        .query_one::<&core::Name>(target)
        .ok()
        .and_then(|mut q| q.get().map(|n| n.as_str().to_string()))
        .unwrap_or_else(|| "Someone".to_string());
    let my_name_str = world
        .query_one::<&core::Name>(entity)
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
            .query_one::<&core::Name>(f.target)
            .ok()
            .and_then(|mut q| q.get().map(|n| n.as_str().to_string()))
            .unwrap_or_else(|| "Someone".to_string());
        let my_name_str = world
            .query_one::<&core::Name>(entity)
            .ok()
            .and_then(|mut q| q.get().map(|n| n.as_str().to_string()))
            .unwrap_or_else(|| "Someone".to_string());

        let _ = world.remove_one::<core::Following>(entity);
        let _ = world.insert(entity, (core::Dirty,));

        conn.send_line(&format!("You stop following {}.", target_name_str));

        let my_room = world
            .query_one::<&core::Position>(entity)
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
    use super::*;
    use oxide_core as core;
    use oxide_core::Exit;
    use oxide_server::ConnectionFlags;
    use std::cell::RefCell;
    use std::collections::VecDeque;

    /// A mock connection that records all send_line calls.
    struct MockConnection {
        lines: RefCell<VecDeque<String>>,
        entity: RefCell<Option<core::Entity>>,
        disconnected: RefCell<bool>,
        flags: RefCell<ConnectionFlags>,
        screen_width: RefCell<u16>,
        access_level: RefCell<core::AccessLevel>,
    }

    impl MockConnection {
        fn new() -> Self {
            MockConnection {
                lines: RefCell::new(VecDeque::new()),
                entity: RefCell::new(None),
                disconnected: RefCell::new(false),
                flags: RefCell::new(ConnectionFlags::new()),
                screen_width: RefCell::new(0),
                access_level: RefCell::new(core::AccessLevel::Player),
            }
        }

        fn take_lines(&self) -> Vec<String> {
            self.lines.borrow_mut().drain(..).collect()
        }

        fn was_disconnected(&self) -> bool {
            *self.disconnected.borrow()
        }
    }

    impl Connection for MockConnection {
        fn send_line(&mut self, text: &str) {
            self.lines.borrow_mut().push_back(text.to_string());
        }
        fn send(&mut self, text: &str) {
            self.lines
                .borrow_mut()
                .push_back(format!("[inline] {text}"));
        }
        fn send_raw(&mut self, _bytes: &[u8]) {}
        fn id(&self) -> u64 {
            0
        }
        fn entity(&self) -> Option<core::Entity> {
            *self.entity.borrow()
        }
        fn set_entity(&mut self, entity: core::Entity) {
            self.entity.borrow_mut().replace(entity);
        }
        fn disconnect(&mut self) {
            self.disconnected.borrow_mut().clone_from(&true);
        }
        fn is_disconnected(&self) -> bool {
            *self.disconnected.borrow()
        }
        fn flags(&self) -> ConnectionFlags {
            *self.flags.borrow()
        }
        fn set_flags(&mut self, flags: ConnectionFlags) {
            self.flags.borrow_mut().clone_from(&flags);
        }
        fn screen_width(&self) -> u16 {
            *self.screen_width.borrow()
        }
        fn set_screen_width(&mut self, width: u16) {
            *self.screen_width.borrow_mut() = width;
        }
        fn access_level(&self) -> core::AccessLevel {
            *self.access_level.borrow()
        }
        fn set_access_level(&mut self, level: core::AccessLevel) {
            *self.access_level.borrow_mut() = level;
        }
        fn output_sender(&self) -> Option<tokio::sync::mpsc::UnboundedSender<Vec<u8>>> {
            None
        }
    }

    /// Build a test world with two rooms connected by exits.
    fn test_world() -> (World, core::Entity, core::Entity, core::Entity) {
        let mut world = World::new();

        let void_room = world.spawn((Room::new("The Void", "Empty void."), VoidRoom));

        let room_a = world.spawn((
            Room::new("Room A", "This is room A."),
            RoomExits(vec![Exit::new(Direction::East, void_room)]), // placeholder
        ));
        let room_b = world.spawn((
            Room::new("Room B", "This is room B."),
            RoomExits(vec![Exit::new(Direction::West, void_room)]), // placeholder
        ));

        // Fix exits with real dests
        let mut q_a = world.query_one::<&mut RoomExits>(room_a).unwrap();
        if let Some(exits) = q_a.get() {
            exits.0[0] = Exit::new(Direction::East, room_b);
        }
        drop(q_a);

        let mut q_b = world.query_one::<&mut RoomExits>(room_b).unwrap();
        if let Some(exits) = q_b.get() {
            exits.0[0] = Exit::new(Direction::West, room_a);
        }
        drop(q_b);

        (world, void_room, room_a, room_b)
    }

    fn test_player(
        world: &mut World,
        room: core::Entity,
    ) -> (core::Entity, MockConnection, ConnectionRegistry) {
        let player = world.spawn((Position::new(room), Name::new("TestPlayer")));
        let mut conn = MockConnection::new();
        conn.set_entity(player);
        let mut registry = ConnectionRegistry::new();
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        registry.register(player, tx);
        (player, conn, registry)
    }

    // ── cmd_look ────────────────────────────────────────────

    #[test]
    fn test_look_in_room() {
        let (mut world, _void, room_a, _room_b) = test_world();
        let (_player, mut conn, registry) = test_player(&mut world, room_a);

        // Spawn a mob in the room for mob listing test
        world.spawn((
            Position::new(room_a),
            Name::new("Test Mob"),
            Npc::new("test_mob"),
        ));

        cmd_look(&mut world, &mut conn, "", "", &registry);

        let lines = conn.take_lines();
        let all = lines.join("|");
        assert!(
            all.contains("Room A"),
            "Expected 'Room A' in lines: {lines:?}"
        );
        assert!(
            all.contains("Test Mob"),
            "Expected 'Test Mob' in mob listing: {lines:?}"
        );
        assert!(lines.len() > 1, "expected more than one line: {lines:?}");
    }

    #[test]
    fn test_look_in_void() {
        let (mut world, void_room, _room_a, _room_b) = test_world();
        let (_player, mut conn, registry) = test_player(&mut world, void_room);

        cmd_look(&mut world, &mut conn, "", "", &registry);

        let lines = conn.take_lines();
        assert!(lines.iter().any(|l| l.contains("The Void")));
        assert!(lines
            .iter()
            .any(|l| l.contains("endless, featureless void")));
    }

    #[test]
    fn test_look_no_entity() {
        let (mut world, _void, _room_a, _room_b) = test_world();
        let mut conn = MockConnection::new(); // no entity set
        let registry = ConnectionRegistry::new();

        cmd_look(&mut world, &mut conn, "", "", &registry);

        let lines = conn.take_lines();
        assert!(lines.iter().any(|l| l.contains("You have no form")));
    }

    #[test]
    fn test_look_no_position() {
        let (mut world, _void, _room_a, _room_b) = test_world();
        let player = world.spawn((Name::new("Ghost"),));
        let mut conn = MockConnection::new();
        conn.set_entity(player);
        let registry = ConnectionRegistry::new();

        cmd_look(&mut world, &mut conn, "", "", &registry);

        let lines = conn.take_lines();
        assert!(lines.iter().any(|l| l.contains("You are nowhere")));
    }

    // ── cmd_say ─────────────────────────────────────────────

    #[test]
    fn test_say_empty_args() {
        let (mut world, _void, room_a, _room_b) = test_world();
        let (_player, mut conn, registry) = test_player(&mut world, room_a);

        cmd_say(&mut world, &mut conn, "", "", &registry);

        let lines = conn.take_lines();
        assert!(lines.iter().any(|l| l.contains("Say what")));
    }

    #[test]
    fn test_say_void() {
        let (mut world, void_room, _room_a, _room_b) = test_world();
        let (_player, mut conn, registry) = test_player(&mut world, void_room);

        cmd_say(&mut world, &mut conn, "", "hello", &registry);

        let lines = conn.take_lines();
        assert!(lines.iter().any(|l| l.contains("echo in the void")));
    }

    #[test]
    fn test_say_broadcasts_to_room() {
        let (mut world, _void, room_a, _room_b) = test_world();
        let (_player, mut conn, registry) = test_player(&mut world, room_a);

        cmd_say(&mut world, &mut conn, "", "Hi there!", &registry);

        let lines = conn.take_lines();
        assert!(lines.iter().any(|l| l.contains("You say")));
        assert!(lines.iter().any(|l| l.contains("Hi there")));
    }

    #[test]
    fn test_say_echoes_to_other_occupants() {
        let (mut world, _void, room_a, _room_b) = test_world();

        let speaker = world.spawn((Position::new(room_a), Name::new("Speaker")));
        let mut conn_speaker = MockConnection::new();
        conn_speaker.set_entity(speaker);

        let listener = world.spawn((Position::new(room_a), Name::new("Listener")));
        let (tx_speaker, _rx_speaker) = tokio::sync::mpsc::unbounded_channel();
        let (tx_listener, mut rx_listener) = tokio::sync::mpsc::unbounded_channel();

        let mut registry = ConnectionRegistry::new();
        registry.register(speaker, tx_speaker);
        registry.register(listener, tx_listener);

        cmd_say(&mut world, &mut conn_speaker, "", "Hello room!", &registry);

        // Listener should have received the broadcast
        let received = rx_listener.try_recv().ok();
        assert!(received.is_some(), "listener should receive broadcast");
        if let Some(bytes) = received {
            let msg = String::from_utf8_lossy(&bytes);
            assert!(msg.contains("Speaker"));
            assert!(msg.contains("Hello room"));
        }
    }

    // ── Movement ────────────────────────────────────────────

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
        let (_player, mut conn, registry) = test_player(&mut world, room_a);

        cmd_move(&mut world, &mut conn, "north", "", &registry);

        let lines = conn.take_lines();
        assert!(lines.iter().any(|l| l.contains("cannot go that way")));
    }

    #[test]
    fn test_move_in_void() {
        let (mut world, void_room, _room_a, _room_b) = test_world();
        let (_player, mut conn, registry) = test_player(&mut world, void_room);

        cmd_move(&mut world, &mut conn, "east", "", &registry);

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
        assert!(lines.iter().any(|l| l.contains("have no form")));
    }

    #[test]
    fn test_move_closed_exit() {
        let (mut world, _void, room_a, _room_b) = test_world();
        let (player, mut conn, registry) = test_player(&mut world, room_a);

        // Close the east exit from room_a
        let mut q = world.query_one::<&mut RoomExits>(room_a).unwrap();
        if let Some(exits) = q.get() {
            exits.0[0].set_closed(true);
        }
        drop(q);

        cmd_move(&mut world, &mut conn, "e", "", &registry);

        let lines = conn.take_lines();
        assert!(lines.iter().any(|l| l.contains("closed")));
        // Position should not have changed
        let mut pos = world.query_one::<&Position>(player).unwrap();
        assert_eq!(pos.get().unwrap().room, room_a);
    }

    #[test]
    fn test_move_locked_exit() {
        let (mut world, _void, room_a, _room_b) = test_world();
        let (_player, mut conn, registry) = test_player(&mut world, room_a);

        // Lock the east exit from room_a
        let mut q = world.query_one::<&mut RoomExits>(room_a).unwrap();
        if let Some(exits) = q.get() {
            exits.0[0].set_locked(true);
        }
        drop(q);

        cmd_move(&mut world, &mut conn, "e", "", &registry);

        let lines = conn.take_lines();
        assert!(lines.iter().any(|l| l.contains("locked")));
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

        let mover = world.spawn((Position::new(room_a), Name::new("Mover")));
        let mut conn_mover = MockConnection::new();
        conn_mover.set_entity(mover);

        // Another player in room_a to receive leave broadcast
        let observer_a = world.spawn((Position::new(room_a), Name::new("ObserverA")));
        let (tx_obs_a, mut rx_obs_a) = tokio::sync::mpsc::unbounded_channel();

        // Another player in room_b to receive enter broadcast
        let observer_b = world.spawn((Position::new(room_b), Name::new("ObserverB")));
        let (tx_obs_b, mut rx_obs_b) = tokio::sync::mpsc::unbounded_channel();

        let mut registry = ConnectionRegistry::new();
        let (tx_mover, _rx_mover) = tokio::sync::mpsc::unbounded_channel();
        registry.register(mover, tx_mover);
        registry.register(observer_a, tx_obs_a);
        registry.register(observer_b, tx_obs_b);

        cmd_move(&mut world, &mut conn_mover, "east", "", &registry);

        // Observer A should see "Mover leaves east."
        let msg_a = rx_obs_a.try_recv().ok();
        assert!(msg_a.is_some(), "observer A should receive leave broadcast");
        if let Some(bytes) = msg_a {
            let text = String::from_utf8_lossy(&bytes);
            assert!(text.contains("Mover"));
            assert!(text.contains("leaves east"));
        }

        // Observer B should see "Mover arrives from the west."
        let msg_b = rx_obs_b.try_recv().ok();
        assert!(msg_b.is_some(), "observer B should receive enter broadcast");
        if let Some(bytes) = msg_b {
            let text = String::from_utf8_lossy(&bytes);
            assert!(text.contains("Mover"));
            assert!(text.contains("arrives"));
            assert!(text.contains("west"));
        }

        // Mover should see auto-look (room B info)
        let mover_lines = conn_mover.take_lines();
        assert!(mover_lines.iter().any(|l| l.contains("Room B")));
    }

    // ── cmd_help ────────────────────────────────────────────

    #[test]
    fn test_help_shows_commands() {
        let mut conn = MockConnection::new();
        let registry = ConnectionRegistry::new();
        let mut world = World::new();

        cmd_help(&mut world, &mut conn, "", "", &registry);

        // COMMANDS static is not initialized in unit tests (set by Server::run).
        // Verify the fallback path sends at least one line without panicking.
        let lines = conn.take_lines();
        assert!(!lines.is_empty());
    }

    // ── cmd_quit ────────────────────────────────────────────

    #[test]
    fn test_quit_says_goodbye_and_disconnects() {
        let mut conn = MockConnection::new();
        let registry = ConnectionRegistry::new();
        let mut world = World::new();

        cmd_quit(&mut world, &mut conn, "", "", &registry);

        let lines = conn.take_lines();
        assert!(lines.iter().any(|l| l.contains("Goodbye")));
        assert!(conn.was_disconnected());
    }

    // ── direction_from_name ─────────────────────────────────

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

    // ── cmd_move with direction_from_name ───────────────────

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

        cmd_move(&mut world, &mut conn, "sideways", "", &registry);

        let lines = conn.take_lines();
        assert!(lines.iter().any(|l| l.contains("Huh")));
    }

    // ── cmd_practice & cmd_train & cmd_score ─────────────────

    #[test]
    fn test_practice_success() {
        let (mut world, _void, room_a, _room_b) = test_world();
        let player = world.spawn((
            Position::new(room_a),
            Name::new("TestPlayer"),
            core::Level(1),
        ));
        let mut skills = core::LearnedSkills::new();
        skills.set_rank("swords", 1);
        world
            .insert(player, (skills, core::PracticePoints(1)))
            .unwrap();

        // Spawn trainer
        world.spawn((Position::new(room_a), core::Trainer::new(vec![])));

        let mut conn = MockConnection::new();
        conn.set_entity(player);
        let registry = ConnectionRegistry::new();

        cmd_practice(&mut world, &mut conn, "practice", "swords", &registry);

        let lines = conn.take_lines();
        let all = lines.join("|");
        assert!(
            all.contains("You practice 'swords' to rank 2"),
            "Expected success message in: {all}"
        );

        // Verify state
        let practice_pts = world
            .query_one::<&core::PracticePoints>(player)
            .unwrap()
            .get()
            .copied()
            .unwrap();
        assert_eq!(practice_pts.0, 0);

        let skills_comp = world
            .query_one::<&core::LearnedSkills>(player)
            .unwrap()
            .get()
            .cloned()
            .unwrap();
        assert_eq!(skills_comp.rank("swords"), 2);

        assert!(world.query_one::<&core::Dirty>(player).is_ok());
    }

    #[test]
    fn test_practice_no_trainer() {
        let (mut world, _void, room_a, _room_b) = test_world();
        let player = world.spawn((
            Position::new(room_a),
            Name::new("TestPlayer"),
            core::Level(1),
        ));
        let mut skills = core::LearnedSkills::new();
        skills.set_rank("swords", 1);
        world
            .insert(player, (skills, core::PracticePoints(1)))
            .unwrap();

        let mut conn = MockConnection::new();
        conn.set_entity(player);
        let registry = ConnectionRegistry::new();

        cmd_practice(&mut world, &mut conn, "practice", "swords", &registry);

        let lines = conn.take_lines();
        let all = lines.join("|");
        assert!(
            all.contains("Seek out a trainer"),
            "Expected trainer warning in: {all}"
        );
    }

    #[test]
    fn test_practice_wrong_trainer() {
        let (mut world, _void, room_a, _room_b) = test_world();
        let player = world.spawn((
            Position::new(room_a),
            Name::new("TestPlayer"),
            core::Level(1),
        ));
        let mut skills = core::LearnedSkills::new();
        skills.set_rank("swords", 1);
        world
            .insert(player, (skills, core::PracticePoints(1)))
            .unwrap();

        // Trainer who only teaches magic
        world.spawn((
            Position::new(room_a),
            core::Trainer::new(vec!["magic".to_string()]),
        ));

        let mut conn = MockConnection::new();
        conn.set_entity(player);
        let registry = ConnectionRegistry::new();

        cmd_practice(&mut world, &mut conn, "practice", "swords", &registry);

        let lines = conn.take_lines();
        let all = lines.join("|");
        assert!(
            all.contains("You can't practice that here"),
            "Expected error in: {all}"
        );
    }

    #[test]
    fn test_practice_not_known_skill() {
        let (mut world, _void, room_a, _room_b) = test_world();
        let player = world.spawn((
            Position::new(room_a),
            Name::new("TestPlayer"),
            core::Level(1),
        ));
        world
            .insert(
                player,
                (core::LearnedSkills::new(), core::PracticePoints(1)),
            )
            .unwrap();
        world.spawn((Position::new(room_a), core::Trainer::new(vec![])));

        let mut conn = MockConnection::new();
        conn.set_entity(player);
        let registry = ConnectionRegistry::new();

        cmd_practice(&mut world, &mut conn, "practice", "swords", &registry);

        let lines = conn.take_lines();
        let all = lines.join("|");
        assert!(
            all.contains("You don't know the skill 'swords'"),
            "Expected error in: {all}"
        );
    }

    #[test]
    fn test_practice_max_rank() {
        let (mut world, _void, room_a, _room_b) = test_world();
        let player = world.spawn((
            Position::new(room_a),
            Name::new("TestPlayer"),
            core::Level(1), // Max rank = 1 * 5 + 5 = 10
        ));
        let mut skills = core::LearnedSkills::new();
        skills.set_rank("swords", 10);
        world
            .insert(player, (skills, core::PracticePoints(1)))
            .unwrap();
        world.spawn((Position::new(room_a), core::Trainer::new(vec![])));

        let mut conn = MockConnection::new();
        conn.set_entity(player);
        let registry = ConnectionRegistry::new();

        cmd_practice(&mut world, &mut conn, "practice", "swords", &registry);

        let lines = conn.take_lines();
        let all = lines.join("|");
        assert!(
            all.contains("beyond rank 10"),
            "Expected max rank error in: {all}"
        );
    }

    #[test]
    fn test_practice_no_points() {
        let (mut world, _void, room_a, _room_b) = test_world();
        let player = world.spawn((
            Position::new(room_a),
            Name::new("TestPlayer"),
            core::Level(1),
        ));
        let mut skills = core::LearnedSkills::new();
        skills.set_rank("swords", 1);
        world
            .insert(player, (skills, core::PracticePoints(0)))
            .unwrap();
        world.spawn((Position::new(room_a), core::Trainer::new(vec![])));

        let mut conn = MockConnection::new();
        conn.set_entity(player);
        let registry = ConnectionRegistry::new();

        cmd_practice(&mut world, &mut conn, "practice", "swords", &registry);

        let lines = conn.take_lines();
        let all = lines.join("|");
        assert!(
            all.contains("only have 0"),
            "Expected no points error in: {all}"
        );
    }

    #[test]
    fn test_train_success() {
        let (mut world, _void, room_a, _room_b) = test_world();
        let player = world.spawn((
            Position::new(room_a),
            Name::new("TestPlayer"),
            core::Attributes::new(10, 10, 10, 10, 10, 10),
            core::PracticePoints(5),
        ));

        // Trainer who teaches attributes
        world.spawn((
            Position::new(room_a),
            core::Trainer::new(vec!["attributes".to_string()]),
        ));

        let mut conn = MockConnection::new();
        conn.set_entity(player);
        let registry = ConnectionRegistry::new();

        cmd_train(&mut world, &mut conn, "train", "strength", &registry);

        let lines = conn.take_lines();
        let all = lines.join("|");
        assert!(
            all.contains("You train Strength to 11"),
            "Expected success message in: {all}"
        );

        // Verify state
        let practice_pts = world
            .query_one::<&core::PracticePoints>(player)
            .unwrap()
            .get()
            .copied()
            .unwrap();
        assert_eq!(practice_pts.0, 0);

        let attrs = world
            .query_one::<&core::Attributes>(player)
            .unwrap()
            .get()
            .cloned()
            .unwrap();
        assert_eq!(attrs.strength, 11);

        assert!(world.query_one::<&core::Dirty>(player).is_ok());
    }

    #[test]
    fn test_train_no_trainer() {
        let (mut world, _void, room_a, _room_b) = test_world();
        let player = world.spawn((
            Position::new(room_a),
            Name::new("TestPlayer"),
            core::Attributes::new(10, 10, 10, 10, 10, 10),
            core::PracticePoints(5),
        ));

        let mut conn = MockConnection::new();
        conn.set_entity(player);
        let registry = ConnectionRegistry::new();

        cmd_train(&mut world, &mut conn, "train", "strength", &registry);

        let lines = conn.take_lines();
        let all = lines.join("|");
        assert!(
            all.contains("Seek out a trainer"),
            "Expected trainer warning in: {all}"
        );
    }

    #[test]
    fn test_train_wrong_trainer() {
        let (mut world, _void, room_a, _room_b) = test_world();
        let player = world.spawn((
            Position::new(room_a),
            Name::new("TestPlayer"),
            core::Attributes::new(10, 10, 10, 10, 10, 10),
            core::PracticePoints(5),
        ));

        // Trainer who only teaches combat
        world.spawn((
            Position::new(room_a),
            core::Trainer::new(vec!["combat".to_string()]),
        ));

        let mut conn = MockConnection::new();
        conn.set_entity(player);
        let registry = ConnectionRegistry::new();

        cmd_train(&mut world, &mut conn, "train", "strength", &registry);

        let lines = conn.take_lines();
        let all = lines.join("|");
        assert!(
            all.contains("You can't train that here"),
            "Expected error in: {all}"
        );
    }

    #[test]
    fn test_train_already_max() {
        let (mut world, _void, room_a, _room_b) = test_world();
        let player = world.spawn((
            Position::new(room_a),
            Name::new("TestPlayer"),
            core::Attributes::new(50, 10, 10, 10, 10, 10),
            core::PracticePoints(5),
        ));
        world.spawn((
            Position::new(room_a),
            core::Trainer::new(vec!["attributes".to_string()]),
        ));

        let mut conn = MockConnection::new();
        conn.set_entity(player);
        let registry = ConnectionRegistry::new();

        cmd_train(&mut world, &mut conn, "train", "strength", &registry);

        let lines = conn.take_lines();
        let all = lines.join("|");
        assert!(
            all.contains("maximum of 50"),
            "Expected bounds error in: {all}"
        );
    }

    #[test]
    fn test_train_no_points() {
        let (mut world, _void, room_a, _room_b) = test_world();
        let player = world.spawn((
            Position::new(room_a),
            Name::new("TestPlayer"),
            core::Attributes::new(10, 10, 10, 10, 10, 10),
            core::PracticePoints(4),
        ));
        world.spawn((
            Position::new(room_a),
            core::Trainer::new(vec!["attributes".to_string()]),
        ));

        let mut conn = MockConnection::new();
        conn.set_entity(player);
        let registry = ConnectionRegistry::new();

        cmd_train(&mut world, &mut conn, "train", "strength", &registry);

        let lines = conn.take_lines();
        let all = lines.join("|");
        assert!(
            all.contains("only have 4"),
            "Expected no points error in: {all}"
        );
    }

    #[test]
    fn test_score_displays_stats() {
        let (mut world, _void, room_a, _room_b) = test_world();
        let player = world.spawn((
            Position::new(room_a),
            Name::new("TestPlayer"),
            core::Level(5),
            core::Experience(1200),
            core::Attributes::new(18, 14, 12, 10, 15, 8),
            core::Health::new(45),
            core::PracticePoints(3),
            core::CombatStats {
                base_attack_bonus: 5,
                fort_save: 4,
                ref_save: 1,
                will_save: 2,
            },
            core::Mana::new(30),
            core::Stamina::new(100),
        ));

        let mut conn = MockConnection::new();
        conn.set_entity(player);
        let registry = ConnectionRegistry::new();

        cmd_score(&mut world, &mut conn, "score", "", &registry);

        let lines = conn.take_lines();
        let all = lines.join("|");
        assert!(all.contains("TestPlayer"));
        assert!(all.contains("Level:           5"));
        assert!(all.contains("Practice Points: 3"));
        assert!(all.contains("HP:              45 / 45"));
        assert!(all.contains("Mana:            30 / 30"));
        assert!(all.contains("Stamina:         100 / 100"));
        assert!(all.contains("BAB:             +5"));
        assert!(all.contains("Fort: +4, Ref: +1, Will: +2"));
        assert!(all.contains("Strength:     18"));
    }

    fn init_test_templates() {
        oxide_server::config::init(std::path::Path::new(""));
        let mut registry = core::templates::TemplateRegistry::new();

        let solaris = core::templates::DeityTemplate {
            id: "solaris".into(),
            name: "Solaris".into(),
            description: "The sun god.".into(),
            alignment: Some("lawful_good".into()),
            symbol: "Sunburst".into(),
            favored_weapon: None,
            tenets: vec![],
            domains: vec![],
            allowed_races: vec![],
            allowed_classes: vec![],
            allowed_alignments: vec![],
            prayer_effect: Some(core::templates::PrayerEffect {
                buff_id: "sun_blessing".into(),
                duration_secs: 60,
                cooldown_secs: 2,
                description: "Solar blessing".into(),
            }),
            params: std::collections::HashMap::new(),
        };
        registry.deities.insert("solaris".into(), solaris);

        // Two-handed weapon template
        let two_handed_tmpl = core::templates::ItemTemplate {
            id: "two_handed_sword".to_string(),
            name: "Greatsword".to_string(),
            description: "A heavy two-handed sword.".to_string(),
            item_type: "weapon".to_string(),
            subtype: "sword".to_string(),
            quality: "common".to_string(),
            level_requirement: 1,
            weight: 8.0,
            value: 100,
            flags: vec![],
            allowed_classes: vec![],
            allowed_races: vec![],
            allowed_alignments: vec![],
            requires_skill: None,
            weapon: Some(core::templates::WeaponDef {
                damage: core::templates::DiceString("2d6".to_string()),
                damage_type: "slash".to_string(),
                speed: 2.0,
                range: "melee".to_string(),
                hands: "TwoHand".to_string(),
            }),
            equipment: None,
            set: None,
            triggers: vec![],
            params: std::collections::HashMap::new(),
        };
        registry
            .items
            .insert("two_handed_sword".to_string(), two_handed_tmpl);

        // Shield template
        let shield_tmpl = core::templates::ItemTemplate {
            id: "wooden_shield".to_string(),
            name: "Wooden Shield".to_string(),
            description: "A simple wooden shield.".to_string(),
            item_type: "armor".to_string(),
            subtype: "shield".to_string(),
            quality: "common".to_string(),
            level_requirement: 1,
            weight: 5.0,
            value: 10,
            flags: vec![],
            allowed_classes: vec![],
            allowed_races: vec![],
            allowed_alignments: vec![],
            requires_skill: None,
            weapon: None,
            equipment: Some(core::templates::EquipmentDef {
                slot: "shield".to_string(),
            }),
            set: None,
            triggers: vec![],
            params: std::collections::HashMap::new(),
        };
        registry
            .items
            .insert("wooden_shield".to_string(), shield_tmpl);

        let world = World::new();
        let _server = oxide_server::Server::new("127.0.0.1:0", world).with_templates(registry);
    }

    #[test]
    fn test_pray_command() {
        init_test_templates();

        let (mut world, _void, room_a, _room_b) = test_world();
        let mut hp = core::Health::new(20);
        hp.current = 10;
        let mut stamina = core::Stamina::new(100);
        stamina.current = 20;
        let mut mana = core::Mana::new(30);
        mana.current = 30;

        let player = world.spawn((
            Position::new(room_a),
            Name::new("TestPlayer"),
            core::Deity(Some("solaris".to_string())),
            hp,
            stamina,
            mana,
            Vec::<core::ActiveEffect>::new(),
        ));

        let mut conn = MockConnection::new();
        conn.set_entity(player);
        let registry = ConnectionRegistry::new();

        cmd_pray(&mut world, &mut conn, "pray", "", &registry);

        let lines = conn.take_lines();
        let all = lines.join("|");
        assert!(all.contains("You bow your head and pray to Solaris"));
        assert!(all.contains("Solar blessing"));
        assert!(all.contains("Solaris' light"));

        let hp = world
            .query_one::<&core::Health>(player)
            .unwrap()
            .get()
            .cloned()
            .unwrap();
        assert_eq!(hp.current, 15);

        cmd_pray(&mut world, &mut conn, "pray", "", &registry);
        let lines2 = conn.take_lines();
        let all2 = lines2.join("|");
        assert!(all2.contains("answered too recently"));
    }

    #[test]
    fn test_toggle_resurrect() {
        let (mut world, _void, room_a, _room_b) = test_world();
        let player = world.spawn((
            Position::new(room_a),
            Name::new("TestPlayer"),
            core::Player {
                account_id: 1,
                prompt: None,
                screen_width: 80,
                no_resurrect: false,
            },
        ));
        let mut conn = MockConnection::new();
        conn.set_entity(player);
        let registry = ConnectionRegistry::new();

        cmd_toggle(&mut world, &mut conn, "toggle", "resurrect", &registry);
        let lines = conn.take_lines();
        let all = lines.join("|");
        assert!(all.contains("prevent unwanted resurrections"));

        let player_comp = world
            .query_one::<&core::Player>(player)
            .unwrap()
            .get()
            .cloned()
            .unwrap();
        assert!(player_comp.no_resurrect);

        cmd_toggle(&mut world, &mut conn, "toggle", "res", &registry);
        let lines2 = conn.take_lines();
        let all2 = lines2.join("|");
        assert!(all2.contains("allow resurrections"));

        let player_comp2 = world
            .query_one::<&core::Player>(player)
            .unwrap()
            .get()
            .cloned()
            .unwrap();
        assert!(!player_comp2.no_resurrect);
    }

    #[test]
    fn test_reclaim_corpse() {
        let (mut world, _void, room_a, _room_b) = test_world();
        let player = world.spawn((
            Position::new(room_a),
            Name::new("TestPlayer"),
            core::PlayerState::Dead,
            core::Health::new(20),
            core::Inventory::new(),
            core::Equipment::new(),
        ));

        let item_in_corpse = world.spawn((core::Item::new("sword"),));

        let corpse = world.spawn((
            core::Corpse {
                owner: Some(player),
                owner_db_id: None,
                created_at: std::time::Instant::now(),
                decay_secs: 1800,
                lootable_by: core::LootRule::OwnerOnly,
            },
            Position::new(room_a),
            core::Inventory(vec![item_in_corpse]),
            core::Equipment::new(),
        ));

        let mut conn = MockConnection::new();
        conn.set_entity(player);
        let registry = ConnectionRegistry::new();

        cmd_reclaim(&mut world, &mut conn, "reclaim", "", &registry);

        let lines = conn.take_lines();
        let all = lines.join("|");
        assert!(all.contains("You reclaim your body"));

        let state = world
            .query_one::<&core::PlayerState>(player)
            .unwrap()
            .get()
            .cloned()
            .unwrap();
        assert!(matches!(
            state,
            core::PlayerState::Resting(core::RestState::Standing)
        ));

        let hp = world
            .query_one::<&core::Health>(player)
            .unwrap()
            .get()
            .cloned()
            .unwrap();
        assert_eq!(hp.current, 20);

        let inv = world
            .query_one::<&core::Inventory>(player)
            .unwrap()
            .get()
            .cloned()
            .unwrap();
        assert!(inv.0.contains(&item_in_corpse));

        assert!(
            world.query_one::<&core::Corpse>(corpse).is_err()
                || world
                    .query_one::<&core::Corpse>(corpse)
                    .unwrap()
                    .get()
                    .is_none()
        );
    }

    #[test]
    fn test_die_command() {
        init_test_templates();
        let (mut world, _void, room_a, _room_b) = test_world();
        let player = world.spawn((
            Position::new(room_a),
            Name::new("TestPlayer"),
            core::PlayerState::Resting(core::RestState::Standing),
            core::Health::new(20),
            core::Inventory::new(),
            core::Equipment::new(),
            core::Player {
                account_id: 1,
                prompt: None,
                screen_width: 80,
                no_resurrect: false,
            },
            core::RecallRoom(room_a),
        ));

        let mut conn = MockConnection::new();
        conn.set_entity(player);
        let registry = ConnectionRegistry::new();

        // Try to die while conscious
        cmd_die(&mut world, &mut conn, "die", "", &registry);
        let lines = conn.take_lines();
        let all = lines.join("|");
        assert!(all.contains("You can only choose to die when you are unconscious."));

        let state = world
            .query_one::<&core::PlayerState>(player)
            .unwrap()
            .get()
            .cloned()
            .unwrap();
        assert!(matches!(state, core::PlayerState::Resting(..)));

        // Now set HP to 0 (unconscious) and call die
        if let Ok(mut q) = world.query_one::<&mut core::Health>(player) {
            if let Some(hp) = q.get() {
                hp.current = 0;
            }
        }

        cmd_die(&mut world, &mut conn, "die", "", &registry);
        let lines = conn.take_lines();
        let all = lines.join("|");
        assert!(all.contains("You choose to submit to death"));
        assert!(all.contains("Alas, you are dead! You are a ghost now..."));

        let state = world
            .query_one::<&core::PlayerState>(player)
            .unwrap()
            .get()
            .cloned()
            .unwrap();
        assert!(matches!(state, core::PlayerState::Dead));
    }

    #[test]
    fn test_revive_at_altar() {
        let mut world = World::new();
        let temple_room = world.spawn((
            core::Room::new("Temple of Altar", "A quiet temple."),
            core::RoomAllowRevive,
            RoomExits(vec![]),
        ));
        let player = world.spawn((
            Position::new(temple_room),
            Name::new("TestPlayer"),
            core::PlayerState::Dead,
            core::Health::new(20),
            core::Inventory::new(),
            core::Equipment::new(),
        ));

        let mut conn = MockConnection::new();
        conn.set_entity(player);
        let registry = ConnectionRegistry::new();

        cmd_revive(&mut world, &mut conn, "revive", "", &registry);

        let lines = conn.take_lines();
        let all = lines.join("|");
        assert!(all.contains("You pray at the altar and are restored to life"));

        let state = world
            .query_one::<&core::PlayerState>(player)
            .unwrap()
            .get()
            .cloned()
            .unwrap();
        assert!(matches!(
            state,
            core::PlayerState::Resting(core::RestState::Standing)
        ));

        let hp = world
            .query_one::<&core::Health>(player)
            .unwrap()
            .get()
            .cloned()
            .unwrap();
        assert_eq!(hp.current, 20);
    }

    #[test]
    fn test_door_interaction() {
        let (mut world, _void, room_a, room_b) = test_world();
        let player = world.spawn((
            Position::new(room_a),
            Name::new("TestPlayer"),
            core::Inventory::new(),
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

        let key_item = world.spawn((core::Item::new("gold_key"),));
        if let Ok(mut q) = world.query_one::<&mut core::Inventory>(player) {
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
            Name::new("Ghosty"),
            core::PlayerState::Dead,
        ));

        let listener_normal = world.spawn((Position::new(room_a), Name::new("NormalPlayer")));

        let listener_detect = world.spawn((
            Position::new(room_a),
            Name::new("DetectPlayer"),
            vec![core::ActiveEffect {
                source: "detect_undead".to_string(),
                stat: None,
                amount: None,
                aura_id: None,
                radius: None,
            }],
        ));

        let (tx1, mut rx_normal) = tokio::sync::mpsc::unbounded_channel();
        let (tx2, mut rx_detect) = tokio::sync::mpsc::unbounded_channel();

        let mut registry = ConnectionRegistry::new();
        registry.register(listener_normal, tx1);
        registry.register(listener_detect, tx2);

        send_leave_broadcast(&world, &registry, ghost, room_a, "east");

        let msg_normal = rx_normal
            .try_recv()
            .ok()
            .map(|b| String::from_utf8_lossy(&b).into_owned())
            .unwrap_or_default();
        assert!(msg_normal.contains("You feel a cold shiver run down your spine"));
        assert!(!msg_normal.contains("Ghosty"));

        let msg_detect = rx_detect
            .try_recv()
            .ok()
            .map(|b| String::from_utf8_lossy(&b).into_owned())
            .unwrap_or_default();
        assert!(msg_detect.contains("The ghost of Ghosty floats east"));

        send_enter_broadcast(&world, &registry, ghost, room_a, "west");

        let msg_normal2 = rx_normal
            .try_recv()
            .ok()
            .map(|b| String::from_utf8_lossy(&b).into_owned())
            .unwrap_or_default();
        assert!(msg_normal2.contains("You feel a sudden chill in the air"));
        assert!(!msg_normal2.contains("Ghosty"));

        let msg_detect2 = rx_detect
            .try_recv()
            .ok()
            .map(|b| String::from_utf8_lossy(&b).into_owned())
            .unwrap_or_default();
        assert!(msg_detect2.contains("The ghost of Ghosty floats in from the west"));
    }

    #[test]
    fn test_ghost_command_restrictions() {
        let (mut world, _void, room_a, _room_b) = test_world();
        let (player, mut conn, registry) = test_player(&mut world, room_a);

        // Turn player into a ghost
        world.insert(player, (core::PlayerState::Dead,)).unwrap();

        // 1. Test say formatting
        cmd_say(&mut world, &mut conn, "", "hello", &registry);
        let lines = conn.take_lines();
        assert!(
            lines
                .iter()
                .any(|l| l
                    .contains("You say, \"{cyan}h{brightblue}e{cyan}l{brightblue}l{cyan}o{/}\""))
        );

        // 2. Test get restriction
        // Spawn an item in the room floor
        let item = world.spawn((Name::new("sword"),));
        world
            .insert(room_a, (core::FloorItems(vec![item]),))
            .unwrap();

        cmd_get(&mut world, &mut conn, "", "sword", &registry);
        let lines = conn.take_lines();
        assert!(lines
            .iter()
            .any(|l| l.contains("You are a ghost! You cannot pick up items.")));

        // 3. Test wear restriction
        // Give player an item in inventory
        let item2 = world.spawn((Name::new("helmet"), core::Item::new("helmet")));
        world
            .insert(player, (core::Inventory(vec![item2]),))
            .unwrap();

        cmd_wear(&mut world, &mut conn, "", "helmet", &registry);
        let lines = conn.take_lines();
        assert!(lines
            .iter()
            .any(|l| l.contains("You are a ghost! You cannot wear items.")));

        // 4. Test wield restriction
        cmd_wield(&mut world, &mut conn, "", "helmet", &registry);
        let lines = conn.take_lines();
        assert!(lines
            .iter()
            .any(|l| l.contains("You are a ghost! You cannot wield items.")));

        // 5. Test remove restriction
        cmd_remove(&mut world, &mut conn, "", "weapon", &registry);
        let lines = conn.take_lines();
        assert!(lines
            .iter()
            .any(|l| l.contains("You are a ghost! You cannot equip or remove items.")));
    }

    #[test]
    fn test_move_in_combat_blocked() {
        let (mut world, _void, room_a, _room_b) = test_world();
        let (player, mut conn, registry) = test_player(&mut world, room_a);

        // Turn player combat state to Engaged
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

        // Attempt to move
        cmd_move(&mut world, &mut conn, "east", "", &registry);

        // Position should not change
        let mut pos = world.query_one::<&Position>(player).unwrap();
        let player_room = pos.get().unwrap().room;
        assert_eq!(player_room, room_a);

        // Should receive the message
        let lines = conn.take_lines();
        assert!(lines
            .iter()
            .any(|l| l.contains("No way! You are fighting for your life!")));
    }

    #[test]
    fn test_loot_rules_checks() {
        let (mut world, _void, room_a, _room_b) = test_world();
        let (player, mut conn, registry) = test_player(&mut world, room_a);

        // Add DbId and inventory to player
        world
            .insert(player, (core::DbId(42), core::Inventory::new()))
            .unwrap();

        // Spawn a sword in the corpse
        let item = world.spawn((core::Item::new("sword"),));

        // Corpse owned by someone else
        let _corpse_other = world.spawn((
            core::Corpse {
                owner: None,
                owner_db_id: Some(99),
                created_at: std::time::Instant::now(),
                decay_secs: 1800,
                lootable_by: core::LootRule::OwnerOnly,
            },
            Position::new(room_a),
            core::Name::new("corpse"),
            core::Inventory(vec![item]),
        ));

        // Attempt to loot
        cmd_loot(&mut world, &mut conn, "loot", "corpse", &registry);
        let lines = conn.take_lines();
        assert!(lines
            .iter()
            .any(|l| l.contains("This corpse does not belong to you.")));

        // Corpse owned by player's database ID
        let _corpse_mine = world.spawn((
            core::Corpse {
                owner: None,
                owner_db_id: Some(42),
                created_at: std::time::Instant::now(),
                decay_secs: 1800,
                lootable_by: core::LootRule::OwnerOnly,
            },
            Position::new(room_a),
            core::Name::new("my_corpse"),
            core::Inventory(vec![item]),
        ));

        // Attempt to loot own corpse
        cmd_loot(&mut world, &mut conn, "loot", "my_corpse", &registry);
        let lines = conn.take_lines();
        assert!(!lines
            .iter()
            .any(|l| l.contains("This corpse does not belong to you.")));
    }

    #[test]
    fn test_two_handed_slot_restrictions() {
        init_test_templates();

        let (mut world, _void, room_a, _room_b) = test_world();
        let (player, mut conn, conn_reg) = test_player(&mut world, room_a);

        // Spawn items
        let weapon_entity = world.spawn((
            core::Item::new("two_handed_sword"),
            core::Name::new("Greatsword"),
            core::Weapon {
                damage_dice: core::dice::DiceRoll::new(2, 6, 0),
                damage_type: core::DamageType::Slash,
                speed: 2.0,
                range: core::WeaponRange::Melee,
                hands: core::WeaponHands::TwoHand,
            },
        ));
        let shield_entity = world.spawn((
            core::Item::new("wooden_shield"),
            core::Name::new("Wooden Shield"),
            core::Armor { base: 2, bonus: 0 },
        ));

        // Insert inventory and equipment components on player
        world
            .insert(
                player,
                (
                    core::Inventory(vec![weapon_entity, shield_entity]),
                    core::Equipment::new(),
                ),
            )
            .unwrap();

        // 1. Wield the two-handed weapon
        cmd_wield(&mut world, &mut conn, "wield", "greatsword", &conn_reg);
        let lines = conn.take_lines();
        assert!(lines.iter().any(|l| l.contains("You wield it.")));

        // Verify two-handed weapon is equipped in Weapon slot
        let has_weapon = world
            .query_one::<&core::Equipment>(player)
            .ok()
            .and_then(|mut q| {
                q.get()
                    .map(|eq| eq.equipped(&core::EquipmentSlot::Weapon).copied())
            })
            .flatten();
        assert_eq!(has_weapon, Some(weapon_entity));

        // 2. Attempt to wear the shield -> should fail because wielding two-handed
        cmd_wear(&mut world, &mut conn, "wear", "wooden shield", &conn_reg);
        let lines = conn.take_lines();
        assert!(lines.iter().any(|l| l
            .contains("You are wielding a two-handed weapon and cannot use a shield/off-hand.")));

        // Verify shield is NOT equipped
        let has_shield = world
            .query_one::<&core::Equipment>(player)
            .ok()
            .and_then(|mut q| {
                q.get()
                    .map(|eq| eq.equipped(&core::EquipmentSlot::Shield).copied())
            })
            .flatten();
        assert!(has_shield.is_none());

        // 3. Remove the weapon, equip the shield first
        cmd_remove(&mut world, &mut conn, "remove", "weapon", &conn_reg);
        let _ = conn.take_lines();

        cmd_wear(&mut world, &mut conn, "wear", "wooden shield", &conn_reg);
        let lines = conn.take_lines();
        assert!(lines.iter().any(|l| l.contains("You wear it.")));

        // 4. Now wield the two-handed weapon -> should automatically unequip the shield!
        cmd_wield(&mut world, &mut conn, "wield", "greatsword", &conn_reg);
        let lines = conn.take_lines();
        assert!(lines
            .iter()
            .any(|l| l.contains("You unequip Wooden Shield to wield the two-handed weapon.")));

        // Verify shield is unequipped and back in inventory, and weapon is wielded
        let has_shield = world
            .query_one::<&core::Equipment>(player)
            .ok()
            .and_then(|mut q| {
                q.get()
                    .map(|eq| eq.equipped(&core::EquipmentSlot::Shield).copied())
            })
            .flatten();
        assert!(has_shield.is_none());

        let has_weapon = world
            .query_one::<&core::Equipment>(player)
            .ok()
            .and_then(|mut q| {
                q.get()
                    .map(|eq| eq.equipped(&core::EquipmentSlot::Weapon).copied())
            })
            .flatten();
        assert_eq!(has_weapon, Some(weapon_entity));
    }

    #[test]
    fn test_help_categories_and_filtering() {
        let mut dispatch = oxide_server::CommandDispatch::new();
        dispatch.register(oxide_server::Command {
            name: "help",
            aliases: &[],
            access: core::AccessLevel::Player,
            category: "General",
            help_text: "Help command description",
            handler: |w, c, n, a, r| cmd_help(w, c, n, a, r),
        });
        dispatch.register(oxide_server::Command {
            name: "look",
            aliases: &[],
            access: core::AccessLevel::Player,
            category: "General",
            help_text: "look description",
            handler: |_, _, _, _, _| {},
        });
        dispatch.register(oxide_server::Command {
            name: "goto",
            aliases: &[],
            access: core::AccessLevel::Immortal,
            category: "Immortal",
            help_text: "goto description",
            handler: |_, _, _, _, _| {},
        });
        dispatch.register(oxide_server::Command {
            name: "@dig",
            aliases: &[],
            access: core::AccessLevel::Builder,
            category: "Builder",
            help_text: "dig description",
            handler: |_, _, _, _, _| {},
        });
        let _ = oxide_server::set_commands(dispatch);

        let mut world = World::new();
        let mut conn = MockConnection::new();
        let conn_reg = ConnectionRegistry::new();

        // 1. Player access (only General, Combat, etc. categories, no Builder/Immortal)
        conn.set_access_level(core::AccessLevel::Player);
        cmd_help(&mut world, &mut conn, "help", "", &conn_reg);
        let lines = conn.take_lines();

        // Assert categories are printed
        assert!(lines
            .iter()
            .any(|l| l.contains("Available Help Categories")));
        assert!(lines.iter().any(|l| l.contains("General")));
        // Assert staff categories are hidden
        assert!(!lines.iter().any(|l| l.contains("Builder")));
        assert!(!lines.iter().any(|l| l.contains("Immortal")));

        // 2. Help query for blocked command should return "No help found"
        cmd_help(&mut world, &mut conn, "help", "goto", &conn_reg);
        let lines = conn.take_lines();
        assert!(lines
            .iter()
            .any(|l| l.contains("No help found for 'goto'.")));

        // 3. Builder access (Builder category visible, Immortal hidden)
        conn.set_access_level(core::AccessLevel::Builder);
        cmd_help(&mut world, &mut conn, "help", "", &conn_reg);
        let lines = conn.take_lines();
        assert!(lines.iter().any(|l| l.contains("Builder")));
        assert!(!lines.iter().any(|l| l.contains("Immortal")));

        // 4. Help query for builder command is successful
        cmd_help(&mut world, &mut conn, "help", "@dig", &conn_reg);
        let lines = conn.take_lines();
        assert!(lines.iter().any(|l| l.contains("@dig")));

        // 5. Immortal access (Immortal category visible)
        conn.set_access_level(core::AccessLevel::Immortal);
        cmd_help(&mut world, &mut conn, "help", "", &conn_reg);
        let lines = conn.take_lines();
        assert!(lines.iter().any(|l| l.contains("Builder")));
        assert!(lines.iter().any(|l| l.contains("Immortal")));
    }

    #[test]
    fn test_olc_commands_integration() {
        let mut world = World::new();
        let mut conn = MockConnection::new();
        conn.set_access_level(core::AccessLevel::Builder);
        let conn_reg = ConnectionRegistry::new();

        let registry = core::templates::TemplateRegistry::new();
        oxide_server::init_templates_for_test(registry);

        cmd_area(
            &mut world,
            &mut conn,
            "area",
            "create test_area Test Area",
            &conn_reg,
        );
        let lines = conn.take_lines();
        assert!(lines
            .iter()
            .any(|l| l.contains("Area 'test_area' created in-memory.")));

        let templates = oxide_server::get_templates().unwrap();
        assert!(templates.areas.contains_key("test_area"));
        drop(templates);

        let start_room_key = "test_area:start".to_string();
        let current_room = world.spawn((
            Room::new("Start Room", "Starting point."),
            core::RoomFlags::default(),
            core::RoomKey(start_room_key.clone()),
            core::ScriptParams::default(),
            core::RoomTags::new(Vec::new()),
            core::RoomExits(Vec::new()),
        ));
        let player = world.spawn((core::Position::new(current_room),));
        conn.set_entity(player);

        let _ = oxide_server::update_templates(|reg| {
            if let Some(area) = reg.areas.get_mut("test_area") {
                area.spawns.push(core::templates::SpawnEntry {
                    room: "start".to_string(),
                    label: "Starting Point".to_string(),
                    description: "Test start room.".to_string(),
                    allowed_races: Vec::new(),
                    allowed_classes: Vec::new(),
                    allowed_alignments: Vec::new(),
                });
                area.rooms.insert(
                    "start".to_string(),
                    core::templates::RoomTemplate {
                        id: "start".to_string(),
                        area: "test_area".to_string(),
                        name: "Start Room".to_string(),
                        description: "Starting point.".to_string(),
                        exits: std::collections::HashMap::new(),
                        portals: Vec::new(),
                        flags: Vec::new(),
                        content: core::templates::RoomContent {
                            mobs: Vec::new(),
                            items: Vec::new(),
                        },
                        allow_revive: false,
                        script: None,
                        params: std::collections::HashMap::new(),
                    },
                );
            }
        });

        cmd_dig(
            &mut world,
            &mut conn,
            "dig",
            "east chamber The Chamber",
            &conn_reg,
        );
        let lines = conn.take_lines();
        assert!(lines
            .iter()
            .any(|l| l.contains("You dig east and create room 'The Chamber'")));

        let templates = oxide_server::get_templates().unwrap();
        let area = templates.areas.get("test_area").unwrap();
        assert!(area.rooms.contains_key("chamber"));
        let start_room_tpl = area.rooms.get("start").unwrap();
        assert!(start_room_tpl.exits.contains_key("east"));
        let dug_room_tpl = area.rooms.get("chamber").unwrap();
        assert!(dug_room_tpl.exits.contains_key("west"));
        drop(templates);

        let dug_room_entity = world
            .query::<&core::RoomKey>()
            .iter()
            .find(|(_, key)| key.0 == "test_area:chamber")
            .map(|(r, _)| r)
            .unwrap();
        let player_ent = conn.entity().unwrap();
        let mut q_pos = world.query_one::<&mut core::Position>(player_ent).unwrap();
        q_pos.get().unwrap().room = core::Entity::from(dug_room_entity);
        drop(q_pos);

        cmd_set(
            &mut world,
            &mut conn,
            "set",
            "room name The Grand Chamber",
            &conn_reg,
        );
        let lines = conn.take_lines();
        assert!(lines
            .iter()
            .any(|l| l.contains("Set room name to 'The Grand Chamber'")));

        cmd_desc(
            &mut world,
            &mut conn,
            "desc",
            "A very large grand chamber.",
            &conn_reg,
        );
        let lines = conn.take_lines();
        assert!(lines
            .iter()
            .any(|l| l.contains("Room description updated.")));

        let templates = oxide_server::get_templates().unwrap();
        let area = templates.areas.get("test_area").unwrap();
        let chamber_tpl = area.rooms.get("chamber").unwrap();
        assert_eq!(chamber_tpl.name, "The Grand Chamber");
        assert_eq!(chamber_tpl.description, "A very large grand chamber.");
        drop(templates);

        cmd_validate(&mut world, &mut conn, "validate", "", &conn_reg);
        let lines = conn.take_lines();
        assert!(lines
            .iter()
            .any(|l| l.contains("Validation complete: 0 errors found.")));

        cmd_area(&mut world, &mut conn, "area", "delete test_area", &conn_reg);
        let lines = conn.take_lines();
        assert!(lines
            .iter()
            .any(|l| l.contains("Area 'test_area' deleted.")));

        let templates = oxide_server::get_templates().unwrap();
        assert!(!templates.areas.contains_key("test_area"));
    }
}
