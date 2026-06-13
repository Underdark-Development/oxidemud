use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::sync::OnceLock;
use std::time::Instant;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;
use tokio::sync::{Mutex, Notify};

use crate::cmd::{AccessLevel, Command, CommandDispatch};
use crate::connection::{Connection, ConnectionState, TelnetConnection};
use crate::registry::ConnectionRegistry;
use crate::telnet::{codec::TelnetReader, INITIAL_NEGOTIATION};
use mud_core::templates::TemplateRegistry;
use mud_core::{Entity, Name, Position, World};

static SERVER_START: OnceLock<Instant> = OnceLock::new();
static MOTD: OnceLock<String> = OnceLock::new();

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
        self.db = Some(Arc::new(Mutex::new(db)));
        self
    }

    pub fn with_templates(mut self, templates: TemplateRegistry) -> Self {
        self.templates = Some(Arc::new(templates));
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
    let _registry_tx = tx.clone();
    let mut conn = TelnetConnection::new_with_tx(conn_id, tx);

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

    // Send initial welcome — will be followed by login prompt
    conn.send_line("Welcome to Mud!");
    conn.send_line("");

    let telnet_reader = TelnetReader::new(reader_half);
    let mut buf_reader = BufReader::new(telnet_reader);
    let mut line = String::new();

    loop {
        line.clear();
        match buf_reader.read_line(&mut line).await {
            Ok(0) => {
                tracing::info!("Connection {conn_id} closed");
                break;
            }
            Ok(_) => {
                let trimmed = line.trim_end_matches(&['\r', '\n'][..]);
                tracing::debug!("Connection {conn_id}: {trimmed}");

                if conn.state().is_playing() {
                    let mut world_lock = world.lock().await;
                    let reg = registry.lock().await;
                    commands.execute(&mut world_lock, &mut conn, trimmed, &reg);
                    drop(reg);
                    drop(world_lock);
                } else {
                    let db = db.clone();
                    let mut w = world.lock().await;
                    let mut reg = registry.lock().await;
                    let _ = handle_login(
                        &mut conn,
                        trimmed,
                        db.as_deref(),
                        templates.as_deref(),
                        &mut w,
                        &mut reg,
                        void_room,
                    )
                    .await;
                    drop(reg);
                    drop(w);
                }

                // If connection was disconnected (3 strikes or quit during login), break
                if conn.state() == ConnectionState::Connected && conn.strikes() >= 3 {
                    tracing::info!("Connection {conn_id} max login strikes reached");
                    break;
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
// Login handler — state machine for pre-game flows
// ---------------------------------------------------------------------------

async fn handle_login(
    conn: &mut dyn Connection,
    input: &str,
    db: Option<&Mutex<mud_data::Database>>,
    templates: Option<&TemplateRegistry>,
    world: &mut World,
    registry: &mut ConnectionRegistry,
    void_room: Entity,
) -> Result<(), String> {
    let state = conn.state();
    match state {
        ConnectionState::Connected => {
            conn.send_line("Enter your username (3-20 letters, numbers, hyphens, underscores):");
            conn.set_state(ConnectionState::Username);
            Ok(())
        }

        ConnectionState::Username => {
            let username = input.trim();
            if !is_valid_username(username) {
                conn.send_line(
                    "Invalid username. Use 3-20 letters, numbers, hyphens, or underscores.",
                );
                return Ok(());
            }

            let db = match db {
                Some(d) => d,
                None => {
                    skip_login(conn, world, registry, void_room);
                    return Ok(());
                }
            };

            let db_guard = db.lock().await;
            let existing = mud_data::get_account_by_username(db_guard.conn(), username)
                .map_err(|e| format!("DB error: {e}"))?;

            if let Some(_account) = existing {
                conn.send_line("Password:");
                let stored_username = Box::leak(username.to_string().into_boxed_str());
                conn.set_state(ConnectionState::Password {
                    username: stored_username,
                    attempts: 0,
                });
            } else {
                conn.send_line("That name isn't registered. Create a new account? (y/n)");
                let stored_username = Box::leak(username.to_string().into_boxed_str());
                conn.set_state(ConnectionState::AccountCreateConfirm {
                    username: stored_username,
                });
            }
            drop(db_guard);
            Ok(())
        }

        ConnectionState::Password { username, attempts } => {
            if input.is_empty() {
                conn.send_line("Password cannot be empty.");
                conn.set_strikes(conn.strikes() + 1);
                check_strikes(conn);
                return Ok(());
            }

            let db = match db {
                Some(d) => d,
                None => {
                    skip_login(conn, world, registry, void_room);
                    return Ok(());
                }
            };

            let db_guard = db.lock().await;
            let account = mud_data::get_account_by_username(db_guard.conn(), username)
                .map_err(|e| format!("DB error: {e}"))?
                .ok_or_else(|| "Account vanished".to_string())?;

            let valid = mud_data::verify_password(input.trim(), &account.password_hash)
                .map_err(|e| format!("Password verify error: {e}"))?;

            if valid {
                mud_data::update_last_login(db_guard.conn(), account.id)
                    .map_err(|e| format!("DB error: {e}"))?;
                drop(db_guard);

                conn.set_account_id(account.id);
                conn.send_line(&format!("Welcome back, {}!", account.username));
                go_to_character_select(conn);
            } else {
                drop(db_guard);
                let new_attempts = attempts + 1;
                if new_attempts >= 3 {
                    conn.send_line("Too many failed attempts. Disconnecting.");
                    conn.disconnect();
                } else {
                    conn.send_line(&format!("Invalid password. ({}/3 attempts)", new_attempts));
                    conn.set_strikes(conn.strikes() + 1);
                    let stored_username = Box::leak(username.to_string().into_boxed_str());
                    conn.set_state(ConnectionState::Password {
                        username: stored_username,
                        attempts: new_attempts,
                    });
                }
            }
            Ok(())
        }

        ConnectionState::AccountCreateConfirm { .. } => {
            match input.trim().to_lowercase().as_str() {
                "y" | "yes" => {
                    if let ConnectionState::AccountCreateConfirm { username } = conn.state() {
                        conn.create_buffer().name = Some(username.to_string());
                    }
                    conn.send_line("Choose a password (minimum 8 characters):");
                    conn.set_state(ConnectionState::AccountCreatePassword);
                }
                "n" | "no" => {
                    conn.send_line(
                        "Enter your username (3-20 letters, numbers, hyphens, underscores):",
                    );
                    conn.set_state(ConnectionState::Username);
                }
                _ => {
                    conn.send_line("Please answer y or n.");
                }
            }
            Ok(())
        }

        ConnectionState::AccountCreatePassword => {
            let password = input.trim();
            if password.len() < 8 {
                conn.send_line("Password must be at least 8 characters.");
                return Ok(());
            }

            let username = conn.create_buffer().name.as_deref().map(|s| s.to_string());
            if username.is_none() {
                conn.send_line("Session error. Starting over. Enter your username:");
                conn.set_state(ConnectionState::Username);
                return Ok(());
            }

            conn.create_buffer().password = Some(password.to_string());
            conn.send_line("Confirm password:");
            conn.set_state(ConnectionState::AccountCreateConfirmPassword);
            Ok(())
        }

        ConnectionState::AccountCreateConfirmPassword => {
            let confirm = input.trim();
            let stored_password = conn
                .create_buffer()
                .password
                .as_deref()
                .map(|s| s.to_string());
            let username = conn.create_buffer().name.as_deref().map(|s| s.to_string());

            if stored_password.is_none() || username.is_none() {
                conn.send_line("Session error. Starting over. Enter your username:");
                conn.set_state(ConnectionState::Username);
                return Ok(());
            }

            if confirm != stored_password.as_deref().unwrap() {
                conn.send_line("Passwords do not match. Try again.");
                conn.send_line("Choose a password (minimum 8 characters):");
                conn.set_state(ConnectionState::AccountCreatePassword);
                return Ok(());
            }

            let db = match db {
                Some(d) => d,
                None => {
                    skip_login(conn, world, registry, void_room);
                    return Ok(());
                }
            };

            let hash = mud_data::hash_password(stored_password.as_deref().unwrap())
                .map_err(|e| format!("Hashing error: {e}"))?;
            let username = username.as_deref().unwrap();

            let db_guard = db.lock().await;
            let existing = mud_data::get_account_by_username(db_guard.conn(), username)
                .map_err(|e| format!("DB error: {e}"))?;

            if existing.is_some() {
                conn.send_line("That username was taken while you were choosing a password. Starting over. Enter your username:");
                conn.set_state(ConnectionState::Username);
                conn.create_buffer().name = None;
                conn.create_buffer().password = None;
                return Ok(());
            }

            mud_data::create_account(db_guard.conn(), username, &hash)
                .map_err(|e| format!("Account creation error: {e}"))?;
            drop(db_guard);

            conn.create_buffer().name = None;
            conn.create_buffer().password = None;

            conn.send_line("Account created! Please log in.");
            conn.set_state(ConnectionState::Username);
            Ok(())
        }

        // -----------------------------------------------------------------------
        // Character selection — show existing characters or option to create
        // -----------------------------------------------------------------------
        ConnectionState::CharacterSelect => {
            let input = input.trim();
            let db = match db {
                Some(d) => d,
                None => {
                    skip_login(conn, world, registry, void_room);
                    return Ok(());
                }
            };

            let account_id = match conn.account_id() {
                Some(id) => id,
                None => {
                    conn.send_line("Session error. Please log in again.");
                    conn.set_state(ConnectionState::Username);
                    return Ok(());
                }
            };

            if input == "c" || input == "C" {
                conn.create_buffer().name = None;
                conn.create_buffer().race = None;
                conn.create_buffer().class = None;
                conn.send_line("");
                conn.send_line("--- Create a New Character ---");
                conn.send_line("Enter your character's name (3-16 letters, hyphens, apostrophes):");
                conn.set_state(ConnectionState::CharacterCreateName);
                return Ok(());
            }

            // Try to parse as a number (character selection)
            if let Ok(idx) = input.parse::<usize>() {
                let db_guard = db.lock().await;
                let chars = mud_data::get_characters_by_account(db_guard.conn(), account_id)
                    .map_err(|e| format!("DB error: {e}"))?;
                drop(db_guard);

                if idx == 0 || idx > chars.len() {
                    conn.send_line("Invalid selection. Pick a number from the list, or type 'c' to create a new character.");
                    return Ok(());
                }

                let char_row = &chars[idx - 1];
                load_character(conn, world, registry, void_room, char_row);
                return Ok(());
            }

            conn.send_line("Type a number to pick a character, or 'c' to create a new one.");
            Ok(())
        }

        // -----------------------------------------------------------------------
        // Character creation wizard
        // -----------------------------------------------------------------------
        ConnectionState::CharacterCreateName => {
            let name = input.trim();
            if !is_valid_character_name(name) {
                conn.send_line("Invalid name. Use 3-16 letters, hyphens, or apostrophes.");
                return Ok(());
            }

            let db = match db {
                Some(d) => d,
                None => {
                    conn.send_line("No database available for character creation.");
                    return Ok(());
                }
            };

            let db_guard = db.lock().await;
            let existing = mud_data::get_character_by_name(db_guard.conn(), name)
                .map_err(|e| format!("DB error: {e}"))?;
            drop(db_guard);

            if existing.is_some() {
                conn.send_line("That name is already taken. Please choose another.");
                return Ok(());
            }

            conn.create_buffer().name = Some(name.to_string());
            show_character_race_prompt(conn, templates);
            Ok(())
        }

        ConnectionState::CharacterCreateRace => {
            let templates = match templates {
                Some(t) => t,
                None => {
                    conn.send_line("No race templates available. Cannot create character.");
                    return Ok(());
                }
            };

            let input = input.trim();
            let races: Vec<&str> = templates.races.keys().map(|s| s.as_str()).collect();

            match input.parse::<usize>() {
                Ok(idx) if idx > 0 && idx <= races.len() => {
                    let race_id = races[idx - 1].to_string();
                    conn.create_buffer().race = Some(race_id);
                    show_character_class_prompt(conn, templates);
                }
                _ => {
                    conn.send_line(&format!("Pick a race by number (1-{}):", races.len()));
                }
            }
            Ok(())
        }

        ConnectionState::CharacterCreateClass => {
            let templates = match templates {
                Some(t) => t,
                None => {
                    conn.send_line("No class templates available. Cannot create character.");
                    return Ok(());
                }
            };

            let race_id = match conn.create_buffer().race.as_deref() {
                Some(r) => r.to_string(),
                None => {
                    conn.send_line("Session error. Starting over.");
                    conn.set_state(ConnectionState::CharacterCreateName);
                    return Ok(());
                }
            };

            let available = templates.available_classes_for_race(&race_id);
            let input = input.trim();

            match input.parse::<usize>() {
                Ok(idx) if idx > 0 && idx <= available.len() => {
                    let class_id = available[idx - 1].id.clone();
                    conn.create_buffer().class = Some(class_id);
                    show_character_confirm(conn, templates, &race_id);
                }
                _ => {
                    conn.send_line(&format!("Pick a class by number (1-{}):", available.len()));
                }
            }
            Ok(())
        }

        ConnectionState::CharacterCreateConfirm => {
            match input.trim().to_lowercase().as_str() {
                "y" | "yes" => {
                    finalize_character(conn, db, world, registry, void_room, templates).await;
                }
                "n" | "no" => {
                    conn.create_buffer().name = None;
                    conn.create_buffer().race = None;
                    conn.create_buffer().class = None;
                    conn.send_line("Character creation cancelled.");
                    go_to_character_select(conn);
                }
                _ => {
                    conn.send_line("Type 'y' to accept or 'n' to cancel.");
                }
            }
            Ok(())
        }

        ConnectionState::Playing => Ok(()),
    }
}

// ---------------------------------------------------------------------------
// Character creation helpers
// ---------------------------------------------------------------------------

fn show_character_race_prompt(conn: &mut dyn Connection, templates: Option<&TemplateRegistry>) {
    let templates = match templates {
        Some(t) => t,
        None => {
            conn.send_line("No race templates available. Cannot create character.");
            return;
        }
    };

    conn.send_line("");
    conn.send_line("--- Choose a Race ---");
    let mut races: Vec<(&str, &mud_core::templates::RaceTemplate)> = templates
        .races
        .iter()
        .map(|(k, v)| (k.as_str(), v))
        .collect();
    races.sort_by(|a, b| a.0.cmp(b.0));

    for (i, (_id, race)) in races.iter().enumerate() {
        conn.send_line(&format!("{}. {} — {}", i + 1, race.name, race.description));
    }
    conn.send_line(&format!("Pick a race by number (1-{}):", races.len()));
    conn.set_state(ConnectionState::CharacterCreateRace);
}

fn show_character_class_prompt(conn: &mut dyn Connection, templates: &TemplateRegistry) {
    let race_id = conn.create_buffer().race.as_deref().unwrap_or("");
    let available = templates.available_classes_for_race(race_id);

    conn.send_line("");
    conn.send_line("--- Choose a Class ---");

    for (i, class) in available.iter().enumerate() {
        conn.send_line(&format!(
            "{}. {} — {}",
            i + 1,
            class.name,
            class.description
        ));
    }
    conn.send_line(&format!("Pick a class by number (1-{}):", available.len()));
    conn.set_state(ConnectionState::CharacterCreateClass);
}

fn show_character_confirm(conn: &mut dyn Connection, templates: &TemplateRegistry, race_id: &str) {
    let (name, class_id) = {
        let buf = conn.create_buffer();
        (
            buf.name.clone().unwrap_or_else(|| "?".to_string()),
            buf.class.clone().unwrap_or_else(|| "?".to_string()),
        )
    };

    let race = templates.get_race(race_id);
    let class = templates.get_class(&class_id);

    let default_attrs = mud_core::templates::RaceAttributes::default();
    let default_mods = mud_core::templates::ClassAttributeMods::default();

    let (race_name, race_attrs) = race
        .map(|r| (r.name.as_str(), &r.attributes))
        .unwrap_or(("?", &default_attrs));

    let (class_name, class_mods) = class
        .map(|c| (c.name.as_str(), &c.attribute_mods))
        .unwrap_or(("?", &default_mods));

    let str = (race_attrs.strength as i16 + class_mods.strength as i16) as u8;
    let dex = (race_attrs.dexterity as i16 + class_mods.dexterity as i16) as u8;
    let int = (race_attrs.intelligence as i16 + class_mods.intelligence as i16) as u8;
    let wis = (race_attrs.wisdom as i16 + class_mods.wisdom as i16) as u8;
    let con = (race_attrs.constitution as i16 + class_mods.constitution as i16) as u8;
    let cha = (race_attrs.charisma as i16 + class_mods.charisma as i16) as u8;

    conn.send_line("");
    conn.send_line("--- Character Summary ---");
    conn.send_line(&format!("  Name:       {name}"));
    conn.send_line(&format!("  Race:       {race_name}"));
    conn.send_line(&format!("  Class:      {class_name}"));
    conn.send_line(&format!(
        "  Attributes: STR {str}, DEX {dex}, INT {int}, WIS {wis}, CON {con}, CHA {cha}"
    ));
    if let Some(r) = race {
        if !r.racial_abilities.is_empty() {
            conn.send_line(&format!("  Abilities:  {}", r.racial_abilities.join(", ")));
        }
    }
    if let Some(c) = class {
        if !c.auto_skills.is_empty() {
            conn.send_line(&format!("  Skills:     {}", c.auto_skills.join(", ")));
        }
    }
    conn.send_line("");
    conn.send_line("Accept this character? (y/n)");
    conn.set_state(ConnectionState::CharacterCreateConfirm);
}

async fn finalize_character(
    conn: &mut dyn Connection,
    db: Option<&Mutex<mud_data::Database>>,
    world: &mut World,
    registry: &mut ConnectionRegistry,
    void_room: Entity,
    templates: Option<&TemplateRegistry>,
) {
    let name = match conn.create_buffer().name.as_deref() {
        Some(n) => n.to_string(),
        None => {
            conn.send_line("Session error. Starting over.");
            go_to_character_select(conn);
            return;
        }
    };

    let race_id = match conn.create_buffer().race.as_deref() {
        Some(r) => r.to_string(),
        None => {
            conn.send_line("Session error. Starting over.");
            go_to_character_select(conn);
            return;
        }
    };

    let class_id = match conn.create_buffer().class.as_deref() {
        Some(c) => c.to_string(),
        None => {
            conn.send_line("Session error. Starting over.");
            go_to_character_select(conn);
            return;
        }
    };

    let account_id = match conn.account_id() {
        Some(id) => id,
        None => {
            conn.send_line("Session error. Starting over.");
            go_to_character_select(conn);
            return;
        }
    };

    let db_con = match db {
        Some(d) => d,
        None => {
            skip_login(conn, world, registry, void_room);
            return;
        }
    };

    // Compute final attributes: race base + class mods
    let (attrs, hp, skills) = compute_character_stats(templates, &race_id, &class_id);

    // Persist to database
    let db_guard = db_con.lock().await;
    let conn_db = db_guard.conn();

    let entity_id = match mud_data::insert_entity(conn_db, "player") {
        Ok(id) => id,
        Err(e) => {
            conn.send_line(&format!("Error creating character: {e}"));
            return;
        }
    };

    if let Err(e) = mud_data::save_player_component(conn_db, entity_id, account_id, "<%hhp %hmhp> ")
    {
        conn.send_line(&format!("Error saving character: {e}"));
        return;
    }

    if let Err(e) = mud_data::save_attributes_component(
        conn_db,
        entity_id,
        &mud_data::AttributesRow {
            strength: attrs.strength,
            dexterity: attrs.dexterity,
            intelligence: attrs.intelligence,
            wisdom: attrs.wisdom,
            constitution: attrs.constitution,
            charisma: attrs.charisma,
        },
    ) {
        conn.send_line(&format!("Error saving attributes: {e}"));
        return;
    }

    if let Err(e) = mud_data::save_health_component(conn_db, entity_id, hp, hp) {
        conn.send_line(&format!("Error saving health: {e}"));
        return;
    }

    if let Err(e) = mud_data::save_level_component(conn_db, entity_id, 1) {
        conn.send_line(&format!("Error saving level: {e}"));
        return;
    }

    if let Err(e) = mud_data::save_experience_component(conn_db, entity_id, 0) {
        conn.send_line(&format!("Error saving experience: {e}"));
        return;
    }

    // Save position (0 = void room for now)
    if let Err(e) = mud_data::save_position_component(conn_db, entity_id, 0) {
        conn.send_line(&format!("Error saving position: {e}"));
        return;
    }

    if let Err(e) = mud_data::create_character(
        conn_db, account_id, &name, &race_id, &class_id, entity_id, 0,
    ) {
        conn.send_line(&format!("Error saving character: {e}"));
        return;
    }

    drop(db_guard);

    // Spawn ECS entity
    let player = world.spawn((
        Position::new(void_room),
        Name::new(name.clone()),
        mud_core::Player::new(account_id),
        attrs,
        mud_core::Health::new(hp),
        skills,
        mud_core::DbId::new(entity_id),
    ));

    conn.set_entity(player);
    if let Some(tx) = conn.output_sender() {
        registry.register(player, tx);
    }

    send_server_greeting(conn, registry);
    conn.send_line(&format!("Welcome, {name}! Your adventure begins."));
    conn.set_state(ConnectionState::Playing);
}

fn compute_character_stats(
    templates: Option<&TemplateRegistry>,
    race_id: &str,
    class_id: &str,
) -> (mud_core::Attributes, i32, mud_core::LearnedSkills) {
    let mut skills = mud_core::LearnedSkills::new();

    let (base_str, base_dex, base_int, base_wis, base_con, base_cha) = templates
        .and_then(|t| t.get_race(race_id))
        .map(|r| {
            for ability in &r.racial_abilities {
                skills.grant(ability);
            }
            (
                r.attributes.strength as i16,
                r.attributes.dexterity as i16,
                r.attributes.intelligence as i16,
                r.attributes.wisdom as i16,
                r.attributes.constitution as i16,
                r.attributes.charisma as i16,
            )
        })
        .unwrap_or((10, 10, 10, 10, 10, 10));

    let (mod_str, mod_dex, mod_int, mod_wis, mod_con, mod_cha, hit_die) = templates
        .and_then(|t| t.get_class(class_id))
        .map(|c| {
            for skill_id in &c.auto_skills {
                skills.grant(skill_id);
            }
            (
                c.attribute_mods.strength,
                c.attribute_mods.dexterity,
                c.attribute_mods.intelligence,
                c.attribute_mods.wisdom,
                c.attribute_mods.constitution,
                c.attribute_mods.charisma,
                c.hit_die,
            )
        })
        .unwrap_or((0, 0, 0, 0, 0, 0, 8));

    let attrs = mud_core::Attributes::new(
        (base_str + mod_str as i16).clamp(3, 50) as u8,
        (base_dex + mod_dex as i16).clamp(3, 50) as u8,
        (base_int + mod_int as i16).clamp(3, 50) as u8,
        (base_wis + mod_wis as i16).clamp(3, 50) as u8,
        (base_con + mod_con as i16).clamp(3, 50) as u8,
        (base_cha + mod_cha as i16).clamp(3, 50) as u8,
    );

    let hp = hit_die as i32 + (attrs.constitution as i32 - 10) / 2;

    (attrs, hp.max(1), skills)
}

/// Load a saved character from the database and spawn them into the world.
fn load_character(
    conn: &mut dyn Connection,
    world: &mut World,
    registry: &mut ConnectionRegistry,
    void_room: Entity,
    char_row: &mud_data::CharacterRow,
) {
    // For now, spawn in void room with basic components.
    // Full entity persistence (Chunk 4) will load all saved components.
    let player = world.spawn((
        Position::new(void_room),
        Name::new(char_row.name.clone()),
        mud_core::Player::new(char_row.account_id),
        mud_core::DbId::new(char_row.entity_id),
    ));

    conn.set_entity(player);
    if let Some(tx) = conn.output_sender() {
        registry.register(player, tx);
    }

    conn.send_line(&format!("Welcome back, {}!", char_row.name));
    send_server_greeting(conn, registry);
    conn.set_state(ConnectionState::Playing);
}

/// Skip login and spawn a guest player in the void.
fn skip_login(
    conn: &mut dyn Connection,
    world: &mut World,
    registry: &mut ConnectionRegistry,
    void_room: Entity,
) {
    let name = format!("Adventurer_{}", conn.id());
    let player = world.spawn((Position::new(void_room), Name::new(name)));
    conn.set_entity(player);
    if let Some(tx) = conn.output_sender() {
        registry.register(player, tx);
    }
    send_server_greeting(conn, registry);
    conn.set_state(ConnectionState::Playing);
}

fn go_to_character_select(conn: &mut dyn Connection) {
    conn.send_line("");
    conn.send_line("--- Character Selection ---");

    // We can't query DB from here (no db access in this fn), so show generic prompt.
    // The DB query happens in the CharacterSelect handler when the user presses enter.
    conn.send_line("Press Enter to see your characters.");
    conn.set_state(ConnectionState::CharacterSelect);
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

fn check_strikes(conn: &mut dyn Connection) {
    if conn.strikes() >= 3 {
        conn.send_line("Too many failed attempts. Disconnecting.");
        conn.disconnect();
    }
}

// ---------------------------------------------------------------------------
// Input validation
// ---------------------------------------------------------------------------

/// Validates a username: 3-20 chars, alphanumeric plus hyphens and underscores.
fn is_valid_username(s: &str) -> bool {
    if s.len() < 3 || s.len() > 20 {
        return false;
    }
    s.chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// Validates a character name: 3-16 chars, letters, hyphens, apostrophes.
/// Must start and end with a letter.
pub fn is_valid_character_name(s: &str) -> bool {
    if !(3..=16).contains(&s.len()) {
        return false;
    }
    let mut chars = s.chars();
    let first = chars.next().unwrap();
    let last = chars.last().unwrap_or(first);
    if !first.is_ascii_alphabetic() || !last.is_ascii_alphabetic() {
        return false;
    }
    s.chars()
        .all(|c| c.is_ascii_alphabetic() || c == '-' || c == '\'')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_valid_username() {
        assert!(is_valid_username("alice"));
        assert!(is_valid_username("bob42"));
        assert!(is_valid_username("a_b_c")); // underscore allowed
        assert!(is_valid_username("cool-dude")); // hyphen allowed
        assert!(!is_valid_username("ab")); // too short
        assert!(!is_valid_username("a".repeat(21).as_str())); // too long
        assert!(!is_valid_username("alice!")); // special char
        assert!(!is_valid_username("")); // empty
    }

    #[test]
    fn test_is_valid_character_name() {
        assert!(is_valid_character_name("Alice"));
        assert!(is_valid_character_name("Bob-Smith"));
        assert!(is_valid_character_name("O'Brien"));
        assert!(!is_valid_character_name("Ab")); // too short
        assert!(!is_valid_character_name("A".repeat(17).as_str())); // too long
        assert!(!is_valid_character_name("alice@")); // special char
        assert!(!is_valid_character_name("-alice")); // starts with hyphen
        assert!(!is_valid_character_name("alice-")); // ends with hyphen
        assert!(!is_valid_character_name("")); // empty
        assert!(!is_valid_character_name("123")); // starts with digit
        assert!(!is_valid_character_name("al_ice")); // underscore not allowed
    }
}
