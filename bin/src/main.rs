mod commands;
mod config;
mod console;
mod init;
mod signals;
mod templates;

use config::Config;
use init::{init_world, spawn_area};
use oxide_server::Server;
use std::path::Path;

fn pluralize(count: usize, singular: &str, plural: &str) -> String {
    if count == 1 {
        format!("{} {}", count, singular)
    } else {
        format!("{} {}", count, plural)
    }
}

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
    oxide_server::load_banner(config.banner_path.as_deref());

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
    tracing::info!(
        "Loaded {}",
        pluralize(templates.races.len(), "race", "races")
    );
    tracing::info!(
        "Loaded {}",
        pluralize(templates.classes.len(), "class", "classes")
    );
    tracing::info!(
        "Loaded {}",
        pluralize(templates.items.len(), "item", "items")
    );
    tracing::info!("Loaded {}", pluralize(templates.mobs.len(), "mob", "mobs"));
    tracing::info!(
        "Loaded {}",
        pluralize(templates.areas.len(), "area", "areas")
    );
    tracing::info!(
        "Loaded {}",
        pluralize(
            templates
                .areas
                .values()
                .map(|a| a.rooms.len())
                .sum::<usize>(),
            "room",
            "rooms"
        )
    );
    tracing::info!(
        "Loaded {}",
        pluralize(templates.shops.len(), "shop", "shops")
    );
    tracing::info!(
        "Loaded {}",
        pluralize(templates.skills.len(), "skill", "skills")
    );
    tracing::info!(
        "Loaded {}",
        pluralize(templates.stances.len(), "stance", "stances")
    );
    tracing::info!("Loaded {}", pluralize(templates.sets.len(), "set", "sets"));
    tracing::info!(
        "Loaded {}",
        pluralize(templates.affixes.len(), "affix", "affixes")
    );
    tracing::info!(
        "Loaded {}",
        pluralize(templates.passives.len(), "passive", "passives")
    );

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
        let room_plural = if area.rooms.len() == 1 {
            "room"
        } else {
            "rooms"
        };
        if sub_count > 0 {
            let sub_plural = if sub_count == 1 {
                "sub-area"
            } else {
                "sub-areas"
            };
            tracing::info!(
                "Spawned area '{}' with {} {}, {} {}",
                area.name,
                area.rooms.len(),
                room_plural,
                sub_count,
                sub_plural,
            );
        } else {
            tracing::info!(
                "Spawned area '{}' with {} {}",
                area.name,
                area.rooms.len(),
                room_plural,
            );
        }
    }

    let commands_cmd_look = commands::cmd_look;

    // Instantiate and register ScriptEngine and MessageOutputBridge
    let script_engine = Box::new(oxide_scripting::ScriptEngine::new(
        content_path.join("scripts"),
    ));
    oxide_scripting::register_award_xp_callback(oxide_server::award_xp);
    oxide_core::scripting::register_scripting_bridge(script_engine);
    oxide_core::scripting::register_message_bridge(Box::new(oxide_server::ServerMessageBridge));

    let mut server = Server::new(config.bind_addr(), world)
        .with_database(db)
        .with_content_path(content_path)
        .with_templates(templates)
        .with_on_entity_spawned(move |world, conn, registry| {
            commands_cmd_look(world, conn, "", "", registry);
        });

    commands::register_all_commands(&mut server);

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
    tracing::info!("Shutdown complete");

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
