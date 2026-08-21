//! Meta/inspection tool handlers (`validate`, `get_stats`, `search`, `get_template_raw`).
//!
//! These operate on the content registry across all template categories and are
//! not simulation or single-entity-category behavior, so they live in their own
//! module rather than `simulation.rs` / `entities.rs`.

use std::fs;

use rmcp::handler::server::wrapper::Parameters;

use crate::content;
use crate::context::HandlerContext;
use crate::params::*;

pub fn validate(ctx: &HandlerContext<'_>) -> String {
    let (registry, _file_map) = ctx.load();
    let errors = registry.validate();
    if errors.is_empty() {
        return "All templates valid.".to_string();
    }
    let mut out = format!("{} validation issue(s):\n", errors.len());
    for err in &errors {
        out.push_str(&format!(
            "  [{}/{}] {}: {}\n",
            err.template_type, err.template_id, err.field, err.message
        ));
    }
    out.trim().to_string()
}

pub fn get_stats(ctx: &HandlerContext<'_>) -> String {
    let (registry, _file_map) = ctx.load();
    let r = &registry;
    let room_count: usize = r.areas.values().map(|a| a.rooms.len()).sum();
    format!(
            "Areas: {}\nRooms: {}\nItems: {}\nMobs: {}\nRaces: {}\nClasses: {}\nSkills: {}\nQuests: {}\nFactions: {}\nRecipes: {}\nShops: {}\nDeities: {}\nStances: {}\nSets: {}\nAffixes: {}\nPassives: {}",
            r.areas.len(),
            room_count,
            r.items.len(),
            r.mobs.len(),
            r.races.len(),
            r.classes.len(),
            r.skills.len(),
            r.quests.len(),
            r.factions.len(),
            r.recipes.len(),
            r.shops.len(),
            r.deities.len(),
            r.stances.len(),
            r.sets.len(),
            r.affixes.len(),
            r.passives.len(),
        )
}

pub fn search(ctx: &HandlerContext<'_>, params: Parameters<SearchParams>) -> String {
    let q = params.0.query.to_lowercase();
    let (registry, _file_map) = ctx.load();
    let r = &registry;
    let mut results: Vec<String> = Vec::new();

    for (id, area) in &r.areas {
        if area.name.to_lowercase().contains(&q) || area.description.to_lowercase().contains(&q) {
            results.push(format!("area:{id} - {name}", name = area.name));
        }
        for (rid, room) in &area.rooms {
            if room.name.to_lowercase().contains(&q) || room.description.to_lowercase().contains(&q)
            {
                results.push(format!("area:{id}/room:{rid} - {name}", name = room.name));
            }
        }
    }

    for (id, item) in &r.items {
        if item.name.to_lowercase().contains(&q) || item.description.to_lowercase().contains(&q) {
            results.push(format!("item:{id} - {name}", name = item.name));
        }
    }
    for (id, mob) in &r.mobs {
        if mob.name.to_lowercase().contains(&q) || mob.description.to_lowercase().contains(&q) {
            results.push(format!("mob:{id} - {name}", name = mob.name));
        }
    }
    for (id, race) in &r.races {
        if race.name.to_lowercase().contains(&q) || race.description.to_lowercase().contains(&q) {
            results.push(format!("race:{id} - {name}", name = race.name));
        }
    }
    for (id, cls) in &r.classes {
        if cls.name.to_lowercase().contains(&q) || cls.description.to_lowercase().contains(&q) {
            results.push(format!("class:{id} - {name}", name = cls.name));
        }
    }
    for (id, skill) in &r.skills {
        if skill.name.to_lowercase().contains(&q) || skill.description.to_lowercase().contains(&q) {
            results.push(format!("skill:{id} - {name}", name = skill.name));
        }
    }
    for (id, quest) in &r.quests {
        if quest.name.to_lowercase().contains(&q) || quest.description.to_lowercase().contains(&q) {
            results.push(format!("quest:{id} - {name}", name = quest.name));
        }
    }
    for (id, faction) in &r.factions {
        if faction.name.to_lowercase().contains(&q)
            || faction.description.to_lowercase().contains(&q)
        {
            results.push(format!("faction:{id} - {name}", name = faction.name));
        }
    }
    for (id, recipe) in &r.recipes {
        if recipe.name.to_lowercase().contains(&q) || recipe.description.to_lowercase().contains(&q)
        {
            results.push(format!("recipe:{id} - {name}", name = recipe.name));
        }
    }
    for (id, shop) in &r.shops {
        if shop.name.to_lowercase().contains(&q) {
            results.push(format!("shop:{id} - {name}", name = shop.name));
        }
    }
    for (id, deity) in &r.deities {
        if deity.name.to_lowercase().contains(&q) || deity.description.to_lowercase().contains(&q) {
            results.push(format!("deity:{id} - {name}", name = deity.name));
        }
    }
    for (id, stance) in &r.stances {
        if stance.name.to_lowercase().contains(&q) {
            results.push(format!("stance:{id} - {name}", name = stance.name));
        }
    }
    for (id, set) in &r.sets {
        if set.name.to_lowercase().contains(&q) {
            results.push(format!("set:{id} - {name}", name = set.name));
        }
    }
    for (id, affix) in &r.affixes {
        if affix.name.to_lowercase().contains(&q) || affix.description.to_lowercase().contains(&q) {
            results.push(format!("affix:{id} - {name}", name = affix.name));
        }
    }
    for (id, passive) in &r.passives {
        if passive.name.to_lowercase().contains(&q)
            || passive.description.to_lowercase().contains(&q)
        {
            results.push(format!("passive:{id} - {name}", name = passive.name));
        }
    }

    if results.is_empty() {
        return format!("No results for '{q}'.");
    }
    results.sort();
    results.insert(0, format!("{} result(s):", results.len()));
    results.join("\n")
}

pub fn get_template_raw(
    ctx: &HandlerContext<'_>,
    params: Parameters<UpdateFieldsParams>,
) -> String {
    let p = params.0;
    let (_registry, file_map) = ctx.load();
    let field = if p.id.is_empty() { &p.category } else { &p.id };
    let path = match content::find_file(&file_map, &p.category, field) {
        Ok(p) => p,
        Err(e) => return format!("Error: {e}"),
    };
    match fs::read_to_string(&path) {
        Ok(c) => c,
        Err(e) => format!("Error: failed to read {}: {e}", path.display()),
    }
}
