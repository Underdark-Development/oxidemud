mod config;
mod init;
mod signals;

use config::Config;
use init::init_world;
use mud_server::{AccessLevel, Connection, Server};

fn cmd_look(world: &mut mud_core::World, conn: &mut dyn Connection, _args: &str) {
    let entity = match conn.entity() {
        Some(e) => e,
        None => {
            conn.send_line("You have no form yet.");
            return;
        }
    };

    let mut q_pos = match world.query_one::<&mud_core::Position>(entity) {
        Ok(q) => q,
        Err(_) => {
            conn.send_line("You are nowhere.");
            return;
        }
    };

    let pos = match q_pos.get() {
        Some(p) => p,
        None => {
            conn.send_line("You are nowhere.");
            return;
        }
    };

    let mut q_room = match world.query_one::<&mud_core::Room>(pos.room) {
        Ok(q) => q,
        Err(_) => {
            conn.send_line("The void stares back.");
            return;
        }
    };

    let room = match q_room.get() {
        Some(r) => r,
        None => {
            conn.send_line("The void stares back.");
            return;
        }
    };

    conn.send_line("");
    conn.send_line(&room.name);
    conn.send_line(&"-".repeat(room.name.len().min(40)));
    conn.send_line(&room.description);
    conn.send_line("");
}

fn cmd_say(_world: &mut mud_core::World, conn: &mut dyn Connection, args: &str) {
    if args.is_empty() {
        conn.send_line("Say what?");
        return;
    }

    conn.send_line(&format!("You say, \"{}\"", args));
}

fn cmd_help(_world: &mut mud_core::World, conn: &mut dyn Connection, _args: &str) {
    conn.send_line("");
    conn.send_line("Available commands:");
    conn.send_line("  look/l          — examine your surroundings");
    conn.send_line("  say <text>       — speak in the room");
    conn.send_line("  help             — this help");
    conn.send_line("  quit             — disconnect");
    conn.send_line("");
}

fn cmd_quit(_world: &mut mud_core::World, conn: &mut dyn Connection, _args: &str) {
    conn.send_line("Goodbye!");
    conn.disconnect();
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let config = Config::parse();
    let (world, void_room) = init_world();

    let mut server = Server::new(config.bind_addr(), world, void_room);

    server.register_command("look", &["l"], AccessLevel::Player, cmd_look);
    server.register_command("say", &[], AccessLevel::Player, cmd_say);
    server.register_command("help", &["h", "?"], AccessLevel::Player, cmd_help);
    server.register_command("quit", &["exit"], AccessLevel::Player, cmd_quit);

    let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);

    // Spawn shutdown watcher
    tokio::spawn(async move {
        signals::shutdown_signal().await;
        tracing::info!("Shutdown requested");
        let _ = shutdown_tx.send(true);
    });

    server.run(shutdown_rx).await
}
