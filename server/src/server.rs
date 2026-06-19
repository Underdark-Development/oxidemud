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
use mud_core::{
    Alignment, Attributes, DbId, Description, Entity, Equipment, Experience, Health, Inventory,
    LearnedSkills, Level, Name, Player, Position, PracticePoints, Room, SpawnKey, Wallet, World,
};

static SERVER_START: OnceLock<Instant> = OnceLock::new();
static MOTD: OnceLock<String> = OnceLock::new();
pub(crate) static DB: OnceLock<Arc<Mutex<mud_data::Database>>> = OnceLock::new();
pub(crate) static TEMPLATES: OnceLock<Arc<TemplateRegistry>> = OnceLock::new();
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
    void_room: Entity,
    spawn_room: Entity,
    db: Option<Arc<Mutex<mud_data::Database>>>,
    templates: Option<Arc<TemplateRegistry>>,
    shutdown_complete: Arc<Notify>,
    on_entity_spawned: Option<Arc<EntitySpawnedCb>>,
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
            spawn_room: void_room,
            db: None,
            templates: None,
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

    pub fn with_spawn_room(mut self, spawn_room: Entity) -> Self {
        self.spawn_room = spawn_room;
        self
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
        let void_room = self.void_room;
        let spawn_room = self.spawn_room;
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
                            conn_id, stream, world, registry, commands, void_room, spawn_room, db,
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

#[allow(clippy::too_many_arguments)]
async fn handle_connection(
    conn_id: u64,
    stream: tokio::net::TcpStream,
    world: Arc<Mutex<World>>,
    registry: Arc<Mutex<ConnectionRegistry>>,
    commands: Arc<CommandDispatch>,
    void_room: Entity,
    spawn_room: Entity,
    db: Option<Arc<Mutex<mud_data::Database>>>,
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
                            void_room,
                            spawn_room,
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
                            if let Ok(mut q) = w.query_one::<&Player>(entity) {
                                if let Some(player) = q.get() {
                                    conn.set_screen_width(player.screen_width);
                                }
                            }
                            if let Some(tx) = conn.output_sender() {
                                reg.register(entity, tx);
                            }
                        }
                        if let Some(ref cb) = on_entity_spawned {
                            cb(&mut w, &mut conn, &reg);
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
                    .query_one::<&mud_core::Mana>(entity)
                    .ok()
                    .and_then(|mut q| q.get().cloned());
                let stamina = w
                    .query_one::<&mud_core::Stamina>(entity)
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

                let room_db_id = position.and_then(|room_entity| {
                    w.query_one::<&DbId>(room_entity)
                        .ok()
                        .and_then(|mut q| q.get().copied())
                        .map(|dbid| dbid.0)
                });

                let room_spawn_key = position.and_then(|room_entity| {
                    w.query_one::<&SpawnKey>(room_entity)
                        .ok()
                        .and_then(|mut q| q.get().map(|sk| sk.0.clone()))
                });

                let mut room_info = None;
                if room_db_id.is_none() {
                    if let Some(room_entity) = position {
                        if let Ok(mut q_room) = w.query_one::<(&Room, &SpawnKey)>(room_entity) {
                            if let Some((r, sk)) = q_room.get() {
                                room_info =
                                    Some((r.name.clone(), r.description.clone(), sk.0.clone()));
                            }
                        }
                    }
                }

                let mut inventory_items = Vec::new();
                if let Ok(mut q) = w.query_one::<&Inventory>(entity) {
                    if let Some(inv) = q.get() {
                        for &item_entity in &inv.0 {
                            if let Ok(mut item_q) =
                                w.query_one::<(&mud_core::Item, Option<&DbId>)>(item_entity)
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
                                w.query_one::<(&mud_core::Item, Option<&DbId>)>(item_entity)
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
                    room_db_id,
                    room_spawn_key,
                    room_info,
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
        let room_to_db_id = if let Some((
            db_id,
            level,
            xp,
            health,
            mana,
            stamina,
            room_entity,
            mut room_db_id,
            room_spawn_key,
            room_info,
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
            let mut new_rid = None;
            if let Some(ref db) = db {
                let db_guard = db.lock().await;
                let conn_db = db_guard.conn();

                // If room has no DB record yet, insert it now
                if room_db_id.is_none() {
                    if let Some(_re) = room_entity {
                        if let Ok(rid) = mud_data::insert_entity(conn_db, "room") {
                            room_db_id = Some(rid);
                            new_rid = Some(rid);
                            if let Some((name, desc, spawn_key)) = &room_info {
                                let _ = mud_data::save_room_component(
                                    conn_db,
                                    rid,
                                    name,
                                    desc,
                                    Some(spawn_key),
                                );
                            }
                        }
                    }
                }

                // Save Level & XP
                if let Some(level) = level {
                    let _ = mud_data::save_level_component(conn_db, db_id.0, level.0 as i64);
                    let xp_val = xp.map(|x| x.0).unwrap_or(0);
                    let _ = mud_data::update_character_level(
                        conn_db,
                        db_id.0,
                        level.0.into(),
                        xp_val as i64,
                    );
                }
                if let Some(xp) = xp {
                    let _ = mud_data::save_experience_component(conn_db, db_id.0, xp.0 as i64);
                }
                // Save Health
                if let Some(health) = health {
                    let _ = mud_data::save_health_component(
                        conn_db,
                        db_id.0,
                        health.current,
                        health.max,
                    );
                }
                // Save Mana
                if let Some(mana) = mana {
                    let _ = mud_data::save_mana_component(conn_db, db_id.0, mana.current as i32);
                }
                // Save Stamina
                if let Some(stamina) = stamina {
                    let _ =
                        mud_data::save_stamina_component(conn_db, db_id.0, stamina.current as i32);
                }
                // Save Wallet
                if let Some(wallet) = wallet {
                    let _ = mud_data::save_golds_component(
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
                    if let Err(e) = mud_data::save_skills(conn_db, db_id.0, &skills.skills) {
                        tracing::error!(entity_id = db_id.0, error = %e, "disconnect: failed to save skills");
                    }
                    if let Some(ref player_comp) = player_comp {
                        let pp = practice_points.map(|p| p.0).unwrap_or(0);
                        if let Err(e) = mud_data::save_player_component(
                            conn_db,
                            db_id.0,
                            player_comp.account_id,
                            player_comp.prompt.as_deref(),
                            player_comp.screen_width,
                            pp,
                        ) {
                            tracing::error!(entity_id = db_id.0, error = %e, "disconnect: failed to save player component");
                        } else {
                            // Readback verify
                            match mud_data::load_player_component(conn_db, db_id.0) {
                                Ok(Some((_, loaded_prompt, _, _))) => {
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
                } else if let Some(ref player_comp) = player_comp {
                    if let Err(e) = mud_data::save_player_component(
                        conn_db,
                        db_id.0,
                        player_comp.account_id,
                        player_comp.prompt.as_deref(),
                        player_comp.screen_width,
                        0,
                    ) {
                        tracing::error!(entity_id = db_id.0, error = %e, "disconnect: failed to save player component");
                    } else {
                        // Readback verify
                        match mud_data::load_player_component(conn_db, db_id.0) {
                            Ok(Some((_, loaded_prompt, _, _))) => {
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
                    let _ = mud_data::save_attributes_component(
                        conn_db,
                        db_id.0,
                        &mud_data::AttributesRow {
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
                    let _ = mud_data::save_alignment_component(conn_db, db_id.0, &alignment.0);
                }
                // Save Description
                if let Some(description) = description {
                    let _ = mud_data::save_description_component(conn_db, db_id.0, &description.0);
                }
                // Save Position
                if let Some(rid) = room_db_id {
                    let _ = mud_data::update_character_position(conn_db, db_id.0, rid);
                    let _ = mud_data::update_character_last_seen(conn_db, db_id.0);
                }
                if let Some(ref spawn_key) = room_spawn_key {
                    let _ = mud_data::update_character_spawn_key(conn_db, db_id.0, spawn_key);
                }

                // Save Inventory
                let _ = mud_data::delete_all_inventory(conn_db, db_id.0);
                for (slot_idx, (template_id, opt_db_id)) in inventory_items.into_iter().enumerate()
                {
                    let item_db_id = match opt_db_id {
                        Some(id) => id,
                        None => {
                            if let Ok(new_id) = mud_data::insert_entity(conn_db, "item") {
                                new_id
                            } else {
                                continue;
                            }
                        }
                    };
                    let _ = mud_data::save_item_component(conn_db, item_db_id, &template_id);
                    let _ =
                        mud_data::add_inventory_item(conn_db, db_id.0, item_db_id, slot_idx as i32);
                }

                // Save Equipment
                let _ = mud_data::delete_all_equipment(conn_db, db_id.0);
                for (slot, template_id, opt_db_id) in equipment_items {
                    let item_db_id = match opt_db_id {
                        Some(id) => id,
                        None => {
                            if let Ok(new_id) = mud_data::insert_entity(conn_db, "item") {
                                new_id
                            } else {
                                continue;
                            }
                        }
                    };
                    let _ = mud_data::save_item_component(conn_db, item_db_id, &template_id);
                    let slot_str = format!("{:?}", slot).to_lowercase();
                    let _ = mud_data::save_equipment_slot(conn_db, db_id.0, &slot_str, item_db_id);
                }
            }
            (room_entity, new_rid)
        } else {
            (None, None)
        };

        // 3. Lock world and registry again to remove connection and despawn
        {
            let mut w = world.lock().await;
            let mut reg = registry.lock().await;

            if let (Some(re), Some(rid)) = room_to_db_id {
                let _ = w.insert(re, (DbId(rid),));
            }

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
/// Returns level-up messages to be sent to the player.
pub fn award_xp(world: &mut World, entity: Entity) -> Vec<String> {
    let level = get_level(world, entity);
    let xp = get_experience(world, entity);

    let threshold = mud_core::Experience::for_level(level + 1);
    if xp < threshold {
        return Vec::new();
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

        let _ = world.insert(entity, (mud_core::Dirty,));

        // Persist to DB
        if let Some(conn_db) = conn_db {
            if let Ok(mut q) = world.query_one::<&mud_core::DbId>(entity) {
                if let Some(db_id) = q.get() {
                    let _ = mud_data::save_level_component(conn_db, db_id.0, new_level as i64);
                    let _ = mud_data::save_experience_component(conn_db, db_id.0, excess as i64);
                }
            }
        }

        // Grant practice points on level-up: (2 + WIS_mod + INT_mod).max(1)
        let wis_mod = (attrs.wisdom as i32 - 10) / 2;
        let int_mod = (attrs.intelligence as i32 - 10) / 2;
        let practice_gain = (2 + wis_mod + int_mod).max(1) as u32;
        if let Ok(mut q) = world.query_one::<&mut mud_core::PracticePoints>(entity) {
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
        if let Ok(mut q) = world.query_one::<&mut mud_core::Health>(entity) {
            if let Some(health) = q.get() {
                health.current = health.max; // Ensure full heal
            }
        }

        // Re-apply passives on level-up
        if let Some(templates) = TEMPLATES.get() {
            mud_core::systems::passive::apply_all_passives(world, entity, templates);
        }
    }

    messages
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

/// Returns a clone of the command dispatch, if initialized.
pub fn get_commands() -> Option<Arc<CommandDispatch>> {
    COMMANDS.get().cloned()
}

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

#[cfg(test)]
mod tests {
    use super::*;

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
}
