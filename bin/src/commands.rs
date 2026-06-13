use std::str::FromStr;

use mud_core as core;
use mud_core::{Direction, Name, Position, Room, RoomExits, VoidRoom, World};
use mud_server::{Connection, ConnectionFlag, ConnectionRegistry};

fn send_formatted(conn: &mut dyn Connection, text: &core::format::RichText) {
    let ansi = conn.flags().has(ConnectionFlag::Ansi);
    let blink = conn.flags().has(ConnectionFlag::Blink);
    conn.send_line(&text.render(ansi, blink));
}

fn section_label(text: &str) -> core::format::Segment {
    core::format::Segment::colored(text, core::format::Color::BrightBlack)
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

pub fn cmd_look(
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

    let room = match get_pos_room(world, entity) {
        Some(r) => r,
        None => {
            conn.send_line("You are nowhere.");
            return;
        }
    };

    if is_void_room(world, room) {
        conn.send_line("");
        send_formatted(conn, &core::format::conventions::room_name("The Void"));
        send_formatted(conn, &core::format::conventions::separator("-".repeat(9)));
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
    send_formatted(
        conn,
        &core::format::conventions::separator("-".repeat(room_name.len().min(40))),
    );
    conn.send_line(&room_desc);

    // Exits
    let exits = get_exits(world, room);
    if !exits.is_empty() {
        send_formatted(
            conn,
            &core::format::conventions::exit_dir(format!("[Exits: {}]", exits.join(" "))),
        );
    }

    // Occupants
    let others: Vec<_> = registry
        .occupants(world, room)
        .into_iter()
        .filter(|&e| e != entity)
        .collect();

    if !others.is_empty() {
        let mut t = core::format::RichText::new();
        t.push(section_label("Players here: "));
        for (i, &other) in others.iter().enumerate() {
            if i > 0 {
                t.push(core::format::Segment::new(", "));
            }
            if let Some(name) = get_name(world, other) {
                t.push(core::format::conventions::player_name_segment(
                    name.as_str(),
                ));
            }
        }
        send_formatted(conn, &t);
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
    let _ = world.insert(entity, (Position::new(dest),));

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
    _args: &str,
    _registry: &ConnectionRegistry,
) {
    conn.send_line("");
    conn.send_line("Available commands:");
    conn.send_line("  look/l         — examine your surroundings");
    conn.send_line("  say <text>     — speak in the room");
    conn.send_line("  score          — display your character stats");
    conn.send_line("  motd           — show message of the day");
    conn.send_line("  north/n        — move north");
    conn.send_line("  south/s        — move south");
    conn.send_line("  east/e         — move east");
    conn.send_line("  west/w         — move west");
    conn.send_line("  up/u           — move up");
    conn.send_line("  down/d         — move down");
    conn.send_line("  help           — this help");
    conn.send_line("  quit           — disconnect");
    conn.send_line("  @award <xp>    — grant XP (builder)");
    conn.send_line("");
}

pub fn cmd_quit(
    _world: &mut World,
    conn: &mut dyn Connection,
    _name: &str,
    _args: &str,
    _registry: &ConnectionRegistry,
) {
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

    // Find target in the same room by name
    let target_name = args.trim().to_lowercase();
    let target = {
        let mut q = world.query::<(&core::Name, &core::Position, &core::Health)>();
        q.iter()
            .map(|(raw, (name, pos, _))| {
                (core::Entity::from(raw), name.as_str().to_lowercase(), pos)
            })
            .find(|(e, n, pos)| pos.room == room && n == &target_name && *e != entity)
            .map(|(e, _, _)| e)
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
    if world.query_one::<&core::Player>(target).is_ok() {
        conn.send_line("You cannot attack other players yet.");
        return;
    }

    let _ = world.insert(entity, (core::CombatTarget(target),));
    conn.send_line("You attack!");
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

    // Try by number first
    if let Ok(idx) = query.parse::<usize>() {
        if idx > 0 && idx <= inv.len() {
            return Some(inv[idx - 1]);
        }
    }

    // Try by name
    let query_lower = query.to_lowercase();
    for item in &inv {
        let name = world
            .query_one::<&core::Name>(*item)
            .ok()
            .and_then(|mut q| q.get().map(|n| n.as_str().to_lowercase()))
            .unwrap_or_default();
        if name == query_lower {
            return Some(*item);
        }
    }

    None
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
    let has_armor = world.query_one::<&core::Armor>(item).is_ok();
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
    let has_weapon = world.query_one::<&core::Weapon>(item).is_ok();
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
        let query = item_name.to_lowercase();
        if let Ok(idx) = query.parse::<usize>() {
            if idx > 0 && idx <= floor.0.len() {
                return Some(floor.0[idx - 1]);
            }
        }
        for item in &floor.0 {
            let name = world
                .query_one::<&core::Name>(*item)
                .ok()
                .and_then(|mut q| q.get().map(|n| n.as_str().to_lowercase()))
                .unwrap_or_default();
            if name == query {
                return Some(*item);
            }
        }
        None
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
                let query = item_name.to_lowercase();
                let idx = if let Ok(n) = query.parse::<usize>() {
                    if n > 0 && n <= floor.0.len() {
                        Some(n - 1)
                    } else {
                        None
                    }
                } else {
                    floor.0.iter().position(|e| {
                        world
                            .query_one::<&core::Name>(*e)
                            .ok()
                            .and_then(|mut q| q.get().map(|n| n.as_str().to_lowercase() == query))
                            .unwrap_or(false)
                    })
                };

                idx.map(|i| floor.0.remove(i))
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
        q.iter()
            .map(|(raw, (_, pos, name))| (core::Entity::from(raw), name.clone(), pos))
            .find(|(_, _, pos)| pos.room == room)
            .and_then(|(e, name, _)| {
                let name_lower = corpse_name.to_lowercase();
                if name.as_str().to_lowercase() == name_lower {
                    Some(e)
                } else {
                    None
                }
            })
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
// Train command (placeholder)
// ---------------------------------------------------------------------------

pub fn cmd_train(
    _world: &mut World,
    conn: &mut dyn Connection,
    _name: &str,
    _args: &str,
    _registry: &ConnectionRegistry,
) {
    conn.send_line("Training is not yet implemented.");
}

#[cfg(test)]
mod tests {
    use super::*;
    use mud_core as core;
    use mud_core::Exit;
    use mud_server::{CharacterCreateBuffer, ConnectionFlags, ConnectionState};
    use std::cell::RefCell;
    use std::collections::VecDeque;

    /// A mock connection that records all send_line calls.
    struct MockConnection {
        lines: RefCell<VecDeque<String>>,
        entity: RefCell<Option<core::Entity>>,
        disconnected: RefCell<bool>,
        flags: RefCell<ConnectionFlags>,
    }

    impl MockConnection {
        fn new() -> Self {
            MockConnection {
                lines: RefCell::new(VecDeque::new()),
                entity: RefCell::new(None),
                disconnected: RefCell::new(false),
                flags: RefCell::new(ConnectionFlags::new()),
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
        fn flags(&self) -> ConnectionFlags {
            *self.flags.borrow()
        }
        fn set_flags(&mut self, flags: ConnectionFlags) {
            self.flags.borrow_mut().clone_from(&flags);
        }
        fn state(&self) -> ConnectionState {
            ConnectionState::Playing
        }
        fn set_state(&mut self, _state: ConnectionState) {}
        fn create_buffer(&mut self) -> &mut CharacterCreateBuffer {
            todo!()
        }
        fn account_id(&self) -> Option<i64> {
            None
        }
        fn set_account_id(&mut self, _id: i64) {}
        fn strikes(&self) -> u8 {
            0
        }
        fn set_strikes(&mut self, _n: u8) {}
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

        cmd_look(&mut world, &mut conn, "", "", &registry);

        let lines = conn.take_lines();
        let all = lines.join("|");
        assert!(
            all.contains("Room A"),
            "Expected 'Room A' in lines: {lines:?}"
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

        let lines = conn.take_lines();
        assert!(lines.iter().any(|l| l.contains("Available commands")));
        assert!(lines.iter().any(|l| l.contains("look")));
        assert!(lines.iter().any(|l| l.contains("say")));
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
