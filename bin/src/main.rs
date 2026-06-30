mod commands;
mod config;
mod console;
mod init;
mod signals;
mod templates;

use config::Config;
use init::{init_world, spawn_area};
use oxide_server::{AccessLevel, Server};
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

const HELP_TRAIN: &str = r#"Spend practice points to increase your core attributes.

Examples:
  train                 show your attributes and cost to train
  train strength        increase your strength by 1"#;

const HELP_PRACTICE: &str = r#"View your skills and spend practice points to increase ranks.

Examples:
  practice              show your skills and unspent points
  practice list         list trainable skills
  practice swords       increase rank in the swords skill"#;

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

const HELP_PROMPT: &str = r#"View or change your prompt template.

Type 'prompt <template>' to set a custom prompt. Available variables:

  %h  %H    current / max hit points
  %m  %M    current / max mana
  %v  %V    current / max vitality (stamina)
  %l        level
  %x        total experience
  %X        experience to next level
  %n        character name
  %g        total wealth (copper)
  %a        alignment
  %r        room name
  %e        exits
  %s  %d    strength / dexterity
  %i  %w    intelligence / wisdom
  %o  %u    constitution / charisma
  %R        rest state (Standing/Sitting/...)
  %C        combat state (In Combat / Not In Combat)
  %c        carriage return (newline)
  %%        literal percent sign

Example:
  prompt [%h/%H]          show just health
  prompt <%h/%Hhp %m/%Mmn>  ROM-style prompt
  prompt reset            revert to server default prompt"#;

const HELP_PRAY: &str = r#"Offer a prayer to your chosen deity to seek their favor and blessing.
Prayers trigger a deity-specific blessing and incur a cooldown.

Example:
  pray                  pray to your deity"#;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::parse();
    oxide_server::config::init(
        config
            .config_path
            .as_deref()
            .unwrap_or(Path::new("content/server.toml")),
    );
    oxide_server::load_motd(config.motd_path.as_deref());

    // Initialize custom rolling file logging + stdout
    let rolling_writer = std::sync::Arc::new(std::sync::Mutex::new(RollingFileWriter::new()?));
    let file_writer = TracingWriter {
        writer: rolling_writer.clone(),
    };

    let file_layer = tracing_subscriber::fmt::layer()
        .with_writer(move || file_writer.clone())
        .with_ansi(false);

    let stdout_layer = tracing_subscriber::fmt::layer().with_writer(std::io::stdout);

    use tracing_subscriber::prelude::*;
    tracing_subscriber::Registry::default()
        .with(stdout_layer)
        .with(file_layer)
        .init();

    // Initial prune of old logs
    let retention_days = oxide_server::config::get().logging.retention_days;
    oxide_server::config::prune_old_logs(retention_days);

    let log_path = rolling_writer.lock().unwrap().current_path.clone();
    tracing::info!(
        "Server logging initialized. Writing logs to: {}",
        log_path.display()
    );

    let db = oxide_data::Database::open(&config.db_path).unwrap_or_else(|e| {
        panic!(
            "Failed to open database at {}: {e}",
            config.db_path.display()
        )
    });

    let mut world = init_world();

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

    // Validate templates before spawning
    let errors = templates.validate();
    if !errors.is_empty() {
        for err in &errors {
            tracing::error!(
                "Validation error in {} '{}': {}",
                err.template_type,
                err.template_id,
                err.message
            );
        }
        panic!("Template validation failed — refusing to start");
    }

    // Spawn all areas into the ECS world
    for area in templates.areas.values() {
        spawn_area(&mut world, area, &templates);
        let sub_count = templates
            .areas
            .keys()
            .filter(|id| id.starts_with(&format!("{}.", area.id)))
            .count();
        if sub_count > 0 {
            tracing::info!(
                "Spawned area '{}' with {} room(s), {} sub-area(s)",
                area.name,
                area.rooms.len(),
                sub_count,
            );
        } else {
            tracing::info!(
                "Spawned area '{}' with {} room(s)",
                area.name,
                area.rooms.len(),
            );
        }
    }

    let commands_cmd_look = commands::cmd_look;

    // Instantiate and register ScriptEngine and MessageOutputBridge
    let script_engine = Box::new(oxide_scripting::ScriptEngine::new(
        content_path.join("scripts"),
    ));
    oxide_core::scripting::register_scripting_bridge(script_engine);
    oxide_core::scripting::register_message_bridge(Box::new(oxide_server::ServerMessageBridge));

    let mut server = Server::new(config.bind_addr(), world)
        .with_database(db)
        .with_templates(templates)
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
        &["stats"],
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
    server.register_command(
        "prompt",
        &[],
        AccessLevel::Player,
        "General",
        HELP_PROMPT,
        commands::cmd_prompt,
    );
    server.register_command(
        "pray",
        &[],
        AccessLevel::Player,
        "Character",
        HELP_PRAY,
        commands::cmd_pray,
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
    server.register_command(
        "practice",
        &[],
        AccessLevel::Player,
        "Character",
        HELP_PRACTICE,
        commands::cmd_practice,
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

    // MVP additions: Sit / Rest / Sleep / Wake / Stand
    server.register_command(
        "sit",
        &[],
        AccessLevel::Player,
        "Character",
        "Sit down to rest or look around",
        commands::cmd_sit,
    );
    server.register_command(
        "rest",
        &[],
        AccessLevel::Player,
        "Character",
        "Rest and recover health/mana/stamina faster",
        commands::cmd_rest,
    );
    server.register_command(
        "sleep",
        &[],
        AccessLevel::Player,
        "Character",
        "Go to sleep for maximum recovery rate",
        commands::cmd_sleep,
    );
    server.register_command(
        "wake",
        &[],
        AccessLevel::Player,
        "Character",
        "Wake up from sleep",
        commands::cmd_wake,
    );
    server.register_command(
        "stand",
        &[],
        AccessLevel::Player,
        "Character",
        "Stand up to allow movement and combat",
        commands::cmd_stand,
    );

    // MVP additions: Communications (tell, reply, shout, whisper)
    server.register_command(
        "tell",
        &[],
        AccessLevel::Player,
        "Communication",
        "Send a private message to another player",
        commands::cmd_tell,
    );
    server.register_command(
        "reply",
        &["r"],
        AccessLevel::Player,
        "Communication",
        "Reply to the last player who messaged you",
        commands::cmd_reply,
    );
    server.register_command(
        "shout",
        &[],
        AccessLevel::Player,
        "Communication",
        "Shout a message to the entire zone",
        commands::cmd_shout,
    );
    server.register_command(
        "whisper",
        &[],
        AccessLevel::Player,
        "Communication",
        "Whisper a message to someone in the same room",
        commands::cmd_whisper,
    );

    // MVP additions: Doors (open, close, lock, unlock)
    server.register_command(
        "open",
        &[],
        AccessLevel::Player,
        "Movement",
        "Open a closed door",
        commands::cmd_open,
    );
    server.register_command(
        "close",
        &[],
        AccessLevel::Player,
        "Movement",
        "Close an open door",
        commands::cmd_close,
    );
    server.register_command(
        "lock",
        &[],
        AccessLevel::Player,
        "Movement",
        "Lock a door using a key",
        commands::cmd_lock,
    );
    server.register_command(
        "unlock",
        &[],
        AccessLevel::Player,
        "Movement",
        "Unlock a door using a key",
        commands::cmd_unlock,
    );
    server.register_command(
        "use",
        &[],
        AccessLevel::Player,
        "Skills",
        "Use a skill or spell",
        commands::cmd_use,
    );
    server.register_command(
        "cast",
        &[],
        AccessLevel::Player,
        "Skills",
        "Cast a spell (equivalent to use)",
        commands::cmd_use,
    );

    // MVP additions: Ghost & Revival (reclaim, revive, toggle)
    server.register_command(
        "die",
        &[],
        AccessLevel::Player,
        "Character",
        "Choose to submit to death when unconscious to instantly respawn as a ghost",
        commands::cmd_die,
    );
    server.register_command(
        "reclaim",
        &[],
        AccessLevel::Player,
        "Character",
        "Reclaim your corpse to return to life with your items",
        commands::cmd_reclaim,
    );
    server.register_command(
        "revive",
        &[],
        AccessLevel::Player,
        "Character",
        "Pray at an altar or reclaim your corpse to return to life",
        commands::cmd_revive,
    );
    server.register_command(
        "toggle",
        &[],
        AccessLevel::Player,
        "Character",
        "Toggle player settings (e.g., 'toggle resurrect')",
        commands::cmd_toggle,
    );

    // MVP additions: Info (time, weather)
    server.register_command(
        "time",
        &[],
        AccessLevel::Player,
        "General",
        "Check the current game time",
        commands::cmd_time,
    );
    server.register_command(
        "weather",
        &[],
        AccessLevel::Player,
        "General",
        "Check the current weather conditions",
        commands::cmd_weather,
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

struct RollingFileWriter {
    current_day: u32,
    file: std::fs::File,
    temp_dir: std::path::PathBuf,
    pub current_path: std::path::PathBuf,
}

impl RollingFileWriter {
    pub fn new() -> std::io::Result<Self> {
        use chrono::Datelike;
        let temp_dir = std::env::temp_dir();
        let now = chrono::Local::now();
        let timestamp = now.format("%Y%m%d_%H%M%S").to_string();
        let filename = format!("oxide_server_log_{}.log", timestamp);
        let path = temp_dir.join(filename);
        let file = std::fs::File::create(&path)?;
        Ok(Self {
            current_day: now.day(),
            file,
            temp_dir,
            current_path: path,
        })
    }

    pub fn write_log(&mut self, buf: &[u8]) -> std::io::Result<()> {
        use chrono::Datelike;
        use std::io::Write;
        let now = chrono::Local::now();
        let day = now.day();
        if day != self.current_day {
            let timestamp = now.format("%Y%m%d_%H%M%S").to_string();
            let filename = format!("oxide_server_log_{}.log", timestamp);
            let path = self.temp_dir.join(filename);
            self.file = std::fs::File::create(&path)?;
            self.current_path = path;
            self.current_day = day;
        }
        self.file.write_all(buf)?;
        self.file.flush()?;
        Ok(())
    }
}

#[derive(Clone)]
struct TracingWriter {
    writer: std::sync::Arc<std::sync::Mutex<RollingFileWriter>>,
}

impl std::io::Write for TracingWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let mut guard = self.writer.lock().unwrap();
        guard.write_log(buf)?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
