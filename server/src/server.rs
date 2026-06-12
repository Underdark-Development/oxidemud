use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;
use tokio::sync::{Mutex, Notify};

use crate::cmd::{AccessLevel, Command, CommandDispatch};
use crate::connection::{Connection, ConnectionState, TelnetConnection};
use crate::registry::ConnectionRegistry;
use crate::telnet::{codec::TelnetReader, INITIAL_NEGOTIATION};
use mud_core::{Entity, Name, Position, World};

pub struct Server {
    bind_addr: String,
    world: Arc<Mutex<World>>,
    registry: Arc<Mutex<ConnectionRegistry>>,
    commands: CommandDispatch,
    next_conn_id: AtomicU64,
    void_room: Entity,
    db: Option<Arc<Mutex<mud_data::Database>>>,
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
            shutdown_complete: Arc::new(Notify::new()),
        }
    }

    pub fn with_database(mut self, db: mud_data::Database) -> Self {
        self.db = Some(Arc::new(Mutex::new(db)));
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

                    tokio::spawn(async move {
                        handle_connection(conn_id, stream, world, registry, commands, void_room, db).await;
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

async fn handle_connection(
    conn_id: u64,
    stream: tokio::net::TcpStream,
    world: Arc<Mutex<World>>,
    registry: Arc<Mutex<ConnectionRegistry>>,
    commands: Arc<CommandDispatch>,
    void_room: Entity,
    db: Option<Arc<Mutex<mud_data::Database>>>,
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
                use mud_core::format::{render, Color, StyledText, Text};
                let mut msg = Text::new();
                msg.push(StyledText::colored(name.as_str(), Color::Red));
                msg.push(StyledText::new(" has disconnected."));
                reg.broadcast_to_room(&w, room, &render(&msg), Some(entity));
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
                    // Username is already stored in the state variant for the confirm step,
                    // but we need it in subsequent steps too. Store it in the create buffer.
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

            // Store plaintext password temporarily in the buffer for confirmation step
            conn.create_buffer().race = Some(password.to_string());
            conn.send_line("Confirm password:");
            conn.set_state(ConnectionState::AccountCreateConfirmPassword);
            Ok(())
        }

        ConnectionState::AccountCreateConfirmPassword => {
            let confirm = input.trim();
            let stored_password = conn.create_buffer().race.as_deref().map(|s| s.to_string());
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

            // Passwords match — hash and create account
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
                conn.create_buffer().race = None;
                return Ok(());
            }

            mud_data::create_account(db_guard.conn(), username, &hash)
                .map_err(|e| format!("Account creation error: {e}"))?;
            drop(db_guard);

            conn.create_buffer().name = None;
            conn.create_buffer().race = None;

            conn.send_line("Account created! Please log in.");
            conn.set_state(ConnectionState::Username);
            Ok(())
        }

        ConnectionState::CharacterSelect => {
            skip_login(conn, world, registry, void_room);
            Ok(())
        }

        ConnectionState::Playing => Ok(()),
    }
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
    conn.send_line("You are in the void. Type 'help' for commands.");
    conn.set_state(ConnectionState::Playing);
}

fn go_to_character_select(conn: &mut dyn Connection) {
    conn.send_line("");
    conn.send_line("Character selection is not yet implemented.");
    conn.send_line("Type 'help' for commands (playing as guest).");
    conn.set_state(ConnectionState::CharacterSelect);
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
