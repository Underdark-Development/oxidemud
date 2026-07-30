use oxide_core as core;
use oxide_core::{get_name, get_pos_room, AccessLevel, World};
use oxide_server::{Command, CommandHelp, Connection, ConnectionRegistry, Server};

pub fn register(server: &mut Server) {
    server.register_command(Command {
        name: "bug",
        aliases: &[],
        access: AccessLevel::Player,
        topic: "Reports",
        help: CommandHelp {
            short: "File a bug report",
            body: Some("Usage: bug <message>\nFile a report that will be reviewed by staff."),
        },
        handler: |w, c, n, a, r| submit_report(w, c, n, a, r, "bug"),
    });
    server.register_command(Command {
        name: "idea",
        aliases: &[],
        access: AccessLevel::Player,
        topic: "Reports",
        help: CommandHelp {
            short: "Suggest an idea or feature",
            body: Some("Usage: idea <message>\nSuggest an idea for the game."),
        },
        handler: |w, c, n, a, r| submit_report(w, c, n, a, r, "idea"),
    });
    server.register_command(Command {
        name: "typo",
        aliases: &[],
        access: AccessLevel::Player,
        topic: "Reports",
        help: CommandHelp {
            short: "Report a typo in game text",
            body: Some("Usage: typo <message>\nReport a typo you found in the game."),
        },
        handler: |w, c, n, a, r| submit_report(w, c, n, a, r, "typo"),
    });
    server.register_command(Command {
        name: "complaint",
        aliases: &["complain"],
        access: AccessLevel::Player,
        topic: "Reports",
        help: CommandHelp {
            short: "File a complaint or appeal",
            body: Some("Usage: complaint <message>\nFile a complaint about another player or appeal a staff action."),
        },
        handler: |w, c, n, a, r| submit_report(w, c, n, a, r, "complaint"),
    });
}

fn submit_report(
    world: &mut World,
    conn: &mut dyn Connection,
    _name: &str,
    args: &str,
    registry: &ConnectionRegistry,
    report_type: &'static str,
) {
    let entity = match conn.entity() {
        Some(e) => e,
        None => return,
    };

    let msg = args.trim();
    if msg.is_empty() {
        conn.send_line(&format!("Report what? Use: {} <message>", report_type));
        return;
    }

    let reporter_name = get_name(world, entity)
        .map(|n| n.0.clone())
        .unwrap_or_else(|| "Unknown".to_string());

    let room_key = get_pos_room(world, entity).and_then(|room| {
        world
            .query_one::<&core::RoomKey>(room)
            .ok()
            .and_then(|mut q| q.get().map(|k| k.0.clone()))
    });

    let db = match oxide_server::get_db() {
        Some(d) => d,
        None => {
            conn.send_line("The report system is not available right now.");
            return;
        }
    };

    let report_id = {
        let guard = match db.try_lock() {
            Ok(g) => g,
            Err(_) => {
                conn.send_line("The report system is busy. Please try again.");
                return;
            }
        };
        match oxide_data::insert_report(
            guard.conn(),
            &reporter_name,
            report_type,
            msg,
            room_key.as_deref(),
        ) {
            Ok(id) => id,
            Err(e) => {
                tracing::error!("Failed to insert report: {e}");
                conn.send_line("Failed to save your report. Please try again.");
                return;
            }
        }
    };

    conn.send_line(&format!(
        "{{green}}{} report #{} filed. Thank you.{{/}}",
        report_type, report_id
    ));

    let alert = format!(
        "{{yellow}}[Report] New {{/}}{{cyan}}{}{{/}}{{yellow}} report #{} by {{/}}{{green}}{}{{/}}",
        report_type, report_id, reporter_name
    );
    let alert_parsed = core::format::parse_tags(&alert);
    let alert_rendered = alert_parsed.render(true, true);
    let alert_bytes = format!("{}\r\n", alert_rendered).into_bytes();

    for &other in &registry.connected_entities() {
        if other == entity {
            continue;
        }
        let has_staff_access = {
            let mut q = world.query_one::<&core::AccessLevel>(other).ok();
            q.as_mut()
                .and_then(|q| q.get().copied())
                .unwrap_or(core::AccessLevel::Player)
                >= core::AccessLevel::Builder
        };
        if has_staff_access {
            if let Some(tx) = registry.sender(other) {
                let _ = tx.send(alert_bytes.clone());
            }
        }
    }
}
