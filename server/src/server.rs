use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;
use tokio::sync::{Mutex, Notify};

const LOGIN_READ_TIMEOUT: Duration = Duration::from_secs(60);
const MAX_LOGIN_LINE_LENGTH: usize = 256;

use crate::cmd::{AccessLevel, Command, CommandDispatch};
use crate::connection::{Connection, TelnetConnection};
use crate::game_loop::spawn_game_loop;
use crate::login::{LoginFlow, LoginState};
use crate::registry::ConnectionRegistry;
use crate::telnet::codec::TelnetReader;
use crate::telnet::INITIAL_NEGOTIATION;
use mud_core::templates::TemplateRegistry;
use mud_core::{Entity, Name, Position, World};

static SERVER_START: OnceLock<Instant> = OnceLock::new();
static MOTD: OnceLock<String> = OnceLock::new();
pub(crate) static DB: OnceLock<Arc<Mutex<mud_data::Database>>> = OnceLock::new();
pub(crate) static TEMPLATES: OnceLock<Arc<TemplateRegistry>> = OnceLock::new();
pub(crate) static WORLD: OnceLock<Arc<Mutex<World>>> = OnceLock::new();
pub(crate) static REGISTRY: OnceLock<Arc<Mutex<ConnectionRegistry>>> = OnceLock::new();

pub struct Server {
    bind_addr: String,
    world: Arc<Mutex<World>>,
    registry: Arc<Mutex<ConnectionRegistry>>,
    commands: CommandDispatch,
    next_conn_id: AtomicU64,
    void_room: Entity,
    db: Option<Arc<Mutex<mud_data::Database>>>,
    templates: Option<Arc<TemplateRegistry>>,
    shutdown_complete: Arc<Notify>,
}

impl Server {
    pub fn new(bind_addr: impl Into<String>, world: World, void_room: Entity) -> Self {
        Server {
            bind_addr: bind_addr.into(),
            world: Arc::new(Mutex::new(world)),
            registry: Arc::new(Mutex::new(ConnectionRegistry::new())),
            commands: CommandDispatch::new(),
            next_conn_id: AtomicU64::new(1),
            void_room,
            db: None,
            templates: None,
            shutdown_complete: Arc::new(Notify::new()),
        }
    }

    pub fn with_database(mut self, db: mud_data::Database) -> Self {
        let db = Arc::new(Mutex::new(db));
        let _ = DB.set(db.clone());
        self.db = Some(db);
        self
    }

    pub fn with_templates(mut self, templates: TemplateRegistry) -> Self {
        let templates = Arc::new(templates);
        let _ = TEMPLATES.set(templates.clone());
        self.templates = Some(templates);
        self
    }

    pub fn register_command(
        &mut self,
        name: &'static str,
        aliases: &'static [&'static str],
        access: AccessLevel,
        handler: crate::cmd::CommandFn,
    ) {
        self.commands.register(Command {
            name,
            aliases,
            access,
            handler,
        });
    }

    pub async fn run(
        self,
        mut shutdown: tokio::sync::watch::Receiver<bool>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let listener = TcpListener::bind(&self.bind_addr).await?;
        tracing::info!("Server listening on {}", self.bind_addr);

        let world = self.world;
        let registry = self.registry;
        let commands = Arc::new(self.commands);
        let void_room = self.void_room;
        let db = self.db;
        let templates = self.templates;
        let shutdown_complete = self.shutdown_complete;

        // Set statics for console access
        let _ = WORLD.set(world.clone());
        let _ = REGISTRY.set(registry.clone());

        // Spawn the game loop for combat/AI/corpse pulses
        let server_shutdown_rx = shutdown.clone();
        spawn_game_loop(world.clone(), db.clone(), server_shutdown_rx);

        loop {
            tokio::select! {
                biased;
                _ = shutdown.changed() => {
                    tracing::info!("Shutdown signal received");
                    break;
                }
                accept = listener.accept() => {
                    let (stream, addr) = accept?;
                    tracing::info!("New connection from {addr}");

                    let conn_id = self.next_conn_id.fetch_add(1, Ordering::SeqCst);
                    let world = world.clone();
                    let registry = registry.clone();
                    let commands = commands.clone();
                    let db = db.clone();

                    let templates = templates.clone();
                    tokio::spawn(async move {
                        handle_connection(conn_id, stream, world, registry, commands, void_room, db, templates).await;
                    });
                }
            }
        }

        tracing::info!("All connections closed");
        shutdown_complete.notify_one();

        Ok(())
    }

    pub async fn wait_shutdown(&self) {
        self.shutdown_complete.notified().await;
    }
}

#[allow(clippy::too_many_arguments)]
async fn handle_connection(
    conn_id: u64,
    stream: tokio::net::TcpStream,
    world: Arc<Mutex<World>>,
    registry: Arc<Mutex<ConnectionRegistry>>,
    commands: Arc<CommandDispatch>,
    void_room: Entity,
    db: Option<Arc<Mutex<mud_data::Database>>>,
    templates: Option<Arc<TemplateRegistry>>,
) {
    let (reader_half, mut writer_half) = stream.into_split();

    let (tx, mut output_rx) = tokio::sync::mpsc::unbounded_channel();
    let mut conn = TelnetConnection::new_with_tx(conn_id, tx);
    let mut login_flow = LoginFlow::new();

    let output_handle = tokio::spawn(async move {
        if let Err(e) = writer_half.write_all(&INITIAL_NEGOTIATION).await {
            tracing::debug!("Connection {conn_id} write error: {e}");
            return;
        }
        while let Some(bytes) = output_rx.recv().await {
            if let Err(e) = writer_half.write_all(&bytes).await {
                tracing::debug!("Connection {conn_id} write error: {e}");
                break;
            }
        }
    });

    // Show server banner + MOTD + stats, then prompt for login — all before read loop
    {
        let reg = registry.lock().await;
        send_server_greeting(&mut conn, &reg);
    }
    conn.send_line("Enter your username:");
    login_flow.state = LoginState::Username;

    let telnet_reader = TelnetReader::new(reader_half);
    let mut buf_reader = BufReader::new(telnet_reader);
    let mut line = String::new();

    loop {
        line.clear();
        let is_login_state = !login_flow.state().is_playing();
        let read_result = if is_login_state {
            match tokio::time::timeout(LOGIN_READ_TIMEOUT, buf_reader.read_line(&mut line)).await {
                Ok(result) => result,
                Err(_elapsed) => {
                    conn.send_line("\r\nTimed out waiting for input.");
                    conn.disconnect();
                    break;
                }
            }
        } else {
            buf_reader.read_line(&mut line).await
        };
        match read_result {
            Ok(0) => {
                tracing::info!("Connection {conn_id} closed");
                break;
            }
            Ok(_) => {
                let trimmed = line.trim_end_matches(&['\r', '\n'][..]);
                if is_login_state && trimmed.len() > MAX_LOGIN_LINE_LENGTH {
                    conn.send_line("\r\nInput too long.");
                    login_flow.strikes += 1;
                    if login_flow.strikes >= 3 {
                        conn.send_line("Too many failed attempts. Disconnecting.");
                        conn.disconnect();
                        break;
                    }
                    continue;
                }
                tracing::debug!("Connection {conn_id}: {trimmed}");

                if login_flow.state().is_playing() {
                    let mut world_lock = world.lock().await;
                    let reg = registry.lock().await;
                    commands.execute(&mut world_lock, &mut conn, trimmed, &reg);
                    conn.send("> ");
                    drop(reg);
                    drop(world_lock);
                } else {
                    let db_clone = db.clone();
                    let mut w = world.lock().await;
                    let mut reg = registry.lock().await;

                    let msgs = login_flow
                        .handle_input(
                            trimmed,
                            db_clone.as_deref(),
                            templates.as_deref(),
                            &mut w,
                            &mut reg,
                            void_room,
                        )
                        .await;

                    let echo = login_flow.take_echo();
                    if echo {
                        conn.set_echo(true);
                    } else {
                        conn.set_echo(false);
                    }

                    for msg in &msgs {
                        conn.send_line(msg);
                    }

                    if login_flow.take_entity_just_spawned() {
                        if let Some(entity) = login_flow.entity() {
                            conn.set_entity(entity);
                            if let Some(tx) = conn.output_sender() {
                                reg.register(entity, tx);
                            }
                        }
                    }

                    if login_flow.take_disconnect() {
                        conn.disconnect();
                        drop(reg);
                        drop(w);
                        break;
                    }

                    let prompt_msgs = login_flow
                        .show_state_prompt(db_clone.as_deref(), templates.as_deref())
                        .await;
                    for msg in &prompt_msgs {
                        conn.send_line(msg);
                    }

                    drop(reg);
                    drop(w);
                }
            }
            Err(e) => {
                tracing::debug!("Connection {conn_id} read error: {e}");
                break;
            }
        }
    }

    // Player cleanup: broadcast departure, unregister, despawn
    {
        let mut w = world.lock().await;
        let mut reg = registry.lock().await;

        if let Some(entity) = conn.entity() {
            let name = w
                .query_one::<&Name>(entity)
                .ok()
                .and_then(|mut q| q.get().cloned())
                .unwrap_or(Name::new("Someone"));

            let room = w
                .query_one::<&Position>(entity)
                .ok()
                .and_then(|mut q| q.get().map(|p| p.room));

            if let Some(room) = room {
                use mud_core::format::{conventions, RichText, Segment};
                let mut msg = RichText::new();
                msg.push(conventions::player_name_segment(name.as_str()));
                msg.push(Segment::new(" has disconnected."));
                reg.broadcast_to_room(&w, room, &msg.render(true, true), Some(entity));
            }

            reg.unregister(entity);
            let _ = w.despawn(entity).inspect_err(|e| {
                tracing::warn!("Failed to despawn entity {entity:?}: {e}");
            });
        }
    }

    let _ = output_handle.await;
}

// ---------------------------------------------------------------------------
// Server greeting — banner, uptime, player count
// ---------------------------------------------------------------------------

fn format_uptime() -> String {
    let elapsed = SERVER_START.get_or_init(Instant::now).elapsed();
    let total_secs = elapsed.as_secs();
    let days = total_secs / 86400;
    let hours = (total_secs % 86400) / 3600;
    let minutes = (total_secs % 3600) / 60;
    let seconds = total_secs % 60;
    format!("Uptime: {days}d {hours}h {minutes}m {seconds}s")
}

fn send_server_greeting(conn: &mut dyn Connection, registry: &ConnectionRegistry) {
    conn.send_line("");
    conn.send_line(" __  __ _   _ ____");
    conn.send_line("|  \\/  | | | |  _ \\");
    conn.send_line("| |\\/| | | | | | | |");
    conn.send_line("| |  | | |_| | |_| |");
    conn.send_line("|_|  |_|\\___/|____/");
    conn.send_line("");
    let motd = MOTD.get_or_init(|| "Welcome to the MUD. A world awaits.".to_string());
    conn.send_line(motd);
    conn.send_line("");
    conn.send_line(&format!(
        "{}  |  Players connected: {}",
        format_uptime(),
        registry.player_count()
    ));
    conn.send_line("");
}

// ---------------------------------------------------------------------------
// XP and Level-up
// ---------------------------------------------------------------------------

/// Grant XP to a player entity, checking for level-ups.
pub fn award_xp(world: &mut World, entity: Entity) {
    let level = get_level(world, entity);
    let xp = get_experience(world, entity);

    let threshold = mud_core::Experience::for_level(level + 1);
    if xp < threshold {
        return;
    }

    let db = DB.get().and_then(|d| d.try_lock().ok());
    let conn_db = db.as_ref().map(|g| g.conn());

    let mut messages: Vec<String> = Vec::new();

    loop {
        let current_level = get_level(world, entity);
        let current_xp = get_experience(world, entity);
        let next_threshold = mud_core::Experience::for_level(current_level + 1);
        if current_xp < next_threshold {
            break;
        }

        let new_level = current_level + 1;
        let excess = current_xp - next_threshold;

        // HP gain: hit die + CON mod
        let attrs = get_attributes(world, entity);
        let con_mod = (attrs.constitution as i32 - 10) / 2;
        let hit_die = get_hit_die();

        // Update components
        if let Ok(mut q) = world.query_one::<&mut mud_core::Health>(entity) {
            if let Some(health) = q.get() {
                let hp_gain = (hit_die + con_mod).max(1);
                health.max += hp_gain;
                health.current = health.max; // Full heal on level-up
            }
        }

        if let Ok(mut q) = world.query_one::<&mut mud_core::Level>(entity) {
            if let Some(level) = q.get() {
                level.0 = new_level;
            }
        }

        if let Ok(mut q) = world.query_one::<&mut mud_core::Experience>(entity) {
            if let Some(xp) = q.get() {
                xp.0 = excess;
            }
        }

        // Persist to DB
        if let Some(conn_db) = conn_db {
            if let Ok(mut q) = world.query_one::<&mud_core::DbId>(entity) {
                if let Some(db_id) = q.get() {
                    let _ = mud_data::save_level_component(conn_db, db_id.0, new_level as i64);
                    let _ = mud_data::save_experience_component(conn_db, db_id.0, excess as i64);
                }
            }
        }

        // Attribute point every 5 levels
        let attr_msg = if new_level % 5 == 0 {
            " You gain an attribute point!"
        } else {
            ""
        };

        messages.push(format!(
            "You advance to level {new_level}! HP increased by {}.{attr_msg}",
            (hit_die + con_mod).max(1),
        ));
    }

    if !messages.is_empty() {
        if let Ok(mut q) = world.query_one::<&mut mud_core::Health>(entity) {
            if let Some(health) = q.get() {
                health.current = health.max; // Ensure full heal
            }
        }
    }
}

fn get_level(world: &World, entity: Entity) -> u8 {
    world
        .query_one::<&mud_core::Level>(entity)
        .ok()
        .and_then(|mut q| q.get().map(|l| l.0))
        .unwrap_or(1)
}

fn get_experience(world: &World, entity: Entity) -> u64 {
    world
        .query_one::<&mud_core::Experience>(entity)
        .ok()
        .and_then(|mut q| q.get().map(|x| x.0))
        .unwrap_or(0)
}

fn get_attributes(world: &World, entity: Entity) -> mud_core::Attributes {
    world
        .query_one::<&mud_core::Attributes>(entity)
        .ok()
        .and_then(|mut q| q.get().cloned())
        .unwrap_or_default()
}

fn get_hit_die() -> i32 {
    TEMPLATES
        .get()
        .and_then(|t| t.classes.values().next().map(|c| c.hit_die as i32))
        .unwrap_or(8)
}

// ---------------------------------------------------------------------------
// MOTD loading
// ---------------------------------------------------------------------------

/// Load the message-of-the-day from a file, or fall back to the built-in
/// default. Safe to call multiple times — only the first call takes effect.
pub fn load_motd(path: Option<&Path>) {
    let _ = MOTD.get_or_init(|| {
        if let Some(path) = path {
            if let Ok(text) = fs::read_to_string(path) {
                let trimmed = text.trim().to_string();
                if !trimmed.is_empty() {
                    return trimmed;
                }
            }
        }
        "Welcome to the MUD. A world awaits.".to_string()
    });
}

/// Returns the message of the day text.
pub fn get_motd() -> &'static str {
    MOTD.get_or_init(|| "Welcome to the MUD. A world awaits.".to_string())
}

// ---------------------------------------------------------------------------
// Server console accessors
// ---------------------------------------------------------------------------

/// Returns a clone of the DB handle, if initialized.
pub fn get_db() -> Option<Arc<Mutex<mud_data::Database>>> {
    DB.get().cloned()
}

/// Returns a clone of the template registry, if initialized.
pub fn get_templates() -> Option<Arc<TemplateRegistry>> {
    TEMPLATES.get().cloned()
}

/// Returns a clone of the world handle, if initialized.
pub fn get_world() -> Option<Arc<Mutex<World>>> {
    WORLD.get().cloned()
}

/// Returns a clone of the connection registry, if initialized.
pub fn get_registry() -> Option<Arc<Mutex<ConnectionRegistry>>> {
    REGISTRY.get().cloned()
}

/// Broadcast a message to all connected players from the server console.
pub async fn console_broadcast(message: &str) -> usize {
    let registry = match REGISTRY.get() {
        Some(r) => r,
        None => return 0,
    };

    let reg = registry.lock().await;

    let bytes = format!("[Server] {}\r\n", message).into_bytes();
    let entities: Vec<Entity> = reg.connected_entities();
    tracing::debug!(
        count = entities.len(),
        "console_broadcast: connected entities"
    );
    let mut sent = 0;
    for entity in entities {
        match reg.sender(entity) {
            Some(tx) => match tx.send(bytes.clone()) {
                Ok(()) => sent += 1,
                Err(e) => tracing::warn!(?entity, error = %e, "console_broadcast: send failed"),
            },
            None => tracing::warn!(?entity, "console_broadcast: entity has no sender"),
        }
    }
    sent
}
