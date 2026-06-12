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
    let (world, void_room) = init_world();

    let mut server = Server::new(config.bind_addr(), world, void_room);

    server.register_command("look", &["l"], AccessLevel::Player, commands::cmd_look);
    server.register_command("say", &[], AccessLevel::Player, commands::cmd_say);
    server.register_command("help", &["h", "?"], AccessLevel::Player, commands::cmd_help);
    server.register_command("quit", &["exit"], AccessLevel::Player, commands::cmd_quit);

    let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);

    tokio::spawn(async move {
        signals::shutdown_signal().await;
        tracing::info!("Shutdown requested");
        let _ = shutdown_tx.send(true);
    });

    server.run(shutdown_rx).await
}
