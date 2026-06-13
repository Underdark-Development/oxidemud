use mud_core::templates::{
    AffixDef, ClassTemplate, ItemTemplate, MobTemplate, RaceTemplate, SetDef, StanceDef,
    TemplateRegistry,
};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

pub fn load_templates(content_path: &Path) -> TemplateRegistry {
    let mut registry = TemplateRegistry::new();
    registry.races = load_races(content_path);
    registry.classes = load_classes(content_path);
    registry.items = load_items(content_path);
    registry.mobs = load_mobs(content_path);
    registry.stances = load_stances(content_path);
    registry.sets = load_sets(content_path);
    registry.affixes = load_affixes(content_path);
    registry
}

fn load_races(content_path: &Path) -> HashMap<String, RaceTemplate> {
    let dir = content_path.join("races");
    let mut map = HashMap::new();
    if !dir.exists() {
        tracing::warn!("Races directory not found: {}", dir.display());
        return map;
    }
    for entry in fs::read_dir(&dir).unwrap().flatten() {
        let path = entry.path();
        if path.extension().is_some_and(|ext| ext == "toml") {
            if let Ok(content) = fs::read_to_string(&path) {
                match toml::from_str::<RaceTemplate>(&content) {
                    Ok(t) => {
                        map.insert(t.id.clone(), t);
                    }
                    Err(e) => tracing::error!("Failed to parse race '{}': {e}", path.display()),
                }
            }
        }
    }
    map
}

fn load_classes(content_path: &Path) -> HashMap<String, ClassTemplate> {
    let dir = content_path.join("classes");
    let mut map = HashMap::new();
    if !dir.exists() {
        tracing::warn!("Classes directory not found: {}", dir.display());
        return map;
    }
    for entry in fs::read_dir(&dir).unwrap().flatten() {
        let path = entry.path();
        if path.extension().is_some_and(|ext| ext == "toml") {
            if let Ok(content) = fs::read_to_string(&path) {
                match toml::from_str::<ClassTemplate>(&content) {
                    Ok(t) => {
                        map.insert(t.id.clone(), t);
                    }
                    Err(e) => tracing::error!("Failed to parse class '{}': {e}", path.display()),
                }
            }
        }
    }
    map
}

fn load_items(content_path: &Path) -> HashMap<String, ItemTemplate> {
    let dir = content_path.join("items");
    let mut map = HashMap::new();
    if !dir.exists() {
        tracing::warn!("Items directory not found: {}", dir.display());
        return map;
    }
    for entry in fs::read_dir(&dir).unwrap().flatten() {
        let path = entry.path();
        if path.extension().is_some_and(|ext| ext == "toml") {
            if let Ok(content) = fs::read_to_string(&path) {
                match toml::from_str::<ItemTemplate>(&content) {
                    Ok(t) => {
                        map.insert(t.id.clone(), t);
                    }
                    Err(e) => tracing::error!("Failed to parse item '{}': {e}", path.display()),
                }
            }
        }
    }
    map
}

fn load_mobs(content_path: &Path) -> HashMap<String, MobTemplate> {
    let dir = content_path.join("mobs");
    let mut map = HashMap::new();
    if !dir.exists() {
        tracing::warn!("Mobs directory not found: {}", dir.display());
        return map;
    }
    for entry in fs::read_dir(&dir).unwrap().flatten() {
        let path = entry.path();
        if path.extension().is_some_and(|ext| ext == "toml") {
            if let Ok(content) = fs::read_to_string(&path) {
                match toml::from_str::<MobTemplate>(&content) {
                    Ok(t) => {
                        map.insert(t.id.clone(), t);
                    }
                    Err(e) => tracing::error!("Failed to parse mob '{}': {e}", path.display()),
                }
            }
        }
    }
    map
}

fn load_stances(content_path: &Path) -> HashMap<String, StanceDef> {
    let dir = content_path.join("stances");
    let mut map = HashMap::new();
    if !dir.exists() {
        tracing::warn!("Stances directory not found: {}", dir.display());
        return map;
    }
    for entry in fs::read_dir(&dir).unwrap().flatten() {
        let path = entry.path();
        if path.extension().is_some_and(|ext| ext == "toml") {
            if let Ok(content) = fs::read_to_string(&path) {
                match toml::from_str::<StanceDef>(&content) {
                    Ok(t) => {
                        map.insert(t.id.clone(), t);
                    }
                    Err(e) => tracing::error!("Failed to parse stance '{}': {e}", path.display()),
                }
            }
        }
    }
    map
}

fn load_sets(content_path: &Path) -> HashMap<String, SetDef> {
    let dir = content_path.join("sets");
    let mut map = HashMap::new();
    if !dir.exists() {
        tracing::warn!("Sets directory not found: {}", dir.display());
        return map;
    }
    for entry in fs::read_dir(&dir).unwrap().flatten() {
        let path = entry.path();
        if path.extension().is_some_and(|ext| ext == "toml") {
            if let Ok(content) = fs::read_to_string(&path) {
                match toml::from_str::<SetDef>(&content) {
                    Ok(t) => {
                        map.insert(t.id.clone(), t);
                    }
                    Err(e) => tracing::error!("Failed to parse set '{}': {e}", path.display()),
                }
            }
        }
    }
    map
}

fn load_affixes(content_path: &Path) -> HashMap<String, AffixDef> {
    let dir = content_path.join("affixes");
    let mut map = HashMap::new();
    if !dir.exists() {
        tracing::warn!("Affixes directory not found: {}", dir.display());
        return map;
    }
    for entry in fs::read_dir(&dir).unwrap().flatten() {
        let path = entry.path();
        if path.extension().is_some_and(|ext| ext == "toml") {
            if let Ok(content) = fs::read_to_string(&path) {
                match toml::from_str::<AffixDef>(&content) {
                    Ok(t) => {
                        map.insert(t.id.clone(), t);
                    }
                    Err(e) => tracing::error!("Failed to parse affix '{}': {e}", path.display()),
                }
            }
        }
    }
    map
}
