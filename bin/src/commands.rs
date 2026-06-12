use mud_core as core;
use mud_core::{Name, Position, Room, RoomExits, VoidRoom, World};
use mud_server::{Connection, ConnectionFlag, ConnectionRegistry};

fn send_formatted(conn: &mut dyn Connection, text: &core::format::Text) {
    if conn.flags().has(ConnectionFlag::Ansi) {
        conn.send_line(&core::format::render(text));
    } else {
        conn.send_line(&core::format::render_plain(text));
    }
}

fn section_label(text: &str) -> core::format::StyledText {
    core::format::StyledText::colored(text, core::format::Color::BrightBlack)
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
    world.query_one::<&VoidRoom>(room).is_ok()
}

fn get_exits(world: &World, room: core::Entity) -> Vec<core::format::StyledText> {
    let mut exits = Vec::new();
    if let Ok(mut q) = world.query_one::<&RoomExits>(room) {
        if let Some(room_exits) = q.get() {
            for exit in &room_exits.0 {
                if !exit.is_hidden() {
                    exits.push(core::format::StyledText::colored(
                        exit.direction.short_name(),
                        core::format::Color::Cyan,
                    ));
                }
            }
        }
    }
    exits
}

pub fn cmd_look(
    world: &mut World,
    conn: &mut dyn Connection,
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
        send_formatted(conn, &core::format::conventions::separator(&"-".repeat(9)));
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
    send_formatted(conn, &core::format::conventions::separator(&"-".repeat(
        room_name.len().min(40),
    )));
    conn.send_line(&room_desc);

    // Exits
    let exits = get_exits(world, room);
    if !exits.is_empty() {
        let mut t = core::format::Text::new();
        t.push(section_label("[Exits: "));
        for (i, exit) in exits.iter().enumerate() {
            if i > 0 {
                t.push(core::format::StyledText::new(" "));
            }
            t.push(exit.clone());
        }
        t.push(section_label("]"));
        send_formatted(conn, &t);
    }

    // Occupants
    let others: Vec<_> = registry
        .occupants(world, room)
        .into_iter()
        .filter(|&e| e != entity)
        .collect();

    if !others.is_empty() {
        let mut t = core::format::Text::new();
        t.push(section_label("Players here: "));
        for (i, &other) in others.iter().enumerate() {
            if i > 0 {
                t.push(core::format::StyledText::new(", "));
            }
            if let Some(name) = get_name(world, other) {
                t.push(core::format::StyledText::colored(
                    name.as_str(),
                    core::format::Color::Green,
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
    let mut speaker_msg = core::format::Text::new();
    speaker_msg.push(core::format::StyledText::new("You say, \""));
    speaker_msg.push(core::format::StyledText::styled(
        args.to_string(),
        core::format::Color::Default,
        core::format::Color::Default,
        {
            let mut m = core::format::Modifier::new();
            m.set(core::format::Modifier::ITALIC);
            m
        },
    ));
    speaker_msg.push(core::format::StyledText::new("\""));
    send_formatted(conn, &speaker_msg);

    // Room broadcast
    let mut room_msg = core::format::Text::new();
    room_msg.push(core::format::StyledText::colored(
        name.as_str(),
        core::format::Color::Green,
    ));
    room_msg.push(core::format::StyledText::new(" says, \""));
    room_msg.push(core::format::StyledText::styled(
        args.to_string(),
        core::format::Color::Default,
        core::format::Color::Default,
        {
            let mut m = core::format::Modifier::new();
            m.set(core::format::Modifier::ITALIC);
            m
        },
    ));
    room_msg.push(core::format::StyledText::new("\""));

    let rendered = core::format::render(&room_msg);
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

pub fn cmd_help(
    _world: &mut World,
    conn: &mut dyn Connection,
    _args: &str,
    _registry: &ConnectionRegistry,
) {
    conn.send_line("");
    conn.send_line("Available commands:");
    conn.send_line("  look/l         — examine your surroundings");
    conn.send_line("  say <text>     — speak in the room");
    conn.send_line("  north/n        — move north");
    conn.send_line("  south/s        — move south");
    conn.send_line("  east/e         — move east");
    conn.send_line("  west/w         — move west");
    conn.send_line("  up/u           — move up");
    conn.send_line("  down/d         — move down");
    conn.send_line("  help           — this help");
    conn.send_line("  quit           — disconnect");
    conn.send_line("");
}

pub fn cmd_quit(
    _world: &mut World,
    conn: &mut dyn Connection,
    _args: &str,
    _registry: &ConnectionRegistry,
) {
    conn.send_line("Goodbye!");
    conn.disconnect();
}
