use axum::{
    extract::{Path, Request},
    http::{HeaderMap, StatusCode},
    middleware::{self, Next},
    response::Response,
    routing::{get, post},
    Json, Router,
};
use std::net::SocketAddr;
use tokio::sync::watch;
use tracing;

use crate::config::ApiConfig;
use oxide_core::Attributes;

#[derive(Debug, serde::Deserialize)]
struct SimulateParams {
    race_id: String,
    class_id: String,
    base_attributes: AttributesJson,
    selected_skills: Option<Vec<String>>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
struct AttributesJson {
    strength: u8,
    dexterity: u8,
    intelligence: u8,
    wisdom: u8,
    constitution: u8,
    charisma: u8,
}

#[derive(Debug, serde::Serialize)]
struct SimulateResponse {
    attributes: Attributes,
    hp: i32,
    mana: u16,
    stamina: u16,
    starting_gold: WalletJson,
    auto_skills: Vec<String>,
}

#[derive(Debug, serde::Serialize)]
struct WalletJson {
    copper: u64,
    silver: u64,
    gold: u64,
    platinum: u64,
}

#[derive(Debug, serde::Deserialize)]
struct PutItemParams {
    player_name: String,
    item_template_id: String,
    count: Option<u32>,
}

#[derive(Debug, serde::Deserialize)]
struct TeleportParams {
    player_name: String,
    room_key: String,
}

#[derive(Debug, serde::Deserialize)]
struct ForceCommandParams {
    player_name: String,
    command: String,
}

pub async fn start_api_server(
    config: ApiConfig,
    mut shutdown_rx: watch::Receiver<bool>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if !config.enabled {
        return Ok(());
    }

    let bind_addr = config.bind_addr.clone();
    let addr: SocketAddr = bind_addr.parse()?;

    // Check loopback binding warning
    let ip = addr.ip();
    if !ip.is_loopback() {
        tracing::warn!(
            "REST API server bound to a public interface ({}) without TLS/HTTPS encryption. \
             It is highly recommended to run this REST API on a loopback interface behind a TLS-terminating reverse proxy.",
            bind_addr
        );
    }

    let app = Router::new()
        .route("/api/players", get(list_players))
        .route("/api/character/simulate", post(simulate_character))
        .route("/api/character/:name", get(get_character_state))
        .route("/api/imm/put_item", post(imm_put_item))
        .route("/api/imm/teleport", post(imm_teleport))
        .route("/api/imm/force_command", post(imm_force_command))
        .layer(middleware::from_fn(auth_middleware));

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("REST API server listening on {}", bind_addr);

    let graceful = async move {
        let _ = shutdown_rx.changed().await;
        tracing::info!("REST API server shutting down gracefully");
    };

    axum::serve(listener, app)
        .with_graceful_shutdown(graceful)
        .await?;

    Ok(())
}

async fn auth_middleware(
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let auth_header = headers
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .ok_or(StatusCode::UNAUTHORIZED)?;

    if !auth_header.starts_with("Bearer ") {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let token = &auth_header["Bearer ".len()..];

    // Ensure it's a valid UUID format before querying the DB to prevent injection / useless lookups
    if uuid::Uuid::parse_str(token).is_err() {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let db_lock = crate::get_db().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let db = db_lock.lock().await;

    let (_account_id, username, access_level) = match oxide_data::validate_api_key(db.conn(), token)
    {
        Ok(Some(info)) => info,
        _ => return Err(StatusCode::UNAUTHORIZED),
    };

    // Log the request with username (masking key)
    let method = request.method().clone();
    let path = request.uri().path().to_string();
    tracing::info!("REST API call: {} {} (user: {})", method, path, username);

    // RBAC: Check access level for IMM routes
    if path.starts_with("/api/imm/") {
        let allowed = matches!(
            access_level.to_lowercase().as_str(),
            "immortal" | "god" | "admin"
        );
        if !allowed {
            tracing::warn!(
                "Unauthorized IMM API access attempt by user '{}' (access level: {}) to path '{}'",
                username,
                access_level,
                path
            );
            return Err(StatusCode::FORBIDDEN);
        }
    }

    Ok(next.run(request).await)
}

async fn list_players() -> Result<Json<serde_json::Value>, StatusCode> {
    let registry_lock = crate::get_registry().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let reg = registry_lock.lock().await;
    let world_lock = crate::get_world().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let world = world_lock.lock().await;

    let mut players_list = Vec::new();
    let entities = reg.connected_entities();

    for entity in entities {
        let name = if let Ok(mut q) = world.query_one::<&oxide_core::Name>(entity) {
            q.get()
                .map(|n| n.0.clone())
                .unwrap_or_else(|| "Unknown".to_string())
        } else {
            "Unknown".to_string()
        };

        let level = if let Ok(mut q) = world.query_one::<&oxide_core::Level>(entity) {
            q.get().map(|l| l.0).unwrap_or(1)
        } else {
            1
        };

        let class = if let Ok(mut q) = world.query_one::<&oxide_core::Class>(entity) {
            q.get()
                .map(|c| c.0.clone())
                .unwrap_or_else(|| "None".to_string())
        } else {
            "None".to_string()
        };

        let race = if let Ok(mut q) = world.query_one::<&oxide_core::Race>(entity) {
            q.get()
                .map(|r| r.0.clone())
                .unwrap_or_else(|| "None".to_string())
        } else {
            "None".to_string()
        };

        let room_key = if let Ok(mut q_pos) = world.query_one::<&oxide_core::Position>(entity) {
            if let Some(pos) = q_pos.get() {
                if let Ok(mut q_rk) = world.query_one::<&oxide_core::RoomKey>(pos.room) {
                    q_rk.get()
                        .map(|rk| rk.0.clone())
                        .unwrap_or_else(|| "Unknown".to_string())
                } else {
                    "Unknown".to_string()
                }
            } else {
                "Unknown".to_string()
            }
        } else {
            "Unknown".to_string()
        };

        players_list.push(serde_json::json!({
            "entity_id": entity.id(),
            "name": name,
            "level": level,
            "class": class,
            "race": race,
            "room_key": room_key,
        }));
    }

    Ok(Json(serde_json::json!(players_list)))
}

async fn simulate_character(
    Json(params): Json<SimulateParams>,
) -> Result<Json<SimulateResponse>, (StatusCode, String)> {
    let templates = crate::get_templates().ok_or((
        StatusCode::INTERNAL_SERVER_ERROR,
        "Templates registry unavailable".to_string(),
    ))?;

    // Validate race & class exist
    if templates.get_race(&params.race_id).is_none() {
        return Err((
            StatusCode::BAD_REQUEST,
            format!("Race '{}' not found", params.race_id),
        ));
    }
    if templates.get_class(&params.class_id).is_none() {
        return Err((
            StatusCode::BAD_REQUEST,
            format!("Class '{}' not found", params.class_id),
        ));
    }

    // Validate attributes
    if let Err(e) = validate_attributes(&params.base_attributes) {
        return Err((
            StatusCode::BAD_REQUEST,
            format!("Invalid base attributes: {e}"),
        ));
    }

    let core_attributes = Attributes::new(
        params.base_attributes.strength,
        params.base_attributes.dexterity,
        params.base_attributes.intelligence,
        params.base_attributes.wisdom,
        params.base_attributes.constitution,
        params.base_attributes.charisma,
    );

    // Call actual character creation calculations
    let (attrs, hp, mut learned_skills) = crate::login::compute_final_attributes(
        Some(&templates),
        &params.race_id,
        &params.class_id,
        &core_attributes,
    );

    // Apply any selected skills
    if let Some(selected) = params.selected_skills {
        for s in selected {
            learned_skills.grant(&s);
        }
    }

    let starting_gold = crate::login::class_starting_gold(Some(&templates), &params.class_id);

    let mana = oxide_core::Mana::from_formula(1, attrs.intelligence as u16, attrs.wisdom as u16);
    let stamina =
        oxide_core::Stamina::from_formula(1, attrs.strength as u16, attrs.dexterity as u16);

    // Collect list of granted skills
    let auto_skills = learned_skills.skills.keys().cloned().collect();

    Ok(Json(SimulateResponse {
        attributes: attrs,
        hp,
        mana: mana.max,
        stamina: stamina.max,
        starting_gold: WalletJson {
            copper: starting_gold.copper,
            silver: starting_gold.silver,
            gold: starting_gold.gold,
            platinum: starting_gold.platinum,
        },
        auto_skills,
    }))
}

fn load_player_data_from_db(
    db: &oxide_data::Database,
    name: &str,
) -> Result<serde_json::Value, (StatusCode, String)> {
    let conn = db.conn();
    let char_row = match oxide_data::get_character_by_name(conn, name) {
        Ok(Some(row)) => row,
        Ok(None) => {
            return Err((
                StatusCode::NOT_FOUND,
                format!("Character '{}' not found", name),
            ))
        }
        Err(e) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Database error: {e}"),
            ))
        }
    };

    let entity_id = char_row.entity_id;

    let attrs = oxide_data::load_attributes_component(conn, entity_id)
        .ok()
        .flatten()
        .unwrap_or(oxide_data::AttributesRow {
            strength: 10,
            dexterity: 10,
            intelligence: 10,
            wisdom: 10,
            constitution: 10,
            charisma: 10,
        });

    let hp = oxide_data::load_health_component(conn, entity_id)
        .ok()
        .flatten()
        .unwrap_or((20, 20));

    let mana_current = oxide_data::load_mana_component(conn, entity_id)
        .ok()
        .flatten()
        .unwrap_or(0);

    let stamina_current = oxide_data::load_stamina_component(conn, entity_id)
        .ok()
        .flatten()
        .unwrap_or(0);

    let level = oxide_data::load_level_component(conn, entity_id)
        .ok()
        .flatten()
        .unwrap_or(1);

    let xp = oxide_data::load_experience_component(conn, entity_id)
        .ok()
        .flatten()
        .unwrap_or(0);

    let alignment = oxide_data::load_alignment_component(conn, entity_id)
        .ok()
        .flatten()
        .unwrap_or_default();

    let _gold = oxide_data::load_golds_component(conn, entity_id)
        .ok()
        .flatten()
        .unwrap_or((0, 0, 0, 0));

    let skills_map = oxide_data::load_skills(conn, entity_id).unwrap_or_default();

    let quest_log = oxide_data::load_quest_log_component(conn, entity_id)
        .ok()
        .flatten()
        .map(|json| serde_json::from_str::<oxide_core::QuestLog>(&json).unwrap_or_default())
        .unwrap_or_default();

    let completed_quests: Vec<String> = quest_log.completed.iter().cloned().collect();

    let faction_standing = oxide_data::load_faction_standing_component(conn, entity_id)
        .ok()
        .flatten()
        .map(|json| serde_json::from_str::<oxide_core::FactionStanding>(&json).unwrap_or_default())
        .unwrap_or_default();

    let inv_rows = oxide_data::load_inventory(conn, entity_id).unwrap_or_default();
    let mut inventory = Vec::new();
    for (item_db_id, _) in inv_rows {
        if let Ok(Some(template_id)) = oxide_data::load_item_component(conn, item_db_id) {
            inventory.push(template_id);
        }
    }

    let eq_rows = oxide_data::load_equipment(conn, entity_id).unwrap_or_default();
    let mut equipment = std::collections::HashMap::new();
    for (slot_str, item_db_id) in eq_rows {
        if let Ok(Some(template_id)) = oxide_data::load_item_component(conn, item_db_id) {
            equipment.insert(slot_str, template_id);
        }
    }

    Ok(serde_json::json!({
        "name": char_row.name,
        "race_id": char_row.race,
        "class_id": char_row.class,
        "level": level,
        "experience": xp,
        "alignment": alignment,
        "attributes": {
            "strength": attrs.strength,
            "dexterity": attrs.dexterity,
            "intelligence": attrs.intelligence,
            "wisdom": attrs.wisdom,
            "constitution": attrs.constitution,
            "charisma": attrs.charisma
        },
        "health": {
            "current": hp.0,
            "max": hp.1
        },
        "mana": {
            "current": mana_current,
            "max": oxide_core::Mana::from_formula(level as u16, attrs.intelligence as u16, attrs.wisdom as u16).max
        },
        "stamina": {
            "current": stamina_current,
            "max": oxide_core::Stamina::from_formula(level as u16, attrs.strength as u16, attrs.dexterity as u16).max
        },
        "skills": skills_map,
        "completed_quests": completed_quests,
        "faction_standings": faction_standing.standings,
        "inventory": inventory,
        "equipment": equipment
    }))
}

async fn get_character_state(
    Path(name): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let db_lock = crate::get_db().ok_or((
        StatusCode::INTERNAL_SERVER_ERROR,
        "Database unavailable".to_string(),
    ))?;
    let db = db_lock.lock().await;
    load_player_data_from_db(&db, &name).map(Json)
}

const POINT_BUY_COST: [u8; 11] = [1, 1, 1, 1, 1, 2, 2, 3, 3, 4, 4];
fn point_buy_cost(current: u8) -> Option<u8> {
    if !(8..18).contains(&current) {
        return None;
    }
    Some(POINT_BUY_COST[(current - 8) as usize])
}

fn validate_attributes(attrs: &AttributesJson) -> Result<(), String> {
    // 1. Check standard array (any permutation of [15, 14, 13, 12, 10, 8])
    let mut vals = [
        attrs.strength,
        attrs.dexterity,
        attrs.intelligence,
        attrs.wisdom,
        attrs.constitution,
        attrs.charisma,
    ];
    vals.sort();
    let expected_array = [8, 10, 12, 13, 14, 15];
    if vals == expected_array {
        return Ok(());
    }

    // 2. Check Point-Buy
    for &v in &vals {
        if !(8..=18).contains(&v) {
            return Err(format!("Stat value {v} must be between 8 and 18."));
        }
    }

    let mut total_cost = 0;
    for &v in &vals {
        let mut current = 8;
        let mut cost = 0;
        while current < v {
            cost += point_buy_cost(current).ok_or_else(|| "Invalid stat value".to_string())?;
            current += 1;
        }
        total_cost += cost;
    }

    if total_cost != 27 {
        return Err(format!(
            "Base attributes must match either the Standard Array or Point-Buy with exactly 27 points spent (spent: {total_cost})."
        ));
    }

    Ok(())
}

async fn imm_put_item(
    Json(params): Json<PutItemParams>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let templates = crate::get_templates().ok_or((
        StatusCode::INTERNAL_SERVER_ERROR,
        "Templates registry unavailable".to_string(),
    ))?;
    let world_lock = crate::get_world().ok_or((
        StatusCode::INTERNAL_SERVER_ERROR,
        "World unavailable".to_string(),
    ))?;
    let registry_lock = crate::get_registry().ok_or((
        StatusCode::INTERNAL_SERVER_ERROR,
        "Connection registry unavailable".to_string(),
    ))?;

    // Check item exists
    let item_def = templates.items.get(&params.item_template_id).ok_or((
        StatusCode::BAD_REQUEST,
        format!("Item template '{}' not found", params.item_template_id),
    ))?;

    let mut world = world_lock.lock().await;

    // Find player entity
    let mut player_raw = None;
    for (entity, (name_comp,)) in world.query::<(&oxide_core::Name,)>().iter() {
        if name_comp.0.to_lowercase() == params.player_name.to_lowercase() {
            player_raw = Some(entity);
            break;
        }
    }

    let player_raw_entity = player_raw.ok_or((
        StatusCode::NOT_FOUND,
        format!("Player '{}' not found", params.player_name),
    ))?;
    let player_entity = oxide_core::Entity::from(player_raw_entity);

    // Spawn item using spawn_loot_item
    let spawn = oxide_core::systems::loot::ItemSpawn {
        template_id: params.item_template_id.clone(),
        count: params.count.unwrap_or(1) as u8,
        quality: oxide_core::systems::loot::QualityTier::Common,
        prefix_ids: vec![],
        suffix_ids: vec![],
    };

    let item_entity = oxide_core::systems::loot::spawn_loot_item(&mut world, &spawn, &templates)
        .ok_or((
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to spawn item entity".to_string(),
        ))?;

    // Add to player's inventory
    let mut added = false;
    if let Ok(mut q) = world.query_one::<&mut oxide_core::Inventory>(player_entity) {
        if let Some(inv) = q.get() {
            inv.0.push(item_entity);
            added = true;
        }
    }

    if !added {
        return Err((
            StatusCode::BAD_REQUEST,
            "Player does not have an inventory component".to_string(),
        ));
    }

    // Send line to player
    let reg = registry_lock.lock().await;
    if let Some(tx) = reg.sender(player_entity) {
        let msg = format!(
            "\r\nAn immortal has placed {} in your inventory!\r\n",
            item_def.name
        );
        let _ = tx.send(msg.into_bytes());
    }

    Ok(Json(serde_json::json!({
        "success": true,
        "message": format!("Placed {} (x{}) in {}'s inventory.", item_def.name, params.count.unwrap_or(1), params.player_name)
    })))
}

async fn imm_teleport(
    Json(params): Json<TeleportParams>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let templates = crate::get_templates().ok_or((
        StatusCode::INTERNAL_SERVER_ERROR,
        "Templates registry unavailable".to_string(),
    ))?;
    let world_lock = crate::get_world().ok_or((
        StatusCode::INTERNAL_SERVER_ERROR,
        "World unavailable".to_string(),
    ))?;
    let registry_lock = crate::get_registry().ok_or((
        StatusCode::INTERNAL_SERVER_ERROR,
        "Connection registry unavailable".to_string(),
    ))?;

    let world = world_lock.lock().await;

    // Find player entity
    let mut player_raw = None;
    for (entity, (name_comp,)) in world.query::<(&oxide_core::Name,)>().iter() {
        if name_comp.0.to_lowercase() == params.player_name.to_lowercase() {
            player_raw = Some(entity);
            break;
        }
    }
    let player_raw_entity = player_raw.ok_or((
        StatusCode::NOT_FOUND,
        format!("Player '{}' not found", params.player_name),
    ))?;
    let player_entity = oxide_core::Entity::from(player_raw_entity);

    // Find room entity
    let target_room = templates
        .find_room_by_key(&world, &params.room_key)
        .ok_or((
            StatusCode::BAD_REQUEST,
            format!("Room key '{}' not found", params.room_key),
        ))?;

    // Perform teleportation
    let mut current_pos = None;
    if let Ok(mut q) = world.query_one::<&mut oxide_core::Position>(player_entity) {
        if let Some(pos) = q.get() {
            current_pos = Some(pos.room);
            pos.room = target_room;
        }
    }

    let current_room = current_pos.ok_or((
        StatusCode::INTERNAL_SERVER_ERROR,
        "Player position component not found".to_string(),
    ))?;

    let reg = registry_lock.lock().await;

    // Broadcast departure
    let player_name_str = if let Ok(mut q) = world.query_one::<&oxide_core::Name>(player_entity) {
        q.get()
            .map(|n| n.0.clone())
            .unwrap_or_else(|| "Someone".to_string())
    } else {
        "Someone".to_string()
    };

    reg.broadcast_to_room(
        &world,
        current_room,
        &format!("\r\n{} disappears in a puff of smoke.\r\n", player_name_str),
        Some(player_entity),
    );
    // Broadcast arrival
    reg.broadcast_to_room(
        &world,
        target_room,
        &format!("\r\n{} arrives in a puff of smoke.\r\n", player_name_str),
        Some(player_entity),
    );

    // Notify player and force look command
    if let Some(tx) = reg.sender(player_entity) {
        let msg = b"\r\nYou disappear in a puff of smoke and reappear elsewhere...\r\n";
        let _ = tx.send(msg.to_vec());
        drop(reg);
        drop(world);
        let _ = execute_forced_command(player_entity, "look").await;
    }

    Ok(Json(serde_json::json!({
        "success": true,
        "message": format!("Teleported {} to room key '{}'.", params.player_name, params.room_key)
    })))
}

async fn imm_force_command(
    Json(params): Json<ForceCommandParams>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let world_lock = crate::get_world().ok_or((
        StatusCode::INTERNAL_SERVER_ERROR,
        "World unavailable".to_string(),
    ))?;

    let world = world_lock.lock().await;
    // Find player entity
    let mut player_raw = None;
    for (entity, (name_comp,)) in world.query::<(&oxide_core::Name,)>().iter() {
        if name_comp.0.to_lowercase() == params.player_name.to_lowercase() {
            player_raw = Some(entity);
            break;
        }
    }
    let player_raw_entity = player_raw.ok_or((
        StatusCode::NOT_FOUND,
        format!("Player '{}' not found", params.player_name),
    ))?;
    let player_entity = oxide_core::Entity::from(player_raw_entity);
    drop(world);

    execute_forced_command(player_entity, &params.command)
        .await
        .map_err(|e| (e, "Failed to execute forced command".to_string()))?;

    Ok(Json(serde_json::json!({
        "success": true,
        "message": format!("Forced {} to run command '{}'.", params.player_name, params.command)
    })))
}

async fn execute_forced_command(
    player: oxide_core::Entity,
    command_text: &str,
) -> Result<(), StatusCode> {
    let world_lock = crate::get_world().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let registry_lock = crate::get_registry().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let command_dispatch = crate::get_commands().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut world = world_lock.lock().await;
    let reg = registry_lock.lock().await;
    let sender = reg.sender(player).cloned();

    struct ApiForceConnection {
        entity: oxide_core::Entity,
        tx: Option<tokio::sync::mpsc::UnboundedSender<Vec<u8>>>,
    }

    impl crate::Connection for ApiForceConnection {
        fn send(&mut self, text: &str) {
            if let Some(ref tx) = self.tx {
                let mut bytes = text.as_bytes().to_vec();
                if !bytes.ends_with(b"\n") {
                    bytes.extend_from_slice(b"\r\n");
                }
                let _ = tx.send(bytes);
            }
        }
        fn send_line(&mut self, text: &str) {
            self.send(text);
        }
        fn send_raw(&mut self, bytes: &[u8]) {
            if let Some(ref tx) = self.tx {
                let _ = tx.send(bytes.to_vec());
            }
        }
        fn id(&self) -> u64 {
            999999
        }
        fn entity(&self) -> Option<oxide_core::Entity> {
            Some(self.entity)
        }
        fn set_entity(&mut self, _entity: oxide_core::Entity) {}
        fn disconnect(&mut self) {}
        fn is_disconnected(&self) -> bool {
            false
        }
        fn flags(&self) -> crate::ConnectionFlags {
            crate::ConnectionFlags::new()
        }
        fn set_flags(&mut self, _flags: crate::ConnectionFlags) {}
        fn output_sender(&self) -> Option<tokio::sync::mpsc::UnboundedSender<Vec<u8>>> {
            self.tx.clone()
        }
    }

    let mut conn = ApiForceConnection {
        entity: player,
        tx: sender,
    };

    command_dispatch.execute(&mut world, &mut conn, command_text, &reg);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_player_data_from_db() {
        let db = oxide_data::Database::open_in_memory().unwrap();
        let conn = db.conn();

        conn.execute(
            "INSERT INTO accounts (id, username, password_hash, access_level) VALUES (1, 'legolas_acc', 'hash', 'player')",
            [],
        )
        .unwrap();

        conn.execute("INSERT INTO entities (id, type) VALUES (100, 'player')", [])
            .unwrap();

        conn.execute(
            "INSERT INTO characters (id, account_id, name, race, class, gender, pronoun_subject, pronoun_object, pronoun_possessive, level, experience, entity_id, spawn_key, current_room_key, recall_room_key) \
             VALUES (1, 1, 'Legolas', 'elf', 'ranger', 'male', 'he', 'him', 'his', 10, 25000, 100, 'room_1', 'room_1', 'room_1')",
            [],
        )
        .unwrap();

        let attrs = oxide_data::AttributesRow {
            strength: 12,
            dexterity: 18,
            intelligence: 11,
            wisdom: 14,
            constitution: 12,
            charisma: 10,
        };
        oxide_data::save_attributes_component(conn, 100, &attrs).unwrap();

        oxide_data::save_health_component(conn, 100, 85, 85).unwrap();
        oxide_data::save_level_component(conn, 100, 10).unwrap();
        oxide_data::save_experience_component(conn, 100, 25000).unwrap();

        let mut skills = std::collections::HashMap::new();
        skills.insert("archery".to_string(), 45);
        oxide_data::save_skills(conn, 100, &skills).unwrap();

        let res = load_player_data_from_db(&db, "Legolas").unwrap();

        assert_eq!(res.get("name").and_then(|v| v.as_str()), Some("Legolas"));
        assert_eq!(res.get("race_id").and_then(|v| v.as_str()), Some("elf"));
        assert_eq!(res.get("class_id").and_then(|v| v.as_str()), Some("ranger"));
        assert_eq!(res.get("level").and_then(|v| v.as_u64()), Some(10));
        assert_eq!(res.get("experience").and_then(|v| v.as_u64()), Some(25000));
        assert_eq!(
            res.pointer("/attributes/strength").and_then(|v| v.as_u64()),
            Some(12)
        );
        assert_eq!(
            res.pointer("/attributes/dexterity")
                .and_then(|v| v.as_u64()),
            Some(18)
        );
        assert_eq!(
            res.pointer("/health/current").and_then(|v| v.as_u64()),
            Some(85)
        );
        assert_eq!(
            res.pointer("/health/max").and_then(|v| v.as_u64()),
            Some(85)
        );
        assert_eq!(
            res.pointer("/skills/archery").and_then(|v| v.as_u64()),
            Some(45)
        );

        let fail_res = load_player_data_from_db(&db, "Nobody");
        assert!(fail_res.is_err());
        assert_eq!(fail_res.unwrap_err().0, StatusCode::NOT_FOUND);
    }
}
