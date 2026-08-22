//! Generic helpers for the uniform `<category>/<id>.toml` template CRUD.
//!
//! When an online WS connection is configured (`--ws`/`--key`) the create and
//! delete operations run through the server's `content.write` / `content.delete`
//! RPC methods so the MUD server's content tree is the source of truth. When
//! offline they write/delete directly on the local `content_path`, unchanged.

use std::collections::HashMap;
use std::fs;

use oxide_core::templates::TemplateRegistry;

use crate::content;
use crate::context::{rpc_error_message, HandlerContext};

/// Create a template file under `<content_path>/<category>/<id>.toml`.
/// Preserves the exact output/error strings of the original per-entity handlers.
pub async fn create<T: serde::Serialize>(
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
            if ctx.has_creds() {
                online_write(ctx, category, id, &content, noun).await
            } else {
                match path.parent() {
                    Some(parent) => {
                        match fs::create_dir_all(parent).and_then(|_| fs::write(&path, &content)) {
                            Ok(()) => format!("Created {noun} '{id}'"),
                            Err(e) => format!("Error: failed to write {noun}: {e}"),
                        }
                    }
                    None => format!("Error: failed to write {noun}: invalid template path"),
                }
            }
        }
        Err(e) => format!("Error: failed to serialize {noun}: {e}"),
    }
}

/// Delete a template file and return a status string (same output as before).
pub async fn delete(ctx: &HandlerContext<'_>, id: &str, category: &str, noun: &str) -> String {
    if ctx.has_creds() {
        online_delete(ctx, category, id, noun).await
    } else {
        let (_registry, file_map) = ctx.load();
        match content::delete_file(&file_map, category, id) {
            Ok(()) => format!("Deleted {noun} '{id}'"),
            Err(e) => format!("Error: {e}"),
        }
    }
}

/// Send a `content.write` RPC, reusing the exact offline success/error strings.
async fn online_write(
    ctx: &HandlerContext<'_>,
    category: &str,
    id: &str,
    content: &str,
    noun: &str,
) -> String {
    let client = match ctx.rpc().await {
        Ok(c) => c,
        Err(e) => return format!("Error: {e}"),
    };
    let params = serde_json::json!({
        "path": format!("{category}/{id}.toml"),
        "content": content,
    });
    match client.call("content.write", params).await {
        Ok(_) => format!("Created {noun} '{id}'"),
        Err(e) => format!("Error: {}", rpc_error_message(e)),
    }
}

/// Send a `content.delete` RPC, reusing the exact offline success/error strings.
async fn online_delete(ctx: &HandlerContext<'_>, category: &str, id: &str, noun: &str) -> String {
    let client = match ctx.rpc().await {
        Ok(c) => c,
        Err(e) => return format!("Error: {e}"),
    };
    // content.delete is a destructive op: the server requires an explicit
    // `confirm: true` (same contract as the imm.* tools).
    let params = serde_json::json!({
        "path": format!("{category}/{id}.toml"),
        "confirm": true,
    });
    match client.call("content.delete", params).await {
        Ok(_) => format!("Deleted {noun} '{id}'"),
        Err(e) => format!("Error: {}", rpc_error_message(e)),
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
