//! Gameplay simulation handler implementations.

use rmcp::handler::server::wrapper::Parameters;

use crate::context::HandlerContext;
use crate::params::*;
use crate::simulator;
use crate::simulator::{
    SimulateCharacterCreationParams, SimulateCombatParams, SimulateSkillUseParams,
};

pub fn simulate_loot(ctx: &HandlerContext<'_>, params: Parameters<SimulateLootParams>) -> String {
    let p = params.0;
    let (registry, _) = ctx.load();
    match simulator::simulate_loot(
        &registry,
        &p.mob_id,
        p.iterations,
        p.detailed.unwrap_or(false),
    ) {
        Ok(result) => result,
        Err(e) => format!("Error simulating loot: {e}"),
    }
}

pub fn simulate_combat(
    ctx: &HandlerContext<'_>,
    params: Parameters<SimulateCombatParams>,
) -> String {
    let p = params.0;
    let (registry, _) = ctx.load();
    match simulator::simulate_combat(&registry, &p) {
        Ok(result) => result,
        Err(e) => format!("Error simulating combat: {e}"),
    }
}

pub fn simulate_progression(
    ctx: &HandlerContext<'_>,
    params: Parameters<SimulateProgressionParams>,
) -> String {
    let p = params.0;
    let (registry, _) = ctx.load();
    match simulator::simulate_progression(
        &registry,
        &p.race_id,
        &p.class_id,
        p.start_level,
        p.end_level,
    ) {
        Ok(result) => result,
        Err(e) => format!("Error simulating progression: {e}"),
    }
}

pub fn simulate_gear_loadout(
    ctx: &HandlerContext<'_>,
    params: Parameters<SimulateGearLoadoutParams>,
) -> String {
    let p = params.0;
    let (registry, _) = ctx.load();
    match simulator::simulate_gear_loadout(
        &registry,
        &p.race_id,
        &p.class_id,
        p.level,
        &p.equipped_items,
    ) {
        Ok(result) => result,
        Err(e) => format!("Error simulating gear loadout: {e}"),
    }
}

pub fn simulate_ai_wander(
    ctx: &HandlerContext<'_>,
    params: Parameters<SimulateAiWanderParams>,
) -> String {
    let p = params.0;
    let (registry, _) = ctx.load();
    match simulator::simulate_ai_wander(&registry, &p.mob_id, &p.start_room_str, p.ticks) {
        Ok(result) => result,
        Err(e) => format!("Error simulating AI wander: {e}"),
    }
}

pub fn simulate_shop_transaction(
    ctx: &HandlerContext<'_>,
    params: Parameters<SimulateShopTransactionParams>,
) -> String {
    let p = params.0;
    let (registry, _) = ctx.load();
    match simulator::simulate_shop_transaction(&registry, &p.shop_id, &p.item_id) {
        Ok(result) => result,
        Err(e) => format!("Error simulating shop transaction: {e}"),
    }
}

pub fn validate_content_dag(ctx: &HandlerContext<'_>) -> String {
    match simulator::validate_content_dag(ctx.content_path()) {
        Ok(result) => result,
        Err(e) => format!("Error validating content DAG: {e}"),
    }
}

pub async fn simulate_crafting(
    ctx: &HandlerContext<'_>,
    params: Parameters<SimulateCraftingParams>,
) -> String {
    let p = params.0;
    let (registry, _) = ctx.load();

    let mut player_level = p.player_level.unwrap_or(1);
    let mut dexterity = p.dexterity.unwrap_or(10);
    let mut intelligence = p.intelligence.unwrap_or(10);
    let mut skill_rank = p.skill_rank.unwrap_or(0);
    let mut loaded_msg = String::new();

    if let Some(ref name) = p.player_name {
        match ctx.fetch_player_state(name).await {
            Ok(player) => {
                player_level = player.level;
                dexterity = player.attributes.dexterity;
                intelligence = player.attributes.intelligence;
                if let Some(recipe) = registry.recipes.get(&p.recipe_id) {
                    if let Some(ref req) = recipe.skill_requirement {
                        skill_rank = player.skills.get(&req.id).copied().unwrap_or(0);
                    }
                }
                loaded_msg = format!(
                    "*   **Simulation Actor**: Loaded from Database (`{}` - Level {} {})\n\n",
                    player.name, player.level, player.class_id
                );
            }
            Err(e) => return format!("Error loading player from database: {e}"),
        }
    }

    match simulator::simulate_crafting(
        &registry,
        &p.recipe_id,
        player_level,
        dexterity,
        intelligence,
        skill_rank,
        p.has_station.unwrap_or(true),
    ) {
        Ok(result) => {
            if !loaded_msg.is_empty() {
                format!("{}{}", loaded_msg, result)
            } else {
                result
            }
        }
        Err(e) => format!("Error simulating crafting: {e}"),
    }
}

pub async fn simulate_skill_use(
    ctx: &HandlerContext<'_>,
    params: Parameters<SimulateSkillUseParams>,
) -> String {
    let p = params.0;
    let (registry, _) = ctx.load();

    let mut actor_level = p.actor_level.unwrap_or(1);
    let mut actor_class = p.actor_class.clone();
    let mut actor_race = p.actor_race.clone();
    let mut strength = p.strength;
    let mut dexterity = p.dexterity;
    let mut intelligence = p.intelligence;
    let mut wisdom = p.wisdom;
    let mut constitution = p.constitution;
    let mut charisma = p.charisma;
    let mut skill_rank = p.skill_rank;
    let mut loaded_msg = String::new();

    if let Some(ref name) = p.actor_name {
        match ctx.fetch_player_state(name).await {
            Ok(player) => {
                actor_level = player.level;
                actor_class = Some(player.class_id.clone());
                actor_race = Some(player.race_id);
                strength = Some(player.attributes.strength);
                dexterity = Some(player.attributes.dexterity);
                intelligence = Some(player.attributes.intelligence);
                wisdom = Some(player.attributes.wisdom);
                constitution = Some(player.attributes.constitution);
                charisma = Some(player.attributes.charisma);
                skill_rank = Some(player.skills.get(&p.skill_id).copied().unwrap_or(0));
                loaded_msg = format!(
                    "*   **Simulation Actor**: Loaded from Database (`{}` - Level {} {})\n\n",
                    player.name, player.level, player.class_id
                );
            }
            Err(e) => return format!("Error loading actor from database: {e}"),
        }
    }

    match simulator::simulate_skill_use(
        &registry,
        &crate::simulator::SimulateSkillUseParams {
            skill_id: p.skill_id,
            actor_name: None,
            actor_level: Some(actor_level),
            actor_class,
            actor_race,
            strength,
            dexterity,
            intelligence,
            wisdom,
            constitution,
            charisma,
            skill_rank,
            target_level: p.target_level,
        },
    ) {
        Ok(result) => {
            if !loaded_msg.is_empty() {
                format!("{}{}", loaded_msg, result)
            } else {
                result
            }
        }
        Err(e) => format!("Error simulating skill use: {e}"),
    }
}

pub async fn simulate_prayer(
    ctx: &HandlerContext<'_>,
    params: Parameters<SimulatePrayerParams>,
) -> String {
    let p = params.0;
    let (registry, _) = ctx.load();

    let mut player_race = p.player_race.unwrap_or_else(|| "human".to_string());
    let mut player_class = p.player_class.unwrap_or_else(|| "cleric".to_string());
    let mut player_alignment = p.player_alignment.unwrap_or_else(|| "Neutral".to_string());
    let mut cleric_level = p.cleric_level;
    let mut wisdom = p.wisdom.unwrap_or(10);
    let mut loaded_msg = String::new();

    if let Some(ref name) = p.player_name {
        match ctx.fetch_player_state(name).await {
            Ok(player) => {
                player_race = player.race_id;
                player_class = player.class_id.clone();
                player_alignment = player.alignment;
                if player_class.to_lowercase() == "cleric"
                    || player_class.to_lowercase() == "paladin"
                {
                    cleric_level = Some(player.level);
                }
                wisdom = player.attributes.wisdom;
                loaded_msg = format!(
                    "*   **Simulation Actor**: Loaded from Database (`{}` - Level {} {})\n\n",
                    player.name, player.level, player.class_id
                );
            }
            Err(e) => return format!("Error loading player from database: {e}"),
        }
    }

    match simulator::simulate_prayer(
        &registry,
        &p.deity_id,
        &player_race,
        &player_class,
        &player_alignment,
        cleric_level,
        wisdom,
    ) {
        Ok(result) => {
            if !loaded_msg.is_empty() {
                format!("{}{}", loaded_msg, result)
            } else {
                result
            }
        }
        Err(e) => format!("Error simulating prayer: {e}"),
    }
}

pub async fn simulate_prestige_eligibility(
    ctx: &HandlerContext<'_>,
    params: Parameters<SimulatePrestigeParams>,
) -> String {
    let p = params.0;
    let (registry, _) = ctx.load();

    let mut base_classes = p.base_classes.unwrap_or_default();
    let mut skill_ranks = p.skill_ranks.unwrap_or_default();
    let mut completed_quests = p.completed_quests.unwrap_or_default();
    let mut faction_standings = p.faction_standings.unwrap_or_default();
    let mut loaded_msg = String::new();

    if let Some(ref name) = p.player_name {
        match ctx.fetch_player_state(name).await {
            Ok(player) => {
                base_classes.clear();
                base_classes.insert(player.class_id.clone(), player.level);
                skill_ranks = player.skills;
                completed_quests = player.completed_quests;
                faction_standings = player.faction_standings;
                loaded_msg = format!(
                    "*   **Simulation Actor**: Loaded from Database (`{}` - Level {} {})\n\n",
                    player.name, player.level, player.class_id
                );
            }
            Err(e) => return format!("Error loading player from database: {e}"),
        }
    }

    match simulator::simulate_prestige_eligibility(
        &registry,
        &p.prestige_class_id,
        &base_classes,
        &skill_ranks,
        &completed_quests,
        &faction_standings,
    ) {
        Ok(result) => {
            if !loaded_msg.is_empty() {
                format!("{}{}", loaded_msg, result)
            } else {
                result
            }
        }
        Err(e) => format!("Error simulating prestige eligibility: {e}"),
    }
}

pub fn simulate_group_formation(
    _s: &HandlerContext<'_>,
    params: Parameters<SimulateGroupParams>,
) -> String {
    let p = params.0;

    let members: Vec<simulator::MockMember> = p
        .members
        .into_iter()
        .map(|m| simulator::MockMember {
            class_id: m.class_id,
            has_shield: m.has_shield,
            is_front_row: m.is_front_row,
        })
        .collect();

    match simulator::simulate_group_formation(&p.formation, &members) {
        Ok(result) => result,
        Err(e) => format!("Error simulating group formation: {e}"),
    }
}

pub async fn simulate_death_penalty(
    ctx: &HandlerContext<'_>,
    params: Parameters<SimulateDeathParams>,
) -> String {
    let p = params.0;
    let mut current_level = p.current_level.unwrap_or(1);
    let mut current_xp = p.current_xp.unwrap_or(0);
    let mut loaded_msg = String::new();

    if let Some(ref name) = p.player_name {
        match ctx.fetch_player_state(name).await {
            Ok(player) => {
                current_level = player.level;
                current_xp = player.experience;
                loaded_msg = format!(
                    "*   **Simulation Actor**: Loaded from Database (`{}` - Level {} {})\n\n",
                    player.name, player.level, player.class_id
                );
            }
            Err(e) => return format!("Error loading player from database: {e}"),
        }
    }

    match simulator::simulate_death_penalty(current_level, current_xp, p.allow_revive_room) {
        Ok(result) => {
            if !loaded_msg.is_empty() {
                format!("{}{}", loaded_msg, result)
            } else {
                result
            }
        }
        Err(e) => format!("Error simulating death penalty: {e}"),
    }
}

pub async fn simulate_character_creation(
    ctx: &HandlerContext<'_>,
    params: Parameters<SimulateCharacterCreationParams>,
) -> String {
    let p = params.0;

    // 1. Try Online Mode if API is configured
    let payload = serde_json::json!({
        "race_id": p.race_id,
        "class_id": p.class_id,
        "base_attributes": {
            "strength": p.strength,
            "dexterity": p.dexterity,
            "intelligence": p.intelligence,
            "wisdom": p.wisdom,
            "constitution": p.constitution,
            "charisma": p.charisma
        },
        "selected_skills": p.selected_skills
    });

    match ctx
        .authenticated_request_with_body(
            reqwest::Method::POST,
            "/api/character/simulate".to_string(),
            Some(&payload),
        )
        .await
    {
        Ok(resp) => {
            let status = resp.status();
            if status.is_success() {
                match resp.json::<serde_json::Value>().await {
                    Ok(sim_res) => {
                        return format_online_simulation_report(&p.race_id, &p.class_id, &sim_res)
                    }
                    Err(e) => return format!("Failed to parse MUD Server response as JSON: {e}"),
                }
            } else {
                match resp.text().await {
                    Ok(err_text) => return format!("MUD Server validation error: {err_text}"),
                    Err(_) => return format!("MUD Server returned error status: {}", status),
                }
            }
        }
        Err(e) => {
            eprintln!(
                "Warning: Failed to connect to MUD server online simulation: {e}. Falling back to offline simulation."
            );
        }
    }

    // 2. Offline Fallback Mode
    let (registry, _) = ctx.load();
    match simulator::simulate_character_creation(
        &registry,
        &crate::simulator::SimulateCharacterCreationParams {
            race_id: p.race_id,
            class_id: p.class_id,
            strength: p.strength,
            dexterity: p.dexterity,
            intelligence: p.intelligence,
            wisdom: p.wisdom,
            constitution: p.constitution,
            charisma: p.charisma,
            selected_skills: p.selected_skills,
        },
    ) {
        Ok(result) => result,
        Err(e) => format!("Error simulating character creation: {e}"),
    }
}
#[allow(dead_code)]
pub(crate) fn format_online_simulation_report(
    race_id: &str,
    class_id: &str,
    sim: &serde_json::Value,
) -> String {
    let mut out = format!(
        "### Character Creation Simulation (Online Mode): Race = `{}`, Class = `{}`\n\n",
        race_id, class_id
    );

    if let Some(attrs) = sim.get("attributes") {
        out.push_str("#### Final Attributes:\n");
        out.push_str(&format!(
            "*   Str: {}\n",
            attrs.get("strength").and_then(|v| v.as_i64()).unwrap_or(10)
        ));
        out.push_str(&format!(
            "*   Dex: {}\n",
            attrs
                .get("dexterity")
                .and_then(|v| v.as_i64())
                .unwrap_or(10)
        ));
        out.push_str(&format!(
            "*   Int: {}\n",
            attrs
                .get("intelligence")
                .and_then(|v| v.as_i64())
                .unwrap_or(10)
        ));
        out.push_str(&format!(
            "*   Wis: {}\n",
            attrs.get("wisdom").and_then(|v| v.as_i64()).unwrap_or(10)
        ));
        out.push_str(&format!(
            "*   Con: {}\n",
            attrs
                .get("constitution")
                .and_then(|v| v.as_i64())
                .unwrap_or(10)
        ));
        out.push_str(&format!(
            "*   Cha: {}\n\n",
            attrs.get("charisma").and_then(|v| v.as_i64()).unwrap_or(10)
        ));
    }

    out.push_str("#### Derived Resources:\n");
    out.push_str(&format!(
        "*   **Hit Points (HP)**: {}\n",
        sim.get("hp").and_then(|v| v.as_i64()).unwrap_or(1)
    ));
    out.push_str(&format!(
        "*   **Mana**: {}\n",
        sim.get("mana").and_then(|v| v.as_i64()).unwrap_or(0)
    ));
    out.push_str(&format!(
        "*   **Stamina**: {}\n\n",
        sim.get("stamina").and_then(|v| v.as_i64()).unwrap_or(0)
    ));

    if let Some(gold) = sim.get("starting_gold") {
        out.push_str("#### Starting Gold:\n");
        out.push_str(&format!(
            "*   Copper: {}, Silver: {}, Gold: {}, Platinum: {}\n\n",
            gold.get("copper").and_then(|v| v.as_i64()).unwrap_or(0),
            gold.get("silver").and_then(|v| v.as_i64()).unwrap_or(0),
            gold.get("gold").and_then(|v| v.as_i64()).unwrap_or(0),
            gold.get("platinum").and_then(|v| v.as_i64()).unwrap_or(0)
        ));
    }

    if let Some(skills) = sim.get("auto_skills").and_then(|v| v.as_array()) {
        out.push_str("#### Auto-Granted Skills:\n");
        if skills.is_empty() {
            out.push_str("*   *(None)*\n");
        } else {
            for s in skills {
                if let Some(s_str) = s.as_str() {
                    out.push_str(&format!("*   `{}`\n", s_str));
                }
            }
        }
    }

    out
}
