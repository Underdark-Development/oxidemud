use axum::{
    extract::{
        ws::{Message as AxumWsMessage, WebSocket, WebSocketUpgrade},
        Path, Request,
    },
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
use crate::connection::{Connection, WsConnection};
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
    #[serde(default)]
    confirm: bool,
}

#[derive(Debug, serde::Deserialize)]
struct SetStatParams {
    player_name: String,
    strength: Option<u8>,
    dexterity: Option<u8>,
    intelligence: Option<u8>,
    wisdom: Option<u8>,
    constitution: Option<u8>,
    charisma: Option<u8>,
    hp: Option<i32>,
    mana: Option<u16>,
    stamina: Option<u16>,
    level: Option<u8>,
    xp: Option<u64>,
}

#[derive(Debug, serde::Deserialize)]
struct LoadMobParams {
    room_key: String,
    mob_template_id: String,
}

#[derive(Debug, serde::Deserialize)]
struct LoadItemParams {
    room_key: String,
    item_template_id: String,
    count: Option<u32>,
}

#[derive(Debug, serde::Deserialize)]
struct GechoParams {
    message: String,
}

#[derive(Debug, serde::Deserialize)]
struct AdvanceParams {
    player_name: String,
    target_level: u8,
}

#[derive(Debug, serde::Deserialize)]
struct StatParams {
    target_name: String,
}

#[derive(Debug, serde::Deserialize)]
struct HealParams {
    target_name: String,
}

#[derive(Debug, serde::Deserialize)]
struct DamageParams {
    target_name: String,
    amount: i32,
}

#[derive(Debug, serde::Deserialize)]
struct KillParams {
    target_name: String,
    #[serde(default)]
    confirm: bool,
}

#[derive(Debug, serde::Deserialize)]
struct ReviveParams {
    target_name: String,
}

#[derive(Debug, serde::Deserialize)]
struct SetAlignmentParams {
    player_name: String,
    alignment: String,
}

#[derive(Debug, serde::Deserialize)]
struct SetFactionParams {
    player_name: String,
    faction_id: String,
    standing: i32,
}

#[derive(Debug, serde::Deserialize)]
struct PurgeRoomParams {
    room_key: String,
    #[serde(default)]
    confirm: bool,
}

#[derive(Debug, serde::Deserialize)]
struct RebootParams {
    #[serde(default)]
    confirm: bool,
    delay_secs: Option<u64>,
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

    // Check TLS & loopback security rules
    let ip = addr.ip();
    let allow_insecure = config.tls.allow_insecure_http.unwrap_or(false);
    let has_tls_config = config.tls.cert_path.is_some()
        || config.tls.acme_domain.is_some()
        || config.tls.auto_dev_cert.unwrap_or(false);

    if !ip.is_loopback() && !has_tls_config && !allow_insecure {
        return Err(
            "Security Error: Plain HTTP/WS is disabled on non-loopback interfaces without TLS configuration or explicit allow_insecure_http setting.".into(),
        );
    }

    if !ip.is_loopback() && !has_tls_config {
        tracing::warn!(
            "API server bound to a public interface ({}) with allow_insecure_http=true. \
             It is highly recommended to enable TLS or run behind a TLS-terminating reverse proxy.",
            bind_addr
        );
    }

    let app = Router::new()
        .route("/ws/play", get(ws_play_handler))
        .route("/ws/spade", get(ws_spade_handler))
        .route("/ws/mcp", get(ws_mcp_handler))
        .route("/api/players", get(list_players))
        .route("/api/character/simulate", post(simulate_character))
        .route("/api/character/:name", get(get_character_state))
        .route("/api/imm/put_item", post(imm_put_item))
        .route("/api/imm/teleport", post(imm_teleport))
        .route("/api/imm/force_command", post(imm_force_command))
        .route("/api/imm/set_stat", post(imm_set_stat))
        .route("/api/imm/load_mob", post(imm_load_mob))
        .route("/api/imm/load_item", post(imm_load_item))
        .route("/api/imm/gecho", post(imm_gecho))
        .route("/api/imm/advance", post(imm_advance))
        .route("/api/imm/stat", post(imm_stat))
        .route("/api/imm/heal", post(imm_heal))
        .route("/api/imm/damage", post(imm_damage))
        .route("/api/imm/kill", post(imm_kill))
        .route("/api/imm/revive", post(imm_revive))
        .route("/api/imm/set_alignment", post(imm_set_alignment))
        .route("/api/imm/set_faction", post(imm_set_faction))
        .route("/api/imm/purge_room", post(imm_purge_room))
        .route("/api/imm/reboot", post(imm_reboot))
        .layer(middleware::from_fn(auth_middleware));

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("REST API & WebSocket server listening on {}", bind_addr);

    let graceful = async move {
        let _ = shutdown_rx.changed().await;
        tracing::info!("REST API & WebSocket server shutting down gracefully");
    };

    axum::serve(listener, app)
        .with_graceful_shutdown(graceful)
        .await?;

    Ok(())
}

async fn ws_play_handler(ws: WebSocketUpgrade) -> Response {
    ws.on_upgrade(handle_ws_play_socket)
}

async fn handle_ws_play_socket(mut socket: WebSocket) {
    let conn_id = format!("ws-{}", uuid::Uuid::new_v4());
    let (mut conn, mut rx) = WsConnection::new(conn_id.clone());

    tracing::info!("New WebSocket player connection established: {}", conn_id);

    let welcome = format!(
        "Welcome to OxideMUD! Connected via WebSocket ({})\r\n",
        conn_id
    );
    let _ = socket.send(AxumWsMessage::Text(welcome)).await;

    loop {
        tokio::select! {
            Some(bytes) = rx.recv() => {
                if let Ok(text) = String::from_utf8(bytes) {
                    if socket.send(AxumWsMessage::Text(text)).await.is_err() {
                        break;
                    }
                }
            }
            res = socket.recv() => {
                match res {
                    Some(Ok(AxumWsMessage::Text(text))) => {
                        let trimmed = text.trim();
                        if !trimmed.is_empty() {
                            tracing::debug!("Received WS command from {}: {}", conn_id, trimmed);
                            conn.send_line(&format!("Received command: {}", trimmed));
                        }
                    }
                    Some(Ok(AxumWsMessage::Close(_))) | None => break,
                    _ => {}
                }
            }
        }
    }

    tracing::info!("WebSocket player connection closed: {}", conn_id);
}

async fn ws_spade_handler(ws: WebSocketUpgrade) -> Response {
    ws.on_upgrade(|mut socket| async move {
        tracing::info!("New Spade WebSocket session established");
        let greeting = serde_json::json!({
            "status": "connected",
            "mode": "online",
            "service": "spade"
        })
        .to_string();
        let _ = socket.send(AxumWsMessage::Text(greeting)).await;

        while let Some(res) = socket.recv().await {
            match res {
                Ok(AxumWsMessage::Text(txt)) if txt.trim() == "ping" => {
                    let _ = socket.send(AxumWsMessage::Text("pong".into())).await;
                }
                Ok(AxumWsMessage::Close(_)) | Err(_) => break,
                _ => {}
            }
        }
    })
}

async fn ws_mcp_handler(ws: WebSocketUpgrade) -> Response {
    ws.on_upgrade(|mut socket| async move {
        tracing::info!("New MCP WebSocket session established");
        let greeting = serde_json::json!({
            "status": "connected",
            "service": "mcp",
            "transport": "websocket"
        })
        .to_string();
        let _ = socket.send(AxumWsMessage::Text(greeting)).await;

        while let Some(res) = socket.recv().await {
            match res {
                Ok(AxumWsMessage::Text(txt)) if txt.trim() == "ping" => {
                    let _ = socket.send(AxumWsMessage::Text("pong".into())).await;
                }
                Ok(AxumWsMessage::Close(_)) | Err(_) => break,
                _ => {}
            }
        }
    })
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

    let (_account_id, username, access_level) =
        match oxide_data::validate_api_key(db.conn(), token, Some("mcp")) {
            Ok(Some(info)) => info,
            _ => return Err(StatusCode::UNAUTHORIZED),
        };

    // Log the request with username (masking key)
    let method = request.method().clone();
    let path = request.uri().path().to_string();
    tracing::info!(
        "Received REST API call: {} {} (user: {})",
        method,
        path,
        username
    );

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
    let player_entity = player_raw_entity;

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
    let player_entity = player_raw_entity;

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
    if !params.confirm {
        return Err((
            StatusCode::BAD_REQUEST,
            "This is a destructive operation. Set `confirm` to true to proceed.".to_string(),
        ));
    }

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
    let player_entity = player_raw_entity;
    drop(world);

    execute_forced_command(player_entity, &params.command)
        .await
        .map_err(|e| (e, "Failed to execute forced command".to_string()))?;

    Ok(Json(serde_json::json!({
        "success": true,
        "message": format!("Forced {} to run command '{}'.", params.player_name, params.command)
    })))
}

async fn imm_set_stat(
    Json(params): Json<SetStatParams>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let world_lock = crate::get_world().ok_or((
        StatusCode::INTERNAL_SERVER_ERROR,
        "World unavailable".to_string(),
    ))?;
    let mut world = world_lock.lock().await;

    let mut player_entity = None;
    for (entity, (name_comp,)) in world.query::<(&oxide_core::Name,)>().iter() {
        if name_comp.0.to_lowercase() == params.player_name.to_lowercase() {
            player_entity = Some(entity);
            break;
        }
    }
    let entity = player_entity.ok_or((
        StatusCode::NOT_FOUND,
        format!("Player '{}' not found", params.player_name),
    ))?;

    let mut updated = Vec::new();

    if params.strength.is_some()
        || params.dexterity.is_some()
        || params.intelligence.is_some()
        || params.wisdom.is_some()
        || params.constitution.is_some()
        || params.charisma.is_some()
    {
        if let Ok(mut q) = world.query_one::<&mut oxide_core::Attributes>(entity) {
            if let Some(attrs) = q.get() {
                if let Some(val) = params.strength {
                    attrs.strength = val;
                    updated.push(format!("strength: {}", val));
                }
                if let Some(val) = params.dexterity {
                    attrs.dexterity = val;
                    updated.push(format!("dexterity: {}", val));
                }
                if let Some(val) = params.intelligence {
                    attrs.intelligence = val;
                    updated.push(format!("intelligence: {}", val));
                }
                if let Some(val) = params.wisdom {
                    attrs.wisdom = val;
                    updated.push(format!("wisdom: {}", val));
                }
                if let Some(val) = params.constitution {
                    attrs.constitution = val;
                    updated.push(format!("constitution: {}", val));
                }
                if let Some(val) = params.charisma {
                    attrs.charisma = val;
                    updated.push(format!("charisma: {}", val));
                }
            }
        }
    }

    if let Some(hp) = params.hp {
        if let Ok(mut q) = world.query_one::<&mut oxide_core::Health>(entity) {
            if let Some(h) = q.get() {
                h.current = hp;
                updated.push(format!("hp: {}", hp));
            }
        }
    }

    if let Some(mana) = params.mana {
        if let Ok(mut q) = world.query_one::<&mut oxide_core::Mana>(entity) {
            if let Some(m) = q.get() {
                m.current = mana;
                updated.push(format!("mana: {}", mana));
            }
        }
    }

    if let Some(stamina) = params.stamina {
        if let Ok(mut q) = world.query_one::<&mut oxide_core::Stamina>(entity) {
            if let Some(s) = q.get() {
                s.current = stamina;
                updated.push(format!("stamina: {}", stamina));
            }
        }
    }

    if let Some(lvl) = params.level {
        if let Ok(mut q) = world.query_one::<&mut oxide_core::Level>(entity) {
            if let Some(l) = q.get() {
                l.0 = lvl;
                updated.push(format!("level: {}", lvl));
            }
        }
    }

    if let Some(xp) = params.xp {
        if let Ok(mut q) = world.query_one::<&mut oxide_core::Experience>(entity) {
            if let Some(x) = q.get() {
                x.0 = xp;
                updated.push(format!("xp: {}", xp));
            }
        }
    }

    let _ = world.insert(entity, (oxide_core::Dirty,));

    Ok(Json(serde_json::json!({
        "success": true,
        "message": format!("Updated stats for {}: {}", params.player_name, updated.join(", "))
    })))
}

async fn imm_load_mob(
    Json(params): Json<LoadMobParams>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let templates = crate::get_templates().ok_or((
        StatusCode::INTERNAL_SERVER_ERROR,
        "Templates registry unavailable".to_string(),
    ))?;
    let world_lock = crate::get_world().ok_or((
        StatusCode::INTERNAL_SERVER_ERROR,
        "World unavailable".to_string(),
    ))?;
    let mut world = world_lock.lock().await;

    let target_room = templates
        .find_room_by_key(&world, &params.room_key)
        .ok_or((
            StatusCode::BAD_REQUEST,
            format!("Room key '{}' not found", params.room_key),
        ))?;

    let mob_tpl = templates.mobs.get(&params.mob_template_id).ok_or((
        StatusCode::BAD_REQUEST,
        format!("Mob template '{}' not found", params.mob_template_id),
    ))?;

    let mob_entity = mob_tpl.spawn(&mut world, target_room, &templates);
    let _ = world.insert(mob_entity, (oxide_core::Dirty,));

    Ok(Json(serde_json::json!({
        "success": true,
        "message": format!("Spawned mob '{}' in room '{}'.", mob_tpl.name, params.room_key)
    })))
}

async fn imm_load_item(
    Json(params): Json<LoadItemParams>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let templates = crate::get_templates().ok_or((
        StatusCode::INTERNAL_SERVER_ERROR,
        "Templates registry unavailable".to_string(),
    ))?;
    let world_lock = crate::get_world().ok_or((
        StatusCode::INTERNAL_SERVER_ERROR,
        "World unavailable".to_string(),
    ))?;
    let mut world = world_lock.lock().await;

    let target_room = templates
        .find_room_by_key(&world, &params.room_key)
        .ok_or((
            StatusCode::BAD_REQUEST,
            format!("Room key '{}' not found", params.room_key),
        ))?;

    let item_def = templates.items.get(&params.item_template_id).ok_or((
        StatusCode::BAD_REQUEST,
        format!("Item template '{}' not found", params.item_template_id),
    ))?;

    let count = params.count.unwrap_or(1) as u8;
    let spawn = oxide_core::systems::loot::ItemSpawn {
        template_id: params.item_template_id.clone(),
        count,
        quality: oxide_core::systems::loot::QualityTier::Common,
        prefix_ids: vec![],
        suffix_ids: vec![],
    };

    let item_entity = oxide_core::systems::loot::spawn_loot_item(&mut world, &spawn, &templates)
        .ok_or((
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to spawn item entity".to_string(),
        ))?;

    let _ = world.insert(item_entity, (oxide_core::Position::new(target_room),));
    let _ = world.insert(item_entity, (oxide_core::Dirty,));

    Ok(Json(serde_json::json!({
        "success": true,
        "message": format!("Spawned item '{}' (x{}) in room '{}'.", item_def.name, count, params.room_key)
    })))
}

async fn imm_gecho(
    Json(params): Json<GechoParams>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let registry_lock = crate::get_registry().ok_or((
        StatusCode::INTERNAL_SERVER_ERROR,
        "Connection registry unavailable".to_string(),
    ))?;

    let reg = registry_lock.lock().await;
    let formatted_msg = format!("\r\n\x1b[1;33m[GLOBAL ECHO] {}\x1b[0m\r\n", params.message);
    reg.broadcast_all(&formatted_msg);

    Ok(Json(serde_json::json!({
        "success": true,
        "message": format!("Broadcasted global echo: '{}'", params.message)
    })))
}

async fn imm_advance(
    Json(params): Json<AdvanceParams>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let world_lock = crate::get_world().ok_or((
        StatusCode::INTERNAL_SERVER_ERROR,
        "World unavailable".to_string(),
    ))?;
    let mut world = world_lock.lock().await;

    let mut player_entity = None;
    for (entity, (name_comp,)) in world.query::<(&oxide_core::Name,)>().iter() {
        if name_comp.0.to_lowercase() == params.player_name.to_lowercase() {
            player_entity = Some(entity);
            break;
        }
    }
    let entity = player_entity.ok_or((
        StatusCode::NOT_FOUND,
        format!("Player '{}' not found", params.player_name),
    ))?;

    if let Ok(mut q) = world.query_one::<&mut oxide_core::Level>(entity) {
        if let Some(lvl) = q.get() {
            lvl.0 = params.target_level;
        }
    }
    let _ = world.insert(entity, (oxide_core::Dirty,));

    Ok(Json(serde_json::json!({
        "success": true,
        "message": format!("Advanced {} to level {}.", params.player_name, params.target_level)
    })))
}

async fn imm_stat(
    Json(params): Json<StatParams>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let world_lock = crate::get_world().ok_or((
        StatusCode::INTERNAL_SERVER_ERROR,
        "World unavailable".to_string(),
    ))?;
    let world = world_lock.lock().await;

    let mut target_entity = None;
    for (entity, (name_comp,)) in world.query::<(&oxide_core::Name,)>().iter() {
        if name_comp.0.to_lowercase() == params.target_name.to_lowercase() {
            target_entity = Some(entity);
            break;
        }
    }
    let entity = target_entity.ok_or((
        StatusCode::NOT_FOUND,
        format!("Target '{}' not found", params.target_name),
    ))?;

    let name = world
        .query_one::<&oxide_core::Name>(entity)
        .ok()
        .and_then(|mut q| q.get().map(|n| n.0.clone()))
        .unwrap_or_default();
    let level = world
        .query_one::<&oxide_core::Level>(entity)
        .ok()
        .and_then(|mut q| q.get().map(|l| l.0))
        .unwrap_or(1);
    let health = world
        .query_one::<&oxide_core::Health>(entity)
        .ok()
        .and_then(|mut q| {
            q.get()
                .map(|h| serde_json::json!({"current": h.current, "max": h.max}))
        });
    let mana = world
        .query_one::<&oxide_core::Mana>(entity)
        .ok()
        .and_then(|mut q| {
            q.get()
                .map(|m| serde_json::json!({"current": m.current, "max": m.max}))
        });
    let stamina = world
        .query_one::<&oxide_core::Stamina>(entity)
        .ok()
        .and_then(|mut q| {
            q.get()
                .map(|s| serde_json::json!({"current": s.current, "max": s.max}))
        });
    let attrs = world
        .query_one::<&oxide_core::Attributes>(entity)
        .ok()
        .and_then(|mut q| {
            q.get().map(|a| {
                serde_json::json!({
                    "strength": a.strength, "dexterity": a.dexterity, "intelligence": a.intelligence,
                    "wisdom": a.wisdom, "constitution": a.constitution, "charisma": a.charisma
                })
            })
        });

    Ok(Json(serde_json::json!({
        "success": true,
        "target": name,
        "level": level,
        "health": health,
        "mana": mana,
        "stamina": stamina,
        "attributes": attrs
    })))
}

async fn imm_heal(
    Json(params): Json<HealParams>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let world_lock = crate::get_world().ok_or((
        StatusCode::INTERNAL_SERVER_ERROR,
        "World unavailable".to_string(),
    ))?;
    let mut world = world_lock.lock().await;

    let mut target_entity = None;
    for (entity, (name_comp,)) in world.query::<(&oxide_core::Name,)>().iter() {
        if name_comp.0.to_lowercase() == params.target_name.to_lowercase() {
            target_entity = Some(entity);
            break;
        }
    }
    let entity = target_entity.ok_or((
        StatusCode::NOT_FOUND,
        format!("Target '{}' not found", params.target_name),
    ))?;

    if let Ok(mut q) = world.query_one::<&mut oxide_core::Health>(entity) {
        if let Some(h) = q.get() {
            h.current = h.max;
        }
    }
    if let Ok(mut q) = world.query_one::<&mut oxide_core::Mana>(entity) {
        if let Some(m) = q.get() {
            m.current = m.max;
        }
    }
    if let Ok(mut q) = world.query_one::<&mut oxide_core::Stamina>(entity) {
        if let Some(s) = q.get() {
            s.current = s.max;
        }
    }
    let _ = world.insert(entity, (oxide_core::Dirty,));

    Ok(Json(serde_json::json!({
        "success": true,
        "message": format!("Fully healed target '{}'.", params.target_name)
    })))
}

async fn imm_damage(
    Json(params): Json<DamageParams>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let world_lock = crate::get_world().ok_or((
        StatusCode::INTERNAL_SERVER_ERROR,
        "World unavailable".to_string(),
    ))?;
    let mut world = world_lock.lock().await;

    let mut target_entity = None;
    for (entity, (name_comp,)) in world.query::<(&oxide_core::Name,)>().iter() {
        if name_comp.0.to_lowercase() == params.target_name.to_lowercase() {
            target_entity = Some(entity);
            break;
        }
    }
    let entity = target_entity.ok_or((
        StatusCode::NOT_FOUND,
        format!("Target '{}' not found", params.target_name),
    ))?;

    let mut new_hp = 0;
    if let Ok(mut q) = world.query_one::<&mut oxide_core::Health>(entity) {
        if let Some(h) = q.get() {
            h.current = (h.current - params.amount).max(0);
            new_hp = h.current;
        }
    }
    let _ = world.insert(entity, (oxide_core::Dirty,));

    Ok(Json(serde_json::json!({
        "success": true,
        "message": format!("Dealt {} damage to {}. Remaining HP: {}.", params.amount, params.target_name, new_hp)
    })))
}

async fn imm_kill(
    Json(params): Json<KillParams>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    if !params.confirm {
        return Err((
            StatusCode::BAD_REQUEST,
            "This is a destructive operation. Set `confirm` to true to proceed.".to_string(),
        ));
    }

    let world_lock = crate::get_world().ok_or((
        StatusCode::INTERNAL_SERVER_ERROR,
        "World unavailable".to_string(),
    ))?;
    let mut world = world_lock.lock().await;

    let mut target_entity = None;
    for (entity, (name_comp,)) in world.query::<(&oxide_core::Name,)>().iter() {
        if name_comp.0.to_lowercase() == params.target_name.to_lowercase() {
            target_entity = Some(entity);
            break;
        }
    }
    let entity = target_entity.ok_or((
        StatusCode::NOT_FOUND,
        format!("Target '{}' not found", params.target_name),
    ))?;

    if let Ok(mut q) = world.query_one::<&mut oxide_core::Health>(entity) {
        if let Some(h) = q.get() {
            h.current = 0;
        }
    }
    let _ = world.insert(entity, (oxide_core::Dirty,));

    Ok(Json(serde_json::json!({
        "success": true,
        "message": format!("Instantly killed target '{}'.", params.target_name)
    })))
}

async fn imm_revive(
    Json(params): Json<ReviveParams>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let world_lock = crate::get_world().ok_or((
        StatusCode::INTERNAL_SERVER_ERROR,
        "World unavailable".to_string(),
    ))?;
    let mut world = world_lock.lock().await;

    let mut target_entity = None;
    for (entity, (name_comp,)) in world.query::<(&oxide_core::Name,)>().iter() {
        if name_comp.0.to_lowercase() == params.target_name.to_lowercase() {
            target_entity = Some(entity);
            break;
        }
    }
    let entity = target_entity.ok_or((
        StatusCode::NOT_FOUND,
        format!("Target '{}' not found", params.target_name),
    ))?;

    if let Ok(mut q) = world.query_one::<&mut oxide_core::Health>(entity) {
        if let Some(h) = q.get() {
            h.current = h.max.max(1);
        }
    }
    let _ = world.insert(entity, (oxide_core::Dirty,));

    Ok(Json(serde_json::json!({
        "success": true,
        "message": format!("Revived target '{}'.", params.target_name)
    })))
}

async fn imm_set_alignment(
    Json(params): Json<SetAlignmentParams>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let world_lock = crate::get_world().ok_or((
        StatusCode::INTERNAL_SERVER_ERROR,
        "World unavailable".to_string(),
    ))?;
    let mut world = world_lock.lock().await;

    let mut player_entity = None;
    for (entity, (name_comp,)) in world.query::<(&oxide_core::Name,)>().iter() {
        if name_comp.0.to_lowercase() == params.player_name.to_lowercase() {
            player_entity = Some(entity);
            break;
        }
    }
    let entity = player_entity.ok_or((
        StatusCode::NOT_FOUND,
        format!("Player '{}' not found", params.player_name),
    ))?;

    if !oxide_core::Alignment::is_valid(&params.alignment) {
        return Err((
            StatusCode::BAD_REQUEST,
            format!(
                "Invalid alignment '{}'. Valid alignments are: {:?}",
                params.alignment,
                oxide_core::Alignment::ALL
            ),
        ));
    }

    let align = oxide_core::Alignment(params.alignment.clone());
    let mut updated_existing = false;
    {
        if let Ok(mut q) = world.query_one::<&mut oxide_core::Alignment>(entity) {
            if let Some(a) = q.get() {
                *a = align.clone();
                updated_existing = true;
            }
        }
    }
    if !updated_existing {
        let _ = world.insert(entity, (align,));
    }
    let _ = world.insert(entity, (oxide_core::Dirty,));

    Ok(Json(serde_json::json!({
        "success": true,
        "message": format!("Set alignment of {} to '{}'.", params.player_name, params.alignment)
    })))
}

async fn imm_set_faction(
    Json(params): Json<SetFactionParams>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let world_lock = crate::get_world().ok_or((
        StatusCode::INTERNAL_SERVER_ERROR,
        "World unavailable".to_string(),
    ))?;
    let mut world = world_lock.lock().await;

    let mut player_entity = None;
    for (entity, (name_comp,)) in world.query::<(&oxide_core::Name,)>().iter() {
        if name_comp.0.to_lowercase() == params.player_name.to_lowercase() {
            player_entity = Some(entity);
            break;
        }
    }
    let entity = player_entity.ok_or((
        StatusCode::NOT_FOUND,
        format!("Player '{}' not found", params.player_name),
    ))?;

    let mut updated_existing = false;
    {
        if let Ok(mut q) = world.query_one::<&mut oxide_core::FactionStanding>(entity) {
            if let Some(standings) = q.get() {
                standings.set_standing(&params.faction_id, params.standing);
                updated_existing = true;
            }
        }
    }
    if !updated_existing {
        let mut standings = oxide_core::FactionStanding::new();
        standings.set_standing(&params.faction_id, params.standing);
        let _ = world.insert(entity, (standings,));
    }
    let _ = world.insert(entity, (oxide_core::Dirty,));

    Ok(Json(serde_json::json!({
        "success": true,
        "message": format!("Set {}'s faction standing with '{}' to {}.", params.player_name, params.faction_id, params.standing)
    })))
}

async fn imm_purge_room(
    Json(params): Json<PurgeRoomParams>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    if !params.confirm {
        return Err((
            StatusCode::BAD_REQUEST,
            "This is a destructive operation. Set `confirm` to true to proceed.".to_string(),
        ));
    }

    let templates = crate::get_templates().ok_or((
        StatusCode::INTERNAL_SERVER_ERROR,
        "Templates registry unavailable".to_string(),
    ))?;
    let world_lock = crate::get_world().ok_or((
        StatusCode::INTERNAL_SERVER_ERROR,
        "World unavailable".to_string(),
    ))?;
    let mut world = world_lock.lock().await;

    let room_entity = templates
        .find_room_by_key(&world, &params.room_key)
        .ok_or((
            StatusCode::BAD_REQUEST,
            format!("Room key '{}' not found", params.room_key),
        ))?;

    let mut to_despawn = Vec::new();
    for (entity, (pos, _npc)) in world
        .query::<(&oxide_core::Position, &oxide_core::Npc)>()
        .iter()
    {
        if pos.room == room_entity {
            to_despawn.push(entity);
        }
    }
    let count = to_despawn.len();
    for e in to_despawn {
        let _ = world.despawn(e);
    }

    Ok(Json(serde_json::json!({
        "success": true,
        "message": format!("Purged {} NPC(s) from room '{}'.", count, params.room_key)
    })))
}

async fn imm_reboot(
    Json(params): Json<RebootParams>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    if !params.confirm {
        return Err((
            StatusCode::BAD_REQUEST,
            "This is a destructive operation. Set `confirm` to true to proceed.".to_string(),
        ));
    }

    let delay = params.delay_secs.unwrap_or(0);
    tracing::info!("Server reboot initiated via REST API in {} seconds", delay);

    tokio::spawn(async move {
        if delay > 0 {
            tokio::time::sleep(std::time::Duration::from_secs(delay)).await;
        }
        std::process::exit(0);
    });

    Ok(Json(serde_json::json!({
        "success": true,
        "message": format!("Server reboot initiated in {} second(s).", delay)
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
        fn id(&self) -> &str {
            "api"
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
