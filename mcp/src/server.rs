use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use mud_core::templates::{
    AffixDef, AreaTemplate, ClassTemplate, HealthBounds, ItemTemplate, LootTable, MobTemplate,
    PassiveDef, RaceAttributes, RaceTemplate, RoomContent, RoomTemplate, SetDef, StanceDef,
    TemplateRegistry,
};
use mud_core::SkillDef;
use rmcp::{
    handler::server::wrapper::Parameters, schemars, tool, tool_router, transport::stdio, ServiceExt,
};
use schemars::JsonSchema;
use serde::Deserialize;

use crate::content;

struct ServerState {
    registry: TemplateRegistry,
    file_map: content::FileMap,
}

#[derive(Clone)]
pub struct MudMcpServer {
    content_path: PathBuf,
    state: Arc<RwLock<ServerState>>,
}

impl MudMcpServer {
    pub fn new(content_path: PathBuf) -> Self {
        let (registry, file_map) = content::load_registry(&content_path);
        MudMcpServer {
            content_path,
            state: Arc::new(RwLock::new(ServerState { registry, file_map })),
        }
    }

    pub async fn run(self) -> anyhow::Result<()> {
        let service = self.serve(stdio()).await?;
        service.waiting().await?;
        Ok(())
    }

    fn reload(&self) {
        let (registry, file_map) = content::load_registry(&self.content_path);
        *self.state.write().unwrap() = ServerState { registry, file_map };
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

#[tool_router(server_handler)]
impl MudMcpServer {
    #[tool(description = "List all areas")]
    fn list_areas(&self) -> String {
        let state = self.state.read().unwrap();
        Self::entity_list(
            &state
                .registry
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
        let state = self.state.read().unwrap();
        match state.registry.get_area(&p.id) {
            Some(area) => {
                let mut out = format!(
                    "id: {}\nname: {}\ndescription: {}",
                    p.id, area.name, area.description
                );
                out.push_str(&format!("\nspawn_room: {}", area.spawn_room));
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
            spawn_room: "start".to_string(),
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
        };
        let room_str = match toml::to_string_pretty(&room) {
            Ok(s) => s,
            Err(e) => return format!("Error: failed to serialize starter room: {e}"),
        };
        if let Err(e) = fs::write(area_dir.join("rooms").join("start.toml"), &room_str) {
            return format!("Error: failed to write starter room: {e}");
        }

        self.reload();
        format!("Created area '{}'", p.id)
    }

    #[tool(description = "Delete an area and its file")]
    fn delete_area(&self, params: Parameters<IdParam>) -> String {
        let p = params.0;
        let state = self.state.read().unwrap();
        let area_path = match state.file_map.get("areas").and_then(|m| m.get(&p.id)) {
            Some(p) => p.clone(),
            None => return format!("Error: area '{}' not found", p.id),
        };
        drop(state);

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

        self.reload();
        format!("Deleted area '{}'", p.id)
    }

    #[tool(description = "List rooms in an area")]
    fn list_rooms(&self, params: Parameters<AreaIdParam>) -> String {
        let p = params.0;
        let state = self.state.read().unwrap();
        match state.registry.get_area(&p.area_id) {
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
        let state = self.state.read().unwrap();
        match state.registry.get_area(&p.area_id) {
            Some(area) => match area.rooms.get(&p.room_id) {
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
                            out.push_str(&format!("\n  {dir}: {}", room.exits[dir]));
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
            },
            None => format!("Error: area '{}' not found", p.area_id),
        }
    }

    #[tool(description = "Create a new room in an area")]
    fn create_room(&self, params: Parameters<CreateRoomParams>) -> String {
        let p = params.0;
        let area_id = &p.area_id;
        let room_id = &p.room_id;
        let state = self.state.read().unwrap();

        let area_dir = match content::area_dir_from_file(&state.file_map, area_id) {
            Ok(d) => d,
            Err(e) => return format!("Error: {e}"),
        };
        drop(state);

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
        self.reload();
        format!("Created room '{}' in area '{}'", room_id, area_id)
    }

    #[tool(description = "Delete a room from an area")]
    fn delete_room(&self, params: Parameters<RoomIdParam>) -> String {
        let p = params.0;
        let state = self.state.read().unwrap();
        let room_path = match state.file_map.get("rooms").and_then(|m| m.get(&p.room_id)) {
            Some(p) => p.clone(),
            None => return format!("Error: room '{}' not found", p.room_id),
        };
        drop(state);

        if let Err(e) = fs::remove_file(&room_path) {
            return format!("Error: failed to delete {}: {e}", room_path.display());
        }
        self.reload();
        format!("Deleted room '{}' from area '{}'", p.room_id, p.area_id)
    }

    #[tool(description = "Link two rooms together by adding an exit")]
    fn link_rooms(&self, params: Parameters<LinkRoomsParams>) -> String {
        let p = params.0;
        let state = self.state.read().unwrap();
        let room_path = match state
            .file_map
            .get("rooms")
            .and_then(|m| m.get(&p.from_room))
        {
            Some(p) => p.clone(),
            None => return format!("Error: room '{}' not found", p.from_room),
        };
        drop(state);

        let room_content = match fs::read_to_string(&room_path) {
            Ok(c) => c,
            Err(e) => return format!("Error: failed to read {}: {e}", room_path.display()),
        };
        let mut room: RoomTemplate = match toml::from_str(&room_content) {
            Ok(r) => r,
            Err(e) => return format!("Error: failed to parse room: {e}"),
        };
        let dest = format!("{}:{}", p.to_area, p.to_room);
        room.exits.insert(p.direction.clone(), dest.clone());
        match toml::to_string_pretty(&room) {
            Ok(out) => {
                if let Err(e) = fs::write(&room_path, &out) {
                    return format!("Error: failed to write {}: {e}", room_path.display());
                }
            }
            Err(e) => return format!("Error: failed to serialize room: {e}"),
        }
        self.reload();
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
        let state = self.state.read().unwrap();
        if p.category == "rooms" {
            return "Error: use update_room for room fields".to_string();
        }
        let path = match content::find_file(&state.file_map, &p.category, &p.id) {
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
                drop(state);
                self.reload();
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
        drop(state);
        self.reload();
        format!(
            "Updated {} field(s) on {}/{}",
            p.fields.len(),
            p.category,
            p.id
        )
    }

    #[tool(description = "List all mob templates")]
    fn list_mobs(&self) -> String {
        let state = self.state.read().unwrap();
        Self::entity_list(
            &state
                .registry
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
        let state = self.state.read().unwrap();
        match state.registry.get_mob(&p.id) {
            Some(mob) => format!(
                "id: {}\nname: {}\nlevel: {}\ndescription: {}\narmor: {}\nai: {}",
                p.id, mob.name, mob.level, mob.description, mob.armor, mob.ai_mode
            ),
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
            aggro_range: 0,
            aggro_players: false,
            aggro_race: Vec::new(),
            faction: None,
            faction_standing: 0,
            trainer_types: Vec::new(),
            languages: Vec::new(),
            skills: Vec::new(),
            scripts: Vec::new(),
        };
        match toml::to_string_pretty(&mob) {
            Ok(content) => {
                if let Err(e) = fs::create_dir_all(path.parent().unwrap())
                    .and_then(|_| fs::write(&path, &content))
                {
                    return format!("Error: failed to write mob: {e}");
                }
                self.reload();
                format!("Created mob '{}'", p.id)
            }
            Err(e) => format!("Error: failed to serialize mob: {e}"),
        }
    }

    #[tool(description = "Delete a mob template")]
    fn delete_mob(&self, params: Parameters<IdParam>) -> String {
        let p = params.0;
        let state = self.state.read().unwrap();
        match content::delete_file(&state.file_map, "mobs", &p.id) {
            Ok(()) => {
                drop(state);
                self.reload();
                format!("Deleted mob '{}'", p.id)
            }
            Err(e) => format!("Error: {e}"),
        }
    }

    #[tool(description = "List all item templates")]
    fn list_items(&self) -> String {
        let state = self.state.read().unwrap();
        Self::entity_list(
            &state
                .registry
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
        let state = self.state.read().unwrap();
        match state.registry.get_item(&p.id) {
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
        };
        match toml::to_string_pretty(&item) {
            Ok(content) => {
                if let Err(e) = fs::create_dir_all(path.parent().unwrap())
                    .and_then(|_| fs::write(&path, &content))
                {
                    return format!("Error: failed to write item: {e}");
                }
                self.reload();
                format!("Created item '{}'", p.id)
            }
            Err(e) => format!("Error: failed to serialize item: {e}"),
        }
    }

    #[tool(description = "Delete an item template")]
    fn delete_item(&self, params: Parameters<IdParam>) -> String {
        let p = params.0;
        let state = self.state.read().unwrap();
        match content::delete_file(&state.file_map, "items", &p.id) {
            Ok(()) => {
                drop(state);
                self.reload();
                format!("Deleted item '{}'", p.id)
            }
            Err(e) => format!("Error: {e}"),
        }
    }

    #[tool(description = "List all skill templates")]
    fn list_skills(&self) -> String {
        let state = self.state.read().unwrap();
        Self::entity_list(
            &state
                .registry
                .skills
                .iter()
                .map(|(k, v)| (k.clone(), v.name.as_str()))
                .collect(),
            "Skills",
        )
    }

    #[tool(description = "List all race templates")]
    fn list_races(&self) -> String {
        let state = self.state.read().unwrap();
        Self::entity_list(
            &state
                .registry
                .races
                .iter()
                .map(|(k, v)| (k.clone(), v.name.as_str()))
                .collect(),
            "Races",
        )
    }

    #[tool(description = "List all class templates")]
    fn list_classes(&self) -> String {
        let state = self.state.read().unwrap();
        Self::entity_list(
            &state
                .registry
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
        let state = self.state.read().unwrap();
        let field = if p.id.is_empty() { &p.category } else { &p.id };
        let path = match content::find_file(&state.file_map, &p.category, field) {
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
        let state = self.state.read().unwrap();
        let errors = state.registry.validate();
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
        let state = self.state.read().unwrap();
        let r = &state.registry;
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
        let state = self.state.read().unwrap();
        let r = &state.registry;
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
}

impl MudMcpServer {
    fn update_room_fields(
        &self,
        area_id: &str,
        room_id: &str,
        fields: &serde_json::Map<String, serde_json::Value>,
    ) -> String {
        let state = self.state.read().unwrap();
        let room_path = match state.file_map.get("rooms").and_then(|m| m.get(room_id)) {
            Some(p) => p.clone(),
            None => return format!("Error: room '{}' not found", room_id),
        };
        drop(state);

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
        self.reload();
        format!(
            "Updated {} field(s) on room {}/{}",
            fields.len(),
            area_id,
            room_id
        )
    }
}
