use axum::{
    extract::{
        ws::{Message as AxumWsMessage, WebSocket, WebSocketUpgrade},
        Extension, Path, Request,
    },
    http::{HeaderMap, StatusCode},
    middleware::{self, Next},
    response::Response,
    routing::{get, post},
    Json, Router,
};
use std::fs;
use std::net::SocketAddr;
use std::path::{Path as FsPath, PathBuf};
use tokio::sync::watch;
use tracing;

use crate::config::ApiConfig;
use crate::connection::{Connection, WsConnection};
use oxide_core::Attributes;
use oxide_ws_rpc::{Request as RpcRequest, Response as RpcResponse, RpcErrorBody};

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

/// Authenticated identity attached to requests by `auth_middleware` so
/// WebSocket handlers can enforce per-method RBAC.
#[derive(Clone)]
struct AuthedUser {
    username: String,
    access_level: String,
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
        .route("/ws/rpc", get(ws_rpc_handler))
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

        let mut interval = tokio::time::interval(std::time::Duration::from_secs(1));

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    let total_mem = sysinfo::System::new_all().total_memory();
                    let used_mem = sysinfo::System::new_all().used_memory();



                    let uptime_secs = crate::get_uptime_secs();

                    let wal_size_bytes = if let Some(db_lock) = crate::get_db() {
                        if let Ok(db) = db_lock.try_lock() {
                            match db.path() {
                                Some(path) => {
                                    let wal_path = format!("{}-wal", path.display());
                                    std::fs::metadata(&wal_path).map(|m| m.len()).unwrap_or(0)
                                }
                                None => 0,
                            }
                        } else {
                            0
                        }
                    } else {
                        0
                    };

                    let (room_count, mob_count, item_count, dirty_count, game_time_str, season_str, weather_str, players_info) = match crate::get_world() {
                        Some(w_lock) => {
                            if let Ok(w) = w_lock.try_lock() {
                                let rooms = w.query::<&oxide_core::RoomKey>().into_iter().count();
                                let mobs = w.query::<&oxide_core::Npc>().into_iter().count();
                                let items = w.query::<&oxide_core::Item>().into_iter().count();
                                let dirty = w.query::<&oxide_core::Dirty>().into_iter().count();

                                let (gt_str, s_str) = w.query::<&oxide_core::GameTime>().into_iter().next()
                                    .map(|(_, gt)| (format!("{}:00 {:?}", gt.hour, gt.period()), format!("{:?}", gt.season)))
                                    .unwrap_or_else(|| ("12:00 PM".into(), "Spring".into()));

                                let mut players = Vec::new();
                                for (entity, (_player, room_key, _db_id)) in w.query::<(&oxide_core::Player, &oxide_core::RoomKey, &oxide_core::DbId)>().iter() {
                                    let level = w.query_one::<&oxide_core::Level>(entity).ok().and_then(|mut q| q.get().copied()).map(|l| l.0).unwrap_or(1);
                                    let class_name = w.query_one::<&oxide_core::Class>(entity).ok().and_then(|mut q| q.get().cloned()).map(|c| c.0).unwrap_or_else(|| "Unknown".into());
                                    let race_name = w.query_one::<&oxide_core::Race>(entity).ok().and_then(|mut q| q.get().cloned()).map(|r| r.0).unwrap_or_else(|| "Human".into());
                                    let name = oxide_core::get_name(&w, entity).map(|n| n.0.clone()).unwrap_or_else(|| "Player".into());

                                    players.push(serde_json::json!({
                                        "name": name,
                                        "level": level,
                                        "class": class_name,
                                        "race": race_name,
                                        "room": room_key.0,
                                        "idle_secs": 0,
                                        "protocol": "Telnet"
                                    }));
                                }
                                (rooms, mobs, items, dirty, gt_str, s_str, "Clear".to_string(), players)
                            } else {
                                (0, 0, 0, 0, "12:00 PM".into(), "Spring".into(), "Clear".into(), Vec::new())
                            }
                        }
                        None => (0, 0, 0, 0, "12:00 PM".into(), "Spring".into(), "Clear".into(), Vec::new()),
                    };

                    let telemetry = serde_json::json!({
                        "status": "connected",
                        "uptime_secs": uptime_secs,
                        "memory_used_bytes": used_mem,
                        "total_memory_bytes": total_mem,
                        "wal_size_bytes": wal_size_bytes,
                        "dirty_entities": dirty_count,
                        "pulse_drift_ms": 0.0,
                        "room_count": room_count,
                        "mob_count": mob_count,
                        "item_count": item_count,
                        "game_time": game_time_str,
                        "season": season_str,
                        "weather": weather_str,
                        "rhai_timers": 0,
                        "players": players_info,
                        "logs": []
                    });

                    if socket.send(AxumWsMessage::Text(telemetry.to_string())).await.is_err() {
                        break;
                    }
                }
                res = socket.recv() => {
                    match res {
                        Some(Ok(AxumWsMessage::Text(txt))) => {
                            let trimmed = txt.trim();
                            if trimmed == "ping" || trimmed.contains("\"Ping\"") {
                                let _ = socket.send(AxumWsMessage::Text("pong".into())).await;
                            }
                        }
                        Some(Ok(AxumWsMessage::Close(_))) | None | Some(Err(_)) => break,
                        _ => {}
                    }
                }
            }
        }
    })
}

/// JSON-RPC 2.0 error codes.
/// Standard codes per spec: parse -32700, invalid params -32602, method not
/// found -32601, internal -32603.
/// App-specific (custom range 32xxx, with method prefix codes):
const ERR_PARSE: i64 = -32700;
const ERR_INVALID_PARAMS: i64 = -32602;
const ERR_METHOD_NOT_FOUND: i64 = -32601;
const ERR_INTERNAL: i64 = -32603;
const ERR_FORBIDDEN: i64 = -32001;
const ERR_CONTENT_VALIDATION: i64 = -32002;
const ERR_CONTENT_NOT_CONFIGURED: i64 = -32003;
const ERR_CONFIRM_REQUIRED: i64 = -32004;

/// Upper bound (bytes) on a single `content.write` `content` string. Content
/// templates/scripts are far smaller; this caps per-message memory/disk use
/// from a hostile or buggy client without touching valid writes.
const MAX_CONTENT_BYTES: usize = 1024 * 1024;
/// Upper bound on the number of path components in a content-relative `path`.
const MAX_PATH_DEPTH: usize = 32;
/// Upper bound on the total length (chars) of a content-relative `path`.
const MAX_PATH_LEN: usize = 512;

/// JSON-RPC 2.0-over-WebSocket dispatcher at `/ws/rpc`.
///
/// The identity is attached by `auth_middleware` as an `AuthedUser` extension,
/// so per-method RBAC is enforced here (immortal-gated writes vs. read-only
/// queries available to any `mcp`-scoped API key).
async fn ws_rpc_handler(ws: WebSocketUpgrade, Extension(user): Extension<AuthedUser>) -> Response {
    ws.on_upgrade(move |socket: WebSocket| async move {
        tracing::info!(
            "New JSON-RPC WebSocket session established (user: {})",
            user.username
        );
        dispatch_loop(socket, user).await;
    })
}

async fn dispatch_loop(mut socket: WebSocket, user: AuthedUser) {
    loop {
        match socket.recv().await {
            Some(Ok(AxumWsMessage::Text(text))) => {
                let response = match serde_json::from_str::<RpcRequest>(&text) {
                    Ok(req) => handle_request(&req, &user).await,
                    // Malformed JSON / non-request frame. JSON-RPC 2.0 wants
                    // `id: null` for parse errors, but our `Response.id` is a
                    // `u64`; we use `0` as that sentinel (documented
                    // simplification).
                    Err(_) => build_parse_error_response(),
                };
                if let Ok(out) = serde_json::to_string(&response) {
                    let _ = socket.send(AxumWsMessage::Text(out)).await;
                }
            }
            Some(Ok(_)) => continue, // ping/pong/binary frames are ignored
            Some(Err(_)) | None => break,
        }
    }
}

/// Build the JSON-RPC parse-error response (`-32700`) for a malformed /
/// non-Request inbound frame. JSON-RPC 2.0 wants `id: null` for parse errors,
/// but our `Response.id` is a `u64`; we use `0` as that sentinel (documented
/// simplification). Factored out so the parse-error path is unit-testable
/// without constructing an axum `WebSocket`.
fn build_parse_error_response() -> RpcResponse {
    RpcResponse {
        jsonrpc: "2.0".to_string(),
        id: 0,
        result: None,
        error: Some(RpcErrorBody {
            code: ERR_PARSE,
            message: "parse error".to_string(),
            data: None,
        }),
    }
}

/// Dispatch a single parsed request, applying per-method RBAC, and build the
/// JSON-RPC response.
async fn handle_request(req: &RpcRequest, user: &AuthedUser) -> RpcResponse {
    let outcome = match req.method.as_str() {
        "ping" => Ok(serde_json::json!("pong")),
        "players.list" => players_list_method().await,
        "player.state" => player_state_method(req.params.clone()).await,
        "content.write" => {
            if let Err(e) = require_immortal(user) {
                Err(e)
            } else {
                content_write_method(user, req).await
            }
        }
        "content.delete" => {
            if let Err(e) = require_immortal(user) {
                Err(e)
            } else if let Err(e) = require_confirm(&req.params) {
                Err(e)
            } else {
                content_delete_method(user, req).await
            }
        }
        _ => {
            if let Some(op) = req.method.strip_prefix("imm.") {
                // All imm.* methods require immortal+ access.
                if let Err(e) = require_immortal(user) {
                    return rpc_response_error(req.id, e);
                }
                // Destructive ops additionally require `confirm: true`
                // (the core fns also enforce this — defense in depth).
                let params = req.params.clone();
                if matches!(op, "force_command" | "kill" | "purge_room" | "reboot") {
                    if let Err(e) = require_confirm(&params) {
                        return rpc_response_error(req.id, e);
                    }
                }
                imm_dispatch(op, params).await
            } else {
                Err(RpcErrorBody {
                    code: ERR_METHOD_NOT_FOUND,
                    message: format!("method not found: {}", req.method),
                    data: None,
                })
            }
        }
    };

    match outcome {
        Ok(value) => RpcResponse {
            jsonrpc: "2.0".to_string(),
            id: req.id,
            result: Some(value),
            error: None,
        },
        Err(e) => rpc_response_error(req.id, e),
    }
}

fn rpc_response_error(id: u64, e: RpcErrorBody) -> RpcResponse {
    RpcResponse {
        jsonrpc: "2.0".to_string(),
        id,
        result: None,
        error: Some(e),
    }
}

fn rpc_error(
    code: i64,
    message: impl Into<String>,
    data: Option<serde_json::Value>,
) -> RpcErrorBody {
    RpcErrorBody {
        code,
        message: message.into(),
        data,
    }
}

/// Require immortal-level access for writes and imm methods.
fn require_immortal(user: &AuthedUser) -> Result<(), RpcErrorBody> {
    let allowed = matches!(
        user.access_level.to_lowercase().as_str(),
        "immortal" | "god" | "admin"
    );
    if allowed {
        Ok(())
    } else {
        Err(rpc_error(
            ERR_FORBIDDEN,
            "forbidden: requires immortal access",
            None,
        ))
    }
}

/// Dispatch an `imm.<op>` method to the shared REST core logic. The core fns
/// were refactored to take typed params and return a raw JSON value, so both
/// the HTTP handlers and this dispatcher reuse the same implementation.
async fn imm_dispatch(
    op: &str,
    params: Option<serde_json::Value>,
) -> Result<serde_json::Value, RpcErrorBody> {
    macro_rules! dispatch {
        ($op:literal, $t:ty, $core:ident) => {
            if op == $op {
                let p: $t =
                    serde_json::from_value(params.clone().unwrap_or(serde_json::Value::Null))
                        .map_err(|e| {
                            rpc_error(ERR_INVALID_PARAMS, format!("invalid params: {e}"), None)
                        })?;
                return $core(p).await.map_err(|(sc, m)| rpc_status_error(sc, m));
            }
        };
    }

    dispatch!("put_item", PutItemParams, imm_put_item_core);
    dispatch!("teleport", TeleportParams, imm_teleport_core);
    dispatch!("force_command", ForceCommandParams, imm_force_command_core);
    dispatch!("set_stat", SetStatParams, imm_set_stat_core);
    dispatch!("load_mob", LoadMobParams, imm_load_mob_core);
    dispatch!("load_item", LoadItemParams, imm_load_item_core);
    dispatch!("gecho", GechoParams, imm_gecho_core);
    dispatch!("advance", AdvanceParams, imm_advance_core);
    dispatch!("stat", StatParams, imm_stat_core);
    dispatch!("heal", HealParams, imm_heal_core);
    dispatch!("damage", DamageParams, imm_damage_core);
    dispatch!("kill", KillParams, imm_kill_core);
    dispatch!("revive", ReviveParams, imm_revive_core);
    dispatch!("set_alignment", SetAlignmentParams, imm_set_alignment_core);
    dispatch!("set_faction", SetFactionParams, imm_set_faction_core);
    dispatch!("purge_room", PurgeRoomParams, imm_purge_room_core);
    dispatch!("reboot", RebootParams, imm_reboot_core);

    Err(rpc_error(
        ERR_METHOD_NOT_FOUND,
        format!("method not found: imm.{op}"),
        None,
    ))
}

fn rpc_status_error(code: StatusCode, msg: String) -> RpcErrorBody {
    let rpc_code = match code {
        StatusCode::BAD_REQUEST => ERR_INVALID_PARAMS,
        StatusCode::NOT_FOUND => ERR_INVALID_PARAMS,
        StatusCode::INTERNAL_SERVER_ERROR => ERR_INTERNAL,
        _ => ERR_INTERNAL,
    };
    rpc_error(rpc_code, msg, None)
}

/// `players.list` — mirrors REST `list_players`, any authenticated key.
async fn players_list_method() -> Result<serde_json::Value, RpcErrorBody> {
    list_players().await.map(|j| j.0).map_err(|e| {
        rpc_error(
            ERR_INTERNAL,
            "failed to list players",
            Some(serde_json::json!({ "status": e.as_u16() })),
        )
    })
}

/// `player.state` — mirrors REST `get_character_state` via the shared
/// `load_player_data_from_db` helper. Params `{ name }`.
async fn player_state_method(
    params: Option<serde_json::Value>,
) -> Result<serde_json::Value, RpcErrorBody> {
    let name = params
        .as_ref()
        .and_then(|p| p.get("name"))
        .and_then(|v| v.as_str())
        .map(str::to_string)
        .ok_or_else(|| {
            rpc_error(
                ERR_INVALID_PARAMS,
                "invalid params: missing string 'name'",
                None,
            )
        })?;
    let db_lock =
        crate::get_db().ok_or_else(|| rpc_error(ERR_INTERNAL, "database unavailable", None))?;
    let db = db_lock.lock().await;
    load_player_data_from_db(&db, &name).map_err(|(sc, m)| rpc_status_error(sc, m))
}

/// Destructive imm ops must carry `confirm: true` in params per JSON-RPC.
fn require_confirm(params: &Option<serde_json::Value>) -> Result<(), RpcErrorBody> {
    let ok = params
        .as_ref()
        .and_then(|p| p.get("confirm"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    if ok {
        Ok(())
    } else {
        Err(rpc_error(
            ERR_CONFIRM_REQUIRED,
            "confirmation required: set 'confirm' to true",
            None,
        ))
    }
}

/// Render a path relative to `base` for client-facing messages, so absolute
/// content-root or temp staging paths never leak back to a remote RPC caller.
/// Falls back to just the file name (never a full server path).
fn redacted_rel_path(path: &FsPath, base: &FsPath) -> String {
    path.strip_prefix(base)
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| {
            path.file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| "<path>".to_string())
        })
}

/// Defense-in-depth against a symlinked *final* path component. The
/// containment check canonicalizes an existing trailing directory, but a FILE
/// symlink as the last component is not canonicalized, so `fs::write`/unlink
/// through it could escape the content root. Reject it here (a *missing*
/// target is fine — it just gets created).
fn reject_final_symlink(resolved: &FsPath, rel: &str) -> Result<(), RpcErrorBody> {
    if fs::symlink_metadata(resolved)
        .map(|m| m.file_type().is_symlink())
        .unwrap_or(false)
    {
        return Err(rpc_error(
            ERR_CONTENT_VALIDATION,
            format!("refusing to operate through a symlink: {rel}"),
            None,
        ));
    }
    Ok(())
}

/// Copy a directory tree (used to build a throwaway staging area for the
/// content-validation gate). Error messages reference paths relative to `base`
/// (the content root) so no absolute server path is surfaced.
fn copy_dir_recursive(src: &FsPath, dst: &FsPath, base: &FsPath) -> Result<(), String> {
    fs::create_dir_all(dst).map_err(|e| format!("failed to create staging dir: {e}"))?;
    let entries = fs::read_dir(src).map_err(|e| format!("failed to read dir: {e}"))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("failed to read dir entry: {e}"))?;
        let ft = entry.file_type().map_err(|e| {
            format!(
                "failed to stat {}: {e}",
                redacted_rel_path(&entry.path(), base)
            )
        })?;
        let to = dst.join(entry.file_name());
        if ft.is_dir() {
            copy_dir_recursive(&entry.path(), &to, base)?;
        } else if ft.is_file() {
            fs::copy(entry.path(), &to).map_err(|e| {
                format!(
                    "failed to copy {}: {e}",
                    redacted_rel_path(&entry.path(), base)
                )
            })?;
        }
    }
    Ok(())
}

/// Resolve a user-supplied content-relative path against the content root,
/// rejecting absolute paths and `..` traversal, and verifying the resolved
/// path stays inside the content directory.
fn resolve_content_path(content_dir: &FsPath, rel: &str) -> Result<PathBuf, RpcErrorBody> {
    if rel.is_empty() {
        return Err(rpc_error(
            ERR_INVALID_PARAMS,
            "path must not be empty",
            None,
        ));
    }
    let p = FsPath::new(rel);
    if p.is_absolute() {
        return Err(rpc_error(
            ERR_INVALID_PARAMS,
            "path must be relative to the content directory",
            None,
        ));
    }
    if p.components()
        .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        return Err(rpc_error(
            ERR_INVALID_PARAMS,
            "path traversal is not allowed",
            None,
        ));
    }
    let resolved = content_dir.join(p);
    // Map the containment check's error (which embeds the absolute content
    // path) to a caller-relative message, so no server path leaks to the RPC
    // client. The server-side absolute path is still logged below.
    if let Err(e) = oxide_core::content::assert_within_content_dir(content_dir, &resolved) {
        tracing::error!(
            "Content path containment check failed for {rel}: {e} (resolved {:?})",
            resolved
        );
        return Err(rpc_error(
            ERR_INVALID_PARAMS,
            format!("path escapes the content directory: {rel}"),
            None,
        ));
    }
    Ok(resolved)
}

/// Cheap pre-write bounds for `content.write`: reject an oversized `content`
/// payload and an overdeep/overlong `path` before any heavy FS work. Pure and
/// unit-testable (no content path needed). Real templates, scripts, and their
/// paths are far smaller than these caps.
fn validate_content_write_bounds(rel: &str, content: &str) -> Result<(), RpcErrorBody> {
    if content.len() > MAX_CONTENT_BYTES {
        return Err(rpc_error(
            ERR_INVALID_PARAMS,
            format!(
                "content too large: {} bytes exceeds limit of {MAX_CONTENT_BYTES}",
                content.len()
            ),
            None,
        ));
    }
    if rel.chars().count() > MAX_PATH_LEN {
        return Err(rpc_error(
            ERR_INVALID_PARAMS,
            format!("path too long: exceeds limit of {MAX_PATH_LEN} chars"),
            None,
        ));
    }
    if FsPath::new(rel).components().count() > MAX_PATH_DEPTH {
        return Err(rpc_error(
            ERR_INVALID_PARAMS,
            "path has too many components",
            None,
        ));
    }
    Ok(())
}

/// `content.write` — params `{ path, content }`. Enforces a hard validation
/// gate: the proposed write is applied to a staged copy of the content tree,
/// which is parsed and validated (templates only; `.rhai` scripts are written
/// directly since they are not part of the template registry). On validation
/// failure the real tree is untouched.
async fn content_write_method(
    _user: &AuthedUser,
    req: &RpcRequest,
) -> Result<serde_json::Value, RpcErrorBody> {
    let rel = req
        .params
        .as_ref()
        .and_then(|p| p.get("path"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            rpc_error(
                ERR_INVALID_PARAMS,
                "invalid params: missing string 'path'",
                None,
            )
        })?;
    let content = req
        .params
        .as_ref()
        .and_then(|p| p.get("content"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            rpc_error(
                ERR_INVALID_PARAMS,
                "invalid params: missing string 'content'",
                None,
            )
        })?;

    // Cheap bounds before any heavy work: cap per-message payload size and
    // path depth/length so one hostile or buggy message cannot drive
    // unbounded memory/disk use (see `validate_content_write_bounds`).
    validate_content_write_bounds(rel, content)?;

    let content_dir = crate::get_content_path().ok_or_else(|| {
        rpc_error(
            ERR_CONTENT_NOT_CONFIGURED,
            "content path not configured",
            None,
        )
    })?;

    // All file-system work (resolve, symlink check, staging copy, registry
    // parse/validate, and the final write) is blocking and off the tokio
    // worker: the closure is `Send + 'static`, owns its inputs, and returns a
    // client-safe result. Errors already carry caller-relative paths only.
    let content_dir = content_dir.clone();
    let rel = rel.to_string();
    let content = content.to_string();
    tokio::task::spawn_blocking(move || write_content_sync(&content_dir, &rel, &content))
        .await
        .map_err(|e| {
            rpc_error(
                ERR_INTERNAL,
                format!("content write task failed: {e}"),
                None,
            )
        })?
        .map(|message| serde_json::json!({ "success": true, "message": message }))
}

/// Blocking half of `content.write` (run inside `spawn_blocking`): resolve the
/// path, reject a symlinked final component, then apply the write behind the
/// validation gate.
fn write_content_sync(
    content_dir: &FsPath,
    rel: &str,
    content: &str,
) -> Result<String, RpcErrorBody> {
    let resolved = resolve_content_path(content_dir, rel)?;
    reject_final_symlink(&resolved, rel)?;

    // Rhai scripts are not part of the template registry, so the template gate
    // does not apply; still write so the hot-reloader can compile it.
    if rel.ends_with(".rhai") {
        write_file_nested(&resolved, rel, content).map_err(|e| rpc_error(ERR_INTERNAL, e, None))?;
        return Ok(format!("written {rel}"));
    }

    // Build a staged copy of the content tree, apply the write, validate.
    // The contaminated (copy + validate) tree is never trusted.
    let staging = std::env::temp_dir().join(format!("oxide_ws_rpc_stage_{}", uuid::Uuid::new_v4()));
    if let Err(e) = copy_dir_recursive(content_dir, &staging, content_dir)
        .and_then(|_| write_file_nested(&staging.join(rel), rel, content))
    {
        let _ = fs::remove_dir_all(&staging);
        return Err(rpc_error(ERR_INTERNAL, e, None));
    }

    let report = oxide_core::content::load_registry_report(&staging);
    let validation_errors = report.registry.validate();
    let _ = fs::remove_dir_all(&staging);

    if !report.errors.is_empty() || !validation_errors.is_empty() {
        let mut errors = Vec::new();
        // Report paths are under the UUID-temp staging dir; surface them
        // relative to the caller, never the staging location.
        for e in &report.errors {
            errors.push(serde_json::json!({
                "category": e.category,
                "path": redacted_rel_path(e.path.as_path(), &staging),
                "message": e.message,
            }));
        }
        for e in &validation_errors {
            errors.push(serde_json::json!({
                "template_type": e.template_type,
                "template_id": e.template_id,
                "field": e.field,
                "message": e.message,
            }));
        }
        return Err(rpc_error(
            ERR_CONTENT_VALIDATION,
            "content validation failed",
            Some(serde_json::json!({ "errors": errors })),
        ));
    }

    // Validated: apply the write to the real content tree.
    write_file_nested(&resolved, rel, content).map_err(|e| rpc_error(ERR_INTERNAL, e, None))?;
    tracing::info!("Content write via /ws/rpc: {rel}");
    Ok(format!("written {rel}"))
}

/// Create parent directories and write a file atomically enough for the
/// hot-reloader (bytes then flush). Only ever called with an already
/// containment-checked path; error messages use the caller-relative
/// `display_name` so no absolute server/staging path leaks to the RPC client.
fn write_file_nested(path: &FsPath, display_name: &str, content: &str) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("failed to create parent dirs for {display_name}: {e}"))?;
    }
    fs::write(path, content).map_err(|e| format!("failed to write {display_name}: {e}"))
}

/// `content.delete` — params `{ path }`. Removes a template/script file inside
/// the content dir. No validation gate needed: deletion just unlinks the file
/// and the hot-reloader drops it.
async fn content_delete_method(
    _user: &AuthedUser,
    req: &RpcRequest,
) -> Result<serde_json::Value, RpcErrorBody> {
    let rel = req
        .params
        .as_ref()
        .and_then(|p| p.get("path"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            rpc_error(
                ERR_INVALID_PARAMS,
                "invalid params: missing string 'path'",
                None,
            )
        })?;
    let content_dir = crate::get_content_path().ok_or_else(|| {
        rpc_error(
            ERR_CONTENT_NOT_CONFIGURED,
            "content path not configured",
            None,
        )
    })?;

    // The symlink check and unlink are blocking; run off the tokio worker.
    let content_dir = content_dir.clone();
    let rel = rel.to_string();
    tokio::task::spawn_blocking(move || delete_content_sync(&content_dir, &rel))
        .await
        .map_err(|e| {
            rpc_error(
                ERR_INTERNAL,
                format!("content delete task failed: {e}"),
                None,
            )
        })?
        .map(|message| serde_json::json!({ "success": true, "message": message }))
}

/// Blocking half of `content.delete` (run inside `spawn_blocking`): refuse to
/// unlink through a symlinked final component, then unlink.
fn delete_content_sync(content_dir: &FsPath, rel: &str) -> Result<String, RpcErrorBody> {
    let resolved = resolve_content_path(content_dir, rel)?;
    reject_final_symlink(&resolved, rel)?;

    // `symlink_metadata` (not `metadata`): a trailing symlink was already
    // rejected above, so this never resolves through one; a missing file maps
    // to "not found".
    let md = fs::symlink_metadata(&resolved)
        .map_err(|_| rpc_error(ERR_INVALID_PARAMS, format!("file not found: {rel}"), None))?;
    if !md.is_file() {
        return Err(rpc_error(
            ERR_INVALID_PARAMS,
            "refusing to delete a directory",
            None,
        ));
    }
    fs::remove_file(&resolved)
        .map_err(|e| rpc_error(ERR_INTERNAL, format!("failed to delete {rel}: {e}"), None))?;
    tracing::info!("Content delete via /ws/rpc: {rel}");
    Ok(format!("deleted {rel}"))
}

async fn auth_middleware(
    headers: HeaderMap,
    mut request: Request,
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

    // Attach the authenticated identity so WebSocket handlers can enforce
    // per-method RBAC (e.g. /ws/rpc).
    request.extensions_mut().insert(AuthedUser {
        username: username.clone(),
        access_level: access_level.clone(),
    });

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

async fn imm_put_item_core(
    params: PutItemParams,
) -> Result<serde_json::Value, (StatusCode, String)> {
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

    Ok(serde_json::json!({
        "success": true,
        "message": format!("Placed {} (x{}) in {}'s inventory.", item_def.name, params.count.unwrap_or(1), params.player_name)
    }))
}

async fn imm_put_item(
    Json(params): Json<PutItemParams>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    Ok(Json(imm_put_item_core(params).await?))
}

async fn imm_teleport_core(
    params: TeleportParams,
) -> Result<serde_json::Value, (StatusCode, String)> {
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

    Ok(serde_json::json!({
        "success": true,
        "message": format!("Teleported {} to room key '{}'.", params.player_name, params.room_key)
    }))
}

async fn imm_teleport(
    Json(params): Json<TeleportParams>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    Ok(Json(imm_teleport_core(params).await?))
}

async fn imm_force_command_core(
    params: ForceCommandParams,
) -> Result<serde_json::Value, (StatusCode, String)> {
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

    Ok(serde_json::json!({
        "success": true,
        "message": format!("Forced {} to run command '{}'.", params.player_name, params.command)
    }))
}

async fn imm_force_command(
    Json(params): Json<ForceCommandParams>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    Ok(Json(imm_force_command_core(params).await?))
}

async fn imm_set_stat_core(
    params: SetStatParams,
) -> Result<serde_json::Value, (StatusCode, String)> {
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

    Ok(serde_json::json!({
        "success": true,
        "message": format!("Updated stats for {}: {}", params.player_name, updated.join(", "))
    }))
}

async fn imm_set_stat(
    Json(params): Json<SetStatParams>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    Ok(Json(imm_set_stat_core(params).await?))
}

async fn imm_load_mob_core(
    params: LoadMobParams,
) -> Result<serde_json::Value, (StatusCode, String)> {
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

    Ok(serde_json::json!({
        "success": true,
        "message": format!("Spawned mob '{}' in room '{}'.", mob_tpl.name, params.room_key)
    }))
}

async fn imm_load_mob(
    Json(params): Json<LoadMobParams>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    Ok(Json(imm_load_mob_core(params).await?))
}

async fn imm_load_item_core(
    params: LoadItemParams,
) -> Result<serde_json::Value, (StatusCode, String)> {
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

    Ok(serde_json::json!({
        "success": true,
        "message": format!("Spawned item '{}' (x{}) in room '{}'.", item_def.name, count, params.room_key)
    }))
}

async fn imm_load_item(
    Json(params): Json<LoadItemParams>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    Ok(Json(imm_load_item_core(params).await?))
}

async fn imm_gecho_core(params: GechoParams) -> Result<serde_json::Value, (StatusCode, String)> {
    let registry_lock = crate::get_registry().ok_or((
        StatusCode::INTERNAL_SERVER_ERROR,
        "Connection registry unavailable".to_string(),
    ))?;

    let reg = registry_lock.lock().await;
    let formatted_msg = format!("\r\n\x1b[1;33m[GLOBAL ECHO] {}\x1b[0m\r\n", params.message);
    reg.broadcast_all(&formatted_msg);

    Ok(serde_json::json!({
        "success": true,
        "message": format!("Broadcasted global echo: '{}'", params.message)
    }))
}

async fn imm_gecho(
    Json(params): Json<GechoParams>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    Ok(Json(imm_gecho_core(params).await?))
}

async fn imm_advance_core(
    params: AdvanceParams,
) -> Result<serde_json::Value, (StatusCode, String)> {
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

    Ok(serde_json::json!({
        "success": true,
        "message": format!("Advanced {} to level {}.", params.player_name, params.target_level)
    }))
}

async fn imm_advance(
    Json(params): Json<AdvanceParams>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    Ok(Json(imm_advance_core(params).await?))
}

async fn imm_stat_core(params: StatParams) -> Result<serde_json::Value, (StatusCode, String)> {
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

    Ok(serde_json::json!({
        "success": true,
        "target": name,
        "level": level,
        "health": health,
        "mana": mana,
        "stamina": stamina,
        "attributes": attrs
    }))
}

async fn imm_stat(
    Json(params): Json<StatParams>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    Ok(Json(imm_stat_core(params).await?))
}

async fn imm_heal_core(params: HealParams) -> Result<serde_json::Value, (StatusCode, String)> {
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

    Ok(serde_json::json!({
        "success": true,
        "message": format!("Fully healed target '{}'.", params.target_name)
    }))
}

async fn imm_heal(
    Json(params): Json<HealParams>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    Ok(Json(imm_heal_core(params).await?))
}

async fn imm_damage_core(params: DamageParams) -> Result<serde_json::Value, (StatusCode, String)> {
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

    Ok(serde_json::json!({
        "success": true,
        "message": format!("Dealt {} damage to {}. Remaining HP: {}.", params.amount, params.target_name, new_hp)
    }))
}

async fn imm_damage(
    Json(params): Json<DamageParams>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    Ok(Json(imm_damage_core(params).await?))
}

async fn imm_kill_core(params: KillParams) -> Result<serde_json::Value, (StatusCode, String)> {
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

    Ok(serde_json::json!({
        "success": true,
        "message": format!("Instantly killed target '{}'.", params.target_name)
    }))
}

async fn imm_kill(
    Json(params): Json<KillParams>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    Ok(Json(imm_kill_core(params).await?))
}

async fn imm_revive_core(params: ReviveParams) -> Result<serde_json::Value, (StatusCode, String)> {
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

    Ok(serde_json::json!({
        "success": true,
        "message": format!("Revived target '{}'.", params.target_name)
    }))
}

async fn imm_revive(
    Json(params): Json<ReviveParams>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    Ok(Json(imm_revive_core(params).await?))
}

async fn imm_set_alignment_core(
    params: SetAlignmentParams,
) -> Result<serde_json::Value, (StatusCode, String)> {
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

    Ok(serde_json::json!({
        "success": true,
        "message": format!("Set alignment of {} to '{}'.", params.player_name, params.alignment)
    }))
}

async fn imm_set_alignment(
    Json(params): Json<SetAlignmentParams>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    Ok(Json(imm_set_alignment_core(params).await?))
}

async fn imm_set_faction_core(
    params: SetFactionParams,
) -> Result<serde_json::Value, (StatusCode, String)> {
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

    Ok(serde_json::json!({
        "success": true,
        "message": format!("Set {}'s faction standing with '{}' to {}.", params.player_name, params.faction_id, params.standing)
    }))
}

async fn imm_set_faction(
    Json(params): Json<SetFactionParams>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    Ok(Json(imm_set_faction_core(params).await?))
}

async fn imm_purge_room_core(
    params: PurgeRoomParams,
) -> Result<serde_json::Value, (StatusCode, String)> {
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

    Ok(serde_json::json!({
        "success": true,
        "message": format!("Purged {} NPC(s) from room '{}'.", count, params.room_key)
    }))
}

async fn imm_purge_room(
    Json(params): Json<PurgeRoomParams>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    Ok(Json(imm_purge_room_core(params).await?))
}

async fn imm_reboot_core(params: RebootParams) -> Result<serde_json::Value, (StatusCode, String)> {
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

    Ok(serde_json::json!({
        "success": true,
        "message": format!("Server reboot initiated in {} second(s).", delay)
    }))
}

async fn imm_reboot(
    Json(params): Json<RebootParams>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    Ok(Json(imm_reboot_core(params).await?))
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

    fn test_content_dir(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("oxide_ws_rpc_{}_{}", name, uuid::Uuid::new_v4()));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn test_req(method: &str, params: serde_json::Value) -> RpcRequest {
        RpcRequest {
            jsonrpc: "2.0".to_string(),
            id: 1,
            method: method.to_string(),
            params: Some(params),
        }
    }

    #[test]
    fn resolve_content_path_rejects_traversal_absolute_and_escape() {
        let content_dir = test_content_dir("traversal");
        // Empty path is rejected.
        assert!(resolve_content_path(&content_dir, "").is_err());
        // Absolute paths are not content-relative.
        assert!(resolve_content_path(&content_dir, "/etc/shadow").is_err());
        assert!(resolve_content_path(&content_dir, "//etc/passwd").is_err());
        // `..` traversal in any position is rejected.
        assert!(resolve_content_path(&content_dir, "../escape.toml").is_err());
        assert!(resolve_content_path(&content_dir, "sub/../../escape.toml").is_err());
        assert!(resolve_content_path(&content_dir, "a/b/../../../etc/motd").is_err());
        // A benign relative path resolves inside the content directory.
        let ok = resolve_content_path(&content_dir, "items/foo.toml").unwrap();
        assert_eq!(ok, content_dir.join("items/foo.toml"));
        let _ = fs::remove_dir_all(&content_dir);
    }

    #[cfg(unix)]
    #[test]
    fn resolve_content_path_rejects_symlink_escape() {
        let content_dir = test_content_dir("symlink");
        let outside = test_content_dir("outside");
        std::os::unix::fs::symlink(&outside, content_dir.join("linkdir")).unwrap();
        // No `..` is present, but the path lands outside the content root via a
        // symlinked directory; the containment check must reject it.
        assert!(resolve_content_path(&content_dir, "linkdir/secret.toml").is_err());
        let _ = fs::remove_dir_all(&content_dir);
        let _ = fs::remove_dir_all(&outside);
    }

    #[cfg(unix)]
    #[test]
    fn content_write_delete_rejects_final_component_symlink() {
        // A FILE symlink as the FINAL path component is not canonicalized by
        // the containment check (only existing dirs are), so it would otherwise
        // pass through; `content.write`/`content.delete` must still refuse it.
        // Tests `write_content_sync`/`delete_content_sync` directly (rather
        // than via the RPC dispatch) so no other test's shared CONTENT_PATH
        // OnceLock is disturbed.
        let content_dir = test_content_dir("symlink-final");
        let outside = test_content_dir("symlink-final-outside");
        fs::create_dir_all(&outside).unwrap();
        let outside_target = outside.join("pwned.toml");
        fs::write(&outside_target, "ORIGINAL").unwrap();
        std::os::unix::fs::symlink(&outside_target, content_dir.join("pwn.toml")).unwrap();

        // `content.write` through the symlink is rejected with -32002 and the
        // outside target is NOT overwritten.
        let err = write_content_sync(&content_dir, "pwn.toml", "EVIL")
            .expect_err("write through a symlink must be rejected");
        assert_eq!(err.code, ERR_CONTENT_VALIDATION);
        assert_eq!(
            fs::read_to_string(&outside_target).unwrap(),
            "ORIGINAL",
            "the symlink target must not be overwritten"
        );

        // `content.delete` through the symlink is refused: the link itself is
        // not unlinked and the outside target is untouched.
        let err = delete_content_sync(&content_dir, "pwn.toml")
            .expect_err("delete through a symlink must be rejected");
        assert_eq!(err.code, ERR_CONTENT_VALIDATION);
        assert!(
            fs::symlink_metadata(content_dir.join("pwn.toml")).is_ok(),
            "the symlink itself must not be unlinked"
        );
        assert_eq!(fs::read_to_string(&outside_target).unwrap(), "ORIGINAL");

        let _ = fs::remove_dir_all(&content_dir);
        let _ = fs::remove_dir_all(&outside);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn content_write_validation_gate_never_touches_real_tree_on_error() {
        // Single shared content dir, set once so the global CONTENT_PATH
        // OnceLock is not raced across parallel tests.
        let content_dir = test_content_dir("gate");
        if crate::get_content_path().is_none() {
            crate::set_content_path(content_dir.clone());
        }
        // Seed a valid area with a spawn point so the world passes the
        // registry's "at least one [[spawns]]" validation. This also proves a
        // pre-existing valid tree is copied into staging untouched by a failed
        // write.
        let area_dir = content_dir.join("areas/test_area");
        fs::create_dir_all(area_dir.join("rooms")).unwrap();
        fs::write(
            area_dir.join("area.toml"),
            "id = \"test_area\"\n\
             name = \"Test Area\"\n\
             description = \"A test area.\"\n\
             [[spawns]]\n\
             room = \"test_room\"\n\
             label = \"Test Spawn\"\n\
             description = \"Spawn here.\"\n",
        )
        .unwrap();
        fs::write(
            area_dir.join("rooms/test_room.toml"),
            "id = \"test_room\"\n\
             name = \"Test Room\"\n\
             description = \"A test room.\"\n",
        )
        .unwrap();
        let admin = AuthedUser {
            username: "tester".into(),
            access_level: "admin".into(),
        };

        // 1. RBAC: a non-immortal key is rejected before any write happens.
        let lurker = AuthedUser {
            username: "lurker".into(),
            access_level: "player".into(),
        };
        let resp = handle_request(
            &test_req(
                "content.write",
                serde_json::json!({ "path": "items/x.toml", "content": "id = \"x\"" }),
            ),
            &lurker,
        )
        .await;
        let err = resp.error.expect("non-immortal write must be rejected");
        assert_eq!(err.code, ERR_FORBIDDEN);
        assert!(!content_dir.join("items/x.toml").exists());

        // 2. Invalid template TOML: validation gate fails with -32002 and the
        //    real content tree is left untouched (staging-only write).
        let bad = test_req(
            "content.write",
            serde_json::json!({ "path": "items/broken.toml", "content": "id = \"unterminated" }),
        );
        let resp = handle_request(&bad, &admin).await;
        let err = resp.error.expect("invalid content must fail validation");
        assert_eq!(err.code, ERR_CONTENT_VALIDATION);
        assert!(
            !content_dir.join("items/broken.toml").exists(),
            "invalid write must not reach the real content dir"
        );

        // 3. Valid template: gate passes and the file lands in the real tree.
        let valid = "\
id = \"test_goblet\"\n\
name = \"Test Goblet\"\n\
description = \"A test goblet.\"\n\
item_type = \"misc\"\n\
subtype = \"trash\"\n\
rarity = \"common\"\n\
level_requirement = 1\n\
weight = 1.0\n\
value = 0\n\
flags = []\n\
allowed_classes = []\n\
allowed_races = []\n\
allowed_alignments = []\n\
triggers = []\n";
        let ok = test_req(
            "content.write",
            serde_json::json!({ "path": "items/test_goblet.toml", "content": valid }),
        );
        let resp = handle_request(&ok, &admin).await;
        assert!(resp.error.is_none(), "valid write failed: {:?}", resp.error);
        let written = fs::read_to_string(content_dir.join("items/test_goblet.toml"))
            .expect("valid write must be persisted");
        assert!(written.contains("test_goblet"));

        let _ = fs::remove_dir_all(&content_dir);
    }

    #[test]
    fn content_write_delete_roundtrip() {
        // `content.delete` roundtrip. Uses `write_content_sync`/`delete_content_sync`
        // directly (rather than via the RPC dispatch) so no other test's shared
        // CONTENT_PATH OnceLock is disturbed, matching the symlink test.
        let content_dir = test_content_dir("roundtrip");
        // Seed a script file through the real write path, then delete it.
        // `.rhai` bypasses the template validation gate (no registry), keeping
        // the write both real and dependency-free.
        write_content_sync(&content_dir, "scripts/thing.rhai", "fn main() {}\n")
            .expect("seeding the file must succeed");
        assert!(content_dir.join("scripts/thing.rhai").exists());

        let msg = delete_content_sync(&content_dir, "scripts/thing.rhai")
            .expect("deleting an existing file must succeed");
        assert!(msg.contains("deleted"));
        assert!(
            !content_dir.join("scripts/thing.rhai").exists(),
            "the file must be removed from the content tree"
        );

        // Deleting a non-existent path is an error (not found), not a no-op.
        let err = delete_content_sync(&content_dir, "scripts/ghost.rhai")
            .expect_err("deleting a missing file must fail");
        assert_eq!(err.code, ERR_INVALID_PARAMS);

        let _ = fs::remove_dir_all(&content_dir);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn content_delete_requires_confirm() {
        // `content.delete` is destructive: like the imm.* ops it must demand
        // `confirm: true` in params, else -32004. The confirm gate runs in the
        // dispatcher BEFORE the content path is touched, so this is testable
        // via handle_request without disturbing the shared CONTENT_PATH OnceLock.
        let admin = AuthedUser {
            username: "tester".into(),
            access_level: "admin".into(),
        };

        // Without `confirm` (or with `confirm: false`) -> -32004.
        for params in [
            serde_json::json!({ "path": "scripts/gone.rhai" }),
            serde_json::json!({ "path": "scripts/gone.rhai", "confirm": false }),
        ] {
            let resp = handle_request(&test_req("content.delete", params), &admin).await;
            let err = resp.error.expect("delete without confirm must be rejected");
            assert_eq!(err.code, ERR_CONFIRM_REQUIRED);
        }

        // With `confirm: true` the gate passes: the response is no longer a
        // confirm-required error (it proceeds to the content handler, which
        // either succeeds or fails for an unrelated reason depending on the
        // shared content path). The actual unlink is covered by
        // `content_write_delete_roundtrip`.
        let resp = handle_request(
            &test_req(
                "content.delete",
                serde_json::json!({ "path": "scripts/gone.rhai", "confirm": true }),
            ),
            &admin,
        )
        .await;
        if let Some(err) = resp.error {
            assert_ne!(
                err.code, ERR_CONFIRM_REQUIRED,
                "confirm:true must clear the confirm gate"
            );
        }
    }

    #[test]
    fn content_write_enforces_size_and_path_bounds() {
        // Cheap pre-write bounds are a pure function, so this needs no content
        // path (and cannot race the shared CONTENT_PATH OnceLock).
        let err = validate_content_write_bounds("items/x.toml", &"x".repeat(MAX_CONTENT_BYTES + 1))
            .expect_err("oversized content must be rejected");
        assert_eq!(err.code, ERR_INVALID_PARAMS);

        let deep = vec!["a"; MAX_PATH_DEPTH + 1].join("/") + "/x.toml";
        let err = validate_content_write_bounds(&deep, "id = \\\"x\\\"")
            .expect_err("overdeep path must be rejected");
        assert_eq!(err.code, ERR_INVALID_PARAMS);

        let long_path = "a/".repeat(MAX_PATH_LEN) + "x.toml";
        let err = validate_content_write_bounds(&long_path, "id = \\\"x\\\"")
            .expect_err("overlong path must be rejected");
        assert_eq!(err.code, ERR_INVALID_PARAMS);

        // A normal short template path + modest content passes the bounds.
        assert!(validate_content_write_bounds("items/x.toml", "id = \\\"x\\\"").is_ok());
    }

    #[test]
    fn dispatch_loop_rejects_malformed_frame() {
        // A malformed frame (invalid JSON, or valid JSON that isn't a Request)
        // must map to a JSON-RPC parse error (-32700) response and never panic
        // in dispatch_loop. We exercise the exact parse branch: assert the
        // frame fails `serde_json::from_str::<RpcRequest>` (the condition
        // dispatch_loop dispatches on) and that it produces the parse-error
        // response via `build_parse_error_response`.
        let frames = ["not json {", r#"{"foo": 1}"#, r#"[1,2,3]"#];
        for frame in frames {
            assert!(
                serde_json::from_str::<RpcRequest>(frame).is_err(),
                "frame should not parse as a Request: {frame:?}"
            );
            let response = build_parse_error_response();
            assert!(response.result.is_none(), "parse error carries no result");
            let err = response
                .error
                .expect("parse-error branch must carry an error body");
            assert_eq!(err.code, ERR_PARSE, "code must be -32700 (ERR_PARSE)");
            assert_eq!(err.message, "parse error");
        }
    }
}
