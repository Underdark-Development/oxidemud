use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use mud_core::systems;
use mud_core::systems::combat::{CombatOutcome, CombatOutcomeKind};
use mud_core::templates::SetDef;
use mud_core::{
    Alignment, Attributes, DbId, Description, Entity, Equipment, Experience, Health, Inventory,
    LearnedSkills, Level, Player, Position, PracticePoints, SpawnKey, Wallet, World,
};
use tokio::sync::Mutex;
use tokio::time::interval;

use crate::registry::ConnectionRegistry;

/// Spawn a background task that runs game systems on fixed intervals.
pub fn spawn_game_loop(
    world: Arc<Mutex<World>>,
    db: Option<Arc<Mutex<mud_data::Database>>>,
    registry: Arc<Mutex<ConnectionRegistry>>,
    mut shutdown: tokio::sync::watch::Receiver<bool>,
) {
    tokio::spawn(async move {
        let mut combat_tick = interval(Duration::from_secs(2));

        // Big tick: regen + prompt broadcast (Merc-style, 30-90s randomized)
        let initial_secs = fastrand::u64(30..=90);
        let mut next_big_tick = tokio::time::Instant::now() + Duration::from_secs(initial_secs);
        let mut last_tick_time = Instant::now();

        let mut maintenance_tick = interval(Duration::from_secs(5));
        let mut set_bonus_tick = interval(Duration::from_secs(10));
        let mut position_save_tick = interval(Duration::from_secs(60));

        loop {
            tokio::select! {
                biased;
                _ = shutdown.changed() => {
                    tracing::info!("Game loop shutting down");
                    break;
                }
                _ = combat_tick.tick() => {
                    let mut w = world.lock().await;
                    let reg = registry.lock().await;
                    let outcomes = systems::combat::run_combat_pulse(&mut w);

                    // Collect involved players before consuming outcomes
                    let involved_players: Vec<Entity> = outcomes
                        .iter()
                        .flat_map(|o| {
                            let mut v = Vec::new();
                            if o.attacker_is_player { v.push(o.attacker); }
                            if o.target_is_player { v.push(o.target); }
                            v
                        })
                        .collect();

                    // Level-up check for kills
                    for outcome in &outcomes {
                        if let CombatOutcomeKind::Killed { .. } = &outcome.kind {
                            if outcome.attacker_is_player {
                                let msgs = crate::award_xp(&mut w, outcome.attacker);
                                for msg in &msgs {
                                    if let Some(tx) = reg.sender(outcome.attacker) {
                                        let _ = tx.send(format!("{msg}\r\n").into_bytes());
                                    }
                                }
                                if let Some(ref db) = db {
                                    if let Ok(db_guard) = db.try_lock() {
                                        save_player_progress(&mut w, outcome.attacker, &db_guard);
                                    }
                                }
                            }
                        }
                    }
                    dispatch_combat_outcomes(&reg, outcomes);

                    // Send prompt to players involved in combat outcomes
                    for entity in involved_players {
                        crate::prompt::send_player_prompt(&w, entity, &reg);
                    }

                    systems::ai::run_ai_pulse(&mut w);
                    systems::stance::run_stance_pulse(&mut w);
                    drop(reg);
                    drop(w);
                }
                _ = tokio::time::sleep_until(next_big_tick) => {
                    let now = Instant::now();
                    let tick_duration = now - last_tick_time;
                    last_tick_time = now;

                    let mut w = world.lock().await;
                    let reg = registry.lock().await;

                    systems::regen::run_regen_pulse(&mut w, tick_duration);
                    crate::prompt::broadcast_prompts(&w, &reg);

                    drop(reg);
                    drop(w);

                    next_big_tick = tokio::time::Instant::now()
                        + Duration::from_secs(fastrand::u64(30..=90));
                }
                _ = maintenance_tick.tick() => {
                    let mut w = world.lock().await;
                    systems::corpse::run_corpse_pulse(&mut w);
                    if let Some(ref db) = db {
                        if let Ok(db_guard) = db.try_lock() {
                            save_online_players(&mut w, &db_guard, false);
                            drop(w);
                        }
                    }
                }
                _ = set_bonus_tick.tick() => {
                    let mut w = world.lock().await;
                    if let Some(templates) = crate::get_templates() {
                        let set_defs: HashMap<String, SetDef> = templates.sets.clone();
                        systems::set_bonus::reconcile_all_set_bonuses(&mut w, &set_defs);
                    }
                    drop(w);
                }
                _ = position_save_tick.tick() => {
                    if let Some(ref db) = db {
                        if let Ok(db_guard) = db.try_lock() {
                            let mut w = world.lock().await;
                            save_player_positions(&mut w, &db_guard);
                            drop(w);
                        }
                    }
                }
            }
        }
    });
}

fn dispatch_combat_outcomes(registry: &ConnectionRegistry, outcomes: Vec<CombatOutcome>) {
    for outcome in outcomes {
        match outcome.kind {
            CombatOutcomeKind::Hit { damage, .. } => {
                if outcome.attacker_is_player {
                    if let Some(tx) = registry.sender(outcome.attacker) {
                        let _ = tx.send(
                            format!("You hit {} for {} damage.\r\n", outcome.target_name, damage)
                                .into_bytes(),
                        );
                    }
                } else if outcome.target_is_player {
                    if let Some(tx) = registry.sender(outcome.target) {
                        let _ = tx.send(
                            format!(
                                "{} hits you for {} damage.\r\n",
                                outcome.attacker_name, damage
                            )
                            .into_bytes(),
                        );
                    }
                }
            }
            CombatOutcomeKind::Miss => {
                if outcome.attacker_is_player {
                    if let Some(tx) = registry.sender(outcome.attacker) {
                        let _ =
                            tx.send(format!("You miss {}.\r\n", outcome.target_name).into_bytes());
                    }
                } else if outcome.target_is_player {
                    if let Some(tx) = registry.sender(outcome.target) {
                        let _ = tx.send(
                            format!("{} misses you.\r\n", outcome.attacker_name).into_bytes(),
                        );
                    }
                }
            }
            CombatOutcomeKind::Killed {
                damage, xp_gained, ..
            } => {
                if outcome.attacker_is_player {
                    if let Some(tx) = registry.sender(outcome.attacker) {
                        let _ = tx.send(
                            format!("You hit {} for {} damage.\r\n", outcome.target_name, damage)
                                .into_bytes(),
                        );
                        let _ = tx.send(
                            format!(
                                "You kill {}! You gain {} experience.\r\n",
                                outcome.target_name, xp_gained
                            )
                            .into_bytes(),
                        );
                    }
                }
                if outcome.target_is_player {
                    if let Some(tx) = registry.sender(outcome.target) {
                        let _ = tx.send(
                            format!(
                                "{} hits you for {} damage.\r\n",
                                outcome.attacker_name, damage
                            )
                            .into_bytes(),
                        );
                        let _ = tx.send(
                            format!("You have been slain by {}!\r\n", outcome.attacker_name)
                                .into_bytes(),
                        );
                    }
                }
            }
            CombatOutcomeKind::FleeSuccess { dest: _, moved } => {
                if outcome.attacker_is_player {
                    if let Some(tx) = registry.sender(outcome.attacker) {
                        if moved {
                            let _ = tx.send("You flee from combat!\r\n".to_string().into_bytes());
                        } else {
                            let _ = tx.send(
                                "You flee from combat, but there is nowhere to go!\r\n"
                                    .to_string()
                                    .into_bytes(),
                            );
                        }
                    }
                }
                if outcome.target_is_player {
                    if let Some(tx) = registry.sender(outcome.target) {
                        let _ = tx.send(
                            format!("{} flees from combat!\r\n", outcome.attacker_name)
                                .into_bytes(),
                        );
                    }
                }
            }
            CombatOutcomeKind::FleeFail { attempts } => {
                if outcome.attacker_is_player {
                    if let Some(tx) = registry.sender(outcome.attacker) {
                        let _ = tx.send(
                            format!("You failed to flee! (Attempt {})\r\n", attempts).into_bytes(),
                        );
                    }
                }
                if outcome.target_is_player {
                    if let Some(tx) = registry.sender(outcome.target) {
                        let _ = tx.send(
                            format!("{} attempts to flee, but fails!\r\n", outcome.attacker_name)
                                .into_bytes(),
                        );
                    }
                }
            }
        }
    }
}

pub(crate) fn save_online_players(world: &mut World, db: &mud_data::Database, force: bool) {
    save_player_positions(world, db);

    let players: Vec<Entity> = if force {
        world
            .query::<(&Player, &DbId)>()
            .iter()
            .map(|(raw, _)| Entity::from(raw))
            .collect()
    } else {
        world
            .query::<(&Player, &DbId, &mud_core::Dirty)>()
            .iter()
            .map(|(raw, _)| Entity::from(raw))
            .collect()
    };

    for player in players {
        save_player_progress(world, player, db);
        let _ = world.remove_one::<mud_core::Dirty>(player);
    }
}

pub(crate) fn save_player_progress(world: &mut World, player: Entity, db: &mud_data::Database) {
    let Some(db_id) = world
        .query_one::<&DbId>(player)
        .ok()
        .and_then(|mut q| q.get().copied())
        .map(|db_id| db_id.0)
    else {
        return;
    };

    let conn = db.conn();

    // 1. Level & Experience
    if let Some(level) = world
        .query_one::<&Level>(player)
        .ok()
        .and_then(|mut q| q.get().copied())
    {
        let _ = mud_data::save_level_component(conn, db_id, level.0 as i64);
        let _ = mud_data::update_character_level(
            conn,
            db_id,
            level.0.into(),
            current_xp(world, player) as i64,
        );
    }

    if let Some(xp) = world
        .query_one::<&Experience>(player)
        .ok()
        .and_then(|mut q| q.get().copied())
    {
        let _ = mud_data::save_experience_component(conn, db_id, xp.0 as i64);
    }

    // 2. Health
    if let Some(health) = world
        .query_one::<&Health>(player)
        .ok()
        .and_then(|mut q| q.get().cloned())
    {
        let _ = mud_data::save_health_component(conn, db_id, health.current, health.max);
    }

    // 3. Mana
    if let Some(mana) = world
        .query_one::<&mud_core::Mana>(player)
        .ok()
        .and_then(|mut q| q.get().cloned())
    {
        let _ = mud_data::save_mana_component(conn, db_id, mana.current as i32);
    }

    // 4. Stamina
    if let Some(stamina) = world
        .query_one::<&mud_core::Stamina>(player)
        .ok()
        .and_then(|mut q| q.get().cloned())
    {
        let _ = mud_data::save_stamina_component(conn, db_id, stamina.current as i32);
    }

    // 5. Wallet / Golds
    if let Some(wallet) = world
        .query_one::<&Wallet>(player)
        .ok()
        .and_then(|mut q| q.get().cloned())
    {
        let _ = mud_data::save_golds_component(
            conn,
            db_id,
            wallet.copper as i64,
            wallet.silver as i64,
            wallet.gold as i64,
            wallet.platinum as i64,
        );
    }

    // 6. LearnedSkills
    if let Some(skills) = world
        .query_one::<&LearnedSkills>(player)
        .ok()
        .and_then(|mut q| q.get().cloned())
    {
        if let Err(e) = mud_data::save_skills(conn, db_id, &skills.skills) {
            tracing::error!(entity_id = db_id, error = %e, "save_player_progress: failed to save skills");
        }

        let practice_points = world
            .query_one::<&PracticePoints>(player)
            .ok()
            .and_then(|mut q| q.get().copied())
            .map(|p| p.0)
            .unwrap_or(0);
        if let Some(player_comp) = world
            .query_one::<&Player>(player)
            .ok()
            .and_then(|mut q| q.get().cloned())
        {
            if let Err(e) = mud_data::save_player_component(
                conn,
                db_id,
                player_comp.account_id,
                player_comp.prompt.as_deref(),
                player_comp.screen_width,
                practice_points,
            ) {
                tracing::error!(entity_id = db_id, error = %e, "save_player_progress: failed to save player component");
            }
        }
    } else if let Some(player_comp) = world
        .query_one::<&Player>(player)
        .ok()
        .and_then(|mut q| q.get().cloned())
    {
        if let Err(e) = mud_data::save_player_component(
            conn,
            db_id,
            player_comp.account_id,
            player_comp.prompt.as_deref(),
            player_comp.screen_width,
            0,
        ) {
            tracing::error!(entity_id = db_id, error = %e, "save_player_progress: failed to save player component");
        }
    }

    // 7. Attributes
    if let Some(attrs) = world
        .query_one::<&Attributes>(player)
        .ok()
        .and_then(|mut q| q.get().cloned())
    {
        let _ = mud_data::save_attributes_component(
            conn,
            db_id,
            &mud_data::AttributesRow {
                strength: attrs.strength,
                dexterity: attrs.dexterity,
                intelligence: attrs.intelligence,
                wisdom: attrs.wisdom,
                constitution: attrs.constitution,
                charisma: attrs.charisma,
            },
        );
    }

    // 8. Alignment
    if let Some(alignment) = world
        .query_one::<&Alignment>(player)
        .ok()
        .and_then(|mut q| q.get().cloned())
    {
        let _ = mud_data::save_alignment_component(conn, db_id, &alignment.0);
    }

    // 9. Description
    if let Some(description) = world
        .query_one::<&Description>(player)
        .ok()
        .and_then(|mut q| q.get().cloned())
    {
        let _ = mud_data::save_description_component(conn, db_id, &description.0);
    }

    // 10. Inventory
    let mut inventory_items = Vec::new();
    if let Some(inventory) = world
        .query_one::<&Inventory>(player)
        .ok()
        .and_then(|mut q| q.get().cloned())
    {
        for &item_entity in &inventory.0 {
            if let Ok(mut item_q) = world.query_one::<(&mud_core::Item, Option<&DbId>)>(item_entity)
            {
                if let Some((item, opt_db_id)) = item_q.get() {
                    inventory_items.push((
                        item_entity,
                        item.template_id.clone(),
                        opt_db_id.map(|d| d.0),
                    ));
                }
            }
        }
    }

    if !inventory_items.is_empty() {
        let _ = mud_data::delete_all_inventory(conn, db_id);
        for (slot_idx, (item_entity, template_id, opt_db_id)) in
            inventory_items.into_iter().enumerate()
        {
            let item_db_id = match opt_db_id {
                Some(id) => id,
                None => {
                    if let Ok(new_id) = mud_data::insert_entity(conn, "item") {
                        let _ = world.insert(item_entity, (DbId::new(new_id),));
                        new_id
                    } else {
                        continue;
                    }
                }
            };
            let _ = mud_data::save_item_component(conn, item_db_id, &template_id);
            let _ = mud_data::add_inventory_item(conn, db_id, item_db_id, slot_idx as i32);
        }
    }

    // 11. Equipment
    let mut equipment_items = Vec::new();
    if let Some(equipment) = world
        .query_one::<&Equipment>(player)
        .ok()
        .and_then(|mut q| q.get().cloned())
    {
        for &(slot, item_entity) in &equipment.slots {
            if let Ok(mut item_q) = world.query_one::<(&mud_core::Item, Option<&DbId>)>(item_entity)
            {
                if let Some((item, opt_db_id)) = item_q.get() {
                    equipment_items.push((
                        slot,
                        item_entity,
                        item.template_id.clone(),
                        opt_db_id.map(|d| d.0),
                    ));
                }
            }
        }
    }

    if !equipment_items.is_empty() {
        let _ = mud_data::delete_all_equipment(conn, db_id);
        for (slot, item_entity, template_id, opt_db_id) in equipment_items {
            let item_db_id = match opt_db_id {
                Some(id) => id,
                None => {
                    if let Ok(new_id) = mud_data::insert_entity(conn, "item") {
                        let _ = world.insert(item_entity, (DbId::new(new_id),));
                        new_id
                    } else {
                        continue;
                    }
                }
            };
            let _ = mud_data::save_item_component(conn, item_db_id, &template_id);
            let slot_str = format!("{:?}", slot).to_lowercase();
            let _ = mud_data::save_equipment_slot(conn, db_id, &slot_str, item_db_id);
        }
    }
}

fn current_xp(world: &World, player: Entity) -> u64 {
    world
        .query_one::<&Experience>(player)
        .ok()
        .and_then(|mut q| q.get().copied())
        .map(|xp| xp.0)
        .unwrap_or(0)
}

/// Save every online player's current room to the database.
/// Inserts a DB entity record for any room that doesn't have one yet.
pub(crate) fn save_player_positions(world: &mut World, db: &mud_data::Database) {
    let conn = db.conn();
    let players: Vec<(i64, Entity)> = world
        .query::<(&DbId, &Position, &Player)>()
        .iter()
        .map(|(_, (db_id, pos, _))| (db_id.0, pos.room))
        .collect();

    for (player_entity_id, room_entity) in players {
        // Persist spawn_key for cross-restart room resolution
        if let Ok(mut q) = world.query_one::<&SpawnKey>(room_entity) {
            if let Some(sk) = q.get() {
                let _ = mud_data::update_character_spawn_key(conn, player_entity_id, &sk.0);
            }
        }

        let existing_room_db_id = world
            .query_one::<&DbId>(room_entity)
            .ok()
            .and_then(|mut q| q.get().copied())
            .map(|dbid| dbid.0);

        let room_db_id = existing_room_db_id.or_else(|| {
            mud_data::insert_entity(conn, "room").ok().inspect(|&id| {
                let _ = world.insert(room_entity, (DbId(id),));
            })
        });

        if let Some(rid) = room_db_id {
            let _ = mud_data::update_character_position(conn, player_entity_id, rid);
            let _ = mud_data::update_character_last_seen(conn, player_entity_id);
        }
    }
}
