use mud_core::templates::{ClassTemplate, RaceTemplate, TemplateRegistry};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

pub fn load_templates(content_path: &Path) -> TemplateRegistry {
    let races = load_races(content_path);
    let classes = load_classes(content_path);
    TemplateRegistry::new(races, classes)
}

fn load_races(content_path: &Path) -> HashMap<String, RaceTemplate> {
    let races_dir = content_path.join("races");
    let mut races = HashMap::new();

    if !races_dir.exists() {
        tracing::warn!("Races directory not found: {}", races_dir.display());
        return races;
    }

    let entries = match fs::read_dir(&races_dir) {
        Ok(e) => e,
        Err(e) => {
            tracing::error!("Failed to read races directory: {e}");
            return races;
        }
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().is_some_and(|ext| ext == "toml") {
            match fs::read_to_string(&path) {
                Ok(content) => match toml::from_str::<RaceTemplate>(&content) {
                    Ok(template) => {
                        let id = template.id.clone();
                        races.insert(id, template);
                    }
                    Err(e) => {
                        tracing::error!("Failed to parse race template '{}': {e}", path.display());
                    }
                },
                Err(e) => {
                    tracing::error!("Failed to read '{}': {e}", path.display());
                }
            }
        }
    }

    races
}

fn load_classes(content_path: &Path) -> HashMap<String, ClassTemplate> {
    let classes_dir = content_path.join("classes");
    let mut classes = HashMap::new();

    if !classes_dir.exists() {
        tracing::warn!("Classes directory not found: {}", classes_dir.display());
        return classes;
    }

    let entries = match fs::read_dir(&classes_dir) {
        Ok(e) => e,
        Err(e) => {
            tracing::error!("Failed to read classes directory: {e}");
            return classes;
        }
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().is_some_and(|ext| ext == "toml") {
            match fs::read_to_string(&path) {
                Ok(content) => match toml::from_str::<ClassTemplate>(&content) {
                    Ok(template) => {
                        let id = template.id.clone();
                        classes.insert(id, template);
                    }
                    Err(e) => {
                        tracing::error!("Failed to parse class template '{}': {e}", path.display());
                    }
                },
                Err(e) => {
                    tracing::error!("Failed to read '{}': {e}", path.display());
                }
            }
        }
    }

    classes
}
