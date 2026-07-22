use oxide_core as core;
use oxide_server::{Connection, ConnectionFlag, ConnectionRegistry};

pub fn send_formatted(conn: &mut dyn Connection, text: &core::format::RichText) {
    let ansi = conn.flags().has(ConnectionFlag::Ansi);
    let blink = conn.flags().has(ConnectionFlag::Blink);
    let width = conn.screen_width() as usize;
    conn.send_line(&text.render_wrapped(width, ansi, blink));
}

pub fn broadcast_to_room_except(
    world: &core::World,
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

pub fn send_to_online_player(registry: &ConnectionRegistry, entity: core::Entity, message: &str) {
    if let Some(tx) = registry.sender(entity) {
        let text = core::format::parse_tags(message);
        let rendered = text.render(true, true);
        let _ = tx.send(format!("{}\r\n", rendered).into_bytes());
    }
}

pub fn send_to_conn(conn: &mut dyn Connection, message: &str) {
    let text = core::format::parse_tags(message);
    send_formatted(conn, &text);
}

pub fn find_online_player(
    world: &core::World,
    registry: &ConnectionRegistry,
    name: &str,
) -> Option<core::Entity> {
    let lower_name = name.to_lowercase();
    let candidates: Vec<core::Entity> = registry
        .connected_entities()
        .into_iter()
        .filter(|&e| {
            if let Some(n) = core::get_name(world, e) {
                n.0.to_lowercase().starts_with(&lower_name)
            } else {
                false
            }
        })
        .collect();

    if let Some(&exact) = candidates.iter().find(|&&e| {
        if let Some(n) = core::get_name(world, e) {
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

pub fn format_ghost_text(text: &str) -> String {
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
