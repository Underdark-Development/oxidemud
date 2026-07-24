use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use crate::simulator::{
    SimulateCharacterCreationParams, SimulateCombatParams, SimulateSkillUseParams,
};
use oxide_core::templates::{
    AffixDef, AreaTemplate, ClassTemplate, DeityTemplate, ExitTemplate, FactionDef, HealthBounds,
    ItemTemplate, LootTable, MobTemplate, PassiveDef, QuestDef, QuestRewards, RaceAttributes,
    RaceTemplate, RecipeDef, RoomContent, RoomTemplate, SetDef, ShopTemplate, StanceDef,
};
use oxide_core::SkillDef;
use rmcp::{
    handler::server::wrapper::Parameters, schemars, tool, tool_router, transport::stdio, ServiceExt,
};
use schemars::JsonSchema;
use serde::Deserialize;

use crate::content;
use crate::simulator;

#[derive(Clone)]
pub struct OxideMcpServer {
    content_path: PathBuf,
    #[allow(dead_code)]
    api_url: Option<String>,
    #[allow(dead_code)]
    api_key: Option<String>,
}

impl OxideMcpServer {
    pub fn new(content_path: PathBuf, api_url: Option<String>, api_key: Option<String>) -> Self {
        OxideMcpServer {
            content_path,
            api_url,
            api_key,
        }
    }

    pub async fn run(self) -> anyhow::Result<()> {
        let service = self.serve(stdio()).await?;
        service.waiting().await?;
        Ok(())
    }

    fn load(&self) -> (oxide_core::templates::TemplateRegistry, content::FileMap) {
        content::load_registry(&self.content_path)
    }

    /// Validate a content ID is safe for filesystem operations and that the
    /// resulting path stays within the content directory.
    fn validate_id(&self, id: &str) -> Result<(), String> {
        content::validate_content_id(id)
    }

    /// Validate a content ID and check that a constructed path is contained
    /// within the content directory.
    fn validate_and_contain(&self, id: &str, path: &std::path::Path) -> Result<(), String> {
        content::validate_content_id(id)?;
        content::assert_within_content_dir(&self.content_path, path)
    }

    fn entity_list(items: &HashMap<String, impl AsRef<str>>, label: &str) -> String {
        if items.is_empty() {
            return format!("No {} found.", label);
        }
        let mut ids: Vec<&String> = items.keys().collect();
        ids.sort();
        let mut out = format!("{}:\n", label);
        for id in ids {
            out.push_str(&format!("  {id}: {}\n", items[id].as_ref()));
        }
        out.trim().to_string()
    }

    async fn fetch_player_state(&self, name: &str) -> Result<LoadedPlayer, String> {
        let (url, key) = match (&self.api_url, &self.api_key) {
            (Some(u), Some(k)) => (u, k),
            _ => return Err("Offline mode: cannot fetch real player data. Provide --url and --key to connect to the MUD server.".to_string()),
        };

        let client = reqwest::Client::new();
        let req_url = format!("{}/api/character/{}", url.trim_end_matches('/'), name);

        match client
            .get(&req_url)
            .header("Authorization", format!("Bearer {}", key))
            .send()
            .await
        {
            Ok(resp) => {
                let status = resp.status();
                if status.is_success() {
                    match resp.json::<LoadedPlayer>().await {
                        Ok(player) => Ok(player),
                        Err(e) => Err(format!("Failed to parse MUD Server response as JSON: {e}")),
                    }
                } else {
                    match resp.text().await {
                        Ok(err_text) => Err(format!("Error from server: {err_text}")),
                        Err(_) => Err(format!("Server returned error status: {}", status)),
                    }
                }
            }
            Err(e) => Err(format!("Failed to connect to MUD server: {e}")),
        }
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
struct IdParam {
    id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct AreaIdParam {
    area_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct RoomIdParam {
    area_id: String,
    room_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct CreateAreaParams {
    id: String,
    #[schemars(description = "Area display name")]
    name: String,
    #[schemars(description = "Optional description")]
    description: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct CreateRoomParams {
    area_id: String,
    room_id: String,
    #[schemars(description = "Room display name")]
    name: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct CreateEntityParams {
    id: String,
    #[schemars(description = "Display name (defaults to id)")]
    name: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct UpdateFieldsParams {
    category: String,
    id: String,
    #[schemars(description = "JSON object of fields to update")]
    fields: serde_json::Map<String, serde_json::Value>,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct UpdateRoomFieldsParams {
    area_id: String,
    room_id: String,
    #[schemars(description = "JSON object of fields to update on the room")]
    fields: serde_json::Map<String, serde_json::Value>,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct LinkRoomsParams {
    area_id: String,
    from_room: String,
    #[schemars(description = "Direction string, e.g. north, south, east, west")]
    direction: String,
    to_area: String,
    to_room: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct SearchParams {
    #[schemars(description = "Search query string")]
    query: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct SimulateLootParams {
    #[schemars(description = "ID of the mob template to simulate drops from")]
    mob_id: String,
    #[schemars(description = "Number of corpses to roll loot for (e.g. 1000)")]
    iterations: u32,
    #[schemars(
        description = "If true, returns detailed per-corpse loot drops including quality and affixes"
    )]
    detailed: Option<bool>,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct SimulateProgressionParams {
    #[schemars(description = "ID of the race template")]
    race_id: String,
    #[schemars(description = "ID of the class template")]
    class_id: String,
    #[schemars(description = "Starting level to show stats for")]
    start_level: u8,
    #[schemars(description = "Ending level (inclusive)")]
    end_level: u8,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct SimulateGearLoadoutParams {
    #[schemars(description = "ID of the race template")]
    race_id: String,
    #[schemars(description = "ID of the class template")]
    class_id: String,
    #[schemars(description = "Character level")]
    level: u8,
    #[schemars(description = "List of item template IDs to equip")]
    equipped_items: Vec<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct SimulateAiWanderParams {
    #[schemars(description = "ID of the mob template")]
    mob_id: String,
    #[schemars(description = "Starting room ID in area:room format (e.g. 'tutorial:hallway')")]
    start_room_str: String,
    #[schemars(description = "Number of wander steps (ticks) to simulate")]
    ticks: u32,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct SimulateShopTransactionParams {
    #[schemars(description = "ID of the shop template")]
    shop_id: String,
    #[schemars(description = "ID of the item template to buy/sell")]
    item_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[allow(dead_code)]
struct PutItemParams {
    player_name: String,
    item_template_id: String,
    count: Option<u32>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[allow(dead_code)]
struct TeleportParams {
    player_name: String,
    room_key: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[allow(dead_code)]
struct ForceCommandParams {
    player_name: String,
    command: String,
    #[schemars(description = "Must be true to confirm this destructive operation")]
    confirm: bool,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, JsonSchema)]
#[allow(dead_code)]
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

#[derive(Debug, serde::Serialize, serde::Deserialize, JsonSchema)]
#[allow(dead_code)]
struct LoadMobParams {
    room_key: String,
    mob_template_id: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, JsonSchema)]
#[allow(dead_code)]
struct LoadItemParams {
    room_key: String,
    item_template_id: String,
    count: Option<u32>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, JsonSchema)]
#[allow(dead_code)]
struct GechoParams {
    message: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, JsonSchema)]
#[allow(dead_code)]
struct AdvanceParams {
    player_name: String,
    target_level: u8,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, JsonSchema)]
#[allow(dead_code)]
struct StatParams {
    target_name: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, JsonSchema)]
#[allow(dead_code)]
struct HealParams {
    target_name: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, JsonSchema)]
#[allow(dead_code)]
struct DamageParams {
    target_name: String,
    amount: i32,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, JsonSchema)]
#[allow(dead_code)]
struct KillParams {
    target_name: String,
    #[schemars(description = "Must be true to confirm this destructive operation")]
    confirm: bool,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, JsonSchema)]
#[allow(dead_code)]
struct ReviveParams {
    target_name: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, JsonSchema)]
#[allow(dead_code)]
struct SetAlignmentParams {
    player_name: String,
    alignment: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, JsonSchema)]
#[allow(dead_code)]
struct SetFactionParams {
    player_name: String,
    faction_id: String,
    standing: i32,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, JsonSchema)]
#[allow(dead_code)]
struct PurgeRoomParams {
    room_key: String,
    #[schemars(description = "Must be true to confirm this destructive operation")]
    confirm: bool,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, JsonSchema)]
#[allow(dead_code)]
struct RebootParams {
    #[schemars(description = "Must be true to confirm this destructive operation")]
    confirm: bool,
    delay_secs: Option<u64>,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct SimulateCraftingParams {
    #[schemars(description = "ID of the recipe template")]
    recipe_id: String,
    #[schemars(description = "Optional real player name from database to load stats from")]
    player_name: Option<String>,
    #[schemars(description = "Character level (optional, default: 1)")]
    player_level: Option<u8>,
    #[schemars(description = "Dexterity modifier check override (optional, default: 10)")]
    dexterity: Option<u8>,
    #[schemars(description = "Intelligence modifier check override (optional, default: 10)")]
    intelligence: Option<u8>,
    #[schemars(description = "Skill rank for crafting (optional, default: 0)")]
    skill_rank: Option<u16>,
    #[schemars(description = "Has required station present in the room (optional, default: true)")]
    has_station: Option<bool>,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct SimulatePrayerParams {
    #[schemars(description = "ID of the deity template")]
    deity_id: String,
    #[schemars(description = "Optional real player name from database to load stats from")]
    player_name: Option<String>,
    #[schemars(description = "ID of the player race template")]
    player_race: Option<String>,
    #[schemars(description = "ID of the player class template")]
    player_class: Option<String>,
    #[schemars(description = "Player alignment (e.g. 'Lawful Good') (optional)")]
    player_alignment: Option<String>,
    #[schemars(description = "Mock cleric level (optional, default: 1)")]
    cleric_level: Option<u8>,
    #[schemars(description = "Player base wisdom (default: 10)")]
    wisdom: Option<u8>,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct SimulatePrestigeParams {
    #[schemars(description = "ID of the prestige class template")]
    prestige_class_id: String,
    #[schemars(description = "Optional real player name from database to load stats from")]
    player_name: Option<String>,
    #[schemars(
        description = "Mock class levels (e.g. { 'warrior': 5 }) (bypassed if player_name matches a real character)"
    )]
    base_classes: Option<HashMap<String, u8>>,
    #[schemars(
        description = "Mock skill ranks (e.g. { 'swordplay': 5 }) (bypassed if player_name matches a real character)"
    )]
    skill_ranks: Option<HashMap<String, u16>>,
    #[schemars(
        description = "Mock list of completed quest IDs (bypassed if player_name matches a real character)"
    )]
    completed_quests: Option<Vec<String>>,
    #[schemars(
        description = "Mock faction standings (e.g. { 'guard': 500 }) (bypassed if player_name matches a real character)"
    )]
    faction_standings: Option<HashMap<String, i32>>,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct MockMemberParam {
    #[schemars(description = "Class template ID")]
    class_id: String,
    #[schemars(description = "Equipped with a shield")]
    has_shield: bool,
    #[schemars(description = "Front row position (otherwise back row)")]
    is_front_row: bool,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct SimulateGroupParams {
    #[schemars(description = "Formation name (Line, Scattered, Column, Wedge, Shield Wall)")]
    formation: String,
    #[schemars(description = "List of party members")]
    members: Vec<MockMemberParam>,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct SimulateDeathParams {
    #[schemars(description = "Optional real player name from database to load stats from")]
    player_name: Option<String>,
    #[schemars(description = "Current character level (1 to 100)")]
    current_level: Option<u8>,
    #[schemars(description = "Current experience points")]
    current_xp: Option<u64>,
    #[schemars(description = "Is current room an allow_revive room")]
    allow_revive_room: bool,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct LoadedPlayer {
    name: String,
    race_id: String,
    class_id: String,
    level: u8,
    experience: u64,
    alignment: String,
    attributes: LoadedAttributes,
    skills: HashMap<String, u16>,
    completed_quests: Vec<String>,
    faction_standings: HashMap<String, i32>,
    inventory: Vec<String>,
    equipment: HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct LoadedAttributes {
    strength: u8,
    dexterity: u8,
    intelligence: u8,
    wisdom: u8,
    constitution: u8,
    charisma: u8,
}

#[tool_router(server_handler)]
impl OxideMcpServer {
    #[tool(description = "List all areas")]
    fn list_areas(&self) -> String {
        let (registry, _) = self.load();
        Self::entity_list(
            &registry
                .areas
                .iter()
                .map(|(k, v)| (k.clone(), v.name.as_str()))
                .collect(),
            "Areas",
        )
    }

    #[tool(description = "Get area details")]
    fn get_area(&self, params: Parameters<IdParam>) -> String {
        let p = params.0;
        let (registry, _) = self.load();
        match registry.get_area(&p.id) {
            Some(area) => {
                let mut out = format!(
                    "id: {}\nname: {}\ndescription: {}",
                    p.id, area.name, area.description
                );
                out.push_str(&format!("\nrooms: {}", area.rooms.len()));
                if let Some(ref lr) = area.level_range {
                    out.push_str(&format!("\nlevel_range: {}-{}", lr[0], lr[1]));
                }
                if !area.flags.is_empty() {
                    out.push_str(&format!("\nflags: {}", area.flags.join(", ")));
                }
                out
            }
            None => format!("Error: area '{}' not found", p.id),
        }
    }

    #[tool(description = "Create a new area")]
    fn create_area(&self, params: Parameters<CreateAreaParams>) -> String {
        let p = params.0;
        if let Err(e) = self.validate_id(&p.id) {
            return format!("Error: {e}");
        }
        let area_dir = self.content_path.join("areas").join(&p.id);
        if let Err(e) = self.validate_and_contain(&p.id, &area_dir) {
            return format!("Error: {e}");
        }

        if let Err(e) = fs::create_dir_all(area_dir.join("rooms"))
            .and_then(|_| fs::create_dir_all(area_dir.join("areas")))
        {
            return format!("Error: failed to create area directories: {e}");
        }

        // Write metadata-only area.toml
        let area = AreaTemplate {
            id: p.id.clone(),
            name: p.name,
            description: p.description.unwrap_or_default(),
            level_range: None,
            flags: Vec::new(),
            weather_zone: None,
            no_weather: false,
            weather_matrix: HashMap::new(),
            reset_interval: None,
            credits: None,
            spawns: Vec::new(),
            rooms: HashMap::new(),
        };
        let area_str = match toml::to_string_pretty(&area) {
            Ok(s) => s,
            Err(e) => return format!("Error: failed to serialize area: {e}"),
        };
        if let Err(e) = fs::write(area_dir.join("area.toml"), &area_str) {
            return format!("Error: failed to write area: {e}");
        }

        // Write starter room file
        let room = RoomTemplate {
            id: "start".to_string(),
            area: p.id.clone(),
            name: "Starting Room".to_string(),
            description: String::new(),
            exits: HashMap::new(),
            portals: Vec::new(),
            flags: Vec::new(),
            content: RoomContent::default(),
            allow_revive: false,
            no_weather: false,
            exclude_weather: Vec::new(),
            additional_weather: HashMap::new(),
            script: None,
            params: HashMap::new(),
        };
        let room_str = match toml::to_string_pretty(&room) {
            Ok(s) => s,
            Err(e) => return format!("Error: failed to serialize starter room: {e}"),
        };
        if let Err(e) = fs::write(area_dir.join("rooms").join("start.toml"), &room_str) {
            return format!("Error: failed to write starter room: {e}");
        }

        format!("Created area '{}'", p.id)
    }

    #[tool(description = "Delete an area and its file")]
    fn delete_area(&self, params: Parameters<IdParam>) -> String {
        let p = params.0;
        let (_, file_map) = self.load();
        let area_path = match file_map.get("areas").and_then(|m| m.get(&p.id)) {
            Some(p) => p.clone(),
            None => return format!("Error: area '{}' not found", p.id),
        };

        // If it's a subdirectory-format area, delete the whole directory.
        // If it's a flat file, just delete the file.
        if area_path.file_name().is_some_and(|n| n == "area.toml") {
            let parent_dir = area_path.parent().unwrap();
            if let Err(e) = fs::remove_dir_all(parent_dir) {
                return format!("Error: failed to delete area directory: {e}");
            }
        } else if let Err(e) = fs::remove_file(&area_path) {
            return format!("Error: failed to delete {}: {e}", area_path.display());
        }

        format!("Deleted area '{}'", p.id)
    }

    #[tool(description = "List rooms in an area")]
    fn list_rooms(&self, params: Parameters<AreaIdParam>) -> String {
        let p = params.0;
        let (registry, _) = self.load();
        match registry.get_area(&p.area_id) {
            Some(area) => {
                let mut ids: Vec<&String> = area.rooms.keys().collect();
                ids.sort();
                let mut out = format!("Rooms in '{}':\n", p.area_id);
                for id in ids {
                    let room = &area.rooms[id];
                    out.push_str(&format!("  {id}: {}\n", room.name));
                }
                out.trim().to_string()
            }
            None => format!("Error: area '{}' not found", p.area_id),
        }
    }

    #[tool(description = "Get room details")]
    fn get_room(&self, params: Parameters<RoomIdParam>) -> String {
        let p = params.0;
        let (registry, _) = self.load();
        match registry.get_room(&p.area_id, &p.room_id) {
            Some(room) => {
                let mut out = format!(
                    "room_id: {}\nname: {}\ndescription: {}",
                    p.room_id, room.name, room.description
                );
                if !room.exits.is_empty() {
                    out.push_str("\nexits:");
                    let mut dirs: Vec<&String> = room.exits.keys().collect();
                    dirs.sort();
                    for dir in dirs {
                        out.push_str(&format!("\n  {dir}: {}", room.exits[dir].dest()));
                    }
                }
                if !room.portals.is_empty() {
                    out.push_str("\nportals:");
                    for portal in &room.portals {
                        out.push_str(&format!(
                            "\n  {} -> {}: {}",
                            portal.keyword, portal.dest, portal.description
                        ));
                        if !portal.flags.is_empty() {
                            out.push_str(&format!(" [{}]", portal.flags.join(", ")));
                        }
                    }
                }
                if !room.content.mobs.is_empty() {
                    out.push_str("\nmob spawns:");
                    for mob in &room.content.mobs {
                        out.push_str(&format!("\n  {} x{}", mob.template_id, mob.count));
                        if let Some(secs) = mob.respawn_secs {
                            out.push_str(&format!(" (respawn {secs}s)"));
                        }
                    }
                }
                if !room.content.items.is_empty() {
                    out.push_str("\nitem spawns:");
                    for item in &room.content.items {
                        out.push_str(&format!("\n  {} x{}", item.template_id, item.count));
                    }
                }
                if !room.flags.is_empty() {
                    out.push_str(&format!("\nflags: {}", room.flags.join(", ")));
                }
                out
            }
            None => format!(
                "Error: room '{}' not found in area '{}'",
                p.room_id, p.area_id
            ),
        }
    }

    #[tool(description = "Create a new room in an area")]
    fn create_room(&self, params: Parameters<CreateRoomParams>) -> String {
        let p = params.0;
        let area_id = &p.area_id;
        let room_id = &p.room_id;
        if let Err(e) = self.validate_id(area_id) {
            return format!("Error: {e}");
        }
        if let Err(e) = self.validate_id(room_id) {
            return format!("Error: {e}");
        }
        let (_, file_map) = self.load();

        let area_dir = match content::area_dir_from_file(&file_map, area_id) {
            Ok(d) => d,
            Err(e) => return format!("Error: {e}"),
        };

        let room_path = area_dir.join("rooms").join(format!("{room_id}.toml"));
        if let Err(e) = self.validate_and_contain(room_id, &room_path) {
            return format!("Error: {e}");
        }
        if room_path.exists() {
            return format!(
                "Error: room '{}' already exists in area '{}'",
                room_id, area_id
            );
        }
        let room = RoomTemplate {
            id: room_id.clone(),
            area: area_id.clone(),
            name: p.name,
            description: String::new(),
            exits: HashMap::new(),
            portals: Vec::new(),
            flags: Vec::new(),
            content: RoomContent::default(),
            allow_revive: false,
            no_weather: false,
            exclude_weather: Vec::new(),
            additional_weather: HashMap::new(),
            script: None,
            params: HashMap::new(),
        };
        let room_str = match toml::to_string_pretty(&room) {
            Ok(s) => s,
            Err(e) => return format!("Error: failed to serialize room: {e}"),
        };
        if let Err(e) = fs::create_dir_all(room_path.parent().unwrap())
            .and_then(|_| fs::write(&room_path, &room_str))
        {
            return format!("Error: failed to write {}: {e}", room_path.display());
        }
        format!("Created room '{}' in area '{}'", room_id, area_id)
    }

    #[tool(description = "Delete a room from an area")]
    fn delete_room(&self, params: Parameters<RoomIdParam>) -> String {
        let p = params.0;
        let (_, file_map) = self.load();
        let room_key = format!("{}:{}", p.area_id, p.room_id);
        let room_path = match file_map.get("rooms").and_then(|m| m.get(&room_key)) {
            Some(p) => p.clone(),
            None => {
                return format!(
                    "Error: room '{}' not found in area '{}'",
                    p.room_id, p.area_id
                )
            }
        };

        if let Err(e) = fs::remove_file(&room_path) {
            return format!("Error: failed to delete {}: {e}", room_path.display());
        }
        format!("Deleted room '{}' from area '{}'", p.room_id, p.area_id)
    }

    #[tool(description = "Link two rooms together by adding an exit")]
    fn link_rooms(&self, params: Parameters<LinkRoomsParams>) -> String {
        let p = params.0;
        let (_, file_map) = self.load();
        let room_key = format!("{}:{}", p.area_id, p.from_room);
        let room_path = match file_map.get("rooms").and_then(|m| m.get(&room_key)) {
            Some(p) => p.clone(),
            None => {
                return format!(
                    "Error: room '{}' not found in area '{}'",
                    p.from_room, p.area_id
                )
            }
        };

        let room_content = match fs::read_to_string(&room_path) {
            Ok(c) => c,
            Err(e) => return format!("Error: failed to read {}: {e}", room_path.display()),
        };
        let mut room: RoomTemplate = match toml::from_str(&room_content) {
            Ok(r) => r,
            Err(e) => return format!("Error: failed to parse room: {e}"),
        };
        let dest = format!("{}:{}", p.to_area, p.to_room);
        room.exits
            .insert(p.direction.clone(), ExitTemplate::Simple(dest.clone()));
        match toml::to_string_pretty(&room) {
            Ok(out) => {
                if let Err(e) = fs::write(&room_path, &out) {
                    return format!("Error: failed to write {}: {e}", room_path.display());
                }
            }
            Err(e) => return format!("Error: failed to serialize room: {e}"),
        }
        format!(
            "Linked {} -> {}:{} via {}.{}",
            p.from_room, p.to_area, p.to_room, p.area_id, p.direction
        )
    }

    #[tool(description = "Add a portal (keyword-based exit) to a room")]
    fn add_portal(&self, params: Parameters<UpdateRoomFieldsParams>) -> String {
        let p = params.0;
        if !p.fields.contains_key("keyword") || !p.fields.contains_key("dest") {
            return "Error: 'keyword' and 'dest' fields are required for a portal".to_string();
        }
        self.update_room_fields(&p.area_id, &p.room_id, &p.fields)
    }

    #[tool(description = "Remove a portal from a room (set keyword to empty)")]
    fn remove_portal(&self, params: Parameters<UpdateRoomFieldsParams>) -> String {
        let p = params.0;
        self.update_room_fields(&p.area_id, &p.room_id, &p.fields)
    }

    #[tool(description = "Update a room's fields inline")]
    fn update_room(&self, params: Parameters<UpdateRoomFieldsParams>) -> String {
        let p = params.0;
        self.update_room_fields(&p.area_id, &p.room_id, &p.fields)
    }

    #[tool(
        description = "Update fields on any template type except rooms (use update_room for rooms)"
    )]
    fn update_template(&self, params: Parameters<UpdateFieldsParams>) -> String {
        let p = params.0;
        let (_, file_map) = self.load();
        if p.category == "rooms" {
            return "Error: use update_room for room fields".to_string();
        }
        let path = match content::find_file(&file_map, &p.category, &p.id) {
            Ok(p) => p,
            Err(e) => return format!("Error: {e}"),
        };
        let content = match fs::read_to_string(&path) {
            Ok(c) => c,
            Err(e) => return format!("Error: failed to read {}: {e}", path.display()),
        };

        // Parse the file, round-trip through JSON to apply patches, then
        // serialize back through the concrete struct type for proper TOML output.
        let toml_val: toml::Value = match content.parse() {
            Ok(v) => v,
            Err(e) => return format!("Error: failed to parse TOML: {e}"),
        };
        let mut json_val: serde_json::Value = match serde_json::to_value(&toml_val) {
            Ok(v) => v,
            Err(e) => return format!("Error: failed to convert to JSON: {e}"),
        };
        if let Some(obj) = json_val.as_object_mut() {
            for (key, value) in &p.fields {
                obj.insert(key.clone(), value.clone());
            }
        }

        let out = match p.category.as_str() {
            "mobs" => {
                let t: MobTemplate = match serde_json::from_value(json_val) {
                    Ok(t) => t,
                    Err(e) => return format!("Error: failed to deserialize mob after patch: {e}"),
                };
                match toml::to_string_pretty(&t) {
                    Ok(s) => s,
                    Err(e) => return format!("Error: failed to serialize mob: {e}"),
                }
            }
            "items" => {
                let t: ItemTemplate = match serde_json::from_value(json_val) {
                    Ok(t) => t,
                    Err(e) => return format!("Error: failed to deserialize item after patch: {e}"),
                };
                match toml::to_string_pretty(&t) {
                    Ok(s) => s,
                    Err(e) => return format!("Error: failed to serialize item: {e}"),
                }
            }
            "races" => {
                let t: RaceTemplate = match serde_json::from_value(json_val) {
                    Ok(t) => t,
                    Err(e) => return format!("Error: failed to deserialize race after patch: {e}"),
                };
                match toml::to_string_pretty(&t) {
                    Ok(s) => s,
                    Err(e) => return format!("Error: failed to serialize race: {e}"),
                }
            }
            "classes" => {
                let t: ClassTemplate = match serde_json::from_value(json_val) {
                    Ok(t) => t,
                    Err(e) => {
                        return format!("Error: failed to deserialize class after patch: {e}")
                    }
                };
                match toml::to_string_pretty(&t) {
                    Ok(s) => s,
                    Err(e) => return format!("Error: failed to serialize class: {e}"),
                }
            }
            "skills" => {
                let t: SkillDef = match serde_json::from_value(json_val) {
                    Ok(t) => t,
                    Err(e) => {
                        return format!("Error: failed to deserialize skill after patch: {e}")
                    }
                };
                match toml::to_string_pretty(&t) {
                    Ok(s) => s,
                    Err(e) => return format!("Error: failed to serialize skill: {e}"),
                }
            }
            "stances" => {
                let t: StanceDef = match serde_json::from_value(json_val) {
                    Ok(t) => t,
                    Err(e) => {
                        return format!("Error: failed to deserialize stance after patch: {e}")
                    }
                };
                match toml::to_string_pretty(&t) {
                    Ok(s) => s,
                    Err(e) => return format!("Error: failed to serialize stance: {e}"),
                }
            }
            "sets" => {
                let t: SetDef = match serde_json::from_value(json_val) {
                    Ok(t) => t,
                    Err(e) => return format!("Error: failed to deserialize set after patch: {e}"),
                };
                match toml::to_string_pretty(&t) {
                    Ok(s) => s,
                    Err(e) => return format!("Error: failed to serialize set: {e}"),
                }
            }
            "affixes" => {
                let t: AffixDef = match serde_json::from_value(json_val) {
                    Ok(t) => t,
                    Err(e) => {
                        return format!("Error: failed to deserialize affix after patch: {e}")
                    }
                };
                match toml::to_string_pretty(&t) {
                    Ok(s) => s,
                    Err(e) => return format!("Error: failed to serialize affix: {e}"),
                }
            }
            "passives" => {
                let t: PassiveDef = match serde_json::from_value(json_val) {
                    Ok(t) => t,
                    Err(e) => {
                        return format!("Error: failed to deserialize passive after patch: {e}")
                    }
                };
                match toml::to_string_pretty(&t) {
                    Ok(s) => s,
                    Err(e) => return format!("Error: failed to serialize passive: {e}"),
                }
            }
            "shops" => {
                let t: ShopTemplate = match serde_json::from_value(json_val) {
                    Ok(t) => t,
                    Err(e) => return format!("Error: failed to deserialize shop after patch: {e}"),
                };
                match toml::to_string_pretty(&t) {
                    Ok(s) => s,
                    Err(e) => return format!("Error: failed to serialize shop: {e}"),
                }
            }
            "deities" => {
                let t: DeityTemplate = match serde_json::from_value(json_val) {
                    Ok(t) => t,
                    Err(e) => {
                        return format!("Error: failed to deserialize deity after patch: {e}")
                    }
                };
                match toml::to_string_pretty(&t) {
                    Ok(s) => s,
                    Err(e) => return format!("Error: failed to serialize deity: {e}"),
                }
            }
            "quests" => {
                let t: QuestDef = match serde_json::from_value(json_val) {
                    Ok(t) => t,
                    Err(e) => {
                        return format!("Error: failed to deserialize quest after patch: {e}")
                    }
                };
                match toml::to_string_pretty(&t) {
                    Ok(s) => s,
                    Err(e) => return format!("Error: failed to serialize quest: {e}"),
                }
            }
            "factions" => {
                let t: FactionDef = match serde_json::from_value(json_val) {
                    Ok(t) => t,
                    Err(e) => {
                        return format!("Error: failed to deserialize faction after patch: {e}")
                    }
                };
                match toml::to_string_pretty(&t) {
                    Ok(s) => s,
                    Err(e) => return format!("Error: failed to serialize faction: {e}"),
                }
            }
            "recipes" => {
                let t: RecipeDef = match serde_json::from_value(json_val) {
                    Ok(t) => t,
                    Err(e) => {
                        return format!("Error: failed to deserialize recipe after patch: {e}")
                    }
                };
                match toml::to_string_pretty(&t) {
                    Ok(s) => s,
                    Err(e) => return format!("Error: failed to serialize recipe: {e}"),
                }
            }
            "areas" => {
                let t: AreaTemplate = match serde_json::from_value(json_val) {
                    Ok(t) => t,
                    Err(e) => return format!("Error: failed to deserialize area after patch: {e}"),
                };
                let rooms = t.rooms.clone();
                let mut meta = t;
                meta.rooms = HashMap::new();
                let area_dir = match path.parent() {
                    Some(d) => d.to_path_buf(),
                    None => return "Error: area path has no parent".to_string(),
                };
                let rooms_dir = area_dir.join("rooms");
                let meta_str = match toml::to_string_pretty(&meta) {
                    Ok(s) => s,
                    Err(e) => return format!("Error: failed to serialize area: {e}"),
                };
                if let Err(e) = fs::create_dir_all(&rooms_dir) {
                    return format!("Error: failed to create rooms dir: {e}");
                }
                if let Err(e) = fs::write(area_dir.join("area.toml"), &meta_str) {
                    return format!("Error: failed to write area.toml: {e}");
                }
                for (room_id, room) in &rooms {
                    let room_str = match toml::to_string_pretty(room) {
                        Ok(s) => s,
                        Err(e) => return format!("Error: failed to serialize room {room_id}: {e}"),
                    };
                    let room_path = area_dir.join("rooms").join(format!("{room_id}.toml"));
                    if let Err(e) = fs::write(&room_path, &room_str) {
                        return format!("Error: failed to write {room_id}: {e}");
                    }
                }
                return format!(
                    "Updated {} field(s) on {}/{}",
                    p.fields.len(),
                    p.category,
                    p.id
                );
            }
            other => return format!("Error: unknown category '{other}'"),
        };

        if let Err(e) = fs::write(&path, &out) {
            return format!("Error: failed to write {}: {e}", path.display());
        }
        format!(
            "Updated {} field(s) on {}/{}",
            p.fields.len(),
            p.category,
            p.id
        )
    }

    #[tool(description = "List all mob templates")]
    fn list_mobs(&self) -> String {
        let (registry, _) = self.load();
        Self::entity_list(
            &registry
                .mobs
                .iter()
                .map(|(k, v)| (k.clone(), v.name.as_str()))
                .collect(),
            "Mobs",
        )
    }

    #[tool(description = "Get mob template details")]
    fn get_mob(&self, params: Parameters<IdParam>) -> String {
        let p = params.0;
        let (registry, _file_map) = self.load();
        match registry.get_mob(&p.id) {
            Some(mob) => {
                let route = if mob.patrol_route.is_empty() {
                    String::new()
                } else {
                    format!("\npatrol_route: {:?}", mob.patrol_route)
                };
                let w_rooms = if mob.wander_rooms.is_empty() {
                    String::new()
                } else {
                    format!("\nwander_rooms: {:?}", mob.wander_rooms)
                };
                let w_area = if mob.wander_area {
                    "\nwander_area: true".to_string()
                } else {
                    String::new()
                };
                format!(
                    "id: {}\nname: {}\nlevel: {}\ndescription: {}\narmor: {}\nai: {}{}{}{}",
                    p.id,
                    mob.name,
                    mob.level,
                    mob.description,
                    mob.armor,
                    mob.ai_mode,
                    route,
                    w_rooms,
                    w_area
                )
            }
            None => format!("Error: mob '{}' not found", p.id),
        }
    }

    #[tool(description = "Create a new mob template")]
    fn create_mob(&self, params: Parameters<CreateEntityParams>) -> String {
        let p = params.0;
        if let Err(e) = self.validate_id(&p.id) {
            return format!("Error: {e}");
        }
        let name = p.name.unwrap_or_else(|| p.id.clone());
        let path = self
            .content_path
            .join("mobs")
            .join(format!("{}.toml", p.id));
        if let Err(e) = self.validate_and_contain(&p.id, &path) {
            return format!("Error: {e}");
        }
        let mob = MobTemplate {
            id: p.id.clone(),
            name,
            description: String::new(),
            short_desc: String::new(),
            level: 1,
            attributes: RaceAttributes::default(),
            health: HealthBounds {
                current: 10,
                max: 10,
            },
            armor: 0,
            damage: None,
            damage_type: None,
            race: None,
            size: "medium".to_string(),
            equipment: Vec::new(),
            xp_value: 0,
            loot: LootTable::default(),
            ai_mode: "idle".to_string(),
            patrol_route: Vec::new(),
            wander_rooms: Vec::new(),
            wander_area: false,
            aggro_range: 0,
            aggro_players: false,
            aggro_mobs: false,
            aggro_race: Vec::new(),
            faction: None,
            faction_standing: 0,
            trainer_types: Vec::new(),
            languages: Vec::new(),
            shop: None,
            friendly: false,
            skills: Vec::new(),
            scripts: Vec::new(),
            params: HashMap::new(),
        };
        match toml::to_string_pretty(&mob) {
            Ok(content) => {
                if let Err(e) = fs::create_dir_all(path.parent().unwrap())
                    .and_then(|_| fs::write(&path, &content))
                {
                    return format!("Error: failed to write mob: {e}");
                }
                format!("Created mob '{}'", p.id)
            }
            Err(e) => format!("Error: failed to serialize mob: {e}"),
        }
    }

    #[tool(description = "Delete a mob template")]
    fn delete_mob(&self, params: Parameters<IdParam>) -> String {
        let p = params.0;
        let (_, file_map) = self.load();
        match content::delete_file(&file_map, "mobs", &p.id) {
            Ok(()) => format!("Deleted mob '{}'", p.id),
            Err(e) => format!("Error: {e}"),
        }
    }

    #[tool(description = "List all item templates")]
    fn list_items(&self) -> String {
        let (registry, _) = self.load();
        Self::entity_list(
            &registry
                .items
                .iter()
                .map(|(k, v)| (k.clone(), v.name.as_str()))
                .collect(),
            "Items",
        )
    }

    #[tool(description = "Get item template details")]
    fn get_item(&self, params: Parameters<IdParam>) -> String {
        let p = params.0;
        let (registry, _file_map) = self.load();
        match registry.get_item(&p.id) {
            Some(item) => format!(
                "id: {}\nname: {}\ntype: {}\nquality: {}\ndescription: {}",
                p.id, item.name, item.item_type, item.quality, item.description
            ),
            None => format!("Error: item '{}' not found", p.id),
        }
    }

    #[tool(description = "Create a new item template")]
    fn create_item(&self, params: Parameters<CreateEntityParams>) -> String {
        let p = params.0;
        if let Err(e) = self.validate_id(&p.id) {
            return format!("Error: {e}");
        }
        let name = p.name.unwrap_or_else(|| p.id.clone());
        let path = self
            .content_path
            .join("items")
            .join(format!("{}.toml", p.id));
        if let Err(e) = self.validate_and_contain(&p.id, &path) {
            return format!("Error: {e}");
        }
        let item = ItemTemplate {
            id: p.id.clone(),
            name,
            description: String::new(),
            item_type: "misc".to_string(),
            subtype: String::new(),
            quality: "common".to_string(),
            level_requirement: 1,
            weight: 1.0,
            value: 0,
            flags: Vec::new(),
            allowed_classes: Vec::new(),
            allowed_races: Vec::new(),
            allowed_alignments: Vec::new(),
            requires_skill: None,
            weapon: None,
            equipment: None,
            set: None,
            triggers: Vec::new(),
            params: HashMap::new(),
        };
        match toml::to_string_pretty(&item) {
            Ok(content) => {
                if let Err(e) = fs::create_dir_all(path.parent().unwrap())
                    .and_then(|_| fs::write(&path, &content))
                {
                    return format!("Error: failed to write item: {e}");
                }
                format!("Created item '{}'", p.id)
            }
            Err(e) => format!("Error: failed to serialize item: {e}"),
        }
    }

    #[tool(description = "Delete an item template")]
    fn delete_item(&self, params: Parameters<IdParam>) -> String {
        let p = params.0;
        let (_, file_map) = self.load();
        match content::delete_file(&file_map, "items", &p.id) {
            Ok(()) => format!("Deleted item '{}'", p.id),
            Err(e) => format!("Error: {e}"),
        }
    }

    #[tool(description = "List all quest templates")]
    fn list_quests(&self) -> String {
        let (registry, _) = self.load();
        Self::entity_list(
            &registry
                .quests
                .iter()
                .map(|(k, v)| (k.clone(), v.name.as_str()))
                .collect(),
            "Quests",
        )
    }

    #[tool(description = "Get quest template details")]
    fn get_quest(&self, params: Parameters<IdParam>) -> String {
        let p = params.0;
        let (registry, _file_map) = self.load();
        match registry.quests.get(&p.id) {
            Some(q) => {
                let objectives: Vec<String> =
                    q.objectives.iter().map(|o| format!("{:?}", o)).collect();
                let rewards_items: Vec<String> = q
                    .rewards
                    .items
                    .iter()
                    .map(|r| format!("{} x{}", r.item_template_id, r.count))
                    .collect();
                let rewards_faction: Vec<String> = q
                    .rewards
                    .faction
                    .iter()
                    .map(|r| format!("{} {:+}", r.faction_id, r.amount))
                    .collect();
                format!(
                    "id: {}\nname: {}\ndescription: {}\nlevel_requirement: {}\nrepeatable: {}\nauto_complete: {}\ngiver_npc: {}\nturn_in_npc: {}\nprerequisites: {:?}\nobjectives: [{}]\nrewards: xp={}, gold={}, items=[{}], faction=[{}]",
                    p.id,
                    q.name,
                    q.description,
                    q.level_requirement,
                    q.repeatable,
                    q.auto_complete,
                    q.giver_npc.as_deref().unwrap_or("none"),
                    q.turn_in_npc.as_deref().unwrap_or("none"),
                    q.prerequisites,
                    objectives.join(", "),
                    q.rewards.xp,
                    q.rewards.gold,
                    rewards_items.join(", "),
                    rewards_faction.join(", "),
                )
            }
            None => format!("Error: quest '{}' not found", p.id),
        }
    }

    #[tool(description = "Create a new quest template")]
    fn create_quest(&self, params: Parameters<CreateEntityParams>) -> String {
        let p = params.0;
        if let Err(e) = self.validate_id(&p.id) {
            return format!("Error: {e}");
        }
        let name = p.name.unwrap_or_else(|| p.id.clone());
        let path = self
            .content_path
            .join("quests")
            .join(format!("{}.toml", p.id));
        if let Err(e) = self.validate_and_contain(&p.id, &path) {
            return format!("Error: {e}");
        }
        let quest = QuestDef {
            id: p.id.clone(),
            name,
            description: String::new(),
            level_requirement: 1,
            repeatable: false,
            auto_complete: false,
            giver_npc: None,
            turn_in_npc: None,
            prerequisites: Vec::new(),
            objectives: Vec::new(),
            rewards: QuestRewards {
                xp: 0,
                gold: 0,
                items: Vec::new(),
                faction: Vec::new(),
            },
            scripts: None,
            params: HashMap::new(),
        };
        match toml::to_string_pretty(&quest) {
            Ok(content) => {
                if let Err(e) = fs::create_dir_all(path.parent().unwrap())
                    .and_then(|_| fs::write(&path, &content))
                {
                    return format!("Error: failed to write quest: {e}");
                }
                format!("Created quest '{}'", p.id)
            }
            Err(e) => format!("Error: failed to serialize quest: {e}"),
        }
    }

    #[tool(description = "Delete a quest template")]
    fn delete_quest(&self, params: Parameters<IdParam>) -> String {
        let p = params.0;
        let (_, file_map) = self.load();
        match content::delete_file(&file_map, "quests", &p.id) {
            Ok(()) => format!("Deleted quest '{}'", p.id),
            Err(e) => format!("Error: {e}"),
        }
    }

    #[tool(description = "List all faction templates")]
    fn list_factions(&self) -> String {
        let (registry, _) = self.load();
        Self::entity_list(
            &registry
                .factions
                .iter()
                .map(|(k, v)| (k.clone(), v.name.as_str()))
                .collect(),
            "Factions",
        )
    }

    #[tool(description = "Get faction template details")]
    fn get_faction(&self, params: Parameters<IdParam>) -> String {
        let p = params.0;
        let (registry, _file_map) = self.load();
        match registry.factions.get(&p.id) {
            Some(f) => {
                let ranks: Vec<String> = f
                    .ranks
                    .iter()
                    .map(|r| format!("{} (threshold: {})", r.name, r.threshold))
                    .collect();
                format!(
                    "id: {}\nname: {}\ndescription: {}\nstarting_standing: {}\nmin_standing: {}\nmax_standing: {}\naggro_below: {}\nranks: [{}]\nrelationships: {:?}",
                    p.id, f.name, f.description, f.starting_standing,
                    f.min_standing, f.max_standing, f.aggro_below,
                    ranks.join(", "), f.relationships,
                )
            }
            None => format!("Error: faction '{}' not found", p.id),
        }
    }

    #[tool(description = "Create a new faction template")]
    fn create_faction(&self, params: Parameters<CreateEntityParams>) -> String {
        let p = params.0;
        if let Err(e) = self.validate_id(&p.id) {
            return format!("Error: {e}");
        }
        let name = p.name.unwrap_or_else(|| p.id.clone());
        let path = self
            .content_path
            .join("factions")
            .join(format!("{}.toml", p.id));
        if let Err(e) = self.validate_and_contain(&p.id, &path) {
            return format!("Error: {e}");
        }
        let faction = FactionDef {
            id: p.id.clone(),
            name,
            description: String::new(),
            starting_standing: 0,
            min_standing: -10000,
            max_standing: 10000,
            ranks: Vec::new(),
            relationships: HashMap::new(),
            aggro_below: -500,
        };
        match toml::to_string_pretty(&faction) {
            Ok(content) => {
                if let Err(e) = fs::create_dir_all(path.parent().unwrap())
                    .and_then(|_| fs::write(&path, &content))
                {
                    return format!("Error: failed to write faction: {e}");
                }
                format!("Created faction '{}'", p.id)
            }
            Err(e) => format!("Error: failed to serialize faction: {e}"),
        }
    }

    #[tool(description = "Delete a faction template")]
    fn delete_faction(&self, params: Parameters<IdParam>) -> String {
        let p = params.0;
        let (_, file_map) = self.load();
        match content::delete_file(&file_map, "factions", &p.id) {
            Ok(()) => format!("Deleted faction '{}'", p.id),
            Err(e) => format!("Error: {e}"),
        }
    }

    #[tool(description = "List all recipe templates")]
    fn list_recipes(&self) -> String {
        let (registry, _) = self.load();
        Self::entity_list(
            &registry
                .recipes
                .iter()
                .map(|(k, v)| (k.clone(), v.name.as_str()))
                .collect(),
            "Recipes",
        )
    }

    #[tool(description = "Get recipe template details")]
    fn get_recipe(&self, params: Parameters<IdParam>) -> String {
        let p = params.0;
        let (registry, _file_map) = self.load();
        match registry.recipes.get(&p.id) {
            Some(r) => {
                let materials: Vec<String> = r
                    .materials
                    .iter()
                    .map(|m| format!("{} x{}", m.template_id, m.quantity))
                    .collect();
                let skill_req = r
                    .skill_requirement
                    .as_ref()
                    .map(|s| format!("{} rank {}", s.id, s.rank))
                    .unwrap_or_else(|| "none".to_string());
                format!(
                    "id: {}\nname: {}\ndescription: {}\nstation: {}\nskill_requirement: {}\ndifficulty: {}\nmaterials: [{}]\nresult: {} x{}\nsuccess_chance: {}\nquality_scaling: {}",
                    p.id, r.name, r.description,
                    r.station.as_deref().unwrap_or("none"),
                    skill_req, r.difficulty,
                    materials.join(", "),
                    r.result.template_id, r.result.quantity,
                    r.success_chance, r.quality_scaling,
                )
            }
            None => format!("Error: recipe '{}' not found", p.id),
        }
    }

    #[tool(description = "Create a new recipe template")]
    fn create_recipe(&self, params: Parameters<CreateEntityParams>) -> String {
        let p = params.0;
        if let Err(e) = self.validate_id(&p.id) {
            return format!("Error: {e}");
        }
        let name = p.name.unwrap_or_else(|| p.id.clone());
        let path = self
            .content_path
            .join("recipes")
            .join(format!("{}.toml", p.id));
        if let Err(e) = self.validate_and_contain(&p.id, &path) {
            return format!("Error: {e}");
        }
        let recipe = RecipeDef {
            id: p.id.clone(),
            name,
            description: String::new(),
            station: None,
            skill_requirement: None,
            difficulty: 1,
            materials: Vec::new(),
            result: oxide_core::templates::RecipeResult {
                template_id: String::new(),
                quantity: 1,
            },
            success_chance: 95,
            quality_scaling: false,
            script: None,
        };
        match toml::to_string_pretty(&recipe) {
            Ok(content) => {
                if let Err(e) = fs::create_dir_all(path.parent().unwrap())
                    .and_then(|_| fs::write(&path, &content))
                {
                    return format!("Error: failed to write recipe: {e}");
                }
                format!("Created recipe '{}'", p.id)
            }
            Err(e) => format!("Error: failed to serialize recipe: {e}"),
        }
    }

    #[tool(description = "Delete a recipe template")]
    fn delete_recipe(&self, params: Parameters<IdParam>) -> String {
        let p = params.0;
        let (_, file_map) = self.load();
        match content::delete_file(&file_map, "recipes", &p.id) {
            Ok(()) => format!("Deleted recipe '{}'", p.id),
            Err(e) => format!("Error: {e}"),
        }
    }

    #[tool(description = "List all shop templates")]
    fn list_shops(&self) -> String {
        let (registry, _) = self.load();
        Self::entity_list(
            &registry
                .shops
                .iter()
                .map(|(k, v)| (k.clone(), v.name.as_str()))
                .collect(),
            "Shops",
        )
    }

    #[tool(description = "Get shop template details")]
    fn get_shop(&self, params: Parameters<IdParam>) -> String {
        let p = params.0;
        let (registry, _file_map) = self.load();
        match registry.shops.get(&p.id) {
            Some(s) => {
                let inv: Vec<String> = s
                    .inventory
                    .iter()
                    .map(|e| {
                        format!(
                            "{} ({}-{}, price: {})",
                            e.item, e.count.min, e.count.max, e.price
                        )
                    })
                    .collect();
                format!(
                    "id: {}\nname: {}\nbuy_rate: {}\nsell_rate: {}\nrestock_secs: {}\ninventory: [{}]",
                    p.id, s.name, s.buy_rate, s.sell_rate, s.restock_secs,
                    inv.join(", "),
                )
            }
            None => format!("Error: shop '{}' not found", p.id),
        }
    }

    #[tool(description = "Create a new shop template")]
    fn create_shop(&self, params: Parameters<CreateEntityParams>) -> String {
        let p = params.0;
        if let Err(e) = self.validate_id(&p.id) {
            return format!("Error: {e}");
        }
        let name = p.name.unwrap_or_else(|| p.id.clone());
        let path = self
            .content_path
            .join("shops")
            .join(format!("{}.toml", p.id));
        if let Err(e) = self.validate_and_contain(&p.id, &path) {
            return format!("Error: {e}");
        }
        let shop = oxide_core::templates::ShopTemplate {
            id: p.id.clone(),
            name,
            buy_rate: 0.5,
            sell_rate: 1.0,
            restock_secs: 3600,
            inventory: Vec::new(),
            params: HashMap::new(),
        };
        match toml::to_string_pretty(&shop) {
            Ok(content) => {
                if let Err(e) = fs::create_dir_all(path.parent().unwrap())
                    .and_then(|_| fs::write(&path, &content))
                {
                    return format!("Error: failed to write shop: {e}");
                }
                format!("Created shop '{}'", p.id)
            }
            Err(e) => format!("Error: failed to serialize shop: {e}"),
        }
    }

    #[tool(description = "Delete a shop template")]
    fn delete_shop(&self, params: Parameters<IdParam>) -> String {
        let p = params.0;
        let (_, file_map) = self.load();
        match content::delete_file(&file_map, "shops", &p.id) {
            Ok(()) => format!("Deleted shop '{}'", p.id),
            Err(e) => format!("Error: {e}"),
        }
    }

    #[tool(description = "List all deity templates")]
    fn list_deities(&self) -> String {
        let (registry, _) = self.load();
        Self::entity_list(
            &registry
                .deities
                .iter()
                .map(|(k, v)| (k.clone(), v.name.as_str()))
                .collect(),
            "Deities",
        )
    }

    #[tool(description = "Get deity template details")]
    fn get_deity(&self, params: Parameters<IdParam>) -> String {
        let p = params.0;
        let (registry, _file_map) = self.load();
        match registry.deities.get(&p.id) {
            Some(d) => {
                let prayer = d
                    .prayer_effect
                    .as_ref()
                    .map(|pe| {
                        format!(
                            "buff={}, duration={}s, cooldown={}s, {}",
                            pe.buff_id, pe.duration_secs, pe.cooldown_secs, pe.description
                        )
                    })
                    .unwrap_or_else(|| "none".to_string());
                format!(
                    "id: {}\nname: {}\ndescription: {}\nalignment: {}\nsymbol: {}\nfavored_weapon: {}\ntenets: {:?}\ndomains: {:?}\nallowed_races: {:?}\nallowed_classes: {:?}\nallowed_alignments: {:?}\nprayer_effect: {}",
                    p.id, d.name, d.description,
                    d.alignment.as_deref().unwrap_or("any"),
                    d.symbol,
                    d.favored_weapon.as_deref().unwrap_or("none"),
                    d.tenets, d.domains, d.allowed_races, d.allowed_classes,
                    d.allowed_alignments, prayer,
                )
            }
            None => format!("Error: deity '{}' not found", p.id),
        }
    }

    #[tool(description = "Create a new deity template")]
    fn create_deity(&self, params: Parameters<CreateEntityParams>) -> String {
        let p = params.0;
        if let Err(e) = self.validate_id(&p.id) {
            return format!("Error: {e}");
        }
        let name = p.name.unwrap_or_else(|| p.id.clone());
        let path = self
            .content_path
            .join("deities")
            .join(format!("{}.toml", p.id));
        if let Err(e) = self.validate_and_contain(&p.id, &path) {
            return format!("Error: {e}");
        }
        let deity = oxide_core::templates::DeityTemplate {
            id: p.id.clone(),
            name,
            description: String::new(),
            alignment: None,
            symbol: String::new(),
            favored_weapon: None,
            tenets: Vec::new(),
            domains: Vec::new(),
            allowed_races: Vec::new(),
            allowed_classes: Vec::new(),
            allowed_alignments: Vec::new(),
            prayer_effect: None,
            params: HashMap::new(),
        };
        match toml::to_string_pretty(&deity) {
            Ok(content) => {
                if let Err(e) = fs::create_dir_all(path.parent().unwrap())
                    .and_then(|_| fs::write(&path, &content))
                {
                    return format!("Error: failed to write deity: {e}");
                }
                format!("Created deity '{}'", p.id)
            }
            Err(e) => format!("Error: failed to serialize deity: {e}"),
        }
    }

    #[tool(description = "Delete a deity template")]
    fn delete_deity(&self, params: Parameters<IdParam>) -> String {
        let p = params.0;
        let (_, file_map) = self.load();
        match content::delete_file(&file_map, "deities", &p.id) {
            Ok(()) => format!("Deleted deity '{}'", p.id),
            Err(e) => format!("Error: {e}"),
        }
    }

    #[tool(description = "List all stance templates")]
    fn list_stances(&self) -> String {
        let (registry, _) = self.load();
        Self::entity_list(
            &registry
                .stances
                .iter()
                .map(|(k, v)| (k.clone(), v.name.as_str()))
                .collect(),
            "Stances",
        )
    }

    #[tool(description = "Get stance template details")]
    fn get_stance(&self, params: Parameters<IdParam>) -> String {
        let p = params.0;
        let (registry, _file_map) = self.load();
        match registry.stances.get(&p.id) {
            Some(s) => format!(
                "id: {}\nname: {}\nac_bonus: {}\nattack_penalty: {}\ndamage_bonus: {}\nac_penalty: {}\nmin_level: {}",
                p.id, s.name, s.ac_bonus, s.attack_penalty, s.damage_bonus, s.ac_penalty, s.min_level,
            ),
            None => format!("Error: stance '{}' not found", p.id),
        }
    }

    #[tool(description = "Create a new stance template")]
    fn create_stance(&self, params: Parameters<CreateEntityParams>) -> String {
        let p = params.0;
        if let Err(e) = self.validate_id(&p.id) {
            return format!("Error: {e}");
        }
        let name = p.name.unwrap_or_else(|| p.id.clone());
        let path = self
            .content_path
            .join("stances")
            .join(format!("{}.toml", p.id));
        if let Err(e) = self.validate_and_contain(&p.id, &path) {
            return format!("Error: {e}");
        }
        let stance = StanceDef {
            id: p.id.clone(),
            name,
            ac_bonus: 0,
            attack_penalty: 0,
            damage_bonus: 0,
            ac_penalty: 0,
            min_level: 1,
            params: HashMap::new(),
        };
        match toml::to_string_pretty(&stance) {
            Ok(content) => {
                if let Err(e) = fs::create_dir_all(path.parent().unwrap())
                    .and_then(|_| fs::write(&path, &content))
                {
                    return format!("Error: failed to write stance: {e}");
                }
                format!("Created stance '{}'", p.id)
            }
            Err(e) => format!("Error: failed to serialize stance: {e}"),
        }
    }

    #[tool(description = "Delete a stance template")]
    fn delete_stance(&self, params: Parameters<IdParam>) -> String {
        let p = params.0;
        let (_, file_map) = self.load();
        match content::delete_file(&file_map, "stances", &p.id) {
            Ok(()) => format!("Deleted stance '{}'", p.id),
            Err(e) => format!("Error: {e}"),
        }
    }

    #[tool(description = "List all item set templates")]
    fn list_sets(&self) -> String {
        let (registry, _) = self.load();
        Self::entity_list(
            &registry
                .sets
                .iter()
                .map(|(k, v)| (k.clone(), v.name.as_str()))
                .collect(),
            "Sets",
        )
    }

    #[tool(description = "Get item set template details")]
    fn get_set(&self, params: Parameters<IdParam>) -> String {
        let p = params.0;
        let (registry, _file_map) = self.load();
        match registry.sets.get(&p.id) {
            Some(s) => {
                let bonuses: Vec<String> = s
                    .bonuses
                    .iter()
                    .map(|b| {
                        let effects: Vec<String> = b
                            .effects
                            .iter()
                            .map(|e| {
                                format!(
                                    "{} {} {:?}",
                                    e.effect_type,
                                    e.stat.as_deref().unwrap_or(""),
                                    e.amount
                                )
                            })
                            .collect();
                        format!("min_pieces={}: [{}]", b.min_pieces, effects.join(", "))
                    })
                    .collect();
                format!(
                    "id: {}\nname: {}\nbonuses: [{}]",
                    p.id,
                    s.name,
                    bonuses.join("; "),
                )
            }
            None => format!("Error: set '{}' not found", p.id),
        }
    }

    #[tool(description = "Create a new item set template")]
    fn create_set(&self, params: Parameters<CreateEntityParams>) -> String {
        let p = params.0;
        if let Err(e) = self.validate_id(&p.id) {
            return format!("Error: {e}");
        }
        let name = p.name.unwrap_or_else(|| p.id.clone());
        let path = self
            .content_path
            .join("sets")
            .join(format!("{}.toml", p.id));
        if let Err(e) = self.validate_and_contain(&p.id, &path) {
            return format!("Error: {e}");
        }
        let set = SetDef {
            id: p.id.clone(),
            name,
            bonuses: Vec::new(),
            params: HashMap::new(),
        };
        match toml::to_string_pretty(&set) {
            Ok(content) => {
                if let Err(e) = fs::create_dir_all(path.parent().unwrap())
                    .and_then(|_| fs::write(&path, &content))
                {
                    return format!("Error: failed to write set: {e}");
                }
                format!("Created set '{}'", p.id)
            }
            Err(e) => format!("Error: failed to serialize set: {e}"),
        }
    }

    #[tool(description = "Delete an item set template")]
    fn delete_set(&self, params: Parameters<IdParam>) -> String {
        let p = params.0;
        let (_, file_map) = self.load();
        match content::delete_file(&file_map, "sets", &p.id) {
            Ok(()) => format!("Deleted set '{}'", p.id),
            Err(e) => format!("Error: {e}"),
        }
    }

    #[tool(description = "List all affix templates")]
    fn list_affixes(&self) -> String {
        let (registry, _) = self.load();
        Self::entity_list(
            &registry
                .affixes
                .iter()
                .map(|(k, v)| (k.clone(), v.name.as_str()))
                .collect(),
            "Affixes",
        )
    }

    #[tool(description = "Get affix template details")]
    fn get_affix(&self, params: Parameters<IdParam>) -> String {
        let p = params.0;
        let (registry, _file_map) = self.load();
        match registry.affixes.get(&p.id) {
            Some(a) => format!(
                "id: {}\nname: {}\ndescription: {}\ntype: {}\nelement: {}\namount: {}\nstat: {}\nquality_min: {}\nslot: {:?}\nweight: {}",
                p.id, a.name, a.description, a.affix_type,
                a.element.as_deref().unwrap_or("none"),
                a.amount.as_deref().unwrap_or("none"),
                a.stat.as_deref().unwrap_or("none"),
                a.quality_min, a.slot, a.weight,
            ),
            None => format!("Error: affix '{}' not found", p.id),
        }
    }

    #[tool(description = "Create a new affix template")]
    fn create_affix(&self, params: Parameters<CreateEntityParams>) -> String {
        let p = params.0;
        if let Err(e) = self.validate_id(&p.id) {
            return format!("Error: {e}");
        }
        let name = p.name.unwrap_or_else(|| p.id.clone());
        let path = self
            .content_path
            .join("affixes")
            .join(format!("{}.toml", p.id));
        if let Err(e) = self.validate_and_contain(&p.id, &path) {
            return format!("Error: {e}");
        }
        let affix = AffixDef {
            id: p.id.clone(),
            name,
            description: String::new(),
            affix_type: "prefix".to_string(),
            element: None,
            amount: None,
            stat: None,
            quality_min: "common".to_string(),
            slot: Vec::new(),
            weight: 1,
            params: HashMap::new(),
        };
        match toml::to_string_pretty(&affix) {
            Ok(content) => {
                if let Err(e) = fs::create_dir_all(path.parent().unwrap())
                    .and_then(|_| fs::write(&path, &content))
                {
                    return format!("Error: failed to write affix: {e}");
                }
                format!("Created affix '{}'", p.id)
            }
            Err(e) => format!("Error: failed to serialize affix: {e}"),
        }
    }

    #[tool(description = "Delete an affix template")]
    fn delete_affix(&self, params: Parameters<IdParam>) -> String {
        let p = params.0;
        let (_, file_map) = self.load();
        match content::delete_file(&file_map, "affixes", &p.id) {
            Ok(()) => format!("Deleted affix '{}'", p.id),
            Err(e) => format!("Error: {e}"),
        }
    }

    #[tool(description = "List all passive templates")]
    fn list_passives(&self) -> String {
        let (registry, _) = self.load();
        Self::entity_list(
            &registry
                .passives
                .iter()
                .map(|(k, v)| (k.clone(), v.name.as_str()))
                .collect(),
            "Passives",
        )
    }

    #[tool(description = "Get passive template details")]
    fn get_passive(&self, params: Parameters<IdParam>) -> String {
        let p = params.0;
        let (registry, _file_map) = self.load();
        match registry.passives.get(&p.id) {
            Some(pas) => {
                let effects: Vec<String> = pas
                    .effects
                    .iter()
                    .map(|e| format!("{} {} {:?}", e.effect_type, e.target, e.amount))
                    .collect();
                format!(
                    "id: {}\nname: {}\ndescription: {}\neffects: [{}]",
                    p.id,
                    pas.name,
                    pas.description,
                    effects.join(", "),
                )
            }
            None => format!("Error: passive '{}' not found", p.id),
        }
    }

    #[tool(description = "Create a new passive template")]
    fn create_passive(&self, params: Parameters<CreateEntityParams>) -> String {
        let p = params.0;
        if let Err(e) = self.validate_id(&p.id) {
            return format!("Error: {e}");
        }
        let name = p.name.unwrap_or_else(|| p.id.clone());
        let path = self
            .content_path
            .join("passives")
            .join(format!("{}.toml", p.id));
        if let Err(e) = self.validate_and_contain(&p.id, &path) {
            return format!("Error: {e}");
        }
        let passive = PassiveDef {
            id: p.id.clone(),
            name,
            description: String::new(),
            effects: Vec::new(),
            params: HashMap::new(),
        };
        match toml::to_string_pretty(&passive) {
            Ok(content) => {
                if let Err(e) = fs::create_dir_all(path.parent().unwrap())
                    .and_then(|_| fs::write(&path, &content))
                {
                    return format!("Error: failed to write passive: {e}");
                }
                format!("Created passive '{}'", p.id)
            }
            Err(e) => format!("Error: failed to serialize passive: {e}"),
        }
    }

    #[tool(description = "Delete a passive template")]
    fn delete_passive(&self, params: Parameters<IdParam>) -> String {
        let p = params.0;
        let (_, file_map) = self.load();
        match content::delete_file(&file_map, "passives", &p.id) {
            Ok(()) => format!("Deleted passive '{}'", p.id),
            Err(e) => format!("Error: {e}"),
        }
    }

    #[tool(description = "List all skill templates")]
    fn list_skills(&self) -> String {
        let (registry, _) = self.load();
        Self::entity_list(
            &registry
                .skills
                .iter()
                .map(|(k, v)| (k.clone(), v.name.as_str()))
                .collect(),
            "Skills",
        )
    }

    #[tool(description = "Get skill template details")]
    fn get_skill(&self, params: Parameters<IdParam>) -> String {
        let p = params.0;
        let (registry, _file_map) = self.load();
        match registry.skills.get(&p.id) {
            Some(s) => format!(
                "id: {}\nname: {}\ndescription: {}\nskill_type: {:?}\nmax_rank: {}\nlevel_requirement: {}\ncooldown_secs: {}\ntargeting: {:?}\ncost: {:?}\nallowed_classes: {:?}\nallowed_races: {:?}\nrequires_skill: {}\nmust_train: {}",
                p.id, s.name, s.description, s.skill_type, s.max_rank,
                s.level_requirement, s.cooldown_secs, s.targeting, s.cost,
                s.allowed_classes, s.allowed_races,
                s.requires_skill.as_deref().unwrap_or("none"), s.must_train,
            ),
            None => format!("Error: skill '{}' not found", p.id),
        }
    }

    #[tool(description = "Create a new skill template")]
    fn create_skill(&self, params: Parameters<CreateEntityParams>) -> String {
        let p = params.0;
        if let Err(e) = self.validate_id(&p.id) {
            return format!("Error: {e}");
        }
        let name = p.name.unwrap_or_else(|| p.id.clone());
        let path = self
            .content_path
            .join("skills")
            .join(format!("{}.toml", p.id));
        if let Err(e) = self.validate_and_contain(&p.id, &path) {
            return format!("Error: {e}");
        }
        let skill = oxide_core::SkillDef::new(
            p.id.clone(),
            name,
            String::new(),
            oxide_core::SkillType::Combat,
        );
        match toml::to_string_pretty(&skill) {
            Ok(content) => {
                if let Err(e) = fs::create_dir_all(path.parent().unwrap())
                    .and_then(|_| fs::write(&path, &content))
                {
                    return format!("Error: failed to write skill: {e}");
                }
                format!("Created skill '{}'", p.id)
            }
            Err(e) => format!("Error: failed to serialize skill: {e}"),
        }
    }

    #[tool(description = "Delete a skill template")]
    fn delete_skill(&self, params: Parameters<IdParam>) -> String {
        let p = params.0;
        let (_, file_map) = self.load();
        match content::delete_file(&file_map, "skills", &p.id) {
            Ok(()) => format!("Deleted skill '{}'", p.id),
            Err(e) => format!("Error: {e}"),
        }
    }

    #[tool(description = "List all race templates")]
    fn list_races(&self) -> String {
        let (registry, _) = self.load();
        Self::entity_list(
            &registry
                .races
                .iter()
                .map(|(k, v)| (k.clone(), v.name.as_str()))
                .collect(),
            "Races",
        )
    }

    #[tool(description = "Get race template details")]
    fn get_race(&self, params: Parameters<IdParam>) -> String {
        let p = params.0;
        let (registry, _file_map) = self.load();
        match registry.races.get(&p.id) {
            Some(r) => format!(
                "id: {}\nname: {}\ndescription: {}\nattributes: STR={} DEX={} INT={} WIS={} CON={} CHA={}\nallowed_classes: {:?}\nallowed_alignments: {:?}\nracial_abilities: {:?}\nage_default: {}\nage_max: {}",
                p.id, r.name, r.description,
                r.attributes.strength, r.attributes.dexterity, r.attributes.intelligence,
                r.attributes.wisdom, r.attributes.constitution, r.attributes.charisma,
                r.allowed_classes, r.allowed_alignments, r.racial_abilities,
                r.age_default, r.age_max,
            ),
            None => format!("Error: race '{}' not found", p.id),
        }
    }

    #[tool(description = "Create a new race template")]
    fn create_race(&self, params: Parameters<CreateEntityParams>) -> String {
        let p = params.0;
        if let Err(e) = self.validate_id(&p.id) {
            return format!("Error: {e}");
        }
        let name = p.name.unwrap_or_else(|| p.id.clone());
        let path = self
            .content_path
            .join("races")
            .join(format!("{}.toml", p.id));
        if let Err(e) = self.validate_and_contain(&p.id, &path) {
            return format!("Error: {e}");
        }
        let race = RaceTemplate {
            id: p.id.clone(),
            name,
            description: String::new(),
            attributes: RaceAttributes::default(),
            allowed_classes: Vec::new(),
            allowed_alignments: Vec::new(),
            racial_abilities: Vec::new(),
            allowed_genders: HashMap::new(),
            appearance_bounds: oxide_core::templates::AppearanceBounds::default(),
            age_default: 20,
            age_max: 100,
            params: HashMap::new(),
        };
        match toml::to_string_pretty(&race) {
            Ok(content) => {
                if let Err(e) = fs::create_dir_all(path.parent().unwrap())
                    .and_then(|_| fs::write(&path, &content))
                {
                    return format!("Error: failed to write race: {e}");
                }
                format!("Created race '{}'", p.id)
            }
            Err(e) => format!("Error: failed to serialize race: {e}"),
        }
    }

    #[tool(description = "Delete a race template")]
    fn delete_race(&self, params: Parameters<IdParam>) -> String {
        let p = params.0;
        let (_, file_map) = self.load();
        match content::delete_file(&file_map, "races", &p.id) {
            Ok(()) => format!("Deleted race '{}'", p.id),
            Err(e) => format!("Error: {e}"),
        }
    }

    #[tool(description = "List all class templates")]
    fn list_classes(&self) -> String {
        let (registry, _) = self.load();
        Self::entity_list(
            &registry
                .classes
                .iter()
                .map(|(k, v)| (k.clone(), v.name.as_str()))
                .collect(),
            "Classes",
        )
    }

    #[tool(description = "Get class template details")]
    fn get_class(&self, params: Parameters<IdParam>) -> String {
        let p = params.0;
        let (registry, _file_map) = self.load();
        match registry.classes.get(&p.id) {
            Some(c) => format!(
                "id: {}\nname: {}\ndescription: {}\nprestige: {}\nhit_die: {}\nbab: {}\nfort_save: {}\nref_save: {}\nwill_save: {}\nallowed_races: {:?}\nallowed_alignments: {:?}\nauto_skills: {:?}\nstarting_skill_slots: {}\ndeity_policy: {:?}",
                p.id, c.name, c.description, c.prestige, c.hit_die,
                c.bab, c.fort_save, c.ref_save, c.will_save,
                c.allowed_races, c.allowed_alignments, c.auto_skills,
                c.starting_skill_slots, c.deity_policy,
            ),
            None => format!("Error: class '{}' not found", p.id),
        }
    }

    #[tool(description = "Create a new class template")]
    fn create_class(&self, params: Parameters<CreateEntityParams>) -> String {
        let p = params.0;
        if let Err(e) = self.validate_id(&p.id) {
            return format!("Error: {e}");
        }
        let name = p.name.unwrap_or_else(|| p.id.clone());
        let path = self
            .content_path
            .join("classes")
            .join(format!("{}.toml", p.id));
        if let Err(e) = self.validate_and_contain(&p.id, &path) {
            return format!("Error: {e}");
        }
        let class = ClassTemplate {
            id: p.id.clone(),
            name,
            description: String::new(),
            prestige: false,
            prestige_gate: None,
            hit_die: 8,
            attribute_mods: oxide_core::templates::ClassAttributeMods::default(),
            bab: "poor".to_string(),
            fort_save: "poor".to_string(),
            ref_save: "poor".to_string(),
            will_save: "poor".to_string(),
            allowed_races: Vec::new(),
            allowed_alignments: Vec::new(),
            auto_skills: Vec::new(),
            params: HashMap::new(),
            skill_pool: Vec::new(),
            starting_skill_slots: 3,
            starting_items: Vec::new(),
            starting_gold: oxide_core::templates::WalletAmount::default(),
            deity_policy: oxide_core::templates::DeityPolicy::Any,
        };
        match toml::to_string_pretty(&class) {
            Ok(content) => {
                if let Err(e) = fs::create_dir_all(path.parent().unwrap())
                    .and_then(|_| fs::write(&path, &content))
                {
                    return format!("Error: failed to write class: {e}");
                }
                format!("Created class '{}'", p.id)
            }
            Err(e) => format!("Error: failed to serialize class: {e}"),
        }
    }

    #[tool(description = "Delete a class template")]
    fn delete_class(&self, params: Parameters<IdParam>) -> String {
        let p = params.0;
        let (_, file_map) = self.load();
        match content::delete_file(&file_map, "classes", &p.id) {
            Ok(()) => format!("Deleted class '{}'", p.id),
            Err(e) => format!("Error: {e}"),
        }
    }

    #[tool(description = "Get template content as TOML for any type")]
    fn get_template_raw(&self, params: Parameters<UpdateFieldsParams>) -> String {
        let p = params.0;
        let (_registry, file_map) = self.load();
        let field = if p.id.is_empty() { &p.category } else { &p.id };
        let path = match content::find_file(&file_map, &p.category, field) {
            Ok(p) => p,
            Err(e) => return format!("Error: {e}"),
        };
        match fs::read_to_string(&path) {
            Ok(c) => c,
            Err(e) => format!("Error: failed to read {}: {e}", path.display()),
        }
    }

    #[tool(description = "Validate all templates for cross-reference errors")]
    fn validate(&self) -> String {
        let (registry, _file_map) = self.load();
        let errors = registry.validate();
        if errors.is_empty() {
            return "All templates valid.".to_string();
        }
        let mut out = format!("{} validation issue(s):\n", errors.len());
        for err in &errors {
            out.push_str(&format!(
                "  [{}/{}] {}: {}\n",
                err.template_type, err.template_id, err.field, err.message
            ));
        }
        out.trim().to_string()
    }

    #[tool(description = "Get content statistics summary")]
    fn get_stats(&self) -> String {
        let (registry, _file_map) = self.load();
        let r = &registry;
        let room_count: usize = r.areas.values().map(|a| a.rooms.len()).sum();
        format!(
            "Areas: {}\nRooms: {}\nItems: {}\nMobs: {}\nRaces: {}\nClasses: {}\nSkills: {}\nQuests: {}\nFactions: {}\nRecipes: {}\nShops: {}\nDeities: {}\nStances: {}\nSets: {}\nAffixes: {}\nPassives: {}",
            r.areas.len(),
            room_count,
            r.items.len(),
            r.mobs.len(),
            r.races.len(),
            r.classes.len(),
            r.skills.len(),
            r.quests.len(),
            r.factions.len(),
            r.recipes.len(),
            r.shops.len(),
            r.deities.len(),
            r.stances.len(),
            r.sets.len(),
            r.affixes.len(),
            r.passives.len(),
        )
    }

    #[tool(description = "Fuzzy search all template names and descriptions")]
    fn search(&self, params: Parameters<SearchParams>) -> String {
        let q = params.0.query.to_lowercase();
        let (registry, _file_map) = self.load();
        let r = &registry;
        let mut results: Vec<String> = Vec::new();

        for (id, area) in &r.areas {
            if area.name.to_lowercase().contains(&q) || area.description.to_lowercase().contains(&q)
            {
                results.push(format!("area:{id} - {name}", name = area.name));
            }
            for (rid, room) in &area.rooms {
                if room.name.to_lowercase().contains(&q)
                    || room.description.to_lowercase().contains(&q)
                {
                    results.push(format!("area:{id}/room:{rid} - {name}", name = room.name));
                }
            }
        }

        for (id, item) in &r.items {
            if item.name.to_lowercase().contains(&q) || item.description.to_lowercase().contains(&q)
            {
                results.push(format!("item:{id} - {name}", name = item.name));
            }
        }
        for (id, mob) in &r.mobs {
            if mob.name.to_lowercase().contains(&q) || mob.description.to_lowercase().contains(&q) {
                results.push(format!("mob:{id} - {name}", name = mob.name));
            }
        }
        for (id, race) in &r.races {
            if race.name.to_lowercase().contains(&q) || race.description.to_lowercase().contains(&q)
            {
                results.push(format!("race:{id} - {name}", name = race.name));
            }
        }
        for (id, cls) in &r.classes {
            if cls.name.to_lowercase().contains(&q) || cls.description.to_lowercase().contains(&q) {
                results.push(format!("class:{id} - {name}", name = cls.name));
            }
        }
        for (id, skill) in &r.skills {
            if skill.name.to_lowercase().contains(&q)
                || skill.description.to_lowercase().contains(&q)
            {
                results.push(format!("skill:{id} - {name}", name = skill.name));
            }
        }
        for (id, quest) in &r.quests {
            if quest.name.to_lowercase().contains(&q)
                || quest.description.to_lowercase().contains(&q)
            {
                results.push(format!("quest:{id} - {name}", name = quest.name));
            }
        }
        for (id, faction) in &r.factions {
            if faction.name.to_lowercase().contains(&q)
                || faction.description.to_lowercase().contains(&q)
            {
                results.push(format!("faction:{id} - {name}", name = faction.name));
            }
        }
        for (id, recipe) in &r.recipes {
            if recipe.name.to_lowercase().contains(&q)
                || recipe.description.to_lowercase().contains(&q)
            {
                results.push(format!("recipe:{id} - {name}", name = recipe.name));
            }
        }
        for (id, shop) in &r.shops {
            if shop.name.to_lowercase().contains(&q) {
                results.push(format!("shop:{id} - {name}", name = shop.name));
            }
        }
        for (id, deity) in &r.deities {
            if deity.name.to_lowercase().contains(&q)
                || deity.description.to_lowercase().contains(&q)
            {
                results.push(format!("deity:{id} - {name}", name = deity.name));
            }
        }
        for (id, stance) in &r.stances {
            if stance.name.to_lowercase().contains(&q) {
                results.push(format!("stance:{id} - {name}", name = stance.name));
            }
        }
        for (id, set) in &r.sets {
            if set.name.to_lowercase().contains(&q) {
                results.push(format!("set:{id} - {name}", name = set.name));
            }
        }
        for (id, affix) in &r.affixes {
            if affix.name.to_lowercase().contains(&q)
                || affix.description.to_lowercase().contains(&q)
            {
                results.push(format!("affix:{id} - {name}", name = affix.name));
            }
        }
        for (id, passive) in &r.passives {
            if passive.name.to_lowercase().contains(&q)
                || passive.description.to_lowercase().contains(&q)
            {
                results.push(format!("passive:{id} - {name}", name = passive.name));
            }
        }

        if results.is_empty() {
            return format!("No results for '{q}'.");
        }
        results.sort();
        results.insert(0, format!("{} result(s):", results.len()));
        results.join("\n")
    }

    #[tool(description = "Simulate loot drops from a mob template")]
    fn simulate_loot(&self, params: Parameters<SimulateLootParams>) -> String {
        let p = params.0;
        let (registry, _) = self.load();
        match simulator::simulate_loot(
            &registry,
            &p.mob_id,
            p.iterations,
            p.detailed.unwrap_or(false),
        ) {
            Ok(result) => result,
            Err(e) => format!("Error simulating loot: {e}"),
        }
    }

    #[tool(description = "Simulate combat rounds between two characters (based on templates)")]
    fn simulate_combat(&self, params: Parameters<SimulateCombatParams>) -> String {
        let p = params.0;
        let (registry, _) = self.load();
        match simulator::simulate_combat(&registry, &p) {
            Ok(result) => result,
            Err(e) => format!("Error simulating combat: {e}"),
        }
    }

    #[tool(description = "Simulate character progression stats level-by-level")]
    fn simulate_progression(&self, params: Parameters<SimulateProgressionParams>) -> String {
        let p = params.0;
        let (registry, _) = self.load();
        match simulator::simulate_progression(
            &registry,
            &p.race_id,
            &p.class_id,
            p.start_level,
            p.end_level,
        ) {
            Ok(result) => result,
            Err(e) => format!("Error simulating progression: {e}"),
        }
    }

    #[tool(description = "Simulate a gear loadout on a mock character and show final stats")]
    fn simulate_gear_loadout(&self, params: Parameters<SimulateGearLoadoutParams>) -> String {
        let p = params.0;
        let (registry, _) = self.load();
        match simulator::simulate_gear_loadout(
            &registry,
            &p.race_id,
            &p.class_id,
            p.level,
            &p.equipped_items,
        ) {
            Ok(result) => result,
            Err(e) => format!("Error simulating gear loadout: {e}"),
        }
    }

    #[tool(description = "Simulate AI random wander paths and room visit frequencies")]
    fn simulate_ai_wander(&self, params: Parameters<SimulateAiWanderParams>) -> String {
        let p = params.0;
        let (registry, _) = self.load();
        match simulator::simulate_ai_wander(&registry, &p.mob_id, &p.start_room_str, p.ticks) {
            Ok(result) => result,
            Err(e) => format!("Error simulating AI wander: {e}"),
        }
    }

    #[tool(
        description = "Simulate shop buying/selling transaction pricing across reputation levels"
    )]
    fn simulate_shop_transaction(
        &self,
        params: Parameters<SimulateShopTransactionParams>,
    ) -> String {
        let p = params.0;
        let (registry, _) = self.load();
        match simulator::simulate_shop_transaction(&registry, &p.shop_id, &p.item_id) {
            Ok(result) => result,
            Err(e) => format!("Error simulating shop transaction: {e}"),
        }
    }

    #[tool(description = "Validate skill prerequisites for circular dependency loops")]
    fn validate_content_dag(&self) -> String {
        match simulator::validate_content_dag(&self.content_path) {
            Ok(result) => result,
            Err(e) => format!("Error validating content DAG: {e}"),
        }
    }

    #[tool(description = "Simulate recipe crafting outcomes based on character stats")]
    async fn simulate_crafting(&self, params: Parameters<SimulateCraftingParams>) -> String {
        let p = params.0;
        let (registry, _) = self.load();

        let mut player_level = p.player_level.unwrap_or(1);
        let mut dexterity = p.dexterity.unwrap_or(10);
        let mut intelligence = p.intelligence.unwrap_or(10);
        let mut skill_rank = p.skill_rank.unwrap_or(0);
        let mut loaded_msg = String::new();

        if let Some(ref name) = p.player_name {
            match self.fetch_player_state(name).await {
                Ok(player) => {
                    player_level = player.level;
                    dexterity = player.attributes.dexterity;
                    intelligence = player.attributes.intelligence;
                    if let Some(recipe) = registry.recipes.get(&p.recipe_id) {
                        if let Some(ref req) = recipe.skill_requirement {
                            skill_rank = player.skills.get(&req.id).copied().unwrap_or(0);
                        }
                    }
                    loaded_msg = format!(
                        "*   **Simulation Actor**: Loaded from Database (`{}` - Level {} {})\n\n",
                        player.name, player.level, player.class_id
                    );
                }
                Err(e) => return format!("Error loading player from database: {e}"),
            }
        }

        match simulator::simulate_crafting(
            &registry,
            &p.recipe_id,
            player_level,
            dexterity,
            intelligence,
            skill_rank,
            p.has_station.unwrap_or(true),
        ) {
            Ok(result) => {
                if !loaded_msg.is_empty() {
                    format!("{}{}", loaded_msg, result)
                } else {
                    result
                }
            }
            Err(e) => format!("Error simulating crafting: {e}"),
        }
    }

    #[tool(description = "Simulate casting spells or using active abilities")]
    async fn simulate_skill_use(&self, params: Parameters<SimulateSkillUseParams>) -> String {
        let p = params.0;
        let (registry, _) = self.load();

        let mut actor_level = p.actor_level.unwrap_or(1);
        let mut actor_class = p.actor_class.clone();
        let mut actor_race = p.actor_race.clone();
        let mut strength = p.strength;
        let mut dexterity = p.dexterity;
        let mut intelligence = p.intelligence;
        let mut wisdom = p.wisdom;
        let mut constitution = p.constitution;
        let mut charisma = p.charisma;
        let mut skill_rank = p.skill_rank;
        let mut loaded_msg = String::new();

        if let Some(ref name) = p.actor_name {
            match self.fetch_player_state(name).await {
                Ok(player) => {
                    actor_level = player.level;
                    actor_class = Some(player.class_id.clone());
                    actor_race = Some(player.race_id);
                    strength = Some(player.attributes.strength);
                    dexterity = Some(player.attributes.dexterity);
                    intelligence = Some(player.attributes.intelligence);
                    wisdom = Some(player.attributes.wisdom);
                    constitution = Some(player.attributes.constitution);
                    charisma = Some(player.attributes.charisma);
                    skill_rank = Some(player.skills.get(&p.skill_id).copied().unwrap_or(0));
                    loaded_msg = format!(
                        "*   **Simulation Actor**: Loaded from Database (`{}` - Level {} {})\n\n",
                        player.name, player.level, player.class_id
                    );
                }
                Err(e) => return format!("Error loading actor from database: {e}"),
            }
        }

        match simulator::simulate_skill_use(
            &registry,
            &crate::simulator::SimulateSkillUseParams {
                skill_id: p.skill_id,
                actor_name: None,
                actor_level: Some(actor_level),
                actor_class,
                actor_race,
                strength,
                dexterity,
                intelligence,
                wisdom,
                constitution,
                charisma,
                skill_rank,
                target_level: p.target_level,
            },
        ) {
            Ok(result) => {
                if !loaded_msg.is_empty() {
                    format!("{}{}", loaded_msg, result)
                } else {
                    result
                }
            }
            Err(e) => format!("Error simulating skill use: {e}"),
        }
    }

    #[tool(description = "Simulate adoption constraints and prayer buff effects for a deity")]
    async fn simulate_prayer(&self, params: Parameters<SimulatePrayerParams>) -> String {
        let p = params.0;
        let (registry, _) = self.load();

        let mut player_race = p.player_race.unwrap_or_else(|| "human".to_string());
        let mut player_class = p.player_class.unwrap_or_else(|| "cleric".to_string());
        let mut player_alignment = p.player_alignment.unwrap_or_else(|| "Neutral".to_string());
        let mut cleric_level = p.cleric_level;
        let mut wisdom = p.wisdom.unwrap_or(10);
        let mut loaded_msg = String::new();

        if let Some(ref name) = p.player_name {
            match self.fetch_player_state(name).await {
                Ok(player) => {
                    player_race = player.race_id;
                    player_class = player.class_id.clone();
                    player_alignment = player.alignment;
                    if player_class.to_lowercase() == "cleric"
                        || player_class.to_lowercase() == "paladin"
                    {
                        cleric_level = Some(player.level);
                    }
                    wisdom = player.attributes.wisdom;
                    loaded_msg = format!(
                        "*   **Simulation Actor**: Loaded from Database (`{}` - Level {} {})\n\n",
                        player.name, player.level, player.class_id
                    );
                }
                Err(e) => return format!("Error loading player from database: {e}"),
            }
        }

        match simulator::simulate_prayer(
            &registry,
            &p.deity_id,
            &player_race,
            &player_class,
            &player_alignment,
            cleric_level,
            wisdom,
        ) {
            Ok(result) => {
                if !loaded_msg.is_empty() {
                    format!("{}{}", loaded_msg, result)
                } else {
                    result
                }
            }
            Err(e) => format!("Error simulating prayer: {e}"),
        }
    }

    #[tool(description = "Check if a character satisfies requirements for a prestige class")]
    async fn simulate_prestige_eligibility(
        &self,
        params: Parameters<SimulatePrestigeParams>,
    ) -> String {
        let p = params.0;
        let (registry, _) = self.load();

        let mut base_classes = p.base_classes.unwrap_or_default();
        let mut skill_ranks = p.skill_ranks.unwrap_or_default();
        let mut completed_quests = p.completed_quests.unwrap_or_default();
        let mut faction_standings = p.faction_standings.unwrap_or_default();
        let mut loaded_msg = String::new();

        if let Some(ref name) = p.player_name {
            match self.fetch_player_state(name).await {
                Ok(player) => {
                    base_classes.clear();
                    base_classes.insert(player.class_id.clone(), player.level);
                    skill_ranks = player.skills;
                    completed_quests = player.completed_quests;
                    faction_standings = player.faction_standings;
                    loaded_msg = format!(
                        "*   **Simulation Actor**: Loaded from Database (`{}` - Level {} {})\n\n",
                        player.name, player.level, player.class_id
                    );
                }
                Err(e) => return format!("Error loading player from database: {e}"),
            }
        }

        match simulator::simulate_prestige_eligibility(
            &registry,
            &p.prestige_class_id,
            &base_classes,
            &skill_ranks,
            &completed_quests,
            &faction_standings,
        ) {
            Ok(result) => {
                if !loaded_msg.is_empty() {
                    format!("{}{}", loaded_msg, result)
                } else {
                    result
                }
            }
            Err(e) => format!("Error simulating prestige eligibility: {e}"),
        }
    }

    #[tool(
        description = "Evaluate stat and AC modifiers applied to a group based on party layout and formation"
    )]
    fn simulate_group_formation(&self, params: Parameters<SimulateGroupParams>) -> String {
        let p = params.0;

        let members: Vec<simulator::MockMember> = p
            .members
            .into_iter()
            .map(|m| simulator::MockMember {
                class_id: m.class_id,
                has_shield: m.has_shield,
                is_front_row: m.is_front_row,
            })
            .collect();

        match simulator::simulate_group_formation(&p.formation, &members) {
            Ok(result) => result,
            Err(e) => format!("Error simulating group formation: {e}"),
        }
    }

    #[tool(
        description = "Calculate XP loss penalties, corpse decay, and ghost parameters when a player dies"
    )]
    async fn simulate_death_penalty(&self, params: Parameters<SimulateDeathParams>) -> String {
        let p = params.0;
        let mut current_level = p.current_level.unwrap_or(1);
        let mut current_xp = p.current_xp.unwrap_or(0);
        let mut loaded_msg = String::new();

        if let Some(ref name) = p.player_name {
            match self.fetch_player_state(name).await {
                Ok(player) => {
                    current_level = player.level;
                    current_xp = player.experience;
                    loaded_msg = format!(
                        "*   **Simulation Actor**: Loaded from Database (`{}` - Level {} {})\n\n",
                        player.name, player.level, player.class_id
                    );
                }
                Err(e) => return format!("Error loading player from database: {e}"),
            }
        }

        match simulator::simulate_death_penalty(current_level, current_xp, p.allow_revive_room) {
            Ok(result) => {
                if !loaded_msg.is_empty() {
                    format!("{}{}", loaded_msg, result)
                } else {
                    result
                }
            }
            Err(e) => format!("Error simulating death penalty: {e}"),
        }
    }

    fn update_room_fields(
        &self,
        area_id: &str,
        room_id: &str,
        fields: &serde_json::Map<String, serde_json::Value>,
    ) -> String {
        let (_registry, file_map) = self.load();
        let room_key = format!("{area_id}:{room_id}");
        let room_path = match file_map.get("rooms").and_then(|m| m.get(&room_key)) {
            Some(p) => p.clone(),
            None => return format!("Error: room '{}' not found in area '{}'", room_id, area_id),
        };

        let content = match fs::read_to_string(&room_path) {
            Ok(c) => c,
            Err(e) => return format!("Error: failed to read {}: {e}", room_path.display()),
        };
        let mut room: RoomTemplate = match toml::from_str(&content) {
            Ok(r) => r,
            Err(e) => return format!("Error: failed to parse room: {e}"),
        };
        // Round-trip through JSON to apply field patches
        let mut room_json = match serde_json::to_value(&room) {
            Ok(v) => v,
            Err(e) => return format!("Error: failed to serialize room: {e}"),
        };
        if let Some(obj) = room_json.as_object_mut() {
            for (key, value) in fields {
                obj.insert(key.clone(), value.clone());
            }
        }
        room = match serde_json::from_value(room_json) {
            Ok(r) => r,
            Err(e) => return format!("Error: failed to deserialize room after patch: {e}"),
        };
        match toml::to_string_pretty(&room) {
            Ok(out) => {
                if let Err(e) = fs::write(&room_path, &out) {
                    return format!("Error: failed to write {}: {e}", room_path.display());
                }
            }
            Err(e) => return format!("Error: failed to serialize room: {e}"),
        }
        format!(
            "Updated {} field(s) on room {}/{}",
            fields.len(),
            area_id,
            room_id
        )
    }

    #[tool(
        description = "Simulate character creation (online if MUD server is connected, otherwise offline fallback)"
    )]
    async fn simulate_character_creation(
        &self,
        params: Parameters<SimulateCharacterCreationParams>,
    ) -> String {
        let p = params.0;

        // 1. Try Online Mode if API is configured
        if let (Some(url), Some(key)) = (&self.api_url, &self.api_key) {
            let client = reqwest::Client::new();
            let req_url = format!("{}/api/character/simulate", url.trim_end_matches('/'));

            let payload = serde_json::json!({
                "race_id": p.race_id,
                "class_id": p.class_id,
                "base_attributes": {
                    "strength": p.strength,
                    "dexterity": p.dexterity,
                    "intelligence": p.intelligence,
                    "wisdom": p.wisdom,
                    "constitution": p.constitution,
                    "charisma": p.charisma
                },
                "selected_skills": p.selected_skills
            });

            match client
                .post(&req_url)
                .header("Authorization", format!("Bearer {}", key))
                .json(&payload)
                .send()
                .await
            {
                Ok(resp) => {
                    let status = resp.status();
                    if status.is_success() {
                        match resp.json::<serde_json::Value>().await {
                            Ok(sim_res) => {
                                return format_online_simulation_report(
                                    &p.race_id,
                                    &p.class_id,
                                    &sim_res,
                                )
                            }
                            Err(e) => {
                                return format!("Failed to parse MUD Server response as JSON: {e}")
                            }
                        }
                    } else {
                        match resp.text().await {
                            Ok(err_text) => {
                                return format!("MUD Server validation error: {err_text}")
                            }
                            Err(_) => {
                                return format!("MUD Server returned error status: {}", status)
                            }
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Warning: Failed to connect to MUD server online simulation: {e}. Falling back to offline simulation.");
                }
            }
        }

        // 2. Offline Fallback Mode
        let (registry, _) = self.load();
        match simulator::simulate_character_creation(
            &registry,
            &crate::simulator::SimulateCharacterCreationParams {
                race_id: p.race_id,
                class_id: p.class_id,
                strength: p.strength,
                dexterity: p.dexterity,
                intelligence: p.intelligence,
                wisdom: p.wisdom,
                constitution: p.constitution,
                charisma: p.charisma,
                selected_skills: p.selected_skills,
            },
        ) {
            Ok(result) => result,
            Err(e) => format!("Error simulating character creation: {e}"),
        }
    }

    #[tool(description = "List all currently connected players in the MUD (Online Only)")]
    async fn list_connected_players(&self) -> String {
        let (url, key) = match (&self.api_url, &self.api_key) {
            (Some(u), Some(k)) => (u, k),
            _ => return "Error: This tool is only available in online mode. Please configure --url and --key to connect to the MUD server.".to_string(),
        };

        let client = reqwest::Client::new();
        let req_url = format!("{}/api/players", url.trim_end_matches('/'));
        match client
            .get(&req_url)
            .header("Authorization", format!("Bearer {}", key))
            .send()
            .await
        {
            Ok(resp) => {
                if resp.status().is_success() {
                    if let Ok(players) = resp.json::<Vec<serde_json::Value>>().await {
                        if players.is_empty() {
                            return "No players currently online.".to_string();
                        }
                        let mut out = "### Connected Players:\n\n".to_string();
                        out.push_str("| Name | Level | Class | Race | Room Key |\n");
                        out.push_str("|---|---|---|---|---|\n");
                        for p in players {
                            out.push_str(&format!(
                                "| {} | {} | {} | {} | {} |\n",
                                p.get("name").and_then(|v| v.as_str()).unwrap_or("Unknown"),
                                p.get("level").and_then(|v| v.as_i64()).unwrap_or(1),
                                p.get("class").and_then(|v| v.as_str()).unwrap_or("None"),
                                p.get("race").and_then(|v| v.as_str()).unwrap_or("None"),
                                p.get("room_key")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("Unknown")
                            ));
                        }
                        out
                    } else {
                        "Error parsing players list from server.".to_string()
                    }
                } else {
                    format!("Server returned error status: {}", resp.status())
                }
            }
            Err(e) => format!("Failed to connect to MUD server: {e}"),
        }
    }

    #[tool(description = "Put an item from a template into a player's inventory (Online Only)")]
    async fn imm_put_item(&self, params: Parameters<PutItemParams>) -> String {
        let p = params.0;
        let (url, key) = match (&self.api_url, &self.api_key) {
            (Some(u), Some(k)) => (u, k),
            _ => return "Error: This tool is only available in online mode. Please configure --url and --key to connect to the MUD server.".to_string(),
        };

        let client = reqwest::Client::new();
        let req_url = format!("{}/api/imm/put_item", url.trim_end_matches('/'));
        let payload = serde_json::json!({
            "player_name": p.player_name,
            "item_template_id": p.item_template_id,
            "count": p.count
        });

        match client
            .post(&req_url)
            .header("Authorization", format!("Bearer {}", key))
            .json(&payload)
            .send()
            .await
        {
            Ok(resp) => {
                let status = resp.status();
                if status.is_success() {
                    match resp.json::<serde_json::Value>().await {
                        Ok(res) => res
                            .get("message")
                            .and_then(|v| v.as_str())
                            .unwrap_or("Success")
                            .to_string(),
                        Err(e) => format!("Failed to parse MUD Server response as JSON: {e}"),
                    }
                } else {
                    match resp.text().await {
                        Ok(err_text) => format!("Error from server: {err_text}"),
                        Err(_) => format!("Server returned error status: {}", status),
                    }
                }
            }
            Err(e) => format!("Failed to connect to MUD server: {e}"),
        }
    }

    #[tool(description = "Teleport a player to a specific room by its key (Online Only)")]
    async fn imm_teleport(&self, params: Parameters<TeleportParams>) -> String {
        let p = params.0;
        let (url, key) = match (&self.api_url, &self.api_key) {
            (Some(u), Some(k)) => (u, k),
            _ => return "Error: This tool is only available in online mode. Please configure --url and --key to connect to the MUD server.".to_string(),
        };

        let client = reqwest::Client::new();
        let req_url = format!("{}/api/imm/teleport", url.trim_end_matches('/'));
        let payload = serde_json::json!({
            "player_name": p.player_name,
            "room_key": p.room_key
        });

        match client
            .post(&req_url)
            .header("Authorization", format!("Bearer {}", key))
            .json(&payload)
            .send()
            .await
        {
            Ok(resp) => {
                let status = resp.status();
                if status.is_success() {
                    match resp.json::<serde_json::Value>().await {
                        Ok(res) => res
                            .get("message")
                            .and_then(|v| v.as_str())
                            .unwrap_or("Success")
                            .to_string(),
                        Err(e) => format!("Failed to parse MUD Server response as JSON: {e}"),
                    }
                } else {
                    match resp.text().await {
                        Ok(err_text) => format!("Error from server: {err_text}"),
                        Err(_) => format!("Server returned error status: {}", status),
                    }
                }
            }
            Err(e) => format!("Failed to connect to MUD server: {e}"),
        }
    }

    #[tool(description = "Force a player to execute a command as if they typed it (Online Only)")]
    async fn imm_force_command(&self, params: Parameters<ForceCommandParams>) -> String {
        let p = params.0;

        if !p.confirm {
            return "Error: This is a destructive operation. Set `confirm` to true to proceed."
                .to_string();
        }

        let (url, key) = match (&self.api_url, &self.api_key) {
            (Some(u), Some(k)) => (u, k),
            _ => return "Error: This tool is only available in online mode. Please configure --url and --key to connect to the MUD server.".to_string(),
        };

        let client = reqwest::Client::new();
        let req_url = format!("{}/api/imm/force_command", url.trim_end_matches('/'));
        let payload = serde_json::json!({
            "player_name": p.player_name,
            "command": p.command
        });

        match client
            .post(&req_url)
            .header("Authorization", format!("Bearer {}", key))
            .json(&payload)
            .send()
            .await
        {
            Ok(resp) => {
                let status = resp.status();
                if status.is_success() {
                    match resp.json::<serde_json::Value>().await {
                        Ok(res) => res
                            .get("message")
                            .and_then(|v| v.as_str())
                            .unwrap_or("Success")
                            .to_string(),
                        Err(e) => format!("Failed to parse MUD Server response as JSON: {e}"),
                    }
                } else {
                    match resp.text().await {
                        Ok(err_text) => format!("Error from server: {err_text}"),
                        Err(_) => format!("Server returned error status: {}", status),
                    }
                }
            }
            Err(e) => format!("Failed to connect to MUD server: {e}"),
        }
    }

    #[tool(description = "Set character attributes, pools, level, or XP (Online Only)")]
    async fn imm_set_stat(&self, params: Parameters<SetStatParams>) -> String {
        let p = params.0;
        let (url, key) = match (&self.api_url, &self.api_key) {
            (Some(u), Some(k)) => (u, k),
            _ => return "Error: This tool is only available in online mode. Please configure --url and --key to connect to the MUD server.".to_string(),
        };

        let client = reqwest::Client::new();
        let req_url = format!("{}/api/imm/set_stat", url.trim_end_matches('/'));
        let payload = serde_json::json!({
            "player_name": p.player_name,
            "strength": p.strength,
            "dexterity": p.dexterity,
            "intelligence": p.intelligence,
            "wisdom": p.wisdom,
            "constitution": p.constitution,
            "charisma": p.charisma,
            "hp": p.hp,
            "mana": p.mana,
            "stamina": p.stamina,
            "level": p.level,
            "xp": p.xp
        });

        match client
            .post(&req_url)
            .header("Authorization", format!("Bearer {}", key))
            .json(&payload)
            .send()
            .await
        {
            Ok(resp) => {
                let status = resp.status();
                if status.is_success() {
                    match resp.json::<serde_json::Value>().await {
                        Ok(res) => res
                            .get("message")
                            .and_then(|v| v.as_str())
                            .unwrap_or("Success")
                            .to_string(),
                        Err(e) => format!("Failed to parse response: {e}"),
                    }
                } else {
                    resp.text()
                        .await
                        .unwrap_or_else(|_| format!("Error status: {status}"))
                }
            }
            Err(e) => format!("Failed to connect to MUD server: {e}"),
        }
    }

    #[tool(description = "Spawn an NPC from a template into a specific room (Online Only)")]
    async fn imm_load_mob(&self, params: Parameters<LoadMobParams>) -> String {
        let p = params.0;
        let (url, key) = match (&self.api_url, &self.api_key) {
            (Some(u), Some(k)) => (u, k),
            _ => return "Error: This tool is only available in online mode. Please configure --url and --key to connect to the MUD server.".to_string(),
        };

        let client = reqwest::Client::new();
        let req_url = format!("{}/api/imm/load_mob", url.trim_end_matches('/'));
        let payload = serde_json::json!({
            "room_key": p.room_key,
            "mob_template_id": p.mob_template_id
        });

        match client
            .post(&req_url)
            .header("Authorization", format!("Bearer {}", key))
            .json(&payload)
            .send()
            .await
        {
            Ok(resp) => {
                let status = resp.status();
                if status.is_success() {
                    match resp.json::<serde_json::Value>().await {
                        Ok(res) => res
                            .get("message")
                            .and_then(|v| v.as_str())
                            .unwrap_or("Success")
                            .to_string(),
                        Err(e) => format!("Failed to parse response: {e}"),
                    }
                } else {
                    resp.text()
                        .await
                        .unwrap_or_else(|_| format!("Error status: {status}"))
                }
            }
            Err(e) => format!("Failed to connect to MUD server: {e}"),
        }
    }

    #[tool(description = "Spawn an item from a template into a specific room (Online Only)")]
    async fn imm_load_item(&self, params: Parameters<LoadItemParams>) -> String {
        let p = params.0;
        let (url, key) = match (&self.api_url, &self.api_key) {
            (Some(u), Some(k)) => (u, k),
            _ => return "Error: This tool is only available in online mode. Please configure --url and --key to connect to the MUD server.".to_string(),
        };

        let client = reqwest::Client::new();
        let req_url = format!("{}/api/imm/load_item", url.trim_end_matches('/'));
        let payload = serde_json::json!({
            "room_key": p.room_key,
            "item_template_id": p.item_template_id,
            "count": p.count
        });

        match client
            .post(&req_url)
            .header("Authorization", format!("Bearer {}", key))
            .json(&payload)
            .send()
            .await
        {
            Ok(resp) => {
                let status = resp.status();
                if status.is_success() {
                    match resp.json::<serde_json::Value>().await {
                        Ok(res) => res
                            .get("message")
                            .and_then(|v| v.as_str())
                            .unwrap_or("Success")
                            .to_string(),
                        Err(e) => format!("Failed to parse response: {e}"),
                    }
                } else {
                    resp.text()
                        .await
                        .unwrap_or_else(|_| format!("Error status: {status}"))
                }
            }
            Err(e) => format!("Failed to connect to MUD server: {e}"),
        }
    }

    #[tool(description = "Broadcast a global echo message to all players (Online Only)")]
    async fn imm_gecho(&self, params: Parameters<GechoParams>) -> String {
        let p = params.0;
        let (url, key) = match (&self.api_url, &self.api_key) {
            (Some(u), Some(k)) => (u, k),
            _ => return "Error: This tool is only available in online mode. Please configure --url and --key to connect to the MUD server.".to_string(),
        };

        let client = reqwest::Client::new();
        let req_url = format!("{}/api/imm/gecho", url.trim_end_matches('/'));
        let payload = serde_json::json!({
            "message": p.message
        });

        match client
            .post(&req_url)
            .header("Authorization", format!("Bearer {}", key))
            .json(&payload)
            .send()
            .await
        {
            Ok(resp) => {
                let status = resp.status();
                if status.is_success() {
                    match resp.json::<serde_json::Value>().await {
                        Ok(res) => res
                            .get("message")
                            .and_then(|v| v.as_str())
                            .unwrap_or("Success")
                            .to_string(),
                        Err(e) => format!("Failed to parse response: {e}"),
                    }
                } else {
                    resp.text()
                        .await
                        .unwrap_or_else(|_| format!("Error status: {status}"))
                }
            }
            Err(e) => format!("Failed to connect to MUD server: {e}"),
        }
    }

    #[tool(description = "Advance a player to a specific level (Online Only)")]
    async fn imm_advance(&self, params: Parameters<AdvanceParams>) -> String {
        let p = params.0;
        let (url, key) = match (&self.api_url, &self.api_key) {
            (Some(u), Some(k)) => (u, k),
            _ => return "Error: This tool is only available in online mode. Please configure --url and --key to connect to the MUD server.".to_string(),
        };

        let client = reqwest::Client::new();
        let req_url = format!("{}/api/imm/advance", url.trim_end_matches('/'));
        let payload = serde_json::json!({
            "player_name": p.player_name,
            "target_level": p.target_level
        });

        match client
            .post(&req_url)
            .header("Authorization", format!("Bearer {}", key))
            .json(&payload)
            .send()
            .await
        {
            Ok(resp) => {
                let status = resp.status();
                if status.is_success() {
                    match resp.json::<serde_json::Value>().await {
                        Ok(res) => res
                            .get("message")
                            .and_then(|v| v.as_str())
                            .unwrap_or("Success")
                            .to_string(),
                        Err(e) => format!("Failed to parse response: {e}"),
                    }
                } else {
                    resp.text()
                        .await
                        .unwrap_or_else(|_| format!("Error status: {status}"))
                }
            }
            Err(e) => format!("Failed to connect to MUD server: {e}"),
        }
    }

    #[tool(
        description = "Inspect ECS stats and components of a target character or NPC (Online Only)"
    )]
    async fn imm_stat(&self, params: Parameters<StatParams>) -> String {
        let p = params.0;
        let (url, key) = match (&self.api_url, &self.api_key) {
            (Some(u), Some(k)) => (u, k),
            _ => return "Error: This tool is only available in online mode. Please configure --url and --key to connect to the MUD server.".to_string(),
        };

        let client = reqwest::Client::new();
        let req_url = format!("{}/api/imm/stat", url.trim_end_matches('/'));
        let payload = serde_json::json!({
            "target_name": p.target_name
        });

        match client
            .post(&req_url)
            .header("Authorization", format!("Bearer {}", key))
            .json(&payload)
            .send()
            .await
        {
            Ok(resp) => {
                let status = resp.status();
                if status.is_success() {
                    match resp.text().await {
                        Ok(t) => t,
                        Err(e) => format!("Failed to read response: {e}"),
                    }
                } else {
                    resp.text()
                        .await
                        .unwrap_or_else(|_| format!("Error status: {status}"))
                }
            }
            Err(e) => format!("Failed to connect to MUD server: {e}"),
        }
    }

    #[tool(description = "Fully heal a target's HP, mana, and stamina (Online Only)")]
    async fn imm_heal(&self, params: Parameters<HealParams>) -> String {
        let p = params.0;
        let (url, key) = match (&self.api_url, &self.api_key) {
            (Some(u), Some(k)) => (u, k),
            _ => return "Error: This tool is only available in online mode. Please configure --url and --key to connect to the MUD server.".to_string(),
        };

        let client = reqwest::Client::new();
        let req_url = format!("{}/api/imm/heal", url.trim_end_matches('/'));
        let payload = serde_json::json!({
            "target_name": p.target_name
        });

        match client
            .post(&req_url)
            .header("Authorization", format!("Bearer {}", key))
            .json(&payload)
            .send()
            .await
        {
            Ok(resp) => {
                let status = resp.status();
                if status.is_success() {
                    match resp.json::<serde_json::Value>().await {
                        Ok(res) => res
                            .get("message")
                            .and_then(|v| v.as_str())
                            .unwrap_or("Success")
                            .to_string(),
                        Err(e) => format!("Failed to parse response: {e}"),
                    }
                } else {
                    resp.text()
                        .await
                        .unwrap_or_else(|_| format!("Error status: {status}"))
                }
            }
            Err(e) => format!("Failed to connect to MUD server: {e}"),
        }
    }

    #[tool(description = "Deal direct damage to a target entity (Online Only)")]
    async fn imm_damage(&self, params: Parameters<DamageParams>) -> String {
        let p = params.0;
        let (url, key) = match (&self.api_url, &self.api_key) {
            (Some(u), Some(k)) => (u, k),
            _ => return "Error: This tool is only available in online mode. Please configure --url and --key to connect to the MUD server.".to_string(),
        };

        let client = reqwest::Client::new();
        let req_url = format!("{}/api/imm/damage", url.trim_end_matches('/'));
        let payload = serde_json::json!({
            "target_name": p.target_name,
            "amount": p.amount
        });

        match client
            .post(&req_url)
            .header("Authorization", format!("Bearer {}", key))
            .json(&payload)
            .send()
            .await
        {
            Ok(resp) => {
                let status = resp.status();
                if status.is_success() {
                    match resp.json::<serde_json::Value>().await {
                        Ok(res) => res
                            .get("message")
                            .and_then(|v| v.as_str())
                            .unwrap_or("Success")
                            .to_string(),
                        Err(e) => format!("Failed to parse response: {e}"),
                    }
                } else {
                    resp.text()
                        .await
                        .unwrap_or_else(|_| format!("Error status: {status}"))
                }
            }
            Err(e) => format!("Failed to connect to MUD server: {e}"),
        }
    }

    #[tool(description = "Instantly kill a target entity (Online Only)")]
    async fn imm_kill(&self, params: Parameters<KillParams>) -> String {
        let p = params.0;
        if !p.confirm {
            return "Error: This is a destructive operation. Set `confirm` to true to proceed."
                .to_string();
        }

        let (url, key) = match (&self.api_url, &self.api_key) {
            (Some(u), Some(k)) => (u, k),
            _ => return "Error: This tool is only available in online mode. Please configure --url and --key to connect to the MUD server.".to_string(),
        };

        let client = reqwest::Client::new();
        let req_url = format!("{}/api/imm/kill", url.trim_end_matches('/'));
        let payload = serde_json::json!({
            "target_name": p.target_name,
            "confirm": p.confirm
        });

        match client
            .post(&req_url)
            .header("Authorization", format!("Bearer {}", key))
            .json(&payload)
            .send()
            .await
        {
            Ok(resp) => {
                let status = resp.status();
                if status.is_success() {
                    match resp.json::<serde_json::Value>().await {
                        Ok(res) => res
                            .get("message")
                            .and_then(|v| v.as_str())
                            .unwrap_or("Success")
                            .to_string(),
                        Err(e) => format!("Failed to parse response: {e}"),
                    }
                } else {
                    resp.text()
                        .await
                        .unwrap_or_else(|_| format!("Error status: {status}"))
                }
            }
            Err(e) => format!("Failed to connect to MUD server: {e}"),
        }
    }

    #[tool(description = "Revive a dead or ghost target entity (Online Only)")]
    async fn imm_revive(&self, params: Parameters<ReviveParams>) -> String {
        let p = params.0;
        let (url, key) = match (&self.api_url, &self.api_key) {
            (Some(u), Some(k)) => (u, k),
            _ => return "Error: This tool is only available in online mode. Please configure --url and --key to connect to the MUD server.".to_string(),
        };

        let client = reqwest::Client::new();
        let req_url = format!("{}/api/imm/revive", url.trim_end_matches('/'));
        let payload = serde_json::json!({
            "target_name": p.target_name
        });

        match client
            .post(&req_url)
            .header("Authorization", format!("Bearer {}", key))
            .json(&payload)
            .send()
            .await
        {
            Ok(resp) => {
                let status = resp.status();
                if status.is_success() {
                    match resp.json::<serde_json::Value>().await {
                        Ok(res) => res
                            .get("message")
                            .and_then(|v| v.as_str())
                            .unwrap_or("Success")
                            .to_string(),
                        Err(e) => format!("Failed to parse response: {e}"),
                    }
                } else {
                    resp.text()
                        .await
                        .unwrap_or_else(|_| format!("Error status: {status}"))
                }
            }
            Err(e) => format!("Failed to connect to MUD server: {e}"),
        }
    }

    #[tool(description = "Set character alignment (Online Only)")]
    async fn imm_set_alignment(&self, params: Parameters<SetAlignmentParams>) -> String {
        let p = params.0;
        let (url, key) = match (&self.api_url, &self.api_key) {
            (Some(u), Some(k)) => (u, k),
            _ => return "Error: This tool is only available in online mode. Please configure --url and --key to connect to the MUD server.".to_string(),
        };

        let client = reqwest::Client::new();
        let req_url = format!("{}/api/imm/set_alignment", url.trim_end_matches('/'));
        let payload = serde_json::json!({
            "player_name": p.player_name,
            "alignment": p.alignment
        });

        match client
            .post(&req_url)
            .header("Authorization", format!("Bearer {}", key))
            .json(&payload)
            .send()
            .await
        {
            Ok(resp) => {
                let status = resp.status();
                if status.is_success() {
                    match resp.json::<serde_json::Value>().await {
                        Ok(res) => res
                            .get("message")
                            .and_then(|v| v.as_str())
                            .unwrap_or("Success")
                            .to_string(),
                        Err(e) => format!("Failed to parse response: {e}"),
                    }
                } else {
                    resp.text()
                        .await
                        .unwrap_or_else(|_| format!("Error status: {status}"))
                }
            }
            Err(e) => format!("Failed to connect to MUD server: {e}"),
        }
    }

    #[tool(description = "Adjust character faction standing (Online Only)")]
    async fn imm_set_faction(&self, params: Parameters<SetFactionParams>) -> String {
        let p = params.0;
        let (url, key) = match (&self.api_url, &self.api_key) {
            (Some(u), Some(k)) => (u, k),
            _ => return "Error: This tool is only available in online mode. Please configure --url and --key to connect to the MUD server.".to_string(),
        };

        let client = reqwest::Client::new();
        let req_url = format!("{}/api/imm/set_faction", url.trim_end_matches('/'));
        let payload = serde_json::json!({
            "player_name": p.player_name,
            "faction_id": p.faction_id,
            "standing": p.standing
        });

        match client
            .post(&req_url)
            .header("Authorization", format!("Bearer {}", key))
            .json(&payload)
            .send()
            .await
        {
            Ok(resp) => {
                let status = resp.status();
                if status.is_success() {
                    match resp.json::<serde_json::Value>().await {
                        Ok(res) => res
                            .get("message")
                            .and_then(|v| v.as_str())
                            .unwrap_or("Success")
                            .to_string(),
                        Err(e) => format!("Failed to parse response: {e}"),
                    }
                } else {
                    resp.text()
                        .await
                        .unwrap_or_else(|_| format!("Error status: {status}"))
                }
            }
            Err(e) => format!("Failed to connect to MUD server: {e}"),
        }
    }

    #[tool(description = "Purge all NPCs and items from a room (Online Only)")]
    async fn imm_purge_room(&self, params: Parameters<PurgeRoomParams>) -> String {
        let p = params.0;
        if !p.confirm {
            return "Error: This is a destructive operation. Set `confirm` to true to proceed."
                .to_string();
        }

        let (url, key) = match (&self.api_url, &self.api_key) {
            (Some(u), Some(k)) => (u, k),
            _ => return "Error: This tool is only available in online mode. Please configure --url and --key to connect to the MUD server.".to_string(),
        };

        let client = reqwest::Client::new();
        let req_url = format!("{}/api/imm/purge_room", url.trim_end_matches('/'));
        let payload = serde_json::json!({
            "room_key": p.room_key,
            "confirm": p.confirm
        });

        match client
            .post(&req_url)
            .header("Authorization", format!("Bearer {}", key))
            .json(&payload)
            .send()
            .await
        {
            Ok(resp) => {
                let status = resp.status();
                if status.is_success() {
                    match resp.json::<serde_json::Value>().await {
                        Ok(res) => res
                            .get("message")
                            .and_then(|v| v.as_str())
                            .unwrap_or("Success")
                            .to_string(),
                        Err(e) => format!("Failed to parse response: {e}"),
                    }
                } else {
                    resp.text()
                        .await
                        .unwrap_or_else(|_| format!("Error status: {status}"))
                }
            }
            Err(e) => format!("Failed to connect to MUD server: {e}"),
        }
    }

    #[tool(description = "Initiate a graceful server reboot (Online Only)")]
    async fn imm_reboot(&self, params: Parameters<RebootParams>) -> String {
        let p = params.0;
        if !p.confirm {
            return "Error: This is a destructive operation. Set `confirm` to true to proceed."
                .to_string();
        }

        let (url, key) = match (&self.api_url, &self.api_key) {
            (Some(u), Some(k)) => (u, k),
            _ => return "Error: This tool is only available in online mode. Please configure --url and --key to connect to the MUD server.".to_string(),
        };

        let client = reqwest::Client::new();
        let req_url = format!("{}/api/imm/reboot", url.trim_end_matches('/'));
        let payload = serde_json::json!({
            "confirm": p.confirm,
            "delay_secs": p.delay_secs
        });

        match client
            .post(&req_url)
            .header("Authorization", format!("Bearer {}", key))
            .json(&payload)
            .send()
            .await
        {
            Ok(resp) => {
                let status = resp.status();
                if status.is_success() {
                    match resp.json::<serde_json::Value>().await {
                        Ok(res) => res
                            .get("message")
                            .and_then(|v| v.as_str())
                            .unwrap_or("Success")
                            .to_string(),
                        Err(e) => format!("Failed to parse response: {e}"),
                    }
                } else {
                    resp.text()
                        .await
                        .unwrap_or_else(|_| format!("Error status: {status}"))
                }
            }
            Err(e) => format!("Failed to connect to MUD server: {e}"),
        }
    }
}

#[allow(dead_code)]
fn format_online_simulation_report(
    race_id: &str,
    class_id: &str,
    sim: &serde_json::Value,
) -> String {
    let mut out = format!(
        "### Character Creation Simulation (Online Mode): Race = `{}`, Class = `{}`\n\n",
        race_id, class_id
    );

    if let Some(attrs) = sim.get("attributes") {
        out.push_str("#### Final Attributes:\n");
        out.push_str(&format!(
            "*   Str: {}\n",
            attrs.get("strength").and_then(|v| v.as_i64()).unwrap_or(10)
        ));
        out.push_str(&format!(
            "*   Dex: {}\n",
            attrs
                .get("dexterity")
                .and_then(|v| v.as_i64())
                .unwrap_or(10)
        ));
        out.push_str(&format!(
            "*   Int: {}\n",
            attrs
                .get("intelligence")
                .and_then(|v| v.as_i64())
                .unwrap_or(10)
        ));
        out.push_str(&format!(
            "*   Wis: {}\n",
            attrs.get("wisdom").and_then(|v| v.as_i64()).unwrap_or(10)
        ));
        out.push_str(&format!(
            "*   Con: {}\n",
            attrs
                .get("constitution")
                .and_then(|v| v.as_i64())
                .unwrap_or(10)
        ));
        out.push_str(&format!(
            "*   Cha: {}\n\n",
            attrs.get("charisma").and_then(|v| v.as_i64()).unwrap_or(10)
        ));
    }

    out.push_str("#### Derived Resources:\n");
    out.push_str(&format!(
        "*   **Hit Points (HP)**: {}\n",
        sim.get("hp").and_then(|v| v.as_i64()).unwrap_or(1)
    ));
    out.push_str(&format!(
        "*   **Mana**: {}\n",
        sim.get("mana").and_then(|v| v.as_i64()).unwrap_or(0)
    ));
    out.push_str(&format!(
        "*   **Stamina**: {}\n\n",
        sim.get("stamina").and_then(|v| v.as_i64()).unwrap_or(0)
    ));

    if let Some(gold) = sim.get("starting_gold") {
        out.push_str("#### Starting Gold:\n");
        out.push_str(&format!(
            "*   Copper: {}, Silver: {}, Gold: {}, Platinum: {}\n\n",
            gold.get("copper").and_then(|v| v.as_i64()).unwrap_or(0),
            gold.get("silver").and_then(|v| v.as_i64()).unwrap_or(0),
            gold.get("gold").and_then(|v| v.as_i64()).unwrap_or(0),
            gold.get("platinum").and_then(|v| v.as_i64()).unwrap_or(0)
        ));
    }

    if let Some(skills) = sim.get("auto_skills").and_then(|v| v.as_array()) {
        out.push_str("#### Auto-Granted Skills:\n");
        if skills.is_empty() {
            out.push_str("*   *(None)*\n");
        } else {
            for s in skills {
                if let Some(s_str) = s.as_str() {
                    out.push_str(&format!("*   `{}`\n", s_str));
                }
            }
        }
    }

    out
}
