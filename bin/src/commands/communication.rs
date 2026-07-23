use oxide_core as core;
use oxide_core::{get_name, get_pos_room, is_void_room, AccessLevel, Name, World};
use oxide_server::{Command, CommandHelp, Connection, ConnectionRegistry, Server};

use super::common::*;

pub fn register(server: &mut Server) {
    server.register_command(Command {
        name: "say",
        aliases: &[],
        access: AccessLevel::Player,
        topic: "Communication",
        help: CommandHelp {
            short: "Speak aloud in the room",
            body: None,
        },
        handler: cmd_say,
    });
    server.register_command(Command {
        name: "tell",
        aliases: &[],
        access: AccessLevel::Player,
        topic: "Communication",
        help: CommandHelp {
            short: "Send a private message to another player",
            body: None,
        },
        handler: cmd_tell,
    });
    server.register_command(Command {
        name: "reply",
        aliases: &["r"],
        access: AccessLevel::Player,
        topic: "Communication",
        help: CommandHelp {
            short: "Reply to the last player who messaged you",
            body: None,
        },
        handler: cmd_reply,
    });
    server.register_command(Command {
        name: "shout",
        aliases: &[],
        access: AccessLevel::Player,
        topic: "Communication",
        help: CommandHelp {
            short: "Shout a message to the entire zone",
            body: None,
        },
        handler: cmd_shout,
    });
    server.register_command(Command {
        name: "whisper",
        aliases: &[],
        access: AccessLevel::Player,
        topic: "Communication",
        help: CommandHelp {
            short: "Whisper a message to someone in the same room",
            body: None,
        },
        handler: cmd_whisper,
    });
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

#[cfg(test)]
mod tests {
    use super::super::test_helpers::*;
    use super::*;
    use oxide_core::Position;

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

        let received = rx_listener.try_recv().ok();
        assert!(received.is_some(), "listener should receive broadcast");
        if let Some(bytes) = received {
            let msg = String::from_utf8_lossy(&bytes);
            assert!(msg.contains("Speaker"));
            assert!(msg.contains("Hello room"));
        }
    }
}
