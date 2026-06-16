use mud_core::templates::TemplateRegistry;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

pub type FileMap = HashMap<String, HashMap<String, PathBuf>>;

pub fn load_templates(content_path: &Path) -> (TemplateRegistry, FileMap) {
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
    registry.areas = load_dir(content_path, "areas", &mut file_map);
    registry.skills = load_dir(content_path, "skills", &mut file_map);
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
