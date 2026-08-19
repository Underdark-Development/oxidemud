//! Area/room/link/portal/template handler implementations.

use std::collections::HashMap;
use std::fs;

use oxide_core::templates::{
    AffixDef, AreaTemplate, ClassTemplate, DeityTemplate, ExitTemplate, FactionDef, ItemTemplate,
    MobTemplate, PassiveDef, QuestDef, RaceTemplate, RecipeDef, RoomContent, RoomTemplate, SetDef,
    ShopTemplate, StanceDef,
};
use oxide_core::SkillDef;
use rmcp::handler::server::wrapper::Parameters;

use crate::content;
use crate::context::HandlerContext;
use crate::params::*;

pub fn list_areas(ctx: &HandlerContext<'_>) -> String {
    let (registry, _) = ctx.load();
    HandlerContext::entity_list(
        &registry
            .areas
            .iter()
            .map(|(k, v)| (k.clone(), v.name.as_str()))
            .collect(),
        "Areas",
    )
}

pub fn get_area(ctx: &HandlerContext<'_>, params: Parameters<IdParam>) -> String {
    let p = params.0;
    let (registry, _) = ctx.load();
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

pub fn create_area(ctx: &HandlerContext<'_>, params: Parameters<CreateAreaParams>) -> String {
    let p = params.0;
    if let Err(e) = ctx.validate_id(&p.id) {
        return format!("Error: {e}");
    }
    let area_dir = ctx.content_path().join("areas").join(&p.id);
    if let Err(e) = ctx.validate_and_contain(&p.id, &area_dir) {
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

pub fn delete_area(ctx: &HandlerContext<'_>, params: Parameters<IdParam>) -> String {
    let p = params.0;
    let (_, file_map) = ctx.load();
    let area_path = match file_map.get("areas").and_then(|m| m.get(&p.id)) {
        Some(p) => p.clone(),
        None => return format!("Error: area '{}' not found", p.id),
    };

    // If it's a subdirectory-format area, delete the whole directory.
    // If it's a flat file, just delete the file.
    if area_path.file_name().is_some_and(|n| n == "area.toml") {
        let parent_dir = match area_path.parent() {
            Some(path) => path,
            None => return "Error: invalid area path".to_string(),
        };
        if let Err(e) = fs::remove_dir_all(parent_dir) {
            return format!("Error: failed to delete area directory: {e}");
        }
    } else if let Err(e) = fs::remove_file(&area_path) {
        return format!("Error: failed to delete {}: {e}", area_path.display());
    }

    format!("Deleted area '{}'", p.id)
}

pub fn list_rooms(ctx: &HandlerContext<'_>, params: Parameters<AreaIdParam>) -> String {
    let p = params.0;
    let (registry, _) = ctx.load();
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

pub fn get_room(ctx: &HandlerContext<'_>, params: Parameters<RoomIdParam>) -> String {
    let p = params.0;
    let (registry, _) = ctx.load();
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

pub fn create_room(ctx: &HandlerContext<'_>, params: Parameters<CreateRoomParams>) -> String {
    let p = params.0;
    let area_id = &p.area_id;
    let room_id = &p.room_id;
    if let Err(e) = ctx.validate_id(area_id) {
        return format!("Error: {e}");
    }
    if let Err(e) = ctx.validate_id(room_id) {
        return format!("Error: {e}");
    }
    let (_, file_map) = ctx.load();

    let area_dir = match content::area_dir_from_file(&file_map, area_id) {
        Ok(d) => d,
        Err(e) => return format!("Error: {e}"),
    };

    let room_path = area_dir.join("rooms").join(format!("{room_id}.toml"));
    if let Err(e) = ctx.validate_and_contain(room_id, &room_path) {
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
    let Some(room_parent) = room_path.parent() else {
        return "Error: invalid room path".to_string();
    };
    if let Err(e) = fs::create_dir_all(room_parent).and_then(|_| fs::write(&room_path, &room_str)) {
        return format!("Error: failed to write {}: {e}", room_path.display());
    }
    format!("Created room '{}' in area '{}'", room_id, area_id)
}

pub fn delete_room(ctx: &HandlerContext<'_>, params: Parameters<RoomIdParam>) -> String {
    let p = params.0;
    let (_, file_map) = ctx.load();
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

pub fn link_rooms(ctx: &HandlerContext<'_>, params: Parameters<LinkRoomsParams>) -> String {
    let p = params.0;
    let (_, file_map) = ctx.load();
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

pub fn add_portal(ctx: &HandlerContext<'_>, params: Parameters<UpdateRoomFieldsParams>) -> String {
    let p = params.0;
    if !p.fields.contains_key("keyword") || !p.fields.contains_key("dest") {
        return "Error: 'keyword' and 'dest' fields are required for a portal".to_string();
    }
    update_room_fields(ctx, &p.area_id, &p.room_id, &p.fields)
}

pub fn remove_portal(
    ctx: &HandlerContext<'_>,
    params: Parameters<UpdateRoomFieldsParams>,
) -> String {
    let p = params.0;
    update_room_fields(ctx, &p.area_id, &p.room_id, &p.fields)
}

pub fn update_room(ctx: &HandlerContext<'_>, params: Parameters<UpdateRoomFieldsParams>) -> String {
    let p = params.0;
    update_room_fields(ctx, &p.area_id, &p.room_id, &p.fields)
}

pub fn update_template(ctx: &HandlerContext<'_>, params: Parameters<UpdateFieldsParams>) -> String {
    let p = params.0;
    let (_, file_map) = ctx.load();
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
                Err(e) => return format!("Error: failed to deserialize class after patch: {e}"),
            };
            match toml::to_string_pretty(&t) {
                Ok(s) => s,
                Err(e) => return format!("Error: failed to serialize class: {e}"),
            }
        }
        "skills" => {
            let t: SkillDef = match serde_json::from_value(json_val) {
                Ok(t) => t,
                Err(e) => return format!("Error: failed to deserialize skill after patch: {e}"),
            };
            match toml::to_string_pretty(&t) {
                Ok(s) => s,
                Err(e) => return format!("Error: failed to serialize skill: {e}"),
            }
        }
        "stances" => {
            let t: StanceDef = match serde_json::from_value(json_val) {
                Ok(t) => t,
                Err(e) => return format!("Error: failed to deserialize stance after patch: {e}"),
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
                Err(e) => return format!("Error: failed to deserialize affix after patch: {e}"),
            };
            match toml::to_string_pretty(&t) {
                Ok(s) => s,
                Err(e) => return format!("Error: failed to serialize affix: {e}"),
            }
        }
        "passives" => {
            let t: PassiveDef = match serde_json::from_value(json_val) {
                Ok(t) => t,
                Err(e) => return format!("Error: failed to deserialize passive after patch: {e}"),
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
                Err(e) => return format!("Error: failed to deserialize deity after patch: {e}"),
            };
            match toml::to_string_pretty(&t) {
                Ok(s) => s,
                Err(e) => return format!("Error: failed to serialize deity: {e}"),
            }
        }
        "quests" => {
            let t: QuestDef = match serde_json::from_value(json_val) {
                Ok(t) => t,
                Err(e) => return format!("Error: failed to deserialize quest after patch: {e}"),
            };
            match toml::to_string_pretty(&t) {
                Ok(s) => s,
                Err(e) => return format!("Error: failed to serialize quest: {e}"),
            }
        }
        "factions" => {
            let t: FactionDef = match serde_json::from_value(json_val) {
                Ok(t) => t,
                Err(e) => return format!("Error: failed to deserialize faction after patch: {e}"),
            };
            match toml::to_string_pretty(&t) {
                Ok(s) => s,
                Err(e) => return format!("Error: failed to serialize faction: {e}"),
            }
        }
        "recipes" => {
            let t: RecipeDef = match serde_json::from_value(json_val) {
                Ok(t) => t,
                Err(e) => return format!("Error: failed to deserialize recipe after patch: {e}"),
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

pub(crate) fn update_room_fields(
    ctx: &HandlerContext<'_>,
    area_id: &str,
    room_id: &str,
    fields: &serde_json::Map<String, serde_json::Value>,
) -> String {
    let (_registry, file_map) = ctx.load();
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
