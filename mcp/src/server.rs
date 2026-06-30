use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use oxide_core::templates::{
    AffixDef, AreaTemplate, ClassTemplate, ExitTemplate, HealthBounds, ItemTemplate, LootTable,
    MobTemplate, PassiveDef, RaceAttributes, RaceTemplate, RoomContent, RoomTemplate, SetDef,
    StanceDef,
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
struct SimulateCombatParams {
    #[schemars(description = "Optional ID of the attacker mob template")]
    attacker_template: Option<String>,
    #[schemars(description = "Optional ID of the weapon template equipped by the attacker")]
    attacker_weapon: Option<String>,
    #[schemars(
        description = "Optional level override for the attacker (defaults to template level or 1)"
    )]
    attacker_level: Option<u8>,
    #[schemars(description = "Optional ID of the defender mob template")]
    defender_template: Option<String>,
    #[schemars(
        description = "Optional level override for the defender (defaults to template level or 1)"
    )]
    defender_level: Option<u8>,
    #[schemars(description = "Optional armor class (AC) override for the defender")]
    defender_ac_override: Option<i32>,
    #[schemars(description = "Number of rounds to simulate")]
    rounds: u32,
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
struct SimulateCharacterCreationParams {
    race_id: String,
    class_id: String,
    #[schemars(description = "Base strength value (8 to 18)")]
    strength: u8,
    #[schemars(description = "Base dexterity value (8 to 18)")]
    dexterity: u8,
    #[schemars(description = "Base intelligence value (8 to 18)")]
    intelligence: u8,
    #[schemars(description = "Base wisdom value (8 to 18)")]
    wisdom: u8,
    #[schemars(description = "Base constitution value (8 to 18)")]
    constitution: u8,
    #[schemars(description = "Base charisma value (8 to 18)")]
    charisma: u8,
    #[schemars(description = "Optional list of additional selected skill IDs to include")]
    selected_skills: Option<Vec<String>>,
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
        let area_dir = self.content_path.join("areas").join(&p.id);

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
        let (_, file_map) = self.load();

        let area_dir = match content::area_dir_from_file(&file_map, area_id) {
            Ok(d) => d,
            Err(e) => return format!("Error: {e}"),
        };

        let room_path = area_dir.join("rooms").join(format!("{room_id}.toml"));
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
        let name = p.name.unwrap_or_else(|| p.id.clone());
        let path = self
            .content_path
            .join("mobs")
            .join(format!("{}.toml", p.id));
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
        let name = p.name.unwrap_or_else(|| p.id.clone());
        let path = self
            .content_path
            .join("items")
            .join(format!("{}.toml", p.id));
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
            "Areas: {}\nRooms: {}\nItems: {}\nMobs: {}\nRaces: {}\nClasses: {}\nSkills: {}\nStances: {}\nSets: {}\nAffixes: {}\nPassives: {}",
            r.areas.len(),
            room_count,
            r.items.len(),
            r.mobs.len(),
            r.races.len(),
            r.classes.len(),
            r.skills.len(),
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
        match simulator::simulate_combat(
            &registry,
            p.attacker_template.as_deref(),
            p.attacker_weapon.as_deref(),
            p.attacker_level,
            p.defender_template.as_deref(),
            p.defender_level,
            p.defender_ac_override,
            p.rounds,
        ) {
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
}

impl OxideMcpServer {
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
            &p.race_id,
            &p.class_id,
            p.strength,
            p.dexterity,
            p.intelligence,
            p.wisdom,
            p.constitution,
            p.charisma,
            &p.selected_skills.unwrap_or_default(),
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
