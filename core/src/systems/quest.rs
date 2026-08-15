use crate::{
    components::{
        Experience, Inventory, Item, Level, Name, ObjectiveProgress, QuestLog, QuestProgress,
        Wallet,
    },
    templates::{QuestObjective, TemplateRegistry},
    Dirty, Entity, World,
};
use std::collections::{HashMap, HashSet};

/// Reconciles all gather objectives for a player by scanning their inventory/equipment.
pub fn reconcile_gather_objectives(
    world: &mut World,
    player: Entity,
    templates: &TemplateRegistry,
) -> Vec<String> {
    let mut messages = Vec::new();

    // 1. Get the current inventory and equipment item counts
    let mut item_counts: HashMap<String, u32> = HashMap::new();

    if let Ok(mut q_inv) = world.query_one::<&Inventory>(player) {
        if let Some(inv) = q_inv.get() {
            for item_entity in &inv.0 {
                if let Ok(mut q_item) = world.query_one::<&Item>(*item_entity) {
                    if let Some(item_comp) = q_item.get() {
                        *item_counts
                            .entry(item_comp.template_id.clone())
                            .or_insert(0) += 1;
                    }
                }
            }
        }
    }

    if let Ok(mut q_eq) = world.query_one::<&crate::Equipment>(player) {
        if let Some(eq) = q_eq.get() {
            for (_slot, item_entity) in &eq.slots {
                if let Ok(mut q_item) = world.query_one::<&Item>(*item_entity) {
                    if let Some(item_comp) = q_item.get() {
                        *item_counts
                            .entry(item_comp.template_id.clone())
                            .or_insert(0) += 1;
                    }
                }
            }
        }
    }

    // 2. Query QuestLog
    let mut q_log = match world.query_one::<&mut QuestLog>(player) {
        Ok(q) => q,
        Err(_) => return messages,
    };
    let quest_log = match q_log.get() {
        Some(log) => log,
        None => return messages,
    };

    let mut dirty = false;

    for (quest_id, progress) in &mut quest_log.active {
        let Some(quest_def) = templates.quests.get(quest_id) else {
            continue;
        };

        let mut quest_updated = false;

        for (objective, obj_progress) in quest_def.objectives.iter().zip(&mut progress.objectives) {
            if let QuestObjective::Gather { item, count } = objective {
                let current_held = *item_counts.get(item).unwrap_or(&0);
                let target_val = current_held.min(*count);

                if obj_progress.current != target_val {
                    obj_progress.current = target_val;
                    let item_name = templates
                        .items
                        .get(item)
                        .map(|i| i.name.as_str())
                        .unwrap_or(item);
                    messages.push(format!(
                        "Quest '{}' progress: Gather {} {}/{}",
                        quest_def.name, item_name, obj_progress.current, count
                    ));

                    let was_completed = obj_progress.completed;
                    obj_progress.completed = obj_progress.current >= *count;

                    if obj_progress.completed && !was_completed {
                        messages.push(format!(
                            "Objective 'Gather {}' completed for quest '{}'!",
                            item_name, quest_def.name
                        ));
                    }

                    quest_updated = true;
                    dirty = true;
                }
            }
        }

        if quest_updated {
            let all_done = progress.objectives.iter().all(|o| o.completed);
            if all_done && !quest_def.auto_complete {
                messages.push(format!(
                    "Quest '{}' is ready to turn in! Return to {} to claim your reward.",
                    quest_def.name,
                    quest_def
                        .turn_in_npc
                        .as_ref()
                        .and_then(|npc| templates.mobs.get(npc))
                        .map(|m| m.name.as_str())
                        .unwrap_or("the quest giver")
                ));
            }
        }
    }

    if dirty {
        drop(q_log);
        let _ = world.insert(player, (Dirty,));
    }

    messages
}

/// Evaluates kill events for the player, advancing matching kill objectives.
pub fn handle_kill_event(
    world: &mut World,
    player: Entity,
    mob_template_id: &str,
    templates: &TemplateRegistry,
) -> Vec<String> {
    let mut messages = Vec::new();
    let mut q_log = match world.query_one::<&mut QuestLog>(player) {
        Ok(q) => q,
        Err(_) => return messages,
    };
    let quest_log = match q_log.get() {
        Some(log) => log,
        None => return messages,
    };

    let mut dirty = false;
    let mut updated_quests = Vec::new();

    for (quest_id, progress) in &mut quest_log.active {
        let Some(quest_def) = templates.quests.get(quest_id) else {
            continue;
        };

        let mut quest_updated = false;

        for (objective, obj_progress) in quest_def.objectives.iter().zip(&mut progress.objectives) {
            if obj_progress.completed {
                continue;
            }

            if let QuestObjective::Kill { mob, count } = objective {
                if mob == mob_template_id {
                    obj_progress.current = (obj_progress.current + 1).min(*count);
                    dirty = true;
                    quest_updated = true;

                    let mob_name = templates
                        .mobs
                        .get(mob)
                        .map(|m| m.name.as_str())
                        .unwrap_or(mob);
                    messages.push(format!(
                        "Quest '{}' progress: Kill {} {}/{}",
                        quest_def.name, mob_name, obj_progress.current, count
                    ));

                    if obj_progress.current >= *count {
                        obj_progress.completed = true;
                        messages.push(format!(
                            "Objective 'Kill {}' completed for quest '{}'!",
                            mob_name, quest_def.name
                        ));
                    }
                }
            }
        }

        if quest_updated {
            updated_quests.push((quest_id.to_string(), quest_def.auto_complete));
            let all_done = progress.objectives.iter().all(|o| o.completed);
            if all_done && !quest_def.auto_complete {
                messages.push(format!(
                    "Quest '{}' is ready to turn in! Return to {} to claim your reward.",
                    quest_def.name,
                    quest_def
                        .turn_in_npc
                        .as_ref()
                        .and_then(|npc| templates.mobs.get(npc))
                        .map(|m| m.name.as_str())
                        .unwrap_or("the quest giver")
                ));
            }
        }
    }

    drop(q_log);
    if dirty {
        let _ = world.insert(player, (Dirty,));
    }

    process_quest_updates(world, player, updated_quests, templates, &mut messages);

    messages
}

/// Evaluates explore events, completing exploration objectives.
pub fn handle_explore_event(
    world: &mut World,
    player: Entity,
    room_key: &str,
    templates: &TemplateRegistry,
) -> Vec<String> {
    let mut messages = Vec::new();
    let mut q_log = match world.query_one::<&mut QuestLog>(player) {
        Ok(q) => q,
        Err(_) => return messages,
    };
    let quest_log = match q_log.get() {
        Some(log) => log,
        None => return messages,
    };

    let mut dirty = false;
    let mut updated_quests = Vec::new();

    for (quest_id, progress) in &mut quest_log.active {
        let Some(quest_def) = templates.quests.get(quest_id) else {
            continue;
        };

        let mut quest_updated = false;

        for (objective, obj_progress) in quest_def.objectives.iter().zip(&mut progress.objectives) {
            if obj_progress.completed {
                continue;
            }

            if let QuestObjective::Explore { room } = objective {
                if room == room_key {
                    obj_progress.current = 1;
                    obj_progress.completed = true;
                    messages.push(format!(
                        "Objective 'Explore {}' completed for quest '{}'!",
                        room, quest_def.name
                    ));
                    quest_updated = true;
                    dirty = true;
                }
            }
        }

        if quest_updated {
            updated_quests.push((quest_id.to_string(), quest_def.auto_complete));
            let all_done = progress.objectives.iter().all(|o| o.completed);
            if all_done && !quest_def.auto_complete {
                messages.push(format!(
                    "Quest '{}' is ready to turn in! Return to {} to claim your reward.",
                    quest_def.name,
                    quest_def
                        .turn_in_npc
                        .as_ref()
                        .and_then(|npc| templates.mobs.get(npc))
                        .map(|m| m.name.as_str())
                        .unwrap_or("the quest giver")
                ));
            }
        }
    }

    drop(q_log);
    if dirty {
        let _ = world.insert(player, (Dirty,));
    }

    process_quest_updates(world, player, updated_quests, templates, &mut messages);

    messages
}

/// Evaluates talk and deliver events when talking to an NPC.
pub fn handle_talk_event(
    world: &mut World,
    player: Entity,
    npc_template_id: &str,
    templates: &TemplateRegistry,
) -> Vec<String> {
    let mut messages = Vec::new();

    // Collect item templates currently in inventory
    let mut inventory_items = HashSet::new();
    if let Ok(mut q_inv) = world.query_one::<&Inventory>(player) {
        if let Some(inv) = q_inv.get() {
            for item_entity in &inv.0 {
                if let Ok(mut q_item) = world.query_one::<&Item>(*item_entity) {
                    if let Some(item_comp) = q_item.get() {
                        inventory_items.insert(item_comp.template_id.clone());
                    }
                }
            }
        }
    }

    let mut q_log = match world.query_one::<&mut QuestLog>(player) {
        Ok(q) => q,
        Err(_) => return messages,
    };
    let quest_log = match q_log.get() {
        Some(log) => log,
        None => return messages,
    };

    let mut dirty = false;
    let mut updated_quests = Vec::new();

    for (quest_id, progress) in &mut quest_log.active {
        let Some(quest_def) = templates.quests.get(quest_id) else {
            continue;
        };

        let mut quest_updated = false;

        for (objective, obj_progress) in quest_def.objectives.iter().zip(&mut progress.objectives) {
            if obj_progress.completed {
                continue;
            }

            match objective {
                QuestObjective::Talk { npc } if npc == npc_template_id => {
                    obj_progress.current = 1;
                    obj_progress.completed = true;
                    let npc_name = templates
                        .mobs
                        .get(npc)
                        .map(|m| m.name.as_str())
                        .unwrap_or(npc);
                    messages.push(format!(
                        "Objective 'Talk to {}' completed for quest '{}'!",
                        npc_name, quest_def.name
                    ));
                    quest_updated = true;
                    dirty = true;
                }
                QuestObjective::Deliver { item, npc }
                    if npc == npc_template_id && inventory_items.contains(item) =>
                {
                    obj_progress.current = 1;
                    obj_progress.completed = true;
                    let item_name = templates
                        .items
                        .get(item)
                        .map(|i| i.name.as_str())
                        .unwrap_or(item);
                    let npc_name = templates
                        .mobs
                        .get(npc)
                        .map(|m| m.name.as_str())
                        .unwrap_or(npc);
                    messages.push(format!(
                        "Objective 'Deliver {} to {}' completed for quest '{}'!",
                        item_name, npc_name, quest_def.name
                    ));
                    quest_updated = true;
                    dirty = true;
                }
                _ => {}
            }
        }

        if quest_updated {
            updated_quests.push((quest_id.to_string(), quest_def.auto_complete));
            let all_done = progress.objectives.iter().all(|o| o.completed);
            if all_done && !quest_def.auto_complete {
                messages.push(format!(
                    "Quest '{}' is ready to turn in! Return to {} to claim your reward.",
                    quest_def.name,
                    quest_def
                        .turn_in_npc
                        .as_ref()
                        .and_then(|npc| templates.mobs.get(npc))
                        .map(|m| m.name.as_str())
                        .unwrap_or("the quest giver")
                ));
            }
        }
    }

    drop(q_log);
    if dirty {
        let _ = world.insert(player, (Dirty,));
    }

    process_quest_updates(world, player, updated_quests, templates, &mut messages);

    messages
}

/// Accepts a quest, checking prerequisites, level, and active status.
pub fn accept_quest(
    world: &mut World,
    player: Entity,
    quest_id: &str,
    templates: &TemplateRegistry,
) -> Result<Vec<String>, String> {
    let quest_def = templates
        .quests
        .get(quest_id)
        .ok_or_else(|| format!("Quest '{}' does not exist.", quest_id))?;

    let mut q_log = world
        .query_one::<&mut QuestLog>(player)
        .map_err(|_| "You have no quest log.".to_string())?;
    let quest_log = q_log
        .get()
        .ok_or_else(|| "You have no quest log.".to_string())?;

    if quest_log.completed.contains(quest_id) && !quest_def.repeatable {
        return Err("You have already completed this quest.".to_string());
    }

    if quest_log.active.contains_key(quest_id) {
        return Err("You are already on this quest.".to_string());
    }

    // Check level gate
    if let Ok(mut q_level) = world.query_one::<&Level>(player) {
        if let Some(level) = q_level.get() {
            if level.0 < quest_def.level_requirement {
                return Err(format!(
                    "You are not high enough level to start this quest (requires level {}).",
                    quest_def.level_requirement
                ));
            }
        }
    }

    // Check prerequisites
    for prereq in &quest_def.prerequisites {
        if !quest_log.completed.contains(prereq) {
            return Err("You have not completed the prerequisites for this quest.".to_string());
        }
    }

    // Accept quest
    let progress = QuestProgress {
        quest_id: quest_id.to_string(),
        started_at_epoch_ms: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64,
        objectives: quest_def
            .objectives
            .iter()
            .map(|_| ObjectiveProgress {
                current: 0,
                completed: false,
            })
            .collect(),
    };

    quest_log.active.insert(quest_id.to_string(), progress);
    drop(q_log);

    let _ = world.insert(player, (Dirty,));

    let mut messages = vec![format!("You accept the quest: {}.", quest_def.name)];

    // Reconcile gather objectives immediately in case player already has the items
    let reconcile_msgs = reconcile_gather_objectives(world, player, templates);
    messages.extend(reconcile_msgs);

    // If auto_complete is true, check completion immediately
    let auto_done = {
        let mut q_log = world.query_one::<&QuestLog>(player).unwrap();
        let log = q_log.get().unwrap();
        log.active
            .get(quest_id)
            .map(|p| p.objectives.iter().all(|o| o.completed))
            .unwrap_or(false)
    };

    if auto_done && quest_def.auto_complete {
        if let Ok(complete_msgs) = complete_quest(world, player, quest_id, templates) {
            messages.extend(complete_msgs);
        }
    }

    if let Some(scripts) = &quest_def.scripts {
        run_quest_script(world, player, quest_id, scripts.on_accept.as_ref());
    }

    Ok(messages)
}

/// Completes a quest, removing it from active, paying out rewards, and adding to completed.
pub fn complete_quest(
    world: &mut World,
    player: Entity,
    quest_id: &str,
    templates: &TemplateRegistry,
) -> Result<Vec<String>, String> {
    let quest_def = templates
        .quests
        .get(quest_id)
        .ok_or_else(|| format!("Quest '{}' does not exist.", quest_id))?;

    // Verify it is active
    let is_active = {
        let mut q_log = world
            .query_one::<&QuestLog>(player)
            .map_err(|_| "You have no quest log.".to_string())?;
        let quest_log = q_log
            .get()
            .ok_or_else(|| "You have no quest log.".to_string())?;
        quest_log.active.contains_key(quest_id)
    };

    if !is_active {
        return Err("You are not on that quest.".to_string());
    }

    // Recalculate gather counts to ensure accuracy
    let _ = reconcile_gather_objectives(world, player, templates);

    // Verify all objectives are complete
    {
        let mut q_log = world.query_one::<&QuestLog>(player).unwrap();
        let quest_log = q_log.get().unwrap();
        let progress = quest_log.active.get(quest_id).unwrap();
        if !progress.objectives.iter().all(|o| o.completed) {
            return Err("You have not completed all objectives for this quest.".to_string());
        }
    }

    // Consume deliverables (Gather and Deliver items are turned in)
    for objective in &quest_def.objectives {
        match objective {
            QuestObjective::Gather { item, count } => {
                consume_items(world, player, item, *count);
            }
            QuestObjective::Deliver { item, .. } => {
                consume_items(world, player, item, 1);
            }
            _ => {}
        }
    }

    // Move from active to completed
    {
        let mut q_log = world.query_one::<&mut QuestLog>(player).unwrap();
        let quest_log = q_log.get().unwrap();
        quest_log.active.remove(quest_id);
        quest_log.completed.insert(quest_id.to_string());
    }

    let mut messages = vec![format!("You have completed the quest: {}!", quest_def.name)];

    // 1. Pay out XP
    if quest_def.rewards.xp > 0 {
        let mut final_xp = quest_def.rewards.xp;
        let penalty_mult = world
            .query_one::<&crate::components::MultiClassInfo>(player)
            .ok()
            .and_then(|mut q| q.get().map(|mc| mc.xp_penalty_multiplier()))
            .unwrap_or(1.0);
        final_xp = ((final_xp as f32) * penalty_mult).round() as u64;

        if let Ok(mut q_xp) = world.query_one::<&mut Experience>(player) {
            if let Some(xp) = q_xp.get() {
                xp.0 = xp.0.saturating_add(final_xp);
                messages.push(format!("You gain {} experience.", final_xp));
            }
        }
    }

    // 2. Pay out Gold (copper)
    if quest_def.rewards.gold > 0 {
        if let Ok(mut q_wallet) = world.query_one::<&mut Wallet>(player) {
            if let Some(wallet) = q_wallet.get() {
                wallet.copper = wallet.copper.saturating_add(quest_def.rewards.gold);
                messages.push(format!("You gain {} copper.", quest_def.rewards.gold));
            }
        }
    }

    // 3. Pay out Item rewards
    for item_reward in &quest_def.rewards.items {
        for _ in 0..item_reward.count {
            if let Some(item_entity) =
                spawn_item_by_id(world, &item_reward.item_template_id, templates)
            {
                if let Ok(mut q_inv) = world.query_one::<&mut Inventory>(player) {
                    if let Some(inv) = q_inv.get() {
                        inv.0.push(item_entity);
                        let item_name = templates
                            .items
                            .get(&item_reward.item_template_id)
                            .map(|i| i.name.as_str())
                            .unwrap_or(&item_reward.item_template_id);
                        messages.push(format!("You receive reward item: {}.", item_name));
                    }
                }
            }
        }
    }

    // 4. Pay out Faction Standing rewards
    for faction_reward in &quest_def.rewards.faction {
        let faction_msgs = crate::systems::faction::adjust_faction_standing(
            world,
            player,
            &faction_reward.faction_id,
            faction_reward.amount,
            templates,
        );
        messages.extend(faction_msgs);
    }

    if let Some(scripts) = &quest_def.scripts {
        run_quest_script(world, player, quest_id, scripts.on_complete.as_ref());
    }

    let _ = world.insert(player, (Dirty,));

    Ok(messages)
}

/// Abandons an active quest.
pub fn abandon_quest(
    world: &mut World,
    player: Entity,
    quest_id: &str,
) -> Result<Vec<String>, String> {
    let mut q_log = world
        .query_one::<&mut QuestLog>(player)
        .map_err(|_| "You have no quest log.".to_string())?;
    let quest_log = q_log
        .get()
        .ok_or_else(|| "You have no quest log.".to_string())?;

    if quest_log.active.remove(quest_id).is_none() {
        return Err("You are not on that quest.".to_string());
    }
    drop(q_log);

    let _ = world.insert(player, (Dirty,));
    Ok(vec![format!("Quest '{}' abandoned.", quest_id)])
}

/// Consumes `count` items with the given `template_id` from the player's inventory.
fn consume_items(world: &mut World, player: Entity, item_template_id: &str, count: u32) {
    let mut to_despawn = Vec::new();

    if let Ok(mut q_inv) = world.query_one::<&mut Inventory>(player) {
        if let Some(inv) = q_inv.get() {
            let mut remaining = count;
            inv.0.retain(|item_entity| {
                if remaining == 0 {
                    return true;
                }
                if let Ok(mut q_item) = world.query_one::<&Item>(*item_entity) {
                    if let Some(item_comp) = q_item.get() {
                        if item_comp.template_id == item_template_id {
                            to_despawn.push(*item_entity);
                            remaining -= 1;
                            return false;
                        }
                    }
                }
                true
            });
        }
    }

    for item_entity in to_despawn {
        let _ = world.despawn(item_entity);
    }
}

/// Spawns a basic item based on its template, setting up basic components.
fn spawn_item_by_id(
    world: &mut World,
    template_id: &str,
    templates: &TemplateRegistry,
) -> Option<Entity> {
    let item_tmpl = templates.items.get(template_id)?;
    let item_entity = world.spawn((
        Item::new(template_id),
        Name::new(&item_tmpl.name),
        crate::ScriptParams(item_tmpl.params.clone()),
    ));

    if let Some(wpn) = &item_tmpl.weapon {
        use crate::components::{Weapon, WeaponHands};
        use crate::dice::DiceRoll;
        use crate::WeaponRange;
        use std::str::FromStr;
        if let Ok(dice) = DiceRoll::from_str(wpn.damage.as_str()) {
            let dmg_type = wpn
                .damage_type
                .parse()
                .unwrap_or(crate::DamageType::Bludgeon);
            let range = match wpn.range.to_lowercase().as_str() {
                "ranged" => WeaponRange::Ranged,
                "reach" => WeaponRange::Reach,
                "thrown" => WeaponRange::Thrown,
                _ => WeaponRange::Melee,
            };
            let hands = match wpn.hands.to_lowercase().as_str() {
                "twohand" | "twohanded" | "two_hand" | "two_handed" => WeaponHands::TwoHand,
                _ => WeaponHands::OneHand,
            };
            let _ = world.insert(
                item_entity,
                (Weapon {
                    damage_dice: dice,
                    damage_type: dmg_type,
                    speed: wpn.speed,
                    range,
                    hands,
                },),
            );
        }
    }

    if let Some(set) = &item_tmpl.set {
        let _ = world.insert(
            item_entity,
            (crate::components::SetMembership::from(set.clone()),),
        );
    }

    if !item_tmpl.triggers.is_empty() {
        let _ = world.insert(
            item_entity,
            (crate::ItemTriggers(item_tmpl.triggers.clone()),),
        );
    }

    if let Some(req) = &item_tmpl.requires_skill {
        let _ = world.insert(
            item_entity,
            (crate::components::ItemSkillRequirement {
                id: req.id.clone(),
                level: req.level,
            },),
        );
    }

    Some(item_entity)
}

fn process_quest_updates(
    world: &mut World,
    player: Entity,
    updated_quests: Vec<(String, bool)>,
    templates: &TemplateRegistry,
    messages: &mut Vec<String>,
) {
    for (quest_id, auto_complete) in updated_quests {
        // Run scripting hook
        if let Some(quest_def) = templates.quests.get(&quest_id) {
            if let Some(scripts) = &quest_def.scripts {
                run_quest_script(world, player, &quest_id, scripts.on_update.as_ref());
            }
        }

        // Auto complete if appropriate
        if auto_complete {
            let should_auto_complete = {
                if let Ok(mut q_log) = world.query_one::<&QuestLog>(player) {
                    if let Some(log) = q_log.get() {
                        log.active
                            .get(&quest_id)
                            .map(|p| p.objectives.iter().all(|o| o.completed))
                            .unwrap_or(false)
                    } else {
                        false
                    }
                } else {
                    false
                }
            };

            if should_auto_complete {
                if let Ok(complete_msgs) = complete_quest(world, player, &quest_id, templates) {
                    messages.extend(complete_msgs);
                }
            }
        }
    }
}

fn run_quest_script(
    world: &mut World,
    player: Entity,
    quest_id: &str,
    script_path: Option<&String>,
) {
    if let Some(path) = script_path {
        if let Some(bridge) = crate::scripting::get_scripting_bridge() {
            if let Err(e) = bridge.execute_quest_hook(path, player, quest_id, world) {
                tracing::error!("Error executing quest script '{}': {}", path, e);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::{Experience, Level, Wallet};
    use crate::templates::{
        ItemTemplate, MobTemplate, QuestDef, QuestObjective, QuestRewardFaction, QuestRewardItem,
        QuestRewards,
    };

    fn make_templates() -> TemplateRegistry {
        let mut t = TemplateRegistry::new();

        // 1. Giver/Turn-in Mob
        let giver: MobTemplate = toml::from_str(r#"
            id = "quest_giver"
            name = "Quest Giver"
            description = "A quest giver."
            short_desc = "A quest giver."
            level = 1
            health = { current = 10, max = 10 }
            attributes = { strength = 10, dexterity = 10, intelligence = 10, wisdom = 10, constitution = 10, charisma = 10 }
        "#).unwrap();
        t.mobs.insert("quest_giver".to_string(), giver);

        // 2. Kill Mob Target
        let wolf: MobTemplate = toml::from_str(r#"
            id = "wolf"
            name = "Wolf"
            description = "A wild wolf."
            short_desc = "A wild wolf."
            level = 1
            health = { current = 5, max = 5 }
            attributes = { strength = 8, dexterity = 12, intelligence = 2, wisdom = 10, constitution = 8, charisma = 3 }
            ai_mode = "wander"
        "#).unwrap();
        t.mobs.insert("wolf".to_string(), wolf);

        // 3. Gather Item Target & Reward Item
        t.items.insert(
            "wolf_pelt".to_string(),
            ItemTemplate {
                id: "wolf_pelt".to_string(),
                name: "Wolf Pelt".to_string(),
                description: "Soft fur pelt.".to_string(),
                item_type: "trash".to_string(),
                subtype: String::new(),
                rarity: "common".to_string(),
                quality: "standard".to_string(),
                level_requirement: 0,
                weight: 1.0,
                value: 5,
                flags: Vec::new(),
                allowed_classes: Vec::new(),
                allowed_races: Vec::new(),
                allowed_alignments: Vec::new(),
                requires_skill: None,
                weapon: None,
                equipment: None,
                set: None,
                consumable: None,
                container: None,
                durability: None,
                triggers: Vec::new(),
                params: HashMap::new(),
            },
        );

        t.items.insert(
            "iron_ring".to_string(),
            ItemTemplate {
                id: "iron_ring".to_string(),
                name: "Iron Ring".to_string(),
                description: "Simple iron ring.".to_string(),
                item_type: "armor".to_string(),
                subtype: "ring".to_string(),
                rarity: "common".to_string(),
                quality: "standard".to_string(),
                level_requirement: 0,
                weight: 0.1,
                value: 10,
                flags: Vec::new(),
                allowed_classes: Vec::new(),
                allowed_races: Vec::new(),
                allowed_alignments: Vec::new(),
                requires_skill: None,
                weapon: None,
                equipment: Some(crate::templates::EquipmentDef {
                    slot: "finger".to_string(),
                }),
                set: None,
                consumable: None,
                container: None,
                durability: None,
                triggers: Vec::new(),
                params: HashMap::new(),
            },
        );

        // 4. Quest Definition
        t.quests.insert(
            "wolf_hunt".to_string(),
            QuestDef {
                id: "wolf_hunt".to_string(),
                name: "Wolf Hunt".to_string(),
                description: "Hunt down wolves and collect their pelts.".to_string(),
                level_requirement: 1,
                repeatable: false,
                auto_complete: false,
                giver_npc: Some("quest_giver".to_string()),
                turn_in_npc: Some("quest_giver".to_string()),
                prerequisites: Vec::new(),
                objectives: vec![
                    QuestObjective::Kill {
                        mob: "wolf".to_string(),
                        count: 3,
                    },
                    QuestObjective::Gather {
                        item: "wolf_pelt".to_string(),
                        count: 2,
                    },
                ],
                rewards: QuestRewards {
                    xp: 100,
                    gold: 50,
                    items: vec![QuestRewardItem {
                        item_template_id: "iron_ring".to_string(),
                        count: 1,
                    }],
                    faction: Vec::new(),
                },
                scripts: None,
                params: HashMap::new(),
            },
        );

        t.quests.insert(
            "faction_quest".to_string(),
            QuestDef {
                id: "faction_quest".to_string(),
                name: "Faction Quest".to_string(),
                description: "Increase standing with guards.".to_string(),
                level_requirement: 1,
                repeatable: false,
                auto_complete: false,
                giver_npc: None,
                turn_in_npc: None,
                prerequisites: Vec::new(),
                objectives: Vec::new(),
                rewards: QuestRewards {
                    xp: 0,
                    gold: 0,
                    items: Vec::new(),
                    faction: vec![QuestRewardFaction {
                        faction_id: "town_guard".to_string(),
                        amount: 15,
                    }],
                },
                scripts: None,
                params: HashMap::new(),
            },
        );

        t.factions.insert(
            "town_guard".to_string(),
            crate::templates::FactionDef {
                id: "town_guard".to_string(),
                name: "Town Guard".to_string(),
                description: "The protectors of the town.".to_string(),
                starting_standing: 0,
                min_standing: -1000,
                max_standing: 1000,
                ranks: vec![
                    crate::templates::FactionRank {
                        name: "Neutral".to_string(),
                        threshold: -100,
                    },
                    crate::templates::FactionRank {
                        name: "Friendly".to_string(),
                        threshold: 100,
                    },
                ],
                relationships: HashMap::new(),
                aggro_below: -500,
            },
        );

        t
    }

    #[test]
    fn test_accept_quest() {
        let mut world = World::new();
        let templates = make_templates();
        let player = world.spawn((
            QuestLog::new(),
            Level(1),
            Experience(0),
            Wallet::new(0, 0, 0, 0),
            Inventory::new(),
            crate::Equipment::new(),
        ));

        // Accept quest successfully
        let msgs = accept_quest(&mut world, player, "wolf_hunt", &templates).unwrap();
        assert!(msgs.iter().any(|m| m.contains("You accept the quest")));

        // Verify it is active
        let mut q_log = world.query_one::<&QuestLog>(player).unwrap();
        let log = q_log.get().unwrap();
        assert!(log.active.contains_key("wolf_hunt"));
    }

    #[test]
    fn test_quest_level_gate() {
        let mut world = World::new();
        let mut templates = make_templates();

        // Increase level requirement to 5
        if let Some(q) = templates.quests.get_mut("wolf_hunt") {
            q.level_requirement = 5;
        }

        let player = world.spawn((
            QuestLog::new(),
            Level(1),
            Experience(0),
            Wallet::new(0, 0, 0, 0),
            Inventory::new(),
            crate::Equipment::new(),
        ));

        let res = accept_quest(&mut world, player, "wolf_hunt", &templates);
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("not high enough level"));
    }

    #[test]
    fn test_kill_objective_progression() {
        let mut world = World::new();
        let templates = make_templates();
        let player = world.spawn((
            QuestLog::new(),
            Level(1),
            Experience(0),
            Wallet::new(0, 0, 0, 0),
            Inventory::new(),
            crate::Equipment::new(),
        ));

        accept_quest(&mut world, player, "wolf_hunt", &templates).unwrap();

        // 1st kill
        let msgs = handle_kill_event(&mut world, player, "wolf", &templates);
        assert!(msgs
            .iter()
            .any(|m| m.contains("Quest 'Wolf Hunt' progress: Kill Wolf 1/3")));

        // 2nd kill
        handle_kill_event(&mut world, player, "wolf", &templates);

        // 3rd kill - should complete objective
        let msgs = handle_kill_event(&mut world, player, "wolf", &templates);
        assert!(msgs
            .iter()
            .any(|m| m.contains("Objective 'Kill Wolf' completed")));
    }

    #[test]
    fn test_gather_objective_progression() {
        let mut world = World::new();
        let templates = make_templates();

        // Spawn pelts
        let pelt1 = world.spawn((
            Item::new("wolf_pelt"),
            Name::new("Wolf Pelt"),
            crate::ScriptParams::default(),
        ));
        let pelt2 = world.spawn((
            Item::new("wolf_pelt"),
            Name::new("Wolf Pelt"),
            crate::ScriptParams::default(),
        ));

        let player = world.spawn((
            QuestLog::new(),
            Level(1),
            Experience(0),
            Wallet::new(0, 0, 0, 0),
            Inventory(vec![pelt1]),
            crate::Equipment::new(),
        ));

        // Accept quest -> gather should reconcile immediately to 1/2
        let msgs = accept_quest(&mut world, player, "wolf_hunt", &templates).unwrap();
        assert!(msgs.iter().any(|m| m.contains("Gather Wolf Pelt 1/2")));

        // Pick up 2nd pelt -> reconcile to 2/2 and complete gather objective
        if let Ok(mut q_inv) = world.query_one::<&mut Inventory>(player) {
            if let Some(inv) = q_inv.get() {
                inv.0.push(pelt2);
            }
        }
        let msgs = reconcile_gather_objectives(&mut world, player, &templates);
        assert!(msgs.iter().any(|m| m.contains("Gather Wolf Pelt 2/2")));
        assert!(msgs
            .iter()
            .any(|m| m.contains("Objective 'Gather Wolf Pelt' completed")));
    }

    #[test]
    fn test_quest_turn_in_rewards() {
        let mut world = World::new();
        let templates = make_templates();

        let pelt1 = world.spawn((
            Item::new("wolf_pelt"),
            Name::new("Wolf Pelt"),
            crate::ScriptParams::default(),
        ));
        let pelt2 = world.spawn((
            Item::new("wolf_pelt"),
            Name::new("Wolf Pelt"),
            crate::ScriptParams::default(),
        ));

        let player = world.spawn((
            QuestLog::new(),
            Level(1),
            Experience(0),
            Wallet::new(0, 0, 0, 0),
            Inventory(vec![pelt1, pelt2]),
            crate::Equipment::new(),
        ));

        accept_quest(&mut world, player, "wolf_hunt", &templates).unwrap();

        // complete kill objectives
        handle_kill_event(&mut world, player, "wolf", &templates);
        handle_kill_event(&mut world, player, "wolf", &templates);
        handle_kill_event(&mut world, player, "wolf", &templates);

        // Turn in quest
        let msgs = complete_quest(&mut world, player, "wolf_hunt", &templates).unwrap();
        assert!(msgs
            .iter()
            .any(|m| m.contains("completed the quest: Wolf Hunt")));
        assert!(msgs.iter().any(|m| m.contains("gain 100 experience")));
        assert!(msgs.iter().any(|m| m.contains("gain 50 copper")));
        assert!(msgs
            .iter()
            .any(|m| m.contains("receive reward item: Iron Ring")));

        // Verify items were consumed from inventory
        let mut q_inv = world.query_one::<&Inventory>(player).unwrap();
        let inv = q_inv.get().unwrap();
        assert_eq!(inv.0.len(), 1);
        let rewarded_item = inv.0[0];
        let mut q_item = world.query_one::<&Item>(rewarded_item).unwrap();
        assert_eq!(q_item.get().unwrap().template_id, "iron_ring");

        // Verify experience and wallet updated
        let xp = world
            .query_one::<&Experience>(player)
            .unwrap()
            .get()
            .unwrap()
            .0;
        assert_eq!(xp, 100);
        let gold = world
            .query_one::<&Wallet>(player)
            .unwrap()
            .get()
            .unwrap()
            .copper;
        assert_eq!(gold, 50);

        // Verify quest completed in quest log
        let mut q_log = world.query_one::<&QuestLog>(player).unwrap();
        let log = q_log.get().unwrap();
        assert!(log.completed.contains("wolf_hunt"));
        assert!(!log.active.contains_key("wolf_hunt"));
    }

    #[test]
    fn test_quest_faction_standing_rewards() {
        let mut world = World::new();
        let templates = make_templates();

        let player = world.spawn((
            QuestLog::new(),
            Level(1),
            Experience(0),
            Wallet::new(0, 0, 0, 0),
            Inventory(Vec::new()),
            crate::Equipment::new(),
        ));

        // Create player faction standing component
        let mut standings = crate::components::FactionStanding::default();
        standings.set_standing("town_guard", 10);
        world.insert(player, (standings,)).unwrap();

        accept_quest(&mut world, player, "faction_quest", &templates).unwrap();

        // Turn in quest
        let msgs = complete_quest(&mut world, player, "faction_quest", &templates).unwrap();
        assert!(msgs
            .iter()
            .any(|m| m.contains("completed the quest: Faction Quest")));
        assert!(msgs
            .iter()
            .any(|m| m.contains("standing with Town Guard has increased by 15")));

        // Verify faction standing increased to 25 (10 + 15)
        let mut q_standings = world
            .query_one::<&crate::components::FactionStanding>(player)
            .unwrap();
        let st = q_standings.get().unwrap();
        assert_eq!(st.standing("town_guard"), 25);
    }

    #[test]
    fn test_quest_auto_complete() {
        let mut world = World::new();
        let mut templates = make_templates();

        // Create an auto-complete quest definition
        templates.quests.insert(
            "auto_complete_quest".to_string(),
            QuestDef {
                id: "auto_complete_quest".to_string(),
                name: "Auto Complete Quest".to_string(),
                description: "Auto completes when target killed.".to_string(),
                level_requirement: 1,
                repeatable: false,
                auto_complete: true,
                giver_npc: None,
                turn_in_npc: None,
                prerequisites: Vec::new(),
                objectives: vec![QuestObjective::Kill {
                    mob: "wolf".to_string(),
                    count: 1,
                }],
                rewards: QuestRewards {
                    xp: 50,
                    gold: 10,
                    items: Vec::new(),
                    faction: Vec::new(),
                },
                scripts: None,
                params: HashMap::new(),
            },
        );

        let player = world.spawn((
            QuestLog::new(),
            Level(1),
            Experience(0),
            Wallet::new(0, 0, 0, 0),
            Inventory(Vec::new()),
            crate::Equipment::new(),
        ));

        accept_quest(&mut world, player, "auto_complete_quest", &templates).unwrap();

        // Kill the wolf target - should trigger progress and immediately auto-complete the quest
        let msgs = handle_kill_event(&mut world, player, "wolf", &templates);
        assert!(msgs
            .iter()
            .any(|m| m.contains("completed the quest: Auto Complete Quest")));
        assert!(msgs.iter().any(|m| m.contains("gain 50 experience")));

        // Verify quest completed in quest log
        let mut q_log = world.query_one::<&QuestLog>(player).unwrap();
        let log = q_log.get().unwrap();
        assert!(log.completed.contains("auto_complete_quest"));
        assert!(!log.active.contains_key("auto_complete_quest"));
    }
}
