mod commands;
mod config;
mod init;
mod signals;

use config::Config;
use init::init_world;
use mud_server::{AccessLevel, Server};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let config = Config::parse();

    let db = mud_data::Database::open(&config.db_path).unwrap_or_else(|e| {
        panic!(
            "Failed to open database at {}: {e}",
            config.db_path.display()
        )
    });

    let (world, void_room) = init_world();

    let mut server = Server::new(config.bind_addr(), world, void_room).with_database(db);

    server.register_command("look", &["l"], AccessLevel::Player, commands::cmd_look);
    server.register_command("say", &[], AccessLevel::Player, commands::cmd_say);
    server.register_command("help", &["h", "?"], AccessLevel::Player, commands::cmd_help);
    server.register_command("quit", &["exit"], AccessLevel::Player, commands::cmd_quit);

    // Movement commands
    server.register_command("north", &["n"], AccessLevel::Player, commands::cmd_move);
    server.register_command("south", &["s"], AccessLevel::Player, commands::cmd_move);
    server.register_command("east", &["e"], AccessLevel::Player, commands::cmd_move);
    server.register_command("west", &["w"], AccessLevel::Player, commands::cmd_move);
    server.register_command("up", &["u"], AccessLevel::Player, commands::cmd_move);
    server.register_command("down", &["d"], AccessLevel::Player, commands::cmd_move);
    server.register_command(
        "northeast",
        &["ne"],
        AccessLevel::Player,
        commands::cmd_move,
    );
    server.register_command(
        "northwest",
        &["nw"],
        AccessLevel::Player,
        commands::cmd_move,
    );
    server.register_command(
        "southeast",
        &["se"],
        AccessLevel::Player,
        commands::cmd_move,
    );
    server.register_command(
        "southwest",
        &["sw"],
        AccessLevel::Player,
        commands::cmd_move,
    );

    let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);

    tokio::spawn(async move {
        signals::shutdown_signal().await;
        tracing::info!("Shutdown requested");
        let _ = shutdown_tx.send(true);
    });

    server.run(shutdown_rx).await
}
