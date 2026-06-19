use std::collections::HashMap;
use std::str::FromStr;

use mud_core as core;
use mud_core::format::preview::{item_look_template, mob_look_template};
use mud_core::templates::{SetDef, SkillResolveError};
use mud_core::{
    Description, Direction, FloorItems, Friendly, Inventory, Item, Name, Npc, Position, Room,
    RoomExits, ShortDesc, VoidRoom, World,
};
use mud_server::{Connection, ConnectionFlag, ConnectionRegistry};

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
            if let Some(templates) = mud_server::get_templates() {
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
            if let Some(templates) = mud_server::get_templates() {
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

    // Speaker message
    let speaker_msg = core::format::conventions::say_text(format!("You say, \"{args}\""));
    send_formatted(conn, &speaker_msg);

    // Room broadcast
    let mut room_msg = core::format::RichText::new();
    room_msg.push(core::format::conventions::player_name_segment(
        name.as_str(),
    ));
    room_msg.push(core::format::Segment::new(format!(" says, \"{args}\"")));

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
}

fn send_leave_broadcast(
    world: &World,
    registry: &ConnectionRegistry,
    entity: core::Entity,
    from_room: core::Entity,
    dir_long: &str,
) {
    let name = get_name(world, entity).unwrap_or(Name::new("Someone"));
    let mut msg = core::format::RichText::new();
    msg.push(core::format::conventions::player_name_segment(
        name.as_str(),
    ));
    msg.push(core::format::Segment::new(format!(" leaves {dir_long}.")));
    let rendered = msg.render(true, true);
    let bytes = format!("{}\r\n", rendered).into_bytes();
    for &other in &registry.occupants(world, from_room) {
        if other == entity {
            continue;
        }
        if let Some(tx) = registry.sender(other) {
            let _ = tx.send(bytes.clone());
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
    let mut msg = core::format::RichText::new();
    msg.push(core::format::conventions::player_name_segment(
        name.as_str(),
    ));
    msg.push(core::format::Segment::new(format!(
        " arrives from the {dir_long}."
    )));
    let rendered = msg.render(true, true);
    let bytes = format!("{}\r\n", rendered).into_bytes();
    for &other in &registry.occupants(world, dest_room) {
        if other == entity {
            continue;
        }
        if let Some(tx) = registry.sender(other) {
            let _ = tx.send(bytes.clone());
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

    // Broadcast leave
    let dir_long = direction.long_name();
    let opposite = direction.opposite();
    let opp_long = opposite.long_name();
    send_leave_broadcast(world, registry, entity, room, dir_long);

    // Broadcast enter
    send_enter_broadcast(world, registry, entity, dest, opp_long);

    // Auto-look
    cmd_look(world, conn, "", "", registry);
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

pub fn cmd_help(
    _world: &mut World,
    conn: &mut dyn Connection,
    _name: &str,
    args: &str,
    _registry: &ConnectionRegistry,
) {
    let dispatch = match mud_server::get_commands() {
        Some(d) => d,
        None => {
            conn.send_line("Help is unavailable.");
            return;
        }
    };

    let query = args.trim();

    if !query.is_empty() {
        match dispatch.find(query) {
            Some(cmd) => {
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
            }
            None => {
                conn.send_line(&format!("No help found for '{query}'."));
            }
        }
        return;
    }

    let groups = dispatch.help_groups();
    conn.send_line("");
    conn.send_line("Available commands  (type 'help <command>' for details)");
    for (category, cmds) in groups {
        conn.send_line("");
        conn.send_line(&format!("  {category}:"));
        for cmd in cmds {
            let name_col = if cmd.aliases.is_empty() {
                cmd.name.to_string()
            } else {
                format!("{} ({})", cmd.name, cmd.aliases.join(", "))
            };
            conn.send_line(&format!("    {name_col}"));
        }
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

    let xp_to_next = xp.to_next_level(level.0);

    conn.send_line("");
    conn.send_line("--- Character Score ---");
    conn.send_line(&format!("  Name:       {name}"));
    conn.send_line(&format!("  Level:      {}", level.0));
    conn.send_line(&format!(
        "  Experience: {} / {} ({} to next level)",
        xp.0,
        core::Experience::for_level(level.0 + 1),
        xp_to_next
    ));
    conn.send_line(&format!("  HP:         {} / {}", hp.current, hp.max));
    conn.send_line("");
    conn.send_line("  Attributes:");
    conn.send_line(&format!("    Strength:     {}", attrs.strength));
    conn.send_line(&format!("    Dexterity:    {}", attrs.dexterity));
    conn.send_line(&format!("    Intelligence: {}", attrs.intelligence));
    conn.send_line(&format!("    Wisdom:      {}", attrs.wisdom));
    conn.send_line(&format!("    Constitution: {}", attrs.constitution));
    conn.send_line(&format!("    Charisma:    {}", attrs.charisma));
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
    conn.send_line(mud_server::get_motd());
    conn.send_line("");
}

pub fn cmd_who(
    world: &mut World,
    conn: &mut dyn Connection,
    _name: &str,
    _args: &str,
    registry: &ConnectionRegistry,
) {
    let lines = mud_server::login::list_who(world, registry);
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
    mud_server::award_xp(world, entity);

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
fn evaluate_equipment_sets(world: &mut World, entity: core::Entity) {
    if let Some(templates) = mud_server::get_templates() {
        let set_defs: HashMap<String, SetDef> = templates.sets.clone();
        core::systems::set_bonus::evaluate_set_bonuses(world, entity, &set_defs);
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

    // Check if it's armor (has Armor component) or general wearable
    let has_armor = world
        .query_one::<&core::Armor>(item)
        .is_ok_and(|mut q| q.get().is_some());
    let slot = if has_armor {
        // Determine slot from item's template or name
        // Default to Torso for armor items
        core::EquipmentSlot::Torso
    } else {
        conn.send_line("You can't wear that.");
        return;
    };

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

    evaluate_equipment_sets(world, entity);
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
            eq.equip(core::EquipmentSlot::Weapon, item);
            conn.send_line("You wield it.");
        }
    }

    evaluate_equipment_sets(world, entity);
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

    evaluate_equipment_sets(world, entity);
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

    conn.send_line(&format!("--- {name} ---"));

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
// Train command
// ---------------------------------------------------------------------------

fn skill_point_cost(current_rank: u16) -> u32 {
    // Cost = 1 + rank / 10 (so rank 0=1, rank 10=2, rank 50=6, rank 100=11)
    1 + (current_rank / 10) as u32
}

fn max_rank_for_level(level: u8) -> u16 {
    // Max any skill rank = level * 5 + 5 (level 1 = 10, level 10 = 55, level 50 = 255)
    (level as u16 * 5) + 5
}

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

    let level = world
        .query_one::<&core::Level>(entity)
        .ok()
        .and_then(|mut q| q.get().copied())
        .unwrap_or(core::Level(1));

    let args = args.trim();

    // `train` with no args — show status
    if args.is_empty() {
        let skills = world
            .query_one::<&core::LearnedSkills>(entity)
            .ok()
            .and_then(|mut q| q.get().cloned())
            .unwrap_or_default();

        conn.send_line("");
        conn.send_line("--- Training ---");
        conn.send_line(&format!("Unspent skill points: {}", skills.unspent_points));
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
                let cost = skill_point_cost(**rank);
                conn.send_line(&format!(
                    "  {skill_id}: rank {rank} ({} point(s) to train)",
                    cost
                ));
            }
        }

        conn.send_line("");
        return;
    }

    // `train list` — show all available skills from templates
    if args == "list" {
        conn.send_line("");
        conn.send_line("Available skills:");
        conn.send_line("  Skills are granted through race/class selection.");
        conn.send_line("  Use 'train <skill>' to increase a known skill's rank.");
        conn.send_line("");
        return;
    }

    // `train <skill>` — train a specific skill
    let skill_id = match resolve_skill_name_for_training(args, world, entity) {
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

    // Check if skill is known
    let current_rank = skills.rank(&skill_id);
    if current_rank == 0 {
        conn.send_line(&format!("You don't know the skill '{skill_id}'."));
        return;
    }

    // Check skill cap by level
    let max_rank = max_rank_for_level(level.0);
    if current_rank >= max_rank {
        conn.send_line(&format!(
            "You cannot train '{skill_id}' beyond rank {max_rank} at your level."
        ));
        return;
    }

    // Check cost
    let cost = skill_point_cost(current_rank);
    if skills.unspent_points < cost {
        conn.send_line(&format!(
            "Training '{skill_id}' costs {cost} point(s), but you only have {}.",
            skills.unspent_points
        ));
        return;
    }

    // Apply training
    skills.unspent_points -= cost;
    let new_rank = current_rank + 1;
    skills.set_rank(&skill_id, new_rank);
    let remaining = skills.unspent_points;

    let _ = world.insert(entity, (skills, core::Dirty));
    conn.send_line(&format!(
        "You train '{skill_id}' to rank {new_rank}. ({remaining} point(s) remaining)",
    ));
}

/// Resolve a skill name (exact or partial) for the `train` command.
/// Falls back to exact match when the template registry is unavailable.
fn resolve_skill_name_for_training(
    input: &str,
    world: &World,
    entity: core::Entity,
) -> Result<String, String> {
    let templates = match mud_server::get_templates() {
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

#[cfg(test)]
mod tests {
    use super::*;
    use mud_core as core;
    use mud_core::Exit;
    use mud_server::ConnectionFlags;
    use std::cell::RefCell;
    use std::collections::VecDeque;

    /// A mock connection that records all send_line calls.
    struct MockConnection {
        lines: RefCell<VecDeque<String>>,
        entity: RefCell<Option<core::Entity>>,
        disconnected: RefCell<bool>,
        flags: RefCell<ConnectionFlags>,
        screen_width: RefCell<u16>,
    }

    impl MockConnection {
        fn new() -> Self {
            MockConnection {
                lines: RefCell::new(VecDeque::new()),
                entity: RefCell::new(None),
                disconnected: RefCell::new(false),
                flags: RefCell::new(ConnectionFlags::new()),
                screen_width: RefCell::new(0),
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
}
