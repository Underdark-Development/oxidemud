use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::templates::{AreaTemplate, RoomTemplate, TemplateRegistry};

pub type FileMap = HashMap<String, HashMap<String, PathBuf>>;

pub fn load_registry(content_path: &Path) -> (TemplateRegistry, FileMap) {
    let mut file_map = FileMap::new();
    let mut registry = TemplateRegistry::new();

    registry.races = load_dir(content_path, "races", &mut file_map);
    registry.classes = load_dir(content_path, "classes", &mut file_map);
    registry.items = load_dir(content_path, "items", &mut file_map);
    registry.mobs = load_dir(content_path, "mobs", &mut file_map);
    registry.stances = load_dir(content_path, "stances", &mut file_map);
    registry.sets = load_dir(content_path, "sets", &mut file_map);
    registry.affixes = load_dir(content_path, "affixes", &mut file_map);
    registry.passives = load_dir(content_path, "passives", &mut file_map);
    registry.areas = load_areas(content_path, &mut file_map);
    registry.skills = load_dir(content_path, "skills", &mut file_map);
    registry.shops = load_dir(content_path, "shops", &mut file_map);
    registry.deities = load_dir(content_path, "deities", &mut file_map);

    registry.build_indices();
    (registry, file_map)
}

fn load_dir<T: serde::de::DeserializeOwned>(
    content_path: &Path,
    subdir: &str,
    file_map: &mut FileMap,
) -> HashMap<String, T> {
    let dir = content_path.join(subdir);
    let mut map = HashMap::new();
    if !dir.exists() {
        return map;
    }
    let mut path_map = HashMap::new();
    if let Ok(entries) = fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "toml") {
                if let Ok(content) = fs::read_to_string(&path) {
                    if let Ok(t) = toml::from_str::<T>(&content) {
                        let id = path
                            .file_stem()
                            .and_then(|s| s.to_str())
                            .unwrap_or("unknown")
                            .to_string();
                        path_map.insert(id.clone(), path);
                        map.insert(id, t);
                    }
                }
            }
        }
    }
    file_map.insert(subdir.to_string(), path_map);
    map
}

fn load_areas(content_path: &Path, file_map: &mut FileMap) -> HashMap<String, AreaTemplate> {
    let dir = content_path.join("areas");
    let mut map = HashMap::new();
    let mut path_map = HashMap::new();
    let mut room_path_map = HashMap::new();

    if !dir.exists() {
        return map;
    }

    load_areas_recursive(&dir, &mut map, &mut path_map, &mut room_path_map, "");
    file_map.insert("areas".to_string(), path_map);
    file_map.insert("rooms".to_string(), room_path_map);
    map
}

fn load_areas_recursive(
    dir: &Path,
    map: &mut HashMap<String, AreaTemplate>,
    path_map: &mut HashMap<String, PathBuf>,
    room_path_map: &mut HashMap<String, PathBuf>,
    prefix: &str,
) {
    for entry in fs::read_dir(dir).unwrap().flatten() {
        let path = entry.path();

        // Only subdirectory format: <dir>/<area_id>/area.toml + rooms/*.toml + areas/*
        if !path.is_dir() {
            continue;
        }

        let area_file = path.join("area.toml");
        if !area_file.exists() {
            continue;
        }
        if let Ok(content) = fs::read_to_string(&area_file) {
            if let Ok(mut area) = toml::from_str::<AreaTemplate>(&content) {
                let area_id = if prefix.is_empty() {
                    area.id.clone()
                } else {
                    format!("{}.{}", prefix, area.id)
                };
                area.id.clone_from(&area_id);
                path_map.insert(area_id.clone(), area_file);

                // Load rooms from <dir>/<area_id>/rooms/*.toml
                let rooms_dir = path.join("rooms");
                if rooms_dir.exists() {
                    if let Ok(room_entries) = fs::read_dir(&rooms_dir) {
                        for room_entry in room_entries.flatten() {
                            let room_path = room_entry.path();
                            if room_path.extension().is_some_and(|ext| ext == "toml") {
                                if let Ok(room_content) = fs::read_to_string(&room_path) {
                                    if let Ok(room) = toml::from_str::<RoomTemplate>(&room_content)
                                    {
                                        let room_id = room.id.clone();
                                        if !room_id.is_empty() {
                                            area.rooms.insert(room_id.clone(), room);
                                            // Use composite key to avoid collisions
                                            room_path_map
                                                .insert(format!("{area_id}:{room_id}"), room_path);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                map.insert(area_id.clone(), area);

                // Recurse into <dir>/<area_id>/areas/* for sub-areas
                let sub_areas_dir = path.join("areas");
                if sub_areas_dir.exists() {
                    load_areas_recursive(&sub_areas_dir, map, path_map, room_path_map, &area_id);
                }
            }
        }
    }
}

/// Resolve a room file path from the FileMap using composite key.
pub fn room_path(file_map: &FileMap, area_id: &str, room_id: &str) -> Option<PathBuf> {
    file_map
        .get("rooms")?
        .get(&format!("{area_id}:{room_id}"))
        .cloned()
}

/// Returns the directory path for an area given its area.toml path.
pub fn area_dir_from_file(file_map: &FileMap, area_id: &str) -> Result<PathBuf, String> {
    let area_file = find_file(file_map, "areas", area_id)?;
    area_file
        .parent()
        .map(|p| p.to_path_buf())
        .ok_or_else(|| format!("area '{}' path has no parent", area_id))
}

pub fn find_file(file_map: &FileMap, category: &str, id: &str) -> Result<PathBuf, String> {
    file_map
        .get(category)
        .and_then(|m| m.get(id))
        .cloned()
        .ok_or_else(|| {
            format!(
                "{} '{}' not found on disk",
                category.trim_end_matches('s'),
                id
            )
        })
}

pub fn delete_file(file_map: &FileMap, category: &str, id: &str) -> Result<(), String> {
    let path = find_file(file_map, category, id)?;
    fs::remove_file(&path).map_err(|e| format!("failed to delete {}: {e}", path.display()))
}
