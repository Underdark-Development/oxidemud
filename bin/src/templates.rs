use mud_core::templates::{
    AffixDef, AreaTemplate, ClassTemplate, ItemTemplate, MobTemplate, PassiveDef, RaceTemplate,
    RoomTemplate, SetDef, StanceDef, TemplateRegistry,
};
use mud_core::SkillDef;
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
    registry.passives = load_passives(content_path);
    registry.areas = load_areas(content_path);
    registry.skills = load_skills(content_path);
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

fn load_passives(content_path: &Path) -> HashMap<String, PassiveDef> {
    let dir = content_path.join("passives");
    let mut map = HashMap::new();
    if !dir.exists() {
        tracing::warn!("Passives directory not found: {}", dir.display());
        return map;
    }
    for entry in fs::read_dir(&dir).unwrap().flatten() {
        let path = entry.path();
        if path.extension().is_some_and(|ext| ext == "toml") {
            if let Ok(content) = fs::read_to_string(&path) {
                match toml::from_str::<PassiveDef>(&content) {
                    Ok(t) => {
                        map.insert(t.id.clone(), t);
                    }
                    Err(e) => tracing::error!("Failed to parse passive '{}': {e}", path.display()),
                }
            }
        }
    }
    map
}

fn load_areas(content_path: &Path) -> HashMap<String, AreaTemplate> {
    let dir = content_path.join("areas");
    let mut map = HashMap::new();
    if !dir.exists() {
        tracing::warn!("Areas directory not found: {}", dir.display());
        return map;
    }
    load_areas_recursive(&dir, &mut map, "");
    map
}

fn load_areas_recursive(dir: &Path, map: &mut HashMap<String, AreaTemplate>, prefix: &str) {
    for entry in fs::read_dir(dir).unwrap().flatten() {
        let path = entry.path();

        // Flat file: <prefix>/areas/<id>.toml (backward compat)
        if path.extension().is_some_and(|ext| ext == "toml") {
            if let Ok(content) = fs::read_to_string(&path) {
                match toml::from_str::<AreaTemplate>(&content) {
                    Ok(mut t) => {
                        let id = if prefix.is_empty() {
                            t.id.clone()
                        } else {
                            format!("{}.{}", prefix, t.id)
                        };
                        t.id.clone_from(&id);
                        map.insert(id, t);
                    }
                    Err(e) => tracing::error!("Failed to parse area '{}': {e}", path.display()),
                }
            }
            continue;
        }

        // Subdirectory: <dir>/<area_id>/area.toml + rooms/*.toml + areas/*
        if path.is_dir() {
            let area_file = path.join("area.toml");
            if !area_file.exists() {
                continue;
            }
            if let Ok(content) = fs::read_to_string(&area_file) {
                match toml::from_str::<AreaTemplate>(&content) {
                    Ok(mut area) => {
                        let area_id = if prefix.is_empty() {
                            area.id.clone()
                        } else {
                            format!("{}.{}", prefix, area.id)
                        };
                        area.id.clone_from(&area_id);

                        // Load rooms from <dir>/<area_id>/rooms/*.toml
                        let rooms_dir = path.join("rooms");
                        if rooms_dir.exists() {
                            if let Ok(room_entries) = fs::read_dir(&rooms_dir) {
                                for room_entry in room_entries.flatten() {
                                    let room_path = room_entry.path();
                                    if room_path.extension().is_some_and(|ext| ext == "toml") {
                                        if let Ok(room_content) = fs::read_to_string(&room_path) {
                                            match toml::from_str::<RoomTemplate>(&room_content) {
                                                Ok(room) => {
                                                    let room_id = room.id.clone();
                                                    if !room_id.is_empty() {
                                                        area.rooms.insert(room_id, room);
                                                    }
                                                }
                                                Err(e) => tracing::error!(
                                                    "Failed to parse room '{}': {e}",
                                                    room_path.display()
                                                ),
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
                            load_areas_recursive(&sub_areas_dir, map, &area_id);
                        }
                    }
                    Err(e) => {
                        tracing::error!("Failed to parse area '{}': {e}", area_file.display())
                    }
                }
            }
        }
    }
}

fn load_skills(content_path: &Path) -> HashMap<String, SkillDef> {
    let dir = content_path.join("skills");
    let mut map = HashMap::new();
    if !dir.exists() {
        tracing::warn!("Skills directory not found: {}", dir.display());
        return map;
    }
    for entry in fs::read_dir(&dir).unwrap().flatten() {
        let path = entry.path();
        if path.extension().is_some_and(|ext| ext == "toml") {
            if let Ok(content) = fs::read_to_string(&path) {
                match toml::from_str::<SkillDef>(&content) {
                    Ok(t) => {
                        map.insert(t.id.clone(), t);
                    }
                    Err(e) => tracing::error!("Failed to parse skill '{}': {e}", path.display()),
                }
            }
        }
    }
    map
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn project_root() -> PathBuf {
        let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        manifest_dir.parent().unwrap().to_path_buf()
    }

    #[test]
    fn test_actual_content_class_filtering() {
        let content_path = project_root().join("content");
        let registry = load_templates(&content_path);

        let orc_classes = registry.available_classes_for_race("orc");
        assert_eq!(orc_classes.len(), 1);
        assert_eq!(orc_classes[0].id, "warrior");

        let mage_races = registry.available_races_for_class("mage");
        assert_eq!(mage_races.len(), 1);
        assert_eq!(mage_races[0].id, "human");
    }
}
