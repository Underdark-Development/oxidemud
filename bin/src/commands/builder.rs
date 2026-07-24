use oxide_core as core;
use oxide_core::{AccessLevel, World};
use oxide_server::{Command, CommandHelp, Connection, ConnectionRegistry, Server};

use super::movement::cmd_look;

pub fn register(server: &mut Server) {
    server.register_command(Command {
        name: "@award",
        aliases: &[],
        access: AccessLevel::Admin,
        topic: "Admin",
        help: CommandHelp {
            short: "Award experience points to yourself",
            body: None,
        },
        handler: cmd_award,
    });
    server.register_command(Command {
        name: "@area",
        aliases: &[],
        access: AccessLevel::Builder,
        topic: "Builder",
        help: CommandHelp {
            short: "Manage area templates",
            body: None,
        },
        handler: cmd_area,
    });
    server.register_command(Command {
        name: "@dig",
        aliases: &[],
        access: AccessLevel::Builder,
        topic: "Builder",
        help: CommandHelp {
            short: "Dig a new room and exit in a specified direction",
            body: None,
        },
        handler: cmd_dig,
    });
    server.register_command(Command {
        name: "@link",
        aliases: &[],
        access: AccessLevel::Builder,
        topic: "Builder",
        help: CommandHelp {
            short: "Link an exit direction to an existing room key",
            body: None,
        },
        handler: cmd_link,
    });
    server.register_command(Command {
        name: "@unlink",
        aliases: &[],
        access: AccessLevel::Builder,
        topic: "Builder",
        help: CommandHelp {
            short: "Remove an exit link in a specified direction",
            body: None,
        },
        handler: cmd_unlink,
    });
    server.register_command(Command {
        name: "@set",
        aliases: &[],
        access: AccessLevel::Builder,
        topic: "Builder",
        help: CommandHelp {
            short: "Set entity or room properties",
            body: None,
        },
        handler: cmd_set,
    });
    server.register_command(Command {
        name: "@desc",
        aliases: &[],
        access: AccessLevel::Builder,
        topic: "Builder",
        help: CommandHelp {
            short: "Set the description of the current room",
            body: None,
        },
        handler: cmd_desc,
    });
    server.register_command(Command {
        name: "@room",
        aliases: &[],
        access: AccessLevel::Builder,
        topic: "Builder",
        help: CommandHelp {
            short: "Manage room instances",
            body: None,
        },
        handler: cmd_room,
    });
    server.register_command(Command {
        name: "@portal",
        aliases: &[],
        access: AccessLevel::Builder,
        topic: "Builder",
        help: CommandHelp {
            short: "Manage room portals",
            body: None,
        },
        handler: cmd_portal,
    });
    server.register_command(Command {
        name: "@mob",
        aliases: &[],
        access: AccessLevel::Builder,
        topic: "Builder",
        help: CommandHelp {
            short: "Manage mob templates and spawn/despawn",
            body: None,
        },
        handler: cmd_mob,
    });
    server.register_command(Command {
        name: "@item",
        aliases: &[],
        access: AccessLevel::Builder,
        topic: "Builder",
        help: CommandHelp {
            short: "Manage item templates",
            body: None,
        },
        handler: cmd_item,
    });
    server.register_command(Command {
        name: "@load",
        aliases: &[],
        access: AccessLevel::Builder,
        topic: "Builder",
        help: CommandHelp {
            short: "Load an item or mob template into the game",
            body: None,
        },
        handler: cmd_load,
    });
    server.register_command(Command {
        name: "@validate",
        aliases: &[],
        access: AccessLevel::Builder,
        topic: "Builder",
        help: CommandHelp {
            short: "Validate content templates for integrity errors",
            body: None,
        },
        handler: cmd_validate,
    });
    server.register_command(Command {
        name: "goto",
        aliases: &[],
        access: AccessLevel::Immortal,
        topic: "Immortal",
        help: CommandHelp {
            short: "Teleport directly to a room key or player",
            body: None,
        },
        handler: cmd_goto,
    });
    server.register_command(Command {
        name: "at",
        aliases: &[],
        access: AccessLevel::Immortal,
        topic: "Immortal",
        help: CommandHelp {
            short: "Execute a command at a distant room or player",
            body: None,
        },
        handler: cmd_at,
    });
    server.register_command(Command {
        name: "force",
        aliases: &[],
        access: AccessLevel::Immortal,
        topic: "Immortal",
        help: CommandHelp {
            short: "Force a target player or mob to execute a command",
            body: None,
        },
        handler: cmd_force,
    });
    server.register_command(Command {
        name: "stat",
        aliases: &[],
        access: AccessLevel::Immortal,
        topic: "Immortal",
        help: CommandHelp {
            short: "Display detailed entity stats and ECS components",
            body: None,
        },
        handler: cmd_stat,
    });
    server.register_command(Command {
        name: "olocate",
        aliases: &[],
        access: AccessLevel::Immortal,
        topic: "Immortal",
        help: CommandHelp {
            short: "Locate all instances of an item in the world",
            body: None,
        },
        handler: cmd_olocate,
    });
    server.register_command(Command {
        name: "gecho",
        aliases: &[],
        access: AccessLevel::Immortal,
        topic: "Immortal",
        help: CommandHelp {
            short: "Broadcast a message to all connected players",
            body: None,
        },
        handler: cmd_gecho,
    });
    server.register_command(Command {
        name: "gtell",
        aliases: &[],
        access: AccessLevel::Builder,
        topic: "Builder",
        help: CommandHelp {
            short: "Send a message to the staff channel",
            body: None,
        },
        handler: cmd_gtell,
    });
    server.register_command(Command {
        name: "wizwho",
        aliases: &[],
        access: AccessLevel::Builder,
        topic: "Builder",
        help: CommandHelp {
            short: "List all online staff members and their ranks",
            body: None,
        },
        handler: cmd_wizwho,
    });
    server.register_command(Command {
        name: "wizin",
        aliases: &[],
        access: AccessLevel::Builder,
        topic: "Builder",
        help: CommandHelp {
            short: "Toggle staff invisibility",
            body: None,
        },
        handler: cmd_wizin,
    });
    server.register_command(Command {
        name: "holylight",
        aliases: &[],
        access: AccessLevel::Builder,
        topic: "Builder",
        help: CommandHelp {
            short: "Toggle holy light to see all dark/hidden areas",
            body: None,
        },
        handler: cmd_holylight,
    });
    server.register_command(Command {
        name: "@teleport",
        aliases: &[],
        access: AccessLevel::Immortal,
        topic: "Immortal",
        help: CommandHelp {
            short: "Teleport another player to a room key or player",
            body: None,
        },
        handler: cmd_teleport,
    });
    server.register_command(Command {
        name: "switch",
        aliases: &[],
        access: AccessLevel::God,
        topic: "God",
        help: CommandHelp {
            short: "Possess and control an NPC mob body",
            body: None,
        },
        handler: cmd_switch,
    });
    server.register_command(Command {
        name: "return",
        aliases: &[],
        access: AccessLevel::God,
        topic: "God",
        help: CommandHelp {
            short: "Return to your original body after possessing a mob",
            body: None,
        },
        handler: cmd_return,
    });
}

fn find_player_by_name(world: &World, name: &str) -> Option<core::Entity> {
    let name_lower = name.to_lowercase();
    let mut q = world.query::<(&core::Name, &core::Player)>();
    for (entity, (n, _)) in q.iter() {
        if n.0.to_lowercase() == name_lower {
            return Some(core::Entity::from(entity));
        }
    }
    None
}

fn find_mob_in_room(
    world: &World,
    room_entity: core::Entity,
    target_name: &str,
) -> Option<core::Entity> {
    let target_lower = target_name.to_lowercase();
    let occupants = core::util::entities_in_room(world, room_entity);
    for entity in occupants {
        let is_player = world
            .query_one::<&core::Player>(entity)
            .ok()
            .map(|mut q| q.get().is_some())
            .unwrap_or(false);
        if is_player {
            continue;
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

    oxide_server::award_xp(world, entity);

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
        entity: Option<core::Entity>,
        output: Vec<String>,
        access: core::AccessLevel,
    }
    impl Connection for MockConnection {
        fn send(&mut self, text: &str) {
            self.output.push(text.to_string());
        }
        fn send_line(&mut self, text: &str) {
            self.output.push(text.to_string());
        }
        fn send_raw(&mut self, _bytes: &[u8]) {}
        fn id(&self) -> &str {
            "0"
        }
        fn entity(&self) -> Option<core::Entity> {
            self.entity
        }
        fn set_entity(&mut self, entity: core::Entity) {
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
        fn access_level(&self) -> core::AccessLevel {
            self.access
        }
        fn set_access_level(&mut self, level: core::AccessLevel) {
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
                if inv.0.contains(&core::Entity::from(entity)) {
                    holder_info = format!("In inventory of {}", owner_name.0);
                }
            }

            let mut q_eq = world.query::<(&core::Equipment, &core::Name)>();
            for (_, (eq, owner_name)) in q_eq.iter() {
                if eq
                    .slots
                    .iter()
                    .any(|(_, item_ent)| *item_ent == core::Entity::from(entity))
                {
                    holder_info = format!("Equipped on {}", owner_name.0);
                }
            }

            let mut q_floor = world.query::<(&core::FloorItems, &core::Name)>();
            for (_, (floor, room_name)) in q_floor.iter() {
                if floor.0.contains(&core::Entity::from(entity)) {
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
                    no_weather: false,
                    exclude_weather: Vec::new(),
                    additional_weather: std::collections::HashMap::new(),
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
                        q_void.iter().next().map(|(e, _)| core::Entity::from(e))
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
                        no_weather: false,
                        weather_matrix: std::collections::HashMap::new(),
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

#[cfg(test)]
mod tests {
    use super::super::general::cmd_help;
    use super::super::test_helpers::*;
    use super::*;
    use oxide_core::{Position, Room};

    #[test]
    fn test_switch_command_success() {
        let _guard = init_test_templates();
        let (mut world, _void, room_a, _room_b) = test_world();
        let (player, mut conn, conn_reg) = test_player(&mut world, room_a);

        let mob = world.spawn((
            Position::new(room_a),
            core::Name::new("Goblin Guard"),
            core::Npc::new("goblin"),
        ));

        cmd_switch(&mut world, &mut conn, "switch", "goblin", &conn_reg);

        let lines = conn.take_lines();
        assert!(lines.iter().any(|l| l.contains("You possess the body.")));

        assert_eq!(conn.entity(), Some(mob));

        let is_switched = world
            .query_one::<&core::Switched>(mob)
            .ok()
            .and_then(|mut q| q.get().map(|s| s.original_entity))
            == Some(player);
        assert!(is_switched);
    }

    #[test]
    fn test_switch_command_not_npc() {
        let (mut world, _void, room_a, _room_b) = test_world();
        let (player, mut conn, conn_reg) = test_player(&mut world, room_a);

        let _other_player = world.spawn((
            Position::new(room_a),
            core::Name::new("Bob"),
            core::Player::new(1),
        ));

        cmd_switch(&mut world, &mut conn, "switch", "bob", &conn_reg);

        let lines = conn.take_lines();
        assert!(lines
            .iter()
            .any(|l| l.contains("No such mob in this room.")));

        assert_eq!(conn.entity(), Some(player));
    }

    #[test]
    fn test_return_command_when_switched() {
        let (mut world, _void, room_a, _room_b) = test_world();
        let (player, mut conn, conn_reg) = test_player(&mut world, room_a);

        let mob = world.spawn((
            Position::new(room_a),
            core::Name::new("Goblin Guard"),
            core::Npc::new("goblin"),
            core::Switched {
                original_entity: player,
            },
        ));

        conn.set_entity(mob);

        cmd_return(&mut world, &mut conn, "return", "", &conn_reg);

        let lines = conn.take_lines();
        assert!(lines
            .iter()
            .any(|l| l.contains("You return to your original form.")));

        assert_eq!(conn.entity(), Some(player));

        let still_switched = world
            .query_one::<&core::Switched>(mob)
            .is_ok_and(|mut q| q.get().is_some());
        assert!(!still_switched);
    }

    #[test]
    fn test_return_command_when_not_switched() {
        let (mut world, _void, room_a, _room_b) = test_world();
        let (player, mut conn, conn_reg) = test_player(&mut world, room_a);

        cmd_return(&mut world, &mut conn, "return", "", &conn_reg);

        let lines = conn.take_lines();
        assert!(lines.iter().any(|l| l.contains("You are not switched.")));

        assert_eq!(conn.entity(), Some(player));
    }

    #[test]
    fn test_help_topics_and_filtering() {
        let mut dispatch = oxide_server::CommandDispatch::new();
        dispatch.register(oxide_server::Command {
            name: "help",
            aliases: &[],
            access: core::AccessLevel::Player,
            topic: "General",
            help: oxide_server::CommandHelp {
                short: "Help command description",
                body: None,
            },
            handler: |w, c, n, a, r| cmd_help(w, c, n, a, r),
        });
        dispatch.register(oxide_server::Command {
            name: "look",
            aliases: &[],
            access: core::AccessLevel::Player,
            topic: "General",
            help: oxide_server::CommandHelp {
                short: "look description",
                body: None,
            },
            handler: |_, _, _, _, _| {},
        });
        dispatch.register(oxide_server::Command {
            name: "goto",
            aliases: &[],
            access: core::AccessLevel::Immortal,
            topic: "Immortal",
            help: oxide_server::CommandHelp {
                short: "goto description",
                body: None,
            },
            handler: |_, _, _, _, _| {},
        });
        dispatch.register(oxide_server::Command {
            name: "@dig",
            aliases: &[],
            access: core::AccessLevel::Builder,
            topic: "Builder",
            help: oxide_server::CommandHelp {
                short: "dig description",
                body: None,
            },
            handler: |_, _, _, _, _| {},
        });
        let _ = oxide_server::set_commands(dispatch);

        let mut world = World::new();
        let mut conn = MockConnection::new();
        let conn_reg = ConnectionRegistry::new();

        conn.set_access_level(core::AccessLevel::Player);
        cmd_help(&mut world, &mut conn, "help", "", &conn_reg);
        let lines = conn.take_lines();

        assert!(lines.iter().any(|l| l.contains("Available Topics")));
        assert!(lines.iter().any(|l| l.contains("General")));
        assert!(!lines.iter().any(|l| l.contains("Builder")));
        assert!(!lines.iter().any(|l| l.contains("Immortal")));

        cmd_help(&mut world, &mut conn, "help", "goto", &conn_reg);
        let lines = conn.take_lines();
        assert!(lines
            .iter()
            .any(|l| l.contains("No help found for 'goto'.")));

        conn.set_access_level(core::AccessLevel::Builder);
        cmd_help(&mut world, &mut conn, "help", "", &conn_reg);
        let lines = conn.take_lines();
        assert!(lines.iter().any(|l| l.contains("Builder")));
        assert!(!lines.iter().any(|l| l.contains("Immortal")));

        cmd_help(&mut world, &mut conn, "help", "@dig", &conn_reg);
        let lines = conn.take_lines();
        assert!(lines.iter().any(|l| l.contains("@dig")));

        conn.set_access_level(core::AccessLevel::Immortal);
        cmd_help(&mut world, &mut conn, "help", "", &conn_reg);
        let lines = conn.take_lines();
        assert!(lines.iter().any(|l| l.contains("Builder")));
        assert!(lines.iter().any(|l| l.contains("Immortal")));
    }

    #[test]
    fn test_olc_commands_integration() {
        let _guard = init_test_templates();
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
                        no_weather: false,
                        exclude_weather: Vec::new(),
                        additional_weather: std::collections::HashMap::new(),
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
