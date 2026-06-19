mod commands;
mod config;
mod console;
mod init;
mod signals;
mod templates;

use config::Config;
use init::{init_world, spawn_area};
use mud_server::{AccessLevel, Server};
use std::path::Path;

const HELP_LOOK: &str = r#"Examine your surroundings, a specific target, or a direction.

Examples:
  look                  look around the room
  look goblin           examine a mob in the room
  look sword            examine an item in the room or inventory
  look north            peek through an exit
  look at goblin        same as 'look goblin'"#;

const HELP_HELP: &str = r#"List all commands or show details for a specific command.

Examples:
  help                  show all available commands
  help look             show details for the look command
  help l                works with aliases
  help ki               works with partial names"#;

const HELP_TRAIN: &str = r#"View your skills and spend skill points to increase ranks.

Examples:
  train                 show your skills and unspent points
  train list            list trainable skills
  train swords          increase rank in the swords skill"#;

const HELP_STANCE: &str = r#"View or change your combat stance.

Examples:
  stance                show your current stance
  stance normal         return to a balanced stance
  stance defensive      reduce damage taken at the cost of offense
  stance aggressive     deal more damage at the cost of defense
  stance berserk        maximum offense, minimal defense"#;

const HELP_WIDTH: &str = r#"View or set your terminal width for text wrapping.

Examples:
  width                 show your current setting
  width 80              set to 80 columns
  width 0               disable wrapping (unlimited)"#;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let config = Config::parse();
    mud_server::load_motd(config.motd_path.as_deref());

    let db = mud_data::Database::open(&config.db_path).unwrap_or_else(|e| {
        panic!(
            "Failed to open database at {}: {e}",
            config.db_path.display()
        )
    });

    let (mut world, void_room) = init_world();

    let content_path = config
        .motd_path
        .as_ref()
        .and_then(|p| Path::new(p).parent())
        .unwrap_or_else(|| Path::new("content"));

    let templates = templates::load_templates(content_path);
    tracing::info!("Loaded {} race(s)", templates.races.len());
    tracing::info!("Loaded {} class(es)", templates.classes.len());
    tracing::info!("Loaded {} item(s)", templates.items.len());
    tracing::info!("Loaded {} mob(s)", templates.mobs.len());
    tracing::info!("Loaded {} area(s)", templates.areas.len());
    tracing::info!(
        "Loaded {} room(s)",
        templates
            .areas
            .values()
            .map(|a| a.rooms.len())
            .sum::<usize>()
    );
    tracing::info!("Loaded {} shop(s)", templates.shops.len());
    tracing::info!("Loaded {} skill(s)", templates.skills.len());
    tracing::info!("Loaded {} stance(s)", templates.stances.len());
    tracing::info!("Loaded {} set(s)", templates.sets.len());
    tracing::info!("Loaded {} affix(es)", templates.affixes.len());
    tracing::info!("Loaded {} passive(s)", templates.passives.len());

    // Spawn all areas into the ECS world
    let spawn_room = {
        let mut entry_room = void_room;
        for area in templates.areas.values() {
            let room = spawn_area(&mut world, area, &templates);
            if entry_room == void_room {
                entry_room = room;
            }
            let sub_count = templates
                .areas
                .keys()
                .filter(|id| id.starts_with(&format!("{}.", area.id)))
                .count();
            if sub_count > 0 {
                tracing::info!(
                    "Spawned area '{}' with {} room(s), {} sub-area(s); spawn room: {}",
                    area.name,
                    area.rooms.len(),
                    sub_count,
                    area.spawn_room
                );
            } else {
                tracing::info!(
                    "Spawned area '{}' with {} room(s); spawn room: {}",
                    area.name,
                    area.rooms.len(),
                    area.spawn_room
                );
            }
        }
        entry_room
    };

    let commands_cmd_look = commands::cmd_look;
    let mut server = Server::new(config.bind_addr(), world, void_room)
        .with_database(db)
        .with_templates(templates)
        .with_spawn_room(spawn_room)
        .with_on_entity_spawned(move |world, conn, registry| {
            commands_cmd_look(world, conn, "", "", registry);
        });

    server.register_command(
        "look",
        &["l"],
        AccessLevel::Player,
        "General",
        HELP_LOOK,
        commands::cmd_look,
    );
    server.register_command(
        "say",
        &[],
        AccessLevel::Player,
        "Communication",
        "Speak aloud in the room",
        commands::cmd_say,
    );
    server.register_command(
        "score",
        &[],
        AccessLevel::Player,
        "Character",
        "Display your character stats",
        commands::cmd_score,
    );
    server.register_command(
        "motd",
        &[],
        AccessLevel::Player,
        "General",
        "Show the message of the day",
        commands::cmd_motd,
    );
    server.register_command(
        "help",
        &["h", "?"],
        AccessLevel::Player,
        "General",
        HELP_HELP,
        commands::cmd_help,
    );
    server.register_command(
        "who",
        &[],
        AccessLevel::Player,
        "General",
        "List connected players",
        commands::cmd_who,
    );
    server.register_command(
        "quit",
        &["exit"],
        AccessLevel::Player,
        "General",
        "Disconnect from the game",
        commands::cmd_quit,
    );
    server.register_command(
        "width",
        &[],
        AccessLevel::Player,
        "General",
        HELP_WIDTH,
        commands::cmd_width,
    );

    // Phase 3 — Combat
    server.register_command(
        "kill",
        &[],
        AccessLevel::Player,
        "Combat",
        "Attack a target",
        commands::cmd_kill,
    );
    server.register_command(
        "flee",
        &[],
        AccessLevel::Player,
        "Combat",
        "Attempt to flee from combat",
        commands::cmd_flee,
    );
    server.register_command(
        "inventory",
        &["inv", "i"],
        AccessLevel::Player,
        "Items",
        "List your carried items",
        commands::cmd_inventory,
    );
    server.register_command(
        "equipment",
        &["eq"],
        AccessLevel::Player,
        "Items",
        "Show what you are wearing and wielding",
        commands::cmd_equipment,
    );
    server.register_command(
        "wear",
        &[],
        AccessLevel::Player,
        "Items",
        "Wear a piece of armor",
        commands::cmd_wear,
    );
    server.register_command(
        "wield",
        &[],
        AccessLevel::Player,
        "Items",
        "Wield a weapon",
        commands::cmd_wield,
    );
    server.register_command(
        "remove",
        &[],
        AccessLevel::Player,
        "Items",
        "Remove an equipped item",
        commands::cmd_remove,
    );
    server.register_command(
        "examine",
        &["exa"],
        AccessLevel::Player,
        "Items",
        "Inspect an item or target",
        commands::cmd_examine,
    );
    server.register_command(
        "get",
        &["take"],
        AccessLevel::Player,
        "Items",
        "Pick up an item",
        commands::cmd_get,
    );
    server.register_command(
        "drop",
        &[],
        AccessLevel::Player,
        "Items",
        "Drop an item",
        commands::cmd_drop,
    );
    server.register_command(
        "put",
        &[],
        AccessLevel::Player,
        "Items",
        "Put an item into a container",
        commands::cmd_put,
    );
    server.register_command(
        "give",
        &[],
        AccessLevel::Player,
        "Items",
        "Give an item to someone",
        commands::cmd_give,
    );
    server.register_command(
        "loot",
        &[],
        AccessLevel::Player,
        "Items",
        "Take all items from a corpse",
        commands::cmd_loot,
    );
    server.register_command(
        "stance",
        &[],
        AccessLevel::Player,
        "Combat",
        HELP_STANCE,
        commands::cmd_stance,
    );
    server.register_command(
        "train",
        &[],
        AccessLevel::Player,
        "Character",
        HELP_TRAIN,
        commands::cmd_train,
    );

    // Builder commands
    server.register_command(
        "@award",
        &[],
        AccessLevel::Builder,
        "Builder",
        "Grant XP to a player",
        commands::cmd_award,
    );

    // Movement commands
    server.register_command(
        "north",
        &["n"],
        AccessLevel::Player,
        "Movement",
        "Move north",
        commands::cmd_move,
    );
    server.register_command(
        "south",
        &["s"],
        AccessLevel::Player,
        "Movement",
        "Move south",
        commands::cmd_move,
    );
    server.register_command(
        "east",
        &["e"],
        AccessLevel::Player,
        "Movement",
        "Move east",
        commands::cmd_move,
    );
    server.register_command(
        "west",
        &["w"],
        AccessLevel::Player,
        "Movement",
        "Move west",
        commands::cmd_move,
    );
    server.register_command(
        "up",
        &["u"],
        AccessLevel::Player,
        "Movement",
        "Move up",
        commands::cmd_move,
    );
    server.register_command(
        "down",
        &["d"],
        AccessLevel::Player,
        "Movement",
        "Move down",
        commands::cmd_move,
    );
    server.register_command(
        "northeast",
        &["ne"],
        AccessLevel::Player,
        "Movement",
        "Move northeast",
        commands::cmd_move,
    );
    server.register_command(
        "northwest",
        &["nw"],
        AccessLevel::Player,
        "Movement",
        "Move northwest",
        commands::cmd_move,
    );
    server.register_command(
        "southeast",
        &["se"],
        AccessLevel::Player,
        "Movement",
        "Move southeast",
        commands::cmd_move,
    );
    server.register_command(
        "southwest",
        &["sw"],
        AccessLevel::Player,
        "Movement",
        "Move southwest",
        commands::cmd_move,
    );

    let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);

    // Spawn OS signal handler
    let signal_shutdown = shutdown_tx.clone();
    tokio::spawn(async move {
        signals::shutdown_signal().await;
        tracing::info!("Shutdown requested");
        let _ = signal_shutdown.send(true);
    });

    // Spawn server console (stdin reader)
    let console_shutdown = shutdown_tx.clone();
    tokio::spawn(async move {
        console::run_console(console_shutdown).await;
    });

    let _ = server.run(shutdown_rx).await;

    // Exit explicitly to kill residual blocking threads (e.g. stdin reader in
    // the console task). All graceful cleanup (connections, DB flush) already
    // happened inside Server::run().
    std::process::exit(0);
}
