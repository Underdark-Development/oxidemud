use oxide_core::{AccessLevel, World};
use oxide_server::{Command, CommandHelp, Connection, ConnectionRegistry, Server};

pub const HELP_SHUTDOWN: &str = r#"Usage: shutdown [now|<minutes>|cancel]
  now        Shut down the server immediately
  <minutes>  Schedule an announced countdown before shutting down
  cancel     Cancel a pending scheduled shutdown
Players are notified as the countdown progresses. Admin access required."#;

pub fn register(server: &mut Server) {
    server.register_command(Command {
        name: "shutdown",
        aliases: &[],
        access: AccessLevel::Admin,
        topic: "Admin",
        help: CommandHelp {
            short: "Shut down or schedule shutdown of the server",
            body: Some(HELP_SHUTDOWN),
        },
        handler: cmd_shutdown,
    });
}

pub fn cmd_shutdown(
    _world: &mut World,
    conn: &mut dyn Connection,
    _name: &str,
    args: &str,
    _registry: &ConnectionRegistry,
) {
    let arg = args.trim().to_lowercase();
    if arg.is_empty() {
        if oxide_server::scheduled_shutdown_pending() {
            conn.send_line(
                "A scheduled shutdown is already active. Type 'shutdown cancel' to cancel it.",
            );
        } else {
            conn.send_line("Usage: shutdown [now|<minutes>|cancel]");
            conn.send_line("No shutdown is currently scheduled.");
        }
        return;
    }

    match arg.as_str() {
        "now" => match oxide_server::request_immediate_shutdown("admin command") {
            Ok(()) => {
                tracing::warn!(
                    target: "audit",
                    executor = ?conn.entity(),
                    "In-game shutdown initiated"
                );
                conn.send_line("Server is shutting down now.");
            }
            Err(e) => conn.send_line(&format!("Failed to request shutdown: {e}")),
        },
        "cancel" => {
            if oxide_server::cancel_scheduled_shutdown("admin command") {
                tracing::warn!(
                    target: "audit",
                    executor = ?conn.entity(),
                    "In-game scheduled shutdown cancelled"
                );
                conn.send_line("Scheduled shutdown cancelled.");
            } else {
                conn.send_line("No scheduled shutdown is active.");
            }
        }
        _ => match arg.parse::<u32>() {
            Ok(mins) => match oxide_server::validate_delay_minutes(mins) {
                Ok(delay) => {
                    match oxide_server::schedule_delayed_shutdown(delay, "admin command") {
                        Ok(()) => {
                            tracing::warn!(
                                target: "audit",
                                executor = ?conn.entity(),
                                delay_mins = mins,
                                "In-game delayed shutdown scheduled"
                            );
                            conn.send_line(&format!("Shutdown scheduled in {mins} minute(s)."));
                        }
                        Err(e) => conn.send_line(&format!("Failed to schedule shutdown: {e}")),
                    }
                }
                Err(e) => conn.send_line(&format!("Invalid delay: {e}")),
            },
            Err(_) => conn.send_line("Usage: shutdown [now|<minutes>|cancel]"),
        },
    }
}
