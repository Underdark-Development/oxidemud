use mud_core::templates::TemplateRegistry;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

pub fn load_templates(content_path: &Path) -> TemplateRegistry {
    let mut registry = TemplateRegistry::new();
    registry.races = load_dir(content_path, "races");
    registry.classes = load_dir(content_path, "classes");
    registry.items = load_dir(content_path, "items");
    registry.mobs = load_dir(content_path, "mobs");
    registry.stances = load_dir(content_path, "stances");
    registry.sets = load_dir(content_path, "sets");
    registry.affixes = load_dir(content_path, "affixes");
    registry.passives = load_dir(content_path, "passives");
    registry.areas = load_dir(content_path, "areas");
    registry.skills = load_dir(content_path, "skills");
    registry
}

fn load_dir<T: serde::de::DeserializeOwned>(
    content_path: &Path,
    subdir: &str,
) -> HashMap<String, T> {
    let dir = content_path.join(subdir);
    let mut map = HashMap::new();
    if !dir.exists() {
        return map;
    }
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
                        map.insert(id, t);
                    }
                }
            }
        }
    }
    map
}
