use oxide_core as core;
use oxide_core::{get_name, get_pos_room, entities_in_room, AccessLevel, World};
use oxide_server::{Command, CommandHelp, Connection, ConnectionRegistry, Server};

use super::common::*;

pub fn register(server: &mut Server) {
    server.register_command(Command {
        name: "socials",
        aliases: &[],
        access: AccessLevel::Player,
        topic: "Communication",
        help: CommandHelp {
            short: "List available social emotes",
            body: None,
        },
        handler: cmd_socials,
    });

    let templates = match core::templates::get_global_templates() {
        Some(t) => t,
        None => return,
    };

    for (id, _) in &templates.socials {
        let name: &'static str = Box::leak(id.clone().into_boxed_str());
        server.register_command(Command {
            name,
            aliases: &[],
            access: AccessLevel::Player,
            topic: "Socials",
            help: CommandHelp {
                short: "Perform a social emote",
                body: None,
            },
            handler: cmd_social,
        });
    }
}

fn cmd_socials(
    _world: &mut World,
    conn: &mut dyn Connection,
    _name: &str,
    _args: &str,
    _registry: &ConnectionRegistry,
) {
    let templates = match core::templates::get_global_templates() {
        Some(t) => t,
        None => return,
    };

    let mut names: Vec<&str> = templates.socials.keys().map(|s| s.as_str()).collect();
    names.sort();

    conn.send_line("Available socials:");
    for chunk in names.chunks(6) {
        conn.send_line(&chunk.join(", "));
    }
}

fn cmd_social(
    world: &mut World,
    conn: &mut dyn Connection,
    name: &str,
    args: &str,
    registry: &ConnectionRegistry,
) {
    let actor = match conn.entity() {
        Some(e) => e,
        None => return,
    };

    let templates = match core::templates::get_global_templates() {
        Some(t) => t,
        None => return,
    };

    let social = match templates.resolve_social(name) {
        Some(s) => s,
        None => {
            conn.send_line("Huh? Type 'help' for a list of commands.");
            return;
        }
    };

    let actor_name = get_name(world, actor)
        .map(|n| n.0.clone())
        .unwrap_or_else(|| "Someone".to_string());
    let actor_gender = world
        .query_one::<&core::Gender>(actor)
        .ok()
        .and_then(|mut q| q.get().cloned())
        .unwrap_or_default();

    let room = match get_pos_room(world, actor) {
        Some(r) => r,
        None => {
            conn.send_line("You are nowhere.");
            return;
        }
    };

    let target_name = args.trim();
    let target_entity = if target_name.is_empty() {
        None
    } else {
        resolve_target_in_room(world, room, actor, target_name)
    };

    if target_name.is_empty() {
        let char_tpl = match &social.char_no_target {
            Some(t) => t,
            None => {
                conn.send_line(&format!("What do you want to {}?", social.name));
                return;
            }
        };
        let room_tpl = match &social.room_no_target {
            Some(t) => t,
            None => {
                conn.send_line(&format!("What do you want to {}?", social.name));
                return;
            }
        };

        let char_msg = interpolate(char_tpl, &actor_name, &actor_gender, None, None);
        let room_msg = interpolate(room_tpl, &actor_name, &actor_gender, None, None);
        send_to_conn(conn, &char_msg);
        broadcast_to_room_except(world, registry, room, actor, &room_msg);
        return;
    }

    let target = match target_entity {
        Some(e) if e == actor => {
            let char_tpl = match &social.char_self {
                Some(t) => t,
                None => {
                    conn.send_line(&format!("You can't {} yourself.", social.name));
                    return;
                }
            };
            let room_tpl = match &social.room_self {
                Some(t) => t,
                None => {
                    conn.send_line(&format!("You can't {} yourself.", social.name));
                    return;
                }
            };

            let char_msg = interpolate(char_tpl, &actor_name, &actor_gender, None, None);
            let room_msg = interpolate(room_tpl, &actor_name, &actor_gender, None, None);
            send_to_conn(conn, &char_msg);
            broadcast_to_room_except(world, registry, room, actor, &room_msg);
            return;
        }
        Some(e) => e,
        None => {
            conn.send_line(&format!("You don't see '{}' here.", target_name));
            return;
        }
    };

    {
        let target_name_str = get_name(world, target)
            .map(|n| n.0.clone())
            .unwrap_or_else(|| "someone".to_string());
        let target_gender = world
            .query_one::<&core::Gender>(target)
            .ok()
            .and_then(|mut q| q.get().cloned())
            .unwrap_or_default();

        let char_tpl = match &social.char_target {
            Some(t) => t,
            None => {
                conn.send_line(&format!("You can't {} at someone.", social.name));
                return;
            }
        };
        let room_tpl = match &social.room_target {
            Some(t) => t,
            None => {
                conn.send_line(&format!("You can't {} at someone.", social.name));
                return;
            }
        };
        let target_tpl = match &social.target_char {
            Some(t) => t,
            None => {
                conn.send_line(&format!("You can't {} at someone.", social.name));
                return;
            }
        };

        let char_msg = interpolate(char_tpl, &actor_name, &actor_gender, Some(&target_name_str), Some(&target_gender));
        let room_msg = interpolate(room_tpl, &actor_name, &actor_gender, Some(&target_name_str), Some(&target_gender));
        let target_msg = interpolate(target_tpl, &actor_name, &actor_gender, Some(&target_name_str), Some(&target_gender));

        send_to_conn(conn, &char_msg);

        let bytes = format!("{}\r\n", room_msg).into_bytes();
        for &other in &registry.occupants(world, room) {
            if other == actor || other == target {
                continue;
            }
            if let Some(tx) = registry.sender(other) {
                let _ = tx.send(bytes.clone());
            }
        }

        send_to_online_player(registry, target, &target_msg);
    }
}

fn resolve_target_in_room(world: &World, room: core::Entity, actor: core::Entity, target_name: &str) -> Option<core::Entity> {
    let lower_target = target_name.to_lowercase();
    let entities = entities_in_room(world, room);
    let candidates: Vec<core::Entity> = entities
        .into_iter()
        .filter(|&e| {
            e != actor
                && get_name(world, e)
                    .map(|n| n.0.to_lowercase().starts_with(&lower_target))
                    .unwrap_or(false)
        })
        .collect();
    if let Some(&exact) = candidates.iter().find(|&&e| {
        get_name(world, e)
            .map(|n| n.0.to_lowercase() == lower_target)
            .unwrap_or(false)
    }) {
        Some(exact)
    } else if candidates.len() == 1 {
        Some(candidates[0])
    } else {
        None
    }
}

fn interpolate(template: &str, actor_name: &str, actor_gender: &core::Gender, target_name: Option<&str>, target_gender: Option<&core::Gender>) -> String {
    core::format::social::interpolate(template, actor_name, actor_gender, target_name, target_gender)
}
