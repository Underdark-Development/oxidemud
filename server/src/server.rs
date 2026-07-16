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
use oxide_core::templates::TemplateRegistry;
use oxide_core::{
    Alignment, Attributes, DbId, Description, Entity, Equipment, Experience, Health, Inventory,
    LearnedSkills, Level, Name, Player, Position, PracticePoints, RoomKey, Wallet, World,
};

static SERVER_START: OnceLock<Instant> = OnceLock::new();
static MOTD: OnceLock<std::sync::RwLock<Option<String>>> = OnceLock::new();
static BANNER: OnceLock<std::sync::RwLock<Option<String>>> = OnceLock::new();
pub(crate) static DB: OnceLock<Arc<Mutex<oxide_data::Database>>> = OnceLock::new();
pub(crate) static TEMPLATES: OnceLock<std::sync::RwLock<Arc<TemplateRegistry>>> = OnceLock::new();
pub(crate) static WORLD: OnceLock<Arc<Mutex<World>>> = OnceLock::new();
pub(crate) static REGISTRY: OnceLock<Arc<Mutex<ConnectionRegistry>>> = OnceLock::new();
static COMMANDS: OnceLock<Arc<CommandDispatch>> = OnceLock::new();

pub type EntitySpawnedCb =
    dyn Fn(&mut World, &mut dyn Connection, &ConnectionRegistry) + Send + Sync;

pub struct Server {
    bind_addr: String,
    world: Arc<Mutex<World>>,
    registry: Arc<Mutex<ConnectionRegistry>>,
    commands: CommandDispatch,
    next_conn_id: AtomicU64,
    db: Option<Arc<Mutex<oxide_data::Database>>>,
    templates: Option<Arc<TemplateRegistry>>,
    content_path: Option<std::path::PathBuf>,
    shutdown_complete: Arc<Notify>,
    on_entity_spawned: Option<Arc<EntitySpawnedCb>>,
}

impl Server {
    pub fn new(bind_addr: impl Into<String>, world: World) -> Self {
        Server {
            bind_addr: bind_addr.into(),
            world: Arc::new(Mutex::new(world)),
            registry: Arc::new(Mutex::new(ConnectionRegistry::new())),
            commands: CommandDispatch::new(),
            next_conn_id: AtomicU64::new(1),
            db: None,
            templates: None,
            content_path: None,
            shutdown_complete: Arc::new(Notify::new()),
            on_entity_spawned: None,
        }
    }

    pub fn with_on_entity_spawned(
        mut self,
        cb: impl Fn(&mut World, &mut dyn Connection, &ConnectionRegistry) + Send + Sync + 'static,
    ) -> Self {
        self.on_entity_spawned = Some(Arc::new(cb));
        self
    }

    pub fn with_database(mut self, db: oxide_data::Database) -> Self {
        let db = Arc::new(Mutex::new(db));
        let _ = DB.set(db.clone());
        self.db = Some(db);
        self
    }

    pub fn with_content_path(mut self, path: impl Into<std::path::PathBuf>) -> Self {
        self.content_path = Some(path.into());
        self
    }

    pub fn with_templates(mut self, templates: TemplateRegistry) -> Self {
        let templates = Arc::new(templates);
        if let Some(lock) = TEMPLATES.get() {
            let mut guard = lock.write().unwrap_or_else(|e| e.into_inner());
            *guard = templates.clone();
        } else {
            let _ = TEMPLATES.set(std::sync::RwLock::new(templates.clone()));
        }
        self.templates = Some(templates);
        self
    }

    pub fn register_command(
        &mut self,
        name: &'static str,
        aliases: &'static [&'static str],
        access: AccessLevel,
        category: &'static str,
        help_text: &'static str,
        handler: crate::cmd::CommandFn,
    ) {
        self.commands.register(Command {
            name,
            aliases,
            access,
            category,
            help_text,
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
        let _ = COMMANDS.set(commands.clone());
        let db = self.db;
        let templates = self.templates;
        let shutdown_complete = self.shutdown_complete;

        // Set statics for console access
        let _ = WORLD.set(world.clone());
        let _ = REGISTRY.set(registry.clone());

        // Spawn the game loop for combat/AI/corpse pulses
        let server_shutdown_rx = shutdown.clone();
        spawn_game_loop(
            world.clone(),
            db.clone(),
            registry.clone(),
            server_shutdown_rx,
        );

        // Start the hot-reload file watcher if content_path is set
        if let Some(content_path) = self.content_path {
            let (tx, mut rx) = tokio::sync::mpsc::channel::<std::path::PathBuf>(100);

            // Spawn the debouncing & processing task
            let watcher_content_path = content_path.clone();
            tokio::spawn(async move {
                loop {
                    // Wait for an event
                    let first_path = match rx.recv().await {
                        Some(p) => p,
                        None => break,
                    };
                    // Wait 100ms for more events to bundle/debounce
                    tokio::time::sleep(Duration::from_millis(100)).await;
                    // Drain the channel
                    let mut has_scripts_change = false;
                    let mut has_templates_change = false;
                    let mut motd_changed_path: Option<std::path::PathBuf> = None;
                    let mut banner_changed_path: Option<std::path::PathBuf> = None;
                    let mut scripts_changed_files: std::collections::HashSet<String> =
                        std::collections::HashSet::new();

                    let mut process_path = |path: std::path::PathBuf| {
                        if let Some(path_str) = path.to_str() {
                            if path_str.contains("/scripts/") {
                                has_scripts_change = true;
                                if let Ok(rel) =
                                    path.strip_prefix(watcher_content_path.join("scripts"))
                                {
                                    if let Some(rel_str) = rel.to_str() {
                                        scripts_changed_files.insert(rel_str.to_string());
                                    }
                                }
                            } else if path.file_name().is_some_and(|name| name == "motd.txt") {
                                motd_changed_path = Some(path);
                            } else if path.file_name().is_some_and(|name| name == "banner.txt") {
                                banner_changed_path = Some(path);
                            } else {
                                has_templates_change = true;
                            }
                        }
                    };

                    process_path(first_path);

                    while let Ok(path) = rx.try_recv() {
                        process_path(path);
                    }

                    if has_scripts_change {
                        if let Some(bridge) = oxide_core::scripting::get_scripting_bridge() {
                            for rel_file in scripts_changed_files {
                                if let Err(e) = bridge.reload_script(&rel_file) {
                                    tracing::error!("Failed to reload script {}: {}", rel_file, e);
                                } else {
                                    tracing::info!(
                                        "Script hot-reloaded successfully: {}",
                                        rel_file
                                    );
                                }
                            }
                        }
                    }

                    if let Some(path) = motd_changed_path {
                        tracing::info!("Hot-reloading MOTD...");
                        crate::server::load_motd(Some(&path));
                    }

                    if let Some(path) = banner_changed_path {
                        tracing::info!("Hot-reloading welcome banner...");
                        crate::server::load_banner(Some(&path));
                    }

                    if has_templates_change {
                        tracing::info!("Hot-reloading content templates...");
                        let (new_registry, _) =
                            oxide_core::content::load_registry(&watcher_content_path);
                        let errors = new_registry.validate();
                        if !errors.is_empty() {
                            for err in &errors {
                                tracing::error!(
                                    "Validation error in {} '{}' during hot-reload: {}",
                                    err.template_type,
                                    err.template_id,
                                    err.message
                                );
                            }
                            tracing::error!(
                                "Template validation failed — keeping previous templates"
                            );
                        } else {
                            let swap_res = crate::server::update_templates(|registry| {
                                *registry = new_registry;
                            });
                            match swap_res {
                                Ok(_) => {
                                    tracing::info!("Content templates hot-reloaded successfully.");
                                    let _event = oxide_core::GameEvent::ContentReloaded;
                                }
                                Err(e) => {
                                    tracing::error!(
                                        "Failed to update template registry during hot-reload: {}",
                                        e
                                    );
                                }
                            }
                        }
                    }
                }
            });

            // Start the notify watcher
            let watcher_path = content_path.clone();
            std::thread::spawn(move || {
                use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};

                let event_handler = move |res: Result<notify::Event, notify::Error>| {
                    if let Ok(event) = res {
                        for path in event.paths {
                            if path
                                .extension()
                                .is_some_and(|ext| ext == "toml" || ext == "rhai" || ext == "txt")
                            {
                                let _ = tx.blocking_send(path);
                            }
                        }
                    }
                };

                if let Ok(mut watcher) = RecommendedWatcher::new(event_handler, Config::default()) {
                    if let Err(e) = watcher.watch(&watcher_path, RecursiveMode::Recursive) {
                        tracing::error!(
                            "Failed to start RecommendedWatcher on {:?}: {}",
                            watcher_path,
                            e
                        );
                    } else {
                        loop {
                            std::thread::sleep(Duration::from_secs(3600));
                        }
                    }
                }
            });
        }

        // Spawn the REST API server if enabled
        let api_config = crate::config::get().api.clone();
        let api_shutdown_rx = shutdown.clone();
        tokio::spawn(async move {
            if api_config.enabled {
                if let Err(e) = crate::api::start_api_server(api_config, api_shutdown_rx).await {
                    tracing::error!("REST API server error: {e}");
                }
            }
        });

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
                    let on_entity_spawned = self.on_entity_spawned.clone();
                    tokio::spawn(async move {
                        handle_connection(
                            conn_id, stream, world, registry, commands, db,
                            templates, on_entity_spawned,
                        )
                        .await;
                    });
                }
            }
        }

        tracing::info!("All connections closed");

        if let Some(ref db) = db {
            let db_guard = db.lock().await;
            let mut w = world.lock().await;
            crate::game_loop::save_online_players(&mut w, &db_guard, true);
            tracing::info!("Online player state saved");
        }

        shutdown_complete.notify_one();

        Ok(())
    }

    pub async fn wait_shutdown(&self) {
        self.shutdown_complete.notified().await;
    }
}

fn handle_negotiation(conn: &mut TelnetConnection, neg: crate::telnet::codec::Negotiation) {
    use crate::telnet::codec::NegotiationAction;
    use crate::telnet::constants;

    match neg.action {
        NegotiationAction::Will(constants::TERMINAL_TYPE) => {
            conn.send_raw(&[
                constants::IAC,
                constants::SB,
                constants::TERMINAL_TYPE,
                constants::TELQUAL_SEND,
                constants::IAC,
                constants::SE,
            ]);
        }
        NegotiationAction::Subneg(constants::TERMINAL_TYPE, params)
            if !params.is_empty() && params[0] == constants::TELQUAL_IS =>
        {
            if let Ok(term) = std::str::from_utf8(&params[1..]) {
                let term = term.trim().to_lowercase();
                conn.set_terminal_type(term);
            }
        }
        _ => {}
    }
}

#[allow(clippy::too_many_arguments)]
async fn handle_connection(
    conn_id: u64,
    stream: tokio::net::TcpStream,
    world: Arc<Mutex<World>>,
    registry: Arc<Mutex<ConnectionRegistry>>,
    commands: Arc<CommandDispatch>,
    db: Option<Arc<Mutex<oxide_data::Database>>>,
    templates: Option<Arc<TemplateRegistry>>,
    on_entity_spawned: Option<Arc<EntitySpawnedCb>>,
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
        let mut last_was_prompt = false;
        while let Some(mut bytes) = output_rx.recv().await {
            if bytes.starts_with(b"\x00\xFFPROMPT\x00") {
                if last_was_prompt {
                    if let Err(e) = writer_half.write_all(b"\r\n").await {
                        tracing::debug!("Connection {conn_id} write error: {e}");
                        break;
                    }
                }
                let prompt = bytes.split_off(8);
                if let Err(e) = writer_half.write_all(&prompt).await {
                    tracing::debug!("Connection {conn_id} write error: {e}");
                    break;
                }
                last_was_prompt = true;
            } else if bytes == b"\x00\xFFRESET\x00" {
                last_was_prompt = false;
            } else {
                if last_was_prompt {
                    if let Err(e) = writer_half.write_all(b"\r\n").await {
                        tracing::debug!("Connection {conn_id} write error: {e}");
                        break;
                    }
                    last_was_prompt = false;
                }
                if let Err(e) = writer_half.write_all(&bytes).await {
                    tracing::debug!("Connection {conn_id} write error: {e}");
                    break;
                }
            }
        }
    });

    // Show server banner + MOTD + stats, then prompt for login — all before read loop
    {
        let reg = registry.lock().await;
        let w = world.lock().await;
        send_server_greeting(&mut conn, &reg, &w);
    }
    conn.send_line("Enter your username:");
    login_flow.state = LoginState::Username;

    let telnet_reader = TelnetReader::new(reader_half);
    let mut buf_reader = BufReader::new(telnet_reader);
    let mut line = String::new();

    loop {
        line.clear();
        let is_login_state = !login_flow.state().is_playing();
        let is_pre_auth = login_flow.state().is_pre_auth();
        let read_result = if is_pre_auth {
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
                let negotiations = buf_reader.get_mut().take_negotiations();
                if !negotiations.is_empty() {
                    let mut world_lock = None;
                    for neg in negotiations {
                        if let crate::telnet::codec::NegotiationAction::Subneg(opt, ref params) =
                            neg.action
                        {
                            if opt == crate::telnet::constants::NAWS && params.len() >= 4 {
                                let width = ((params[0] as u16) << 8) | (params[1] as u16);
                                conn.set_screen_width(width);
                                if let Some(entity) = conn.entity() {
                                    if world_lock.is_none() {
                                        world_lock = Some(world.lock().await);
                                    }
                                    if let Some(ref mut w) = world_lock {
                                        if let Ok(mut q) = w.query_one::<&mut Player>(entity) {
                                            if let Some(player) = q.get() {
                                                player.screen_width = width;
                                            }
                                        }
                                        let _ = w.insert(entity, (oxide_core::Dirty,));
                                    }
                                }
                            } else {
                                handle_negotiation(&mut conn, neg);
                            }
                        } else {
                            handle_negotiation(&mut conn, neg);
                        }
                    }
                    drop(world_lock);
                }

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

                if login_flow.state().is_playing() {
                    if let Some(tx) = conn.output_sender() {
                        let _ = tx.send(b"\x00\xFFRESET\x00".to_vec());
                    }
                    let mut world_lock = world.lock().await;
                    let reg = registry.lock().await;
                    commands.execute(&mut world_lock, &mut conn, trimmed, &reg);
                    drop(reg);
                    if conn.is_disconnected() {
                        drop(world_lock);
                        break;
                    }
                    if let Some(entity) = conn.entity() {
                        let reg = registry.lock().await;
                        crate::prompt::send_player_prompt(&world_lock, entity, &reg);
                        drop(reg);
                    }
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

                            let player_db_id = w
                                .query_one::<&DbId>(entity)
                                .ok()
                                .and_then(|mut q| q.get().copied())
                                .map(|d| d.0)
                                .unwrap_or(0);
                            let player_name = w
                                .query_one::<&Name>(entity)
                                .ok()
                                .and_then(|mut q| q.get().map(|n| n.as_str().to_string()))
                                .unwrap_or_default();
                            oxide_core::handle_player_login_group(
                                &mut w,
                                entity,
                                player_db_id,
                                &player_name,
                            );

                            if let Ok(mut q) = w.query_one::<&Player>(entity) {
                                if let Some(player) = q.get() {
                                    conn.set_screen_width(player.screen_width);
                                }
                            }
                            if let Ok(mut q) = w.query_one::<&oxide_core::AccessLevel>(entity) {
                                if let Some(&level) = q.get() {
                                    conn.set_access_level(level);
                                }
                            }
                            if let Some(tx) = conn.output_sender() {
                                reg.register(entity, tx);
                            }
                        }
                        if let Some(ref cb) = on_entity_spawned {
                            cb(&mut w, &mut conn, &reg);
                        }
                        if let Some(entity) = login_flow.entity() {
                            crate::prompt::send_player_prompt(&w, entity, &reg);
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

    // Player cleanup: save progress, broadcast departure, unregister, despawn
    if let Some(entity) = conn.entity() {
        // 1. Extract player data for saving
        let player_save_data = {
            let w = world.lock().await;
            if let Some(db_id) = w
                .query_one::<&DbId>(entity)
                .ok()
                .and_then(|mut q| q.get().copied())
            {
                let level = w
                    .query_one::<&Level>(entity)
                    .ok()
                    .and_then(|mut q| q.get().copied());
                let xp = w
                    .query_one::<&Experience>(entity)
                    .ok()
                    .and_then(|mut q| q.get().copied());
                let health = w
                    .query_one::<&Health>(entity)
                    .ok()
                    .and_then(|mut q| q.get().cloned());
                let mana = w
                    .query_one::<&oxide_core::Mana>(entity)
                    .ok()
                    .and_then(|mut q| q.get().cloned());
                let stamina = w
                    .query_one::<&oxide_core::Stamina>(entity)
                    .ok()
                    .and_then(|mut q| q.get().cloned());
                let position = w
                    .query_one::<&Position>(entity)
                    .ok()
                    .and_then(|mut q| q.get().map(|p| p.room));
                let wallet = w
                    .query_one::<&Wallet>(entity)
                    .ok()
                    .and_then(|mut q| q.get().cloned());
                let skills = w
                    .query_one::<&LearnedSkills>(entity)
                    .ok()
                    .and_then(|mut q| q.get().cloned());
                let practice_points = w
                    .query_one::<&PracticePoints>(entity)
                    .ok()
                    .and_then(|mut q| q.get().copied());
                let player_comp = w
                    .query_one::<&Player>(entity)
                    .ok()
                    .and_then(|mut q| q.get().cloned());
                let attrs = w
                    .query_one::<&Attributes>(entity)
                    .ok()
                    .and_then(|mut q| q.get().cloned());
                let alignment = w
                    .query_one::<&Alignment>(entity)
                    .ok()
                    .and_then(|mut q| q.get().cloned());
                let description = w
                    .query_one::<&Description>(entity)
                    .ok()
                    .and_then(|mut q| q.get().cloned());

                let room_key = position.and_then(|room_entity| {
                    w.query_one::<&RoomKey>(room_entity)
                        .ok()
                        .and_then(|mut q| q.get().map(|sk| sk.0.clone()))
                });

                let mut inventory_items = Vec::new();
                if let Ok(mut q) = w.query_one::<&Inventory>(entity) {
                    if let Some(inv) = q.get() {
                        for &item_entity in &inv.0 {
                            if let Ok(mut item_q) =
                                w.query_one::<(&oxide_core::Item, Option<&DbId>)>(item_entity)
                            {
                                if let Some((item, opt_db_id)) = item_q.get() {
                                    inventory_items
                                        .push((item.template_id.clone(), opt_db_id.map(|d| d.0)));
                                }
                            }
                        }
                    }
                }

                let mut equipment_items = Vec::new();
                if let Ok(mut q) = w.query_one::<&Equipment>(entity) {
                    if let Some(eq) = q.get() {
                        for &(slot, item_entity) in &eq.slots {
                            if let Ok(mut item_q) =
                                w.query_one::<(&oxide_core::Item, Option<&DbId>)>(item_entity)
                            {
                                if let Some((item, opt_db_id)) = item_q.get() {
                                    equipment_items.push((
                                        slot,
                                        item.template_id.clone(),
                                        opt_db_id.map(|d| d.0),
                                    ));
                                }
                            }
                        }
                    }
                }

                Some((
                    db_id,
                    level,
                    xp,
                    health,
                    mana,
                    stamina,
                    position,
                    room_key,
                    wallet,
                    skills,
                    practice_points,
                    player_comp,
                    attrs,
                    alignment,
                    description,
                    inventory_items,
                    equipment_items,
                ))
            } else {
                None
            }
        };

        // 2. Save player progress to DB while not holding world lock
        if let Some((
            db_id,
            level,
            xp,
            health,
            mana,
            stamina,
            _room_entity,
            room_key,
            wallet,
            skills,
            practice_points,
            player_comp,
            attrs,
            alignment,
            description,
            inventory_items,
            equipment_items,
        )) = player_save_data
        {
            if let Some(ref db) = db {
                let db_guard = db.lock().await;
                let conn_db = db_guard.conn();

                // Save Level & XP
                if let Some(level) = level {
                    let _ = oxide_data::save_level_component(conn_db, db_id.0, level.0 as i64);
                    let xp_val = xp.map(|x| x.0).unwrap_or(0);
                    let _ = oxide_data::update_character_level(
                        conn_db,
                        db_id.0,
                        level.0.into(),
                        xp_val as i64,
                    );
                }
                if let Some(xp) = xp {
                    let _ = oxide_data::save_experience_component(conn_db, db_id.0, xp.0 as i64);
                }
                // Save Health
                if let Some(health) = health {
                    let _ = oxide_data::save_health_component(
                        conn_db,
                        db_id.0,
                        health.current,
                        health.max,
                    );
                }
                // Save Mana
                if let Some(mana) = mana {
                    let _ = oxide_data::save_mana_component(conn_db, db_id.0, mana.current as i32);
                }
                // Save Stamina
                if let Some(stamina) = stamina {
                    let _ = oxide_data::save_stamina_component(
                        conn_db,
                        db_id.0,
                        stamina.current as i32,
                    );
                }
                // Save Wallet
                if let Some(wallet) = wallet {
                    let _ = oxide_data::save_golds_component(
                        conn_db,
                        db_id.0,
                        wallet.copper as i64,
                        wallet.silver as i64,
                        wallet.gold as i64,
                        wallet.platinum as i64,
                    );
                }
                // Save LearnedSkills
                if let Some(skills) = skills {
                    if let Err(e) = oxide_data::save_skills(conn_db, db_id.0, &skills.skills) {
                        tracing::error!(entity_id = db_id.0, error = %e, "disconnect: failed to save skills");
                    }
                }

                // Save PracticePoints
                if let Some(pp) = practice_points {
                    let _ = oxide_data::save_practice_points(conn_db, db_id.0, pp.0 as i64);
                }

                // Save Player component
                if let Some(ref player_comp) = player_comp {
                    if let Err(e) = oxide_data::save_player_component(
                        conn_db,
                        db_id.0,
                        player_comp.account_id,
                        player_comp.prompt.as_deref(),
                        player_comp.screen_width,
                    ) {
                        tracing::error!(entity_id = db_id.0, error = %e, "disconnect: failed to save player component");
                    } else {
                        // Readback verify
                        match oxide_data::load_player_component(conn_db, db_id.0) {
                            Ok(Some((_, loaded_prompt, _))) => {
                                tracing::debug!(
                                    entity_id = db_id.0,
                                    saved_prompt = ?player_comp.prompt,
                                    loaded_prompt = ?loaded_prompt,
                                    "disconnect: player component readback verified"
                                );
                            }
                            Ok(None) => {
                                tracing::error!(
                                    entity_id = db_id.0,
                                    "disconnect: player component not found after save"
                                );
                            }
                            Err(e) => {
                                tracing::error!(entity_id = db_id.0, error = %e, "disconnect: readback failed");
                            }
                        }
                    }
                }
                // Save Attributes
                if let Some(attrs) = attrs {
                    let _ = oxide_data::save_attributes_component(
                        conn_db,
                        db_id.0,
                        &oxide_data::AttributesRow {
                            strength: attrs.strength,
                            dexterity: attrs.dexterity,
                            intelligence: attrs.intelligence,
                            wisdom: attrs.wisdom,
                            constitution: attrs.constitution,
                            charisma: attrs.charisma,
                        },
                    );
                }
                // Save Alignment
                if let Some(alignment) = alignment {
                    let _ = oxide_data::save_alignment_component(conn_db, db_id.0, &alignment.0);
                }
                // Save Description
                if let Some(description) = description {
                    let _ =
                        oxide_data::save_description_component(conn_db, db_id.0, &description.0);
                }
                // Save Position (room key)
                if let Some(ref key) = room_key {
                    let _ = oxide_data::update_character_current_room_key(conn_db, db_id.0, key);
                    let _ = oxide_data::update_character_last_seen(conn_db, db_id.0);
                }

                // Save Inventory
                let _ = oxide_data::delete_all_inventory(conn_db, db_id.0);
                for (slot_idx, (template_id, opt_db_id)) in inventory_items.into_iter().enumerate()
                {
                    let item_db_id = match opt_db_id {
                        Some(id) => id,
                        None => {
                            if let Ok(new_id) = oxide_data::insert_entity(conn_db, "item") {
                                new_id
                            } else {
                                continue;
                            }
                        }
                    };
                    let _ = oxide_data::save_item_component(conn_db, item_db_id, &template_id);
                    let _ = oxide_data::add_inventory_item(
                        conn_db,
                        db_id.0,
                        item_db_id,
                        slot_idx as i32,
                    );
                }

                // Save Equipment
                let _ = oxide_data::delete_all_equipment(conn_db, db_id.0);
                for (slot, template_id, opt_db_id) in equipment_items {
                    let item_db_id = match opt_db_id {
                        Some(id) => id,
                        None => {
                            if let Ok(new_id) = oxide_data::insert_entity(conn_db, "item") {
                                new_id
                            } else {
                                continue;
                            }
                        }
                    };
                    let _ = oxide_data::save_item_component(conn_db, item_db_id, &template_id);
                    let slot_str = format!("{:?}", slot).to_lowercase();
                    let _ =
                        oxide_data::save_equipment_slot(conn_db, db_id.0, &slot_str, item_db_id);
                }
            }
        }

        // 3. Lock world and registry again to remove connection and despawn
        {
            let mut w = world.lock().await;
            let mut reg = registry.lock().await;

            // Despawn inventory and equipment items
            let mut items_to_despawn = Vec::new();
            if let Ok(mut q) = w.query_one::<&Inventory>(entity) {
                if let Some(inv) = q.get() {
                    items_to_despawn.extend(inv.0.iter().copied());
                }
            }
            if let Ok(mut q) = w.query_one::<&Equipment>(entity) {
                if let Some(eq) = q.get() {
                    for &(_, item) in &eq.slots {
                        items_to_despawn.push(item);
                    }
                }
            }
            for item in items_to_despawn {
                let _ = w.despawn(item);
            }

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
                use oxide_core::format::{conventions, RichText, Segment};
                let mut msg = RichText::new();
                msg.push(conventions::player_name_segment(name.as_str()));
                msg.push(Segment::new(" has disconnected."));
                reg.broadcast_to_room(&w, room, &msg.render(true, true), Some(entity));
            }

            reg.unregister(entity);
            oxide_core::handle_player_disconnect_group(&mut w, entity);
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

fn send_server_greeting(conn: &mut dyn Connection, registry: &ConnectionRegistry, world: &World) {
    let ansi = conn.flags().has(crate::connection::ConnectionFlag::Ansi);
    let allow_blink = conn.flags().has(crate::connection::ConnectionFlag::Blink);

    if let Some(banner) = get_banner() {
        conn.send_line("");
        let rich = oxide_core::format::parse_tags(&banner);
        for line in rich.render(ansi, allow_blink).lines() {
            conn.send_line(line);
        }
        conn.send_line("");
    }

    let config = crate::config::get();
    let version = format!("v{}", env!("CARGO_PKG_VERSION"));
    let mut server_id = config.server_name.clone();

    if let Some(ref ver) = config.server_version {
        let clean_ver = if ver.starts_with('v') || ver.starts_with('V') {
            ver.clone()
        } else {
            format!("v{}", ver)
        };
        server_id.push_str(&format!(" {}", clean_ver));
    }

    if let Some(ref url) = config.server_url {
        server_id.push_str(&format!(" ({})", url));
    }

    conn.send_line(&format!("{} - powered by OxideMUD {}", server_id, version));

    let uptime = format_uptime();
    let current_conns = registry.player_count();
    let max_conns = config.max_clients;
    let room_count = world.query::<&oxide_core::RoomKey>().iter().count();
    let mob_count = world.query::<&oxide_core::Npc>().iter().count();
    let item_count = world.query::<&oxide_core::Item>().iter().count();

    conn.send_line(&format!(
        "{} | Players Online: {}/{} | Rooms: {} | Mobs: {} | Items: {}",
        uptime, current_conns, max_conns, room_count, mob_count, item_count
    ));
    conn.send_line("");

    if let Some(motd) = get_motd() {
        let rich = oxide_core::format::parse_tags(&motd);
        conn.send_line(&rich.render(ansi, allow_blink));
        conn.send_line("");
    }
}

// ---------------------------------------------------------------------------
// XP and Level-up
// ---------------------------------------------------------------------------

/// Grant XP to a player entity, checking for level-ups.
/// Returns level-up messages to be sent to the player.
pub fn award_xp(world: &mut World, entity: Entity) -> Vec<String> {
    let level = get_level(world, entity);
    let xp = get_experience(world, entity);

    let threshold = oxide_core::Experience::for_level(level + 1);
    if xp < threshold {
        return Vec::new();
    }

    if world
        .query_one::<&oxide_core::MultiClassInfo>(entity)
        .is_ok_and(|mut q| q.get().is_some())
    {
        return vec![format!(
            "\r\n*** You have enough experience to advance to level {}! Use '@advance <class>' to assign this level, or '@multi_class <class>' to adopt a new class.",
            level + 1
        )];
    }

    let db = DB.get().and_then(|d| d.try_lock().ok());
    let conn_db = db.as_ref().map(|g| g.conn());

    let mut messages: Vec<String> = Vec::new();

    loop {
        let current_level = get_level(world, entity);
        let current_xp = get_experience(world, entity);
        let next_threshold = oxide_core::Experience::for_level(current_level + 1);
        if current_xp < next_threshold {
            break;
        }

        let new_level = current_level + 1;
        let excess = current_xp - next_threshold;

        // HP gain: hit die + CON mod
        let attrs = get_attributes(world, entity);
        let con_mod = (attrs.constitution as i32 - 10) / 2;
        let hit_die = get_hit_die(world, entity);

        // Update components
        if let Ok(mut q) = world.query_one::<&mut oxide_core::Health>(entity) {
            if let Some(health) = q.get() {
                let hp_gain = (hit_die + con_mod).max(1);
                health.max += hp_gain;
                health.current = health.max; // Full heal on level-up
            }
        }

        if let Ok(mut q) = world.query_one::<&mut oxide_core::Level>(entity) {
            if let Some(level) = q.get() {
                level.0 = new_level;
            }
        }

        if let Ok(mut q) = world.query_one::<&mut oxide_core::Experience>(entity) {
            if let Some(xp) = q.get() {
                xp.0 = excess;
            }
        }

        // Recalculate Mana pool: from_formula(level, int, wis), current clamped to new max
        if let Ok(mut q) = world.query_one::<&mut oxide_core::Mana>(entity) {
            if let Some(mana) = q.get() {
                let formula_mana = oxide_core::Mana::from_formula(
                    new_level as u16,
                    attrs.intelligence as u16,
                    attrs.wisdom as u16,
                );
                mana.max = formula_mana.max;
                mana.current = mana.current.min(mana.max);
            }
        }

        // Recalculate Stamina pool: from_formula(level, str, dex), current clamped to new max
        if let Ok(mut q) = world.query_one::<&mut oxide_core::Stamina>(entity) {
            if let Some(stamina) = q.get() {
                let formula_stamina = oxide_core::Stamina::from_formula(
                    new_level as u16,
                    attrs.strength as u16,
                    attrs.dexterity as u16,
                );
                stamina.max = formula_stamina.max;
                stamina.current = stamina.current.min(stamina.max);
            }
        }

        // Recalculate CombatStats per class progression
        let class_id = world
            .query_one::<&oxide_core::Class>(entity)
            .ok()
            .and_then(|mut q| q.get().map(|c| c.0.clone()));
        if let (Some(c_id), Some(t)) = (class_id, get_templates()) {
            if let Some(class_template) = t.get_class(&c_id) {
                let new_combat_stats = class_template.calculate_combat_stats(new_level);
                let _ = world.insert(entity, (new_combat_stats,));
            }
        }

        // Emit PlayerLeveled event
        let _event = oxide_core::GameEvent::PlayerLeveled {
            entity,
            old_level: current_level,
            new_level,
        };

        let _ = world.insert(entity, (oxide_core::Dirty,));

        // Persist to DB
        if let Some(conn_db) = conn_db {
            if let Ok(mut q) = world.query_one::<&oxide_core::DbId>(entity) {
                if let Some(db_id) = q.get() {
                    let _ = oxide_data::save_level_component(conn_db, db_id.0, new_level as i64);
                    let _ = oxide_data::save_experience_component(conn_db, db_id.0, excess as i64);
                }
            }
        }

        // Grant practice points on level-up: (2 + WIS_mod + INT_mod).max(1)
        let wis_mod = (attrs.wisdom as i32 - 10) / 2;
        let int_mod = (attrs.intelligence as i32 - 10) / 2;
        let practice_gain = (2 + wis_mod + int_mod).max(1) as u32;
        if let Ok(mut q) = world.query_one::<&mut oxide_core::PracticePoints>(entity) {
            if let Some(pp) = q.get() {
                pp.0 = pp.0.saturating_add(practice_gain);
            }
        }

        let pp_msg = format!(" {} practice point(s).", practice_gain);

        messages.push(format!(
            "You advance to level {new_level}! HP increased by {}.{}",
            (hit_die + con_mod).max(1),
            pp_msg,
        ));
    }

    if !messages.is_empty() {
        if let Ok(mut q) = world.query_one::<&mut oxide_core::Health>(entity) {
            if let Some(health) = q.get() {
                health.current = health.max; // Ensure full heal
            }
        }

        // Re-apply passives on level-up
        if let Some(templates) = get_templates() {
            oxide_core::systems::passive::apply_all_passives(world, entity, &templates);
        }
    }

    messages
}

fn get_level(world: &World, entity: Entity) -> u8 {
    world
        .query_one::<&oxide_core::Level>(entity)
        .ok()
        .and_then(|mut q| q.get().map(|l| l.0))
        .unwrap_or(1)
}

fn get_experience(world: &World, entity: Entity) -> u64 {
    world
        .query_one::<&oxide_core::Experience>(entity)
        .ok()
        .and_then(|mut q| q.get().map(|x| x.0))
        .unwrap_or(0)
}

fn get_attributes(world: &World, entity: Entity) -> oxide_core::Attributes {
    world
        .query_one::<&oxide_core::Attributes>(entity)
        .ok()
        .and_then(|mut q| q.get().cloned())
        .unwrap_or_default()
}

fn get_hit_die(world: &World, entity: Entity) -> i32 {
    let class_id = world
        .query_one::<&oxide_core::Class>(entity)
        .ok()
        .and_then(|mut q| q.get().map(|c| c.0.clone()));

    if let Some(c_id) = class_id {
        if let Some(t) = get_templates() {
            if let Some(class_template) = t.get_class(&c_id) {
                return class_template.hit_die as i32;
            }
        }
    }
    8
}

// ---------------------------------------------------------------------------
// MOTD loading
// ---------------------------------------------------------------------------

/// Load the message-of-the-day from a file.
pub fn load_motd(path: Option<&Path>) {
    let lock = MOTD.get_or_init(|| std::sync::RwLock::new(None));
    if let Ok(mut writer) = lock.write() {
        if let Some(path) = path {
            if let Ok(text) = fs::read_to_string(path) {
                let trimmed = text.trim().to_string();
                if !trimmed.is_empty() {
                    *writer = Some(trimmed);
                    return;
                }
            }
        }
        *writer = None;
    }
}

/// Returns the message of the day text.
pub fn get_motd() -> Option<String> {
    MOTD.get()
        .and_then(|lock| lock.read().ok())
        .and_then(|guard| guard.clone())
}

// ---------------------------------------------------------------------------
// Banner loading
// ---------------------------------------------------------------------------

/// Load the server banner from a file.
pub fn load_banner(path: Option<&Path>) {
    let lock = BANNER.get_or_init(|| std::sync::RwLock::new(None));
    if let Ok(mut writer) = lock.write() {
        if let Some(path) = path {
            if let Ok(text) = fs::read_to_string(path) {
                let trimmed = text.trim_end().to_string();
                if !trimmed.is_empty() {
                    *writer = Some(trimmed);
                    return;
                }
            }
        }
        *writer = None;
    }
}

/// Returns the server banner text.
pub fn get_banner() -> Option<String> {
    BANNER
        .get()
        .and_then(|lock| lock.read().ok())
        .and_then(|guard| guard.clone())
}

// ---------------------------------------------------------------------------
// Server console accessors
// ---------------------------------------------------------------------------

/// Returns a clone of the command dispatch, if initialized.
pub fn get_commands() -> Option<Arc<CommandDispatch>> {
    COMMANDS.get().cloned()
}

/// Sets the command dispatch, returning an error if already set.
pub fn set_commands(dispatch: CommandDispatch) -> Result<(), Arc<CommandDispatch>> {
    COMMANDS.set(Arc::new(dispatch))
}

/// Returns a clone of the DB handle, if initialized.
pub fn get_db() -> Option<Arc<Mutex<oxide_data::Database>>> {
    DB.get().cloned()
}

/// Returns a clone of the template registry, if initialized.
pub fn get_templates() -> Option<Arc<TemplateRegistry>> {
    TEMPLATES.get().map(|lock| {
        let guard = lock.read().unwrap_or_else(|e| e.into_inner());
        (*guard).clone()
    })
}

/// Mutates the active template registry in-memory.
pub fn update_templates<F, R>(f: F) -> Result<R, String>
where
    F: FnOnce(&mut TemplateRegistry) -> R,
{
    let lock = TEMPLATES
        .get()
        .ok_or_else(|| "Template registry not initialized".to_string())?;
    let mut guard = lock.write().unwrap_or_else(|e| e.into_inner());
    let mut registry_cloned = (**guard).clone();
    let result = f(&mut registry_cloned);
    *guard = Arc::new(registry_cloned);
    Ok(result)
}

/// Initializes or replaces the active templates for tests.
pub fn init_templates_for_test(templates: TemplateRegistry) {
    let templates = Arc::new(templates);
    if let Some(lock) = TEMPLATES.get() {
        let mut guard = lock.write().unwrap_or_else(|e| e.into_inner());
        *guard = templates;
    } else {
        let _ = TEMPLATES.set(std::sync::RwLock::new(templates));
    }
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

pub fn advance_player_class(
    world: &mut World,
    player: Entity,
    class_id: &str,
) -> Result<Vec<String>, String> {
    let mut mc_info = world
        .query_one::<&mut oxide_core::MultiClassInfo>(player)
        .map_err(|_| "You do not have class information.")?
        .get()
        .cloned()
        .ok_or("Failed to load class information.")?;

    if !mc_info.has_class(class_id) {
        return Err(format!(
            "You do not have the class '{}'. Use '@multi_class' to adopt a new class.",
            class_id
        ));
    }

    let current_level = get_level(world, player);
    let next_level = current_level + 1;
    let xp = get_experience(world, player);
    let threshold = oxide_core::Experience::for_level(next_level);

    if xp < threshold {
        return Err(format!(
            "You need {} XP to advance to level {}, but you only have {}.",
            threshold, next_level, xp
        ));
    }

    let templates = crate::get_templates().ok_or("Template registry not found.")?;
    let class_template = templates
        .get_class(class_id)
        .ok_or(format!("Class template for '{}' not found.", class_id))?;

    mc_info.advance_class(class_id);

    let attrs = get_attributes(world, player);
    let con_mod = (attrs.constitution as i32 - 10) / 2;
    let hit_die = class_template.hit_die;
    let hp_gain = (hit_die as i32 + con_mod).max(1);

    if let Ok(mut q) = world.query_one::<&mut oxide_core::Health>(player) {
        if let Some(health) = q.get() {
            health.max += hp_gain;
            health.current = health.max;
        }
    }

    if let Ok(mut q) = world.query_one::<&mut Level>(player) {
        if let Some(level) = q.get() {
            level.0 = next_level;
        }
    }

    if let Ok(mut q) = world.query_one::<&mut oxide_core::Mana>(player) {
        if let Some(mana) = q.get() {
            let formula_mana = oxide_core::Mana::from_formula(
                next_level as u16,
                attrs.intelligence as u16,
                attrs.wisdom as u16,
            );
            mana.max = formula_mana.max;
            mana.current = mana.max;
        }
    }

    if let Ok(mut q) = world.query_one::<&mut oxide_core::Stamina>(player) {
        if let Some(stamina) = q.get() {
            let formula_stamina = oxide_core::Stamina::from_formula(
                next_level as u16,
                attrs.strength as u16,
                attrs.dexterity as u16,
            );
            stamina.max = formula_stamina.max;
            stamina.current = stamina.max;
        }
    }

    let new_combat_stats = oxide_core::calculate_multiclass_combat_stats(&mc_info, &templates);
    let _ = world.insert(player, (new_combat_stats,));

    let _ = world.insert(player, (mc_info,));

    let db = DB.get().and_then(|d| d.try_lock().ok());
    let conn_db = db.as_ref().map(|g| g.conn());
    if let Some(conn_db) = conn_db {
        if let Ok(mut q) = world.query_one::<&DbId>(player) {
            if let Some(db_id) = q.get() {
                let _ = oxide_data::save_level_component(conn_db, db_id.0, next_level as i64);
            }
        }
    }

    if let Ok(mut q) = world.query_one::<&mut Experience>(player) {
        if let Some(xp_comp) = q.get() {
            xp_comp.0 = xp.saturating_sub(threshold);
            if let Some(conn_db) = conn_db {
                if let Ok(mut q_db) = world.query_one::<&DbId>(player) {
                    if let Some(db_id) = q_db.get() {
                        let _ = oxide_data::save_experience_component(
                            conn_db,
                            db_id.0,
                            xp_comp.0 as i64,
                        );
                    }
                }
            }
        }
    }

    let _ = world.insert(player, (oxide_core::Dirty,));

    let msgs = vec![
        format!(
            "You successfully advance your class: {}!",
            class_template.name
        ),
        format!("You are now level {}.", next_level),
        format!("You gained {} max HP.", hp_gain),
    ];
    Ok(msgs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use oxide_core::templates::{ClassAttributeMods, ClassTemplate, DeityPolicy, WalletAmount};
    use oxide_core::CombatStats;
    use std::collections::HashMap;

    fn make_player(world: &mut World, level: u8, xp: u64, attrs: Attributes) -> Entity {
        let e = world.spawn(());
        world
            .insert(
                e,
                (
                    Health::new(50),
                    Level(level),
                    Experience(xp),
                    attrs,
                    PracticePoints(0),
                ),
            )
            .unwrap();
        e
    }

    #[test]
    fn no_level_up_below_threshold() {
        let mut world = World::new();
        let e = make_player(&mut world, 1, 50, Attributes::default());
        award_xp(&mut world, e);

        assert_eq!(get_level(&world, e), 1);
        assert_eq!(get_experience(&world, e), 50);
        let mut q = world.query_one::<&PracticePoints>(e).unwrap();
        let pp = q.get().unwrap();
        assert_eq!(pp.0, 0);
    }

    #[test]
    fn single_level_up_grants_practice_points() {
        let mut world = World::new();
        let e = make_player(&mut world, 1, 1000, Attributes::default());
        award_xp(&mut world, e);

        assert_eq!(get_level(&world, e), 2);
        assert_eq!(get_experience(&world, e), 200);

        let mut q = world.query_one::<&Health>(e).unwrap();
        let health = q.get().unwrap();
        assert_eq!(health.max, 58);
        assert_eq!(health.current, 58);

        let mut q = world.query_one::<&PracticePoints>(e).unwrap();
        let pp = q.get().unwrap();
        assert_eq!(pp.0, 2);
    }

    #[test]
    fn multiple_level_ups_grant_practice_points_each() {
        let mut world = World::new();
        let e = make_player(&mut world, 1, 5000, Attributes::default());
        award_xp(&mut world, e);

        assert_eq!(get_level(&world, e), 3);
        assert_eq!(get_experience(&world, e), 1500);

        let mut q = world.query_one::<&PracticePoints>(e).unwrap();
        let pp = q.get().unwrap();
        assert_eq!(pp.0, 4);
    }

    #[test]
    fn practice_points_scales_with_wisdom_and_intelligence() {
        let mut world = World::new();
        let attrs = Attributes::new(10, 10, 12, 14, 10, 10);
        let e = make_player(&mut world, 1, 1000, attrs);
        award_xp(&mut world, e);

        let mut q = world.query_one::<&PracticePoints>(e).unwrap();
        let pp = q.get().unwrap();
        assert_eq!(pp.0, 5);
    }

    #[test]
    fn practice_points_minimum_of_one() {
        let mut world = World::new();
        let attrs = Attributes::new(10, 10, 4, 4, 10, 10);
        let e = make_player(&mut world, 1, 1000, attrs);
        award_xp(&mut world, e);

        let mut q = world.query_one::<&PracticePoints>(e).unwrap();
        let pp = q.get().unwrap();
        assert_eq!(pp.0, 1);
    }

    #[test]
    fn hp_gain_at_least_one() {
        let mut world = World::new();
        let attrs = Attributes::new(10, 10, 10, 10, 3, 10);
        let e = make_player(&mut world, 1, 1000, attrs);
        award_xp(&mut world, e);

        let mut q = world.query_one::<&Health>(e).unwrap();
        let health = q.get().unwrap();
        let gain = health.max - 50;
        assert!(gain >= 1, "HP gain should be at least 1, got {gain}");
    }

    #[test]
    fn full_heal_on_level_up() {
        let mut world = World::new();
        let e = world.spawn(());
        world
            .insert(
                e,
                (
                    Health {
                        current: 10,
                        max: 50,
                    },
                    Level(1),
                    Experience(1000),
                    Attributes::default(),
                    PracticePoints(0),
                ),
            )
            .unwrap();

        award_xp(&mut world, e);

        let mut q = world.query_one::<&Health>(e).unwrap();
        let health = q.get().unwrap();
        assert_eq!(health.current, health.max);
    }

    fn init_test_templates() {
        let mut registry = TemplateRegistry::new();
        let warrior = ClassTemplate {
            id: "warrior".to_string(),
            name: "Warrior".to_string(),
            description: "A warrior".to_string(),
            hit_die: 10,
            attribute_mods: ClassAttributeMods::default(),
            bab: "full".to_string(),
            fort_save: "good".to_string(),
            ref_save: "poor".to_string(),
            will_save: "poor".to_string(),
            allowed_races: vec![],
            allowed_alignments: vec![],
            auto_skills: vec![],
            skill_pool: vec![],
            starting_skill_slots: 3,
            starting_items: vec![],
            starting_gold: WalletAmount::default(),
            deity_policy: DeityPolicy::Any,
            params: HashMap::new(),
            prestige: false,
            prestige_gate: None,
        };
        registry.classes.insert("warrior".to_string(), warrior);
        let registry = Arc::new(registry);
        if let Some(lock) = TEMPLATES.get() {
            let mut guard = lock.write().unwrap_or_else(|e| e.into_inner());
            *guard = registry;
        } else {
            let _ = TEMPLATES.set(std::sync::RwLock::new(registry));
        }
    }

    #[test]
    fn test_class_progression_calculations() {
        let warrior = ClassTemplate {
            id: "warrior".to_string(),
            name: "Warrior".to_string(),
            description: "A warrior".to_string(),
            hit_die: 10,
            attribute_mods: ClassAttributeMods::default(),
            bab: "full".to_string(),
            fort_save: "good".to_string(),
            ref_save: "poor".to_string(),
            will_save: "poor".to_string(),
            allowed_races: vec![],
            allowed_alignments: vec![],
            auto_skills: vec![],
            skill_pool: vec![],
            starting_skill_slots: 3,
            starting_items: vec![],
            starting_gold: WalletAmount::default(),
            deity_policy: DeityPolicy::Any,
            params: HashMap::new(),
            prestige: false,
            prestige_gate: None,
        };

        // Level 1
        let cs1 = warrior.calculate_combat_stats(1);
        assert_eq!(cs1.base_attack_bonus, 1);
        assert_eq!(cs1.fort_save, 2);
        assert_eq!(cs1.ref_save, 0);
        assert_eq!(cs1.will_save, 0);

        // Level 5
        let cs5 = warrior.calculate_combat_stats(5);
        assert_eq!(cs5.base_attack_bonus, 5);
        assert_eq!(cs5.fort_save, 4);
        assert_eq!(cs5.ref_save, 1);
        assert_eq!(cs5.will_save, 1);

        // Level 10
        let cs10 = warrior.calculate_combat_stats(10);
        assert_eq!(cs10.base_attack_bonus, 10);
        assert_eq!(cs10.fort_save, 7);
        assert_eq!(cs10.ref_save, 3);
        assert_eq!(cs10.will_save, 3);

        let mage = ClassTemplate {
            id: "mage".to_string(),
            name: "Mage".to_string(),
            description: "A mage".to_string(),
            hit_die: 6,
            attribute_mods: ClassAttributeMods::default(),
            bab: "poor".to_string(),
            fort_save: "poor".to_string(),
            ref_save: "poor".to_string(),
            will_save: "good".to_string(),
            allowed_races: vec![],
            allowed_alignments: vec![],
            auto_skills: vec![],
            skill_pool: vec![],
            starting_skill_slots: 4,
            starting_items: vec![],
            starting_gold: WalletAmount::default(),
            deity_policy: DeityPolicy::Any,
            params: HashMap::new(),
            prestige: false,
            prestige_gate: None,
        };

        // Level 5
        let cs_m5 = mage.calculate_combat_stats(5);
        assert_eq!(cs_m5.base_attack_bonus, 2);
        assert_eq!(cs_m5.fort_save, 1);
        assert_eq!(cs_m5.will_save, 4);
    }

    #[test]
    fn test_level_up_recalculates_combat_stats() {
        init_test_templates();

        let mut world = World::new();
        let e = world.spawn(());
        world
            .insert(
                e,
                (
                    Health::new(50),
                    Level(1),
                    Experience(1000),
                    Attributes::default(),
                    PracticePoints(0),
                    oxide_core::Class("warrior".to_string()),
                    CombatStats::default(),
                ),
            )
            .unwrap();

        award_xp(&mut world, e);

        assert_eq!(get_level(&world, e), 2);

        let mut q = world.query_one::<&CombatStats>(e).unwrap();
        let cs = q.get().unwrap();
        assert_eq!(cs.base_attack_bonus, 2);
        assert_eq!(cs.fort_save, 3);
        assert_eq!(cs.ref_save, 0);
    }

    #[test]
    fn test_get_hit_die_custom_class() {
        init_test_templates();

        let mut world = World::new();
        let e = world.spawn((oxide_core::Class("warrior".to_string()),));
        let hd = get_hit_die(&world, e);
        assert_eq!(hd, 10);

        let e_no_class = world.spawn(());
        let hd_fallback = get_hit_die(&world, e_no_class);
        assert_eq!(hd_fallback, 8);
    }

    #[test]
    fn test_level_up_recalculates_mana_and_stamina() {
        init_test_templates();

        let mut world = World::new();
        let e = world.spawn(());

        // Attributes: INT = 12, WIS = 14, STR = 10, DEX = 10
        let attrs = Attributes::new(10, 10, 12, 14, 10, 10);

        world
            .insert(
                e,
                (
                    Health::new(50),
                    Level(1),
                    Experience(1000),
                    attrs,
                    PracticePoints(0),
                    oxide_core::Mana {
                        current: 20,
                        max: 56,
                    },
                    oxide_core::Stamina {
                        current: 15,
                        max: 52,
                    },
                ),
            )
            .unwrap();

        award_xp(&mut world, e);

        assert_eq!(get_level(&world, e), 2);

        // Level 2 Mana should be: 2 * 4 + 12 * 2 + 14 * 2 = 8 + 24 + 28 = 60.
        // Current mana should be preserved: 20/60.
        let mut q_mana = world.query_one::<&oxide_core::Mana>(e).unwrap();
        let mana = q_mana.get().unwrap();
        assert_eq!(mana.max, 60);
        assert_eq!(mana.current, 20);

        // Level 2 Stamina should be: 2 * 12 + 10 * 2 + 10 * 2 = 24 + 20 + 20 = 64.
        // Current stamina should be preserved: 15/64.
        let mut q_stamina = world.query_one::<&oxide_core::Stamina>(e).unwrap();
        let stamina = q_stamina.get().unwrap();
        assert_eq!(stamina.max, 64);
        assert_eq!(stamina.current, 15);

        // Now let's test clamping if current is somehow above max.
        drop(q_mana);
        drop(q_stamina);

        {
            let mut q_mana = world.query_one::<&mut oxide_core::Mana>(e).unwrap();
            let mana = q_mana.get().unwrap();
            mana.current = 100;
        }
        {
            let mut q_stamina = world.query_one::<&mut oxide_core::Stamina>(e).unwrap();
            let stamina = q_stamina.get().unwrap();
            stamina.current = 100;
        }
        {
            let mut q_xp = world.query_one::<&mut Experience>(e).unwrap();
            let xp = q_xp.get().unwrap();
            xp.0 = 3000; // Enough for next level (for_level(3) = 2700)
        }

        award_xp(&mut world, e);

        assert_eq!(get_level(&world, e), 3);

        let mut q_mana = world.query_one::<&oxide_core::Mana>(e).unwrap();
        let mana = q_mana.get().unwrap();
        assert_eq!(mana.max, 64);
        assert_eq!(mana.current, 64); // Clamped to max

        let mut q_stamina = world.query_one::<&oxide_core::Stamina>(e).unwrap();
        let stamina = q_stamina.get().unwrap();
        assert_eq!(stamina.max, 76);
        assert_eq!(stamina.current, 76); // Clamped to max
    }

    #[tokio::test]
    async fn test_template_hot_reloading() {
        crate::config::init(std::path::Path::new("nonexistent.toml"));
        let temp_dir = std::env::current_dir()
            .unwrap()
            .join(format!("temp_templates_test_{}", fastrand::u32(..)));
        std::fs::create_dir_all(temp_dir.join("areas/midgaard/rooms")).unwrap();

        // 1. Write initial area.toml
        let area_toml = r#"
id = "midgaard"
name = "Midgaard"
description = "Midgaard City"

[[spawns]]
room = "1"
label = "Temple Square"
description = "Default spawn point"
"#;
        std::fs::write(temp_dir.join("areas/midgaard/area.toml"), area_toml).unwrap();

        // Write a basic room to satisfy validation
        let room_toml = r#"
id = "1"
area = "midgaard"
name = "Temple Square"
description = "Center of Midgaard"
"#;
        std::fs::write(temp_dir.join("areas/midgaard/rooms/1.toml"), room_toml).unwrap();

        // Give macOS filesystem a moment to register/index directory creation
        tokio::time::sleep(Duration::from_millis(200)).await;

        let (initial_registry, _) = oxide_core::content::load_registry(&temp_dir);
        let server = Server::new("127.0.0.1:0", World::new())
            .with_templates(initial_registry)
            .with_content_path(&temp_dir);

        // Verify loaded template
        {
            let templates = get_templates().unwrap();
            let area = templates.areas.get("midgaard").unwrap();
            assert_eq!(area.name, "Midgaard");
        }

        // Spawn Server run watcher task
        let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);
        let server_task = tokio::spawn(async move {
            let _ = server.run(shutdown_rx).await;
        });

        // 2. Modify area.toml to trigger hot-reload
        let updated_area_toml = r#"
id = "midgaard"
name = "Midgaard Plaza"
description = "Midgaard City"

[[spawns]]
room = "1"
label = "Temple Square"
description = "Default spawn point"
"#;
        // Wait a short moment to ensure watcher is bound
        tokio::time::sleep(Duration::from_millis(3000)).await;

        std::fs::write(temp_dir.join("areas/midgaard/area.toml"), updated_area_toml).unwrap();

        // Wait for reload (which is debounced by 100ms)
        let start = Instant::now();
        let mut reloaded = false;
        while start.elapsed() < Duration::from_secs(10) {
            tokio::time::sleep(Duration::from_millis(50)).await;
            if let Some(templates) = get_templates() {
                if let Some(area) = templates.areas.get("midgaard") {
                    if area.name == "Midgaard Plaza" {
                        reloaded = true;
                        break;
                    }
                }
            }
        }

        assert!(reloaded, "Template was not hot-reloaded within timeout");

        // Clean up server task and files
        let _ = shutdown_tx.send(true);
        let _ = server_task.await;
        std::fs::remove_dir_all(&temp_dir).unwrap();
    }
}
