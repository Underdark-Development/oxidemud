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
    server.register_command(Command {
        name: "channel",
        aliases: &["channels"],
        access: AccessLevel::Player,
        topic: "Communication",
        help: CommandHelp {
            short: "List or toggle chat channel preferences",
            body: Some("Usage: channel [<name> [on|off]]"),
        },
        handler: cmd_channel,
    });
    server.register_command(Command {
        name: "ooc",
        aliases: &[],
        access: AccessLevel::Player,
        topic: "Communication",
        help: CommandHelp {
            short: "Send a message on the OOC channel",
            body: None,
        },
        handler: cmd_ooc,
    });
    server.register_command(Command {
        name: "gossip",
        aliases: &["goss"],
        access: AccessLevel::Player,
        topic: "Communication",
        help: CommandHelp {
            short: "Send a message on the Gossip channel",
            body: None,
        },
        handler: cmd_gossip,
    });
    server.register_command(Command {
        name: "newbie",
        aliases: &["newb"],
        access: AccessLevel::Player,
        topic: "Communication",
        help: CommandHelp {
            short: "Send a message on the Newbie channel (level 1-5)",
            body: None,
        },
        handler: cmd_newbie,
    });
    server.register_command(Command {
        name: "auction",
        aliases: &["auc"],
        access: AccessLevel::Player,
        topic: "Communication",
        help: CommandHelp {
            short: "Send a message on the Auction channel",
            body: None,
        },
        handler: cmd_auction,
    });
    server.register_command(Command {
        name: "emote",
        aliases: &[],
        access: AccessLevel::Player,
        topic: "Communication",
        help: CommandHelp {
            short: "Perform an emote visible in the room",
            body: Some("Usage: emote <message>\nExample: emote waves happily."),
        },
        handler: cmd_emote,
    });
    server.register_command(Command {
        name: "gsay",
        aliases: &[],
        access: AccessLevel::Player,
        topic: "Communication",
        help: CommandHelp {
            short: "Speak to group members in the same room",
            body: Some(
                "Usage: gsay <message>\nOnly group members in your current room will hear it.",
            ),
        },
        handler: cmd_gsay,
    });
    server.register_command(Command {
        name: "gtell",
        aliases: &[],
        access: AccessLevel::Player,
        topic: "Communication",
        help: CommandHelp {
            short: "Send a message to all group members anywhere",
            body: Some("Usage: gtell <message>\nAll online group members will receive it regardless of location."),
        },
        handler: cmd_gtell,
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
                targets.push(item_eid);
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

// ---------------------------------------------------------------------------
// Channel system — helpers
// ---------------------------------------------------------------------------

/// Find a channel def by id or alias.
fn resolve_channel(query: &str) -> Option<core::ChannelDef> {
    let defs = core::default_channel_defs();
    let lower = query.to_lowercase();
    defs.into_iter()
        .find(|c| c.id == lower || c.aliases.contains(&lower))
}

/// Broadcast a message to all players who have the given channel enabled.
/// Returns an error message string if the channel cannot be sent on.
fn send_to_channel(
    world: &World,
    registry: &ConnectionRegistry,
    entity: core::Entity,
    channel_id: &str,
    message: &str,
) -> Result<(), String> {
    let ch =
        resolve_channel(channel_id).ok_or_else(|| format!("Unknown channel '{channel_id}'."))?;

    // Level gate
    let level = world
        .query_one::<&core::Level>(entity)
        .ok()
        .and_then(|mut q| q.get().copied())
        .unwrap_or(core::Level(1));
    if level.0 < ch.min_level_send {
        return Err(format!(
            "You must be at least level {} to send on {}.",
            ch.min_level_send, ch.name
        ));
    }
    if ch.max_level_send > 0 && level.0 > ch.max_level_send {
        return Err(format!(
            "Characters above level {} cannot send on {}.",
            ch.max_level_send, ch.name
        ));
    }

    let player_name = get_name(world, entity)
        .map(|n| n.0.clone())
        .unwrap_or_else(|| "Someone".to_string());

    let msg_fmt = if ch.is_ooc {
        message.to_string()
    } else {
        format_ghost_text(message)
    };

    // Sender message — "You" instead of player name
    let sender_msg = ch.render(&player_name, &msg_fmt, true);
    let sender_parsed = core::format::parse_tags(&sender_msg);
    send_to_conn_simple(world, registry, entity, &sender_parsed.render(true, true));

    // Recipient message — actual player name
    let recipient_msg = ch.render(&player_name, &msg_fmt, false);
    let recipient_parsed = core::format::parse_tags(&recipient_msg);
    let recipient_rendered = recipient_parsed.render(true, true);
    let bytes = format!("{}\r\n", recipient_rendered).into_bytes();

    // Determine recipients based on channel scope
    let targets: Vec<core::Entity> = match ch.scope {
        core::ChannelScope::Global => registry.connected_entities().to_vec(),
        _ => {
            let room = match core::get_pos_room(world, entity) {
                Some(r) => r,
                None => return Ok(()),
            };
            let rooms = core::collect_rooms_by_scope(world, room, &ch.scope);
            let mut seen = std::collections::HashSet::new();
            seen.insert(entity);
            let mut targets = Vec::new();
            for r in rooms {
                for occ in registry.occupants(world, r) {
                    if seen.insert(occ) {
                        targets.push(occ);
                    }
                }
            }
            targets
        }
    };

    for &other in &targets {
        if other == entity {
            continue;
        }
        // Check that recipient has this channel enabled
        let is_enabled = match world.query_one::<&core::ChannelPrefs>(other) {
            Ok(mut q) => q.get().map(|p| p.is_enabled(&ch.id)).unwrap_or(true),
            _ => true,
        };
        if !is_enabled {
            continue;
        }
        if let Some(tx) = registry.sender(other) {
            let _ = tx.send(bytes.clone());
        }
    }

    Ok(())
}

/// Send a simple message to a single player via their connection.
fn send_to_conn_simple(
    _world: &World,
    registry: &ConnectionRegistry,
    entity: core::Entity,
    message: &str,
) {
    if let Some(tx) = registry.sender(entity) {
        let _ = tx.send(format!("{}\r\n", message).into_bytes());
    }
}

/// Broadcast a message to all online group members.
/// If `room_only`, only members in the same room as the sender receive it.
pub(super) fn broadcast_to_group(
    world: &World,
    registry: &ConnectionRegistry,
    entity: core::Entity,
    message: &str,
    room_only: bool,
) -> Result<(), String> {
    let gm = world
        .query_one::<&core::GroupMember>(entity)
        .ok()
        .and_then(|mut q| q.get().copied())
        .ok_or_else(|| "You are not in a group.".to_string())?;

    let name = get_name(world, entity)
        .map(|n| n.0.clone())
        .unwrap_or_else(|| "Someone".to_string());

    let sender_room = if room_only {
        get_pos_room(world, entity)
    } else {
        None
    };

    let formatted = format!("[Group] {}: {}\r\n", name, message).into_bytes();

    if let Ok(mut q) = world.query_one::<&core::Group>(gm.group_id) {
        if let Some(group) = q.get() {
            for m in &group.members {
                let m_ent = match m.entity {
                    Some(e) => e,
                    None => continue,
                };
                if room_only {
                    let in_same_room = sender_room
                        .and_then(|sr| {
                            world
                                .query_one::<&core::Position>(m_ent)
                                .ok()
                                .and_then(|mut q| q.get().map(|p| p.room == sr))
                        })
                        .unwrap_or(false);
                    if !in_same_room {
                        continue;
                    }
                }
                if let Some(tx) = registry.sender(m_ent) {
                    let _ = tx.send(formatted.clone());
                }
            }
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Channel command: channel [<name> [on|off]]
// ---------------------------------------------------------------------------

pub fn cmd_channel(
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

    let defs = core::default_channel_defs();

    let parts: Vec<&str> = args.split_whitespace().collect();
    match parts.len() {
        0 => {
            let prefs = world
                .query_one::<&core::ChannelPrefs>(entity)
                .ok()
                .and_then(|mut q| q.get().cloned())
                .unwrap_or_default();
            conn.send_line("Available channels:");
            for ch in &defs {
                let status = if prefs.is_enabled(&ch.id) {
                    "ON"
                } else {
                    "OFF"
                };
                let shortcut = if ch.shortcut.is_empty() {
                    String::new()
                } else {
                    format!(" (shortcut: {})", ch.shortcut)
                };
                conn.send_line(&format!("  {:10}  {}{shortcut}", ch.name, status));
            }
            conn.send_line("Usage: channel <name> [on|off] — toggle or set a channel.");
        }
        1 => {
            // Toggle channel
            let ch_name = parts[0];
            let ch = match resolve_channel(ch_name) {
                Some(c) => c,
                None => {
                    conn.send_line(&format!(
                        "Unknown channel '{ch_name}'. Type 'channel' for a list."
                    ));
                    return;
                }
            };
            let mut prefs = world
                .query_one::<&core::ChannelPrefs>(entity)
                .ok()
                .and_then(|mut q| q.get().cloned())
                .unwrap_or_default();
            let new_state = prefs.toggle(&ch.id);
            let _ = world.insert(entity, (prefs, core::Dirty));
            let state_str = if new_state { "enabled" } else { "disabled" };
            conn.send_line(&format!("{} channel {state_str}.", ch.name));
        }
        2 => {
            // Set channel on/off
            let ch_name = parts[0];
            let setting = parts[1].to_lowercase();
            let ch = match resolve_channel(ch_name) {
                Some(c) => c,
                None => {
                    conn.send_line(&format!(
                        "Unknown channel '{ch_name}'. Type 'channel' for a list."
                    ));
                    return;
                }
            };
            let enabled = match setting.as_str() {
                "on" => true,
                "off" => false,
                _ => {
                    conn.send_line("Usage: channel <name> [on|off]");
                    return;
                }
            };
            let mut prefs = world
                .query_one::<&core::ChannelPrefs>(entity)
                .ok()
                .and_then(|mut q| q.get().cloned())
                .unwrap_or_default();
            prefs.set_enabled(ch.id.clone(), enabled);
            let _ = world.insert(entity, (prefs, core::Dirty));
            let state_str = if enabled { "enabled" } else { "disabled" };
            conn.send_line(&format!("{} channel {state_str}.", ch.name));
        }
        _ => {
            conn.send_line("Usage: channel [<name> [on|off]]");
        }
    }
}

// ---------------------------------------------------------------------------
// Channel send commands: ooc, gossip, newbie, auction
// ---------------------------------------------------------------------------

fn handle_channel_send(
    world: &mut World,
    conn: &mut dyn Connection,
    _name: &str,
    args: &str,
    registry: &ConnectionRegistry,
    channel_id: &'static str,
) {
    let entity = match conn.entity() {
        Some(e) => e,
        None => return,
    };

    if args.is_empty() {
        let ch = resolve_channel(channel_id)
            .map(|c| c.name.clone())
            .unwrap_or_else(|| channel_id.to_string());
        conn.send_line(&format!("Send what on {ch}?"));
        return;
    }

    match send_to_channel(world, registry, entity, channel_id, args) {
        Ok(()) => {}
        Err(msg) => {
            conn.send_line(&msg);
        }
    }
}

pub fn cmd_ooc(
    world: &mut World,
    conn: &mut dyn Connection,
    name: &str,
    args: &str,
    registry: &ConnectionRegistry,
) {
    handle_channel_send(world, conn, name, args, registry, "ooc");
}

pub fn cmd_gossip(
    world: &mut World,
    conn: &mut dyn Connection,
    name: &str,
    args: &str,
    registry: &ConnectionRegistry,
) {
    handle_channel_send(world, conn, name, args, registry, "gossip");
}

pub fn cmd_newbie(
    world: &mut World,
    conn: &mut dyn Connection,
    name: &str,
    args: &str,
    registry: &ConnectionRegistry,
) {
    handle_channel_send(world, conn, name, args, registry, "newbie");
}

pub fn cmd_auction(
    world: &mut World,
    conn: &mut dyn Connection,
    name: &str,
    args: &str,
    registry: &ConnectionRegistry,
) {
    handle_channel_send(world, conn, name, args, registry, "auction");
}

// ---------------------------------------------------------------------------
// Emote command
// ---------------------------------------------------------------------------

pub fn cmd_emote(
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
        conn.send_line("Emote what?");
        return;
    }

    let room = match get_pos_room(world, entity) {
        Some(r) => r,
        None => {
            conn.send_line("You are nowhere.");
            return;
        }
    };

    if is_void_room(world, room) {
        conn.send_line("There is no one here to see your emote.");
        return;
    }

    let name = get_name(world, entity).unwrap_or(Name::new("Someone"));

    // Sender sees: "You wave happily."
    let sender_msg = format!("You {}", args);
    send_to_conn(conn, &sender_msg);

    // Room sees: "PlayerName waves happily."
    let room_msg = format!("{} {}", name, args);
    let room_parsed = core::format::parse_tags(&room_msg);
    let room_rendered = room_parsed.render(true, true);
    let bytes = format!("{}\r\n", room_rendered).into_bytes();

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

// ---------------------------------------------------------------------------
// Group chat commands: gsay, gtell
// ---------------------------------------------------------------------------

pub fn cmd_gsay(
    world: &mut World,
    conn: &mut dyn Connection,
    _name: &str,
    args: &str,
    registry: &ConnectionRegistry,
) {
    if args.is_empty() {
        conn.send_line("Say what to your group?");
        return;
    }

    let entity = match conn.entity() {
        Some(e) => e,
        None => return,
    };

    match broadcast_to_group(world, registry, entity, args, true) {
        Ok(()) => {}
        Err(msg) => conn.send_line(&msg),
    }
}

pub fn cmd_gtell(
    world: &mut World,
    conn: &mut dyn Connection,
    _name: &str,
    args: &str,
    registry: &ConnectionRegistry,
) {
    if args.is_empty() {
        conn.send_line("Tell what to your group?");
        return;
    }

    let entity = match conn.entity() {
        Some(e) => e,
        None => return,
    };

    match broadcast_to_group(world, registry, entity, args, false) {
        Ok(()) => {}
        Err(msg) => conn.send_line(&msg),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

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
