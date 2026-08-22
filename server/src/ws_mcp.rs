//! Server-side WebSocket MCP endpoint.
//!
//! Exposes the live OxideMUD server's state to MCP clients over `/ws/mcp` via
//! rmcp's server machinery. Tools read the in-process world/registry/templates
//! directly (no HTTP hop), so a remote `oxide-mcp` client can administer a
//! running server.
//!
//! The tool set here is intentionally focused (read-only state + a few safe
//! immortal operations) and mirrors the descriptions used by the offline
//! `oxide-mcp` crate so tool names stay consistent across both surfaces.

use std::sync::Arc;

use rmcp::handler::server::wrapper::Parameters;
use rmcp::{tool, tool_router};
use schemars::JsonSchema;
use serde::Deserialize;

use oxide_core::templates::TemplateRegistry;
use oxide_core::World;

// ---- Parameter structs -------------------------------------------------

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct PlayerNameParam {
    pub(crate) player_name: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct GechoParams {
    pub(crate) message: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct PutItemParams {
    pub(crate) player_name: String,
    pub(crate) item_template_id: String,
    #[schemars(description = "Number of items to give (default 1)")]
    pub(crate) count: Option<u32>,
}

// ---- Server tool handler ------------------------------------------------

/// The MCP server tool router for the live game server.
#[derive(Clone)]
pub struct OxideServerMcp {
    templates: Arc<TemplateRegistry>,
    world: Arc<tokio::sync::Mutex<World>>,
    registry: Arc<tokio::sync::Mutex<crate::registry::ConnectionRegistry>>,
}

impl OxideServerMcp {
    /// Build the handler from the server's global singletons. Returns `None`
    /// if the server hasn't initialized them yet.
    pub fn from_server() -> Option<Self> {
        Some(Self {
            templates: crate::get_templates()?,
            world: crate::get_world()?,
            registry: crate::get_registry()?,
        })
    }
}

#[tool_router(server_handler)]
impl OxideServerMcp {
    #[tool(description = "List all currently connected players in the MUD (Online Only)")]
    fn list_connected_players(&self) -> String {
        let reg = self.registry.try_lock();
        let Ok(reg) = reg else {
            return "Connection registry busy".to_string();
        };
        let world = self.world.try_lock();
        let Ok(world) = world else {
            return "World busy".to_string();
        };
        let entities = reg.connected_entities();
        if entities.is_empty() {
            return "No players currently online.".to_string();
        }
        let mut out = "### Connected Players:\n\n| Name | Room Key |\n|---|---|\n".to_string();
        for entity in entities {
            let name = world
                .query_one::<&oxide_core::Name>(entity)
                .ok()
                .and_then(|mut q| q.get().map(|n| n.0.clone()))
                .unwrap_or_else(|| "Unknown".to_string());
            let room = world
                .query_one::<&oxide_core::Room>(entity)
                .ok()
                .and_then(|mut q| q.get().map(|r| r.name.clone()))
                .unwrap_or_else(|| "-".to_string());
            out.push_str(&format!("| {name} | {room} |\n"));
        }
        out.trim().to_string()
    }

    #[tool(description = "Get details for a connected player by name (Online Only)")]
    fn get_player(&self, params: Parameters<PlayerNameParam>) -> String {
        let name = params.0.player_name;
        let world = self.world.try_lock();
        let Ok(world) = world else {
            return "World busy".to_string();
        };
        for (entity, (name_comp,)) in world.query::<(&oxide_core::Name,)>().iter() {
            if name_comp.0.to_lowercase() == name.to_lowercase() {
                let room = world
                    .query_one::<&oxide_core::Room>(entity)
                    .ok()
                    .and_then(|mut q| q.get().map(|r| r.name.clone()))
                    .unwrap_or_else(|| "-".to_string());
                return format!("Player '{name}' is online in room {room}.");
            }
        }
        format!("Player '{name}' is not online.")
    }

    #[tool(description = "Broadcast a global echo message to all players (Online Only)")]
    fn imm_gecho(&self, params: Parameters<GechoParams>) -> String {
        let message = params.0.message;
        let reg = self.registry.try_lock();
        let Ok(reg) = reg else {
            return "Connection registry busy".to_string();
        };
        let mut formatted = format!("[IMM] {message}");
        if !formatted.ends_with('\n') {
            formatted.push_str("\r\n");
        }
        reg.broadcast_all(&formatted);
        "Broadcast sent.".to_string()
    }

    #[tool(description = "Put an item from a template into a player's inventory (Online Only)")]
    fn imm_put_item(&self, params: Parameters<PutItemParams>) -> String {
        let p = params.0;
        let templates = self.templates.clone();
        let item_def = templates.items.get(&p.item_template_id);
        let Some(_item_def) = item_def else {
            return format!("Item template '{}' not found", p.item_template_id);
        };
        let world = self.world.try_lock();
        let Ok(mut world) = world else {
            return "World busy".to_string();
        };
        // Find player entity
        let mut player = None;
        for (entity, (name_comp,)) in world.query::<(&oxide_core::Name,)>().iter() {
            if name_comp.0.to_lowercase() == p.player_name.to_lowercase() {
                player = Some(entity);
                break;
            }
        }
        let Some(player_entity) = player else {
            return format!("Player '{}' not found", p.player_name);
        };
        // Spawn item
        let spawn = oxide_core::systems::loot::ItemSpawn {
            template_id: p.item_template_id.clone(),
            count: p.count.unwrap_or(1) as u8,
            quality: oxide_core::systems::loot::QualityTier::Common,
            prefix_ids: vec![],
            suffix_ids: vec![],
        };
        let Some(item_entity) =
            oxide_core::systems::loot::spawn_loot_item(&mut world, &spawn, &templates)
        else {
            return "Failed to spawn item entity".to_string();
        };
        let mut added = false;
        if let Ok(mut q) = world.query_one::<&mut oxide_core::Inventory>(player_entity) {
            if let Some(inv) = q.get() {
                inv.0.push(item_entity);
                added = true;
            }
        }
        if !added {
            return "Player does not have an inventory component".to_string();
        }
        format!(
            "Item '{}' given to '{}'.",
            p.item_template_id, p.player_name
        )
    }
}
