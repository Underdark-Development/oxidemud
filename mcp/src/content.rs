use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use mud_core::templates::TemplateRegistry;

pub type FileMap = HashMap<String, HashMap<String, PathBuf>>;

pub fn load_registry(path: &Path) -> (TemplateRegistry, FileMap) {
    let mut file_map = FileMap::new();
    let mut registry = TemplateRegistry::new();

    registry.races = load_dir(path, "races", &mut file_map);
    registry.classes = load_dir(path, "classes", &mut file_map);
    registry.items = load_dir(path, "items", &mut file_map);
    registry.mobs = load_dir(path, "mobs", &mut file_map);
    registry.stances = load_dir(path, "stances", &mut file_map);
    registry.sets = load_dir(path, "sets", &mut file_map);
    registry.affixes = load_dir(path, "affixes", &mut file_map);
    registry.passives = load_dir(path, "passives", &mut file_map);
    registry.areas = load_dir(path, "areas", &mut file_map);
    registry.skills = load_dir(path, "skills", &mut file_map);

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
