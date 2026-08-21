//! Request/parameter structs for the MCP tool handlers.
//!
//! Moved verbatim out of the former `server.rs` monolith; no semantic changes.

use std::collections::HashMap;

use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct IdParam {
    pub(crate) id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct AreaIdParam {
    pub(crate) area_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct RoomIdParam {
    pub(crate) area_id: String,
    pub(crate) room_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct CreateAreaParams {
    pub(crate) id: String,
    #[schemars(description = "Area display name")]
    pub(crate) name: String,
    #[schemars(description = "Optional description")]
    pub(crate) description: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct CreateRoomParams {
    pub(crate) area_id: String,
    pub(crate) room_id: String,
    #[schemars(description = "Room display name")]
    pub(crate) name: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct CreateEntityParams {
    pub(crate) id: String,
    #[schemars(description = "Display name (defaults to id)")]
    pub(crate) name: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct UpdateFieldsParams {
    pub(crate) category: String,
    pub(crate) id: String,
    #[schemars(description = "JSON object of fields to update")]
    pub(crate) fields: serde_json::Map<String, serde_json::Value>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct UpdateRoomFieldsParams {
    pub(crate) area_id: String,
    pub(crate) room_id: String,
    #[schemars(description = "JSON object of fields to update on the room")]
    pub(crate) fields: serde_json::Map<String, serde_json::Value>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct LinkRoomsParams {
    pub(crate) area_id: String,
    pub(crate) from_room: String,
    #[schemars(description = "Direction string, e.g. north, south, east, west")]
    pub(crate) direction: String,
    pub(crate) to_area: String,
    pub(crate) to_room: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct SearchParams {
    #[schemars(description = "Search query string")]
    pub(crate) query: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct SimulateLootParams {
    #[schemars(description = "ID of the mob template to simulate drops from")]
    pub(crate) mob_id: String,
    #[schemars(description = "Number of corpses to roll loot for (e.g. 1000)")]
    pub(crate) iterations: u32,
    #[schemars(
        description = "If true, returns detailed per-corpse loot drops including quality and affixes"
    )]
    pub(crate) detailed: Option<bool>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct SimulateProgressionParams {
    #[schemars(description = "ID of the race template")]
    pub(crate) race_id: String,
    #[schemars(description = "ID of the class template")]
    pub(crate) class_id: String,
    #[schemars(description = "Starting level to show stats for")]
    pub(crate) start_level: u8,
    #[schemars(description = "Ending level (inclusive)")]
    pub(crate) end_level: u8,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct SimulateGearLoadoutParams {
    #[schemars(description = "ID of the race template")]
    pub(crate) race_id: String,
    #[schemars(description = "ID of the class template")]
    pub(crate) class_id: String,
    #[schemars(description = "Character level")]
    pub(crate) level: u8,
    #[schemars(description = "List of item template IDs to equip")]
    pub(crate) equipped_items: Vec<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct SimulateAiWanderParams {
    #[schemars(description = "ID of the mob template")]
    pub(crate) mob_id: String,
    #[schemars(description = "Starting room ID in area:room format (e.g. 'tutorial:hallway')")]
    pub(crate) start_room_str: String,
    #[schemars(description = "Number of wander steps (ticks) to simulate")]
    pub(crate) ticks: u32,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct SimulateShopTransactionParams {
    #[schemars(description = "ID of the shop template")]
    pub(crate) shop_id: String,
    #[schemars(description = "ID of the item template to buy/sell")]
    pub(crate) item_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct PutItemParams {
    pub(crate) player_name: String,
    pub(crate) item_template_id: String,
    pub(crate) count: Option<u32>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct TeleportParams {
    pub(crate) player_name: String,
    pub(crate) room_key: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct ForceCommandParams {
    pub(crate) player_name: String,
    pub(crate) command: String,
    #[schemars(description = "Must be true to confirm this destructive operation")]
    pub(crate) confirm: bool,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, JsonSchema)]
pub(crate) struct SetStatParams {
    pub(crate) player_name: String,
    pub(crate) strength: Option<u8>,
    pub(crate) dexterity: Option<u8>,
    pub(crate) intelligence: Option<u8>,
    pub(crate) wisdom: Option<u8>,
    pub(crate) constitution: Option<u8>,
    pub(crate) charisma: Option<u8>,
    pub(crate) hp: Option<i32>,
    pub(crate) mana: Option<u16>,
    pub(crate) stamina: Option<u16>,
    pub(crate) level: Option<u8>,
    pub(crate) xp: Option<u64>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, JsonSchema)]
pub(crate) struct LoadMobParams {
    pub(crate) room_key: String,
    pub(crate) mob_template_id: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, JsonSchema)]
pub(crate) struct LoadItemParams {
    pub(crate) room_key: String,
    pub(crate) item_template_id: String,
    pub(crate) count: Option<u32>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, JsonSchema)]
pub(crate) struct GechoParams {
    pub(crate) message: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, JsonSchema)]
pub(crate) struct AdvanceParams {
    pub(crate) player_name: String,
    pub(crate) target_level: u8,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, JsonSchema)]
pub(crate) struct StatParams {
    pub(crate) target_name: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, JsonSchema)]
pub(crate) struct HealParams {
    pub(crate) target_name: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, JsonSchema)]
pub(crate) struct DamageParams {
    pub(crate) target_name: String,
    pub(crate) amount: i32,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, JsonSchema)]
pub(crate) struct KillParams {
    pub(crate) target_name: String,
    #[schemars(description = "Must be true to confirm this destructive operation")]
    pub(crate) confirm: bool,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, JsonSchema)]
pub(crate) struct ReviveParams {
    pub(crate) target_name: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, JsonSchema)]
pub(crate) struct SetAlignmentParams {
    pub(crate) player_name: String,
    pub(crate) alignment: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, JsonSchema)]
pub(crate) struct SetFactionParams {
    pub(crate) player_name: String,
    pub(crate) faction_id: String,
    pub(crate) standing: i32,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, JsonSchema)]
pub(crate) struct PurgeRoomParams {
    pub(crate) room_key: String,
    #[schemars(description = "Must be true to confirm this destructive operation")]
    pub(crate) confirm: bool,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, JsonSchema)]
pub(crate) struct RebootParams {
    #[schemars(description = "Must be true to confirm this destructive operation")]
    pub(crate) confirm: bool,
    pub(crate) delay_secs: Option<u64>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct SimulateCraftingParams {
    #[schemars(description = "ID of the recipe template")]
    pub(crate) recipe_id: String,
    #[schemars(description = "Optional real player name from database to load stats from")]
    pub(crate) player_name: Option<String>,
    #[schemars(description = "Character level (optional, default: 1)")]
    pub(crate) player_level: Option<u8>,
    #[schemars(description = "Dexterity modifier check override (optional, default: 10)")]
    pub(crate) dexterity: Option<u8>,
    #[schemars(description = "Intelligence modifier check override (optional, default: 10)")]
    pub(crate) intelligence: Option<u8>,
    #[schemars(description = "Skill rank for crafting (optional, default: 0)")]
    pub(crate) skill_rank: Option<u16>,
    #[schemars(description = "Has required station present in the room (optional, default: true)")]
    pub(crate) has_station: Option<bool>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct SimulatePrayerParams {
    #[schemars(description = "ID of the deity template")]
    pub(crate) deity_id: String,
    #[schemars(description = "Optional real player name from database to load stats from")]
    pub(crate) player_name: Option<String>,
    #[schemars(description = "ID of the player race template")]
    pub(crate) player_race: Option<String>,
    #[schemars(description = "ID of the player class template")]
    pub(crate) player_class: Option<String>,
    #[schemars(description = "Player alignment (e.g. 'Lawful Good') (optional)")]
    pub(crate) player_alignment: Option<String>,
    #[schemars(description = "Mock cleric level (optional, default: 1)")]
    pub(crate) cleric_level: Option<u8>,
    #[schemars(description = "Player base wisdom (default: 10)")]
    pub(crate) wisdom: Option<u8>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct SimulatePrestigeParams {
    #[schemars(description = "ID of the prestige class template")]
    pub(crate) prestige_class_id: String,
    #[schemars(description = "Optional real player name from database to load stats from")]
    pub(crate) player_name: Option<String>,
    #[schemars(
        description = "Mock class levels (e.g. { 'warrior': 5 }) (bypassed if player_name matches a real character)"
    )]
    pub(crate) base_classes: Option<HashMap<String, u8>>,
    #[schemars(
        description = "Mock skill ranks (e.g. { 'swordplay': 5 }) (bypassed if player_name matches a real character)"
    )]
    pub(crate) skill_ranks: Option<HashMap<String, u16>>,
    #[schemars(
        description = "Mock list of completed quest IDs (bypassed if player_name matches a real character)"
    )]
    pub(crate) completed_quests: Option<Vec<String>>,
    #[schemars(
        description = "Mock faction standings (e.g. { 'guard': 500 }) (bypassed if player_name matches a real character)"
    )]
    pub(crate) faction_standings: Option<HashMap<String, i32>>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct MockMemberParam {
    #[schemars(description = "Class template ID")]
    pub(crate) class_id: String,
    #[schemars(description = "Equipped with a shield")]
    pub(crate) has_shield: bool,
    #[schemars(description = "Front row position (otherwise back row)")]
    pub(crate) is_front_row: bool,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct SimulateGroupParams {
    #[schemars(description = "Formation name (Line, Scattered, Column, Wedge, Shield Wall)")]
    pub(crate) formation: String,
    #[schemars(description = "List of party members")]
    pub(crate) members: Vec<MockMemberParam>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct SimulateDeathParams {
    #[schemars(description = "Optional real player name from database to load stats from")]
    pub(crate) player_name: Option<String>,
    #[schemars(description = "Current character level (1 to 100)")]
    pub(crate) current_level: Option<u8>,
    #[schemars(description = "Current experience points")]
    pub(crate) current_xp: Option<u64>,
    #[schemars(description = "Is current room an allow_revive room")]
    pub(crate) allow_revive_room: bool,
}

#[derive(Debug, Deserialize)]
pub(crate) struct LoadedPlayer {
    pub(crate) name: String,
    pub(crate) race_id: String,
    pub(crate) class_id: String,
    pub(crate) level: u8,
    pub(crate) experience: u64,
    pub(crate) alignment: String,
    pub(crate) attributes: LoadedAttributes,
    pub(crate) skills: HashMap<String, u16>,
    pub(crate) completed_quests: Vec<String>,
    pub(crate) faction_standings: HashMap<String, i32>,
    // Present in the server response; retained for deserialization completeness.
    #[allow(dead_code)]
    pub(crate) inventory: Vec<String>,
    #[allow(dead_code)]
    pub(crate) equipment: HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct LoadedAttributes {
    pub(crate) strength: u8,
    pub(crate) dexterity: u8,
    pub(crate) intelligence: u8,
    pub(crate) wisdom: u8,
    pub(crate) constitution: u8,
    pub(crate) charisma: u8,
}
