//! Generic helpers for the uniform `<category>/<id>.toml` template CRUD.

use std::collections::HashMap;
use std::fs;

use oxide_core::templates::TemplateRegistry;

use crate::content;
use crate::context::HandlerContext;

/// Create a template file under `<content_path>/<category>/<id>.toml`.
/// Preserves the exact output/error strings of the original per-entity handlers.
pub fn create<T: serde::Serialize>(
    ctx: &HandlerContext<'_>,
    id: &str,
    category: &str,
    noun: &str,
    template: T,
) -> String {
    if let Err(e) = ctx.validate_id(id) {
        return format!("Error: {e}");
    }
    let path = ctx.content_path().join(category).join(format!("{id}.toml"));
    if let Err(e) = ctx.validate_and_contain(id, &path) {
        return format!("Error: {e}");
    }
    match toml::to_string_pretty(&template) {
        Ok(content) => {
            if let Err(e) = match path.parent() {
                Some(parent) => fs::create_dir_all(parent).and_then(|_| fs::write(&path, &content)),
                None => return format!("Error: failed to write {noun}: invalid template path"),
            } {
                return format!("Error: failed to write {noun}: {e}");
            }
            format!("Created {noun} '{id}'")
        }
        Err(e) => format!("Error: failed to serialize {noun}: {e}"),
    }
}

/// Delete a template file and return a status string (same output as before).
pub fn delete(ctx: &HandlerContext<'_>, id: &str, category: &str, noun: &str) -> String {
    let (_registry, file_map) = ctx.load();
    match content::delete_file(&file_map, category, id) {
        Ok(()) => format!("Deleted {noun} '{id}'"),
        Err(e) => format!("Error: {e}"),
    }
}

/// List templates as a sorted `label:` block, matching `entity_list` formatting
/// ("No <label> found." when empty; "label:\n  id: name" trimmed).
pub fn list(
    ctx: &HandlerContext<'_>,
    label: &str,
    collect: impl FnOnce(&TemplateRegistry) -> HashMap<String, String>,
) -> String {
    let (registry, _) = ctx.load();
    let items = collect(&registry);
    crate::context::entity_list(&items, label)
}
