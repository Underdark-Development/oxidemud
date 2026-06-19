use mud_core::{prompt, Entity, Player, World};

use crate::config;
use crate::registry::ConnectionRegistry;

pub fn send_player_prompt(world: &World, entity: Entity, registry: &ConnectionRegistry) {
    let template = world
        .query_one::<&Player>(entity)
        .ok()
        .and_then(|mut q| q.get().cloned())
        .and_then(|p| p.prompt)
        .unwrap_or_else(|| config::get().default_prompt.clone());

    let vars = prompt::build_vars(world, entity);
    let rendered = prompt::render_prompt(&template, &vars);

    if let Some(tx) = registry.sender(entity) {
        let _ = tx.send(rendered.into_bytes());
    }
}

pub fn broadcast_prompts(world: &World, registry: &ConnectionRegistry) {
    let template_map: Vec<(Entity, String)> = registry
        .connected_entities()
        .iter()
        .map(|&entity| {
            let template = world
                .query_one::<&Player>(entity)
                .ok()
                .and_then(|mut q| q.get().cloned())
                .and_then(|p| p.prompt)
                .unwrap_or_else(|| config::get().default_prompt.clone());
            (entity, template)
        })
        .collect();

    for (entity, template) in template_map {
        let vars = prompt::build_vars(world, entity);
        let rendered = prompt::render_prompt(&template, &vars);
        if let Some(tx) = registry.sender(entity) {
            let _ = tx.send(rendered.into_bytes());
        }
    }
}
