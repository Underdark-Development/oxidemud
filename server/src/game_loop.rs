use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use mud_core::systems;
use mud_core::systems::combat::{CombatOutcome, CombatOutcomeKind};
use mud_core::templates::SetDef;
use mud_core::{DbId, Entity, Experience, Health, Level, Player, Position, World};
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
        let mut regen_tick = interval(Duration::from_secs(6));
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
                    // Level-up check for kills
                    for outcome in &outcomes {
                        if let CombatOutcomeKind::Killed { .. } = &outcome.kind {
                            if outcome.attacker_is_player {
                                crate::award_xp(&mut w, outcome.attacker);
                                if let Some(ref db) = db {
                                    if let Ok(db_guard) = db.try_lock() {
                                        save_player_progress(&w, outcome.attacker, &db_guard);
                                    }
                                }
                            }
                        }
                    }
                    dispatch_combat_outcomes(&reg, outcomes);
                    systems::ai::run_ai_pulse(&mut w);
                    systems::stance::run_stance_pulse(&mut w);
                    drop(reg);
                    drop(w);
                }
                _ = regen_tick.tick() => {
                    let mut w = world.lock().await;
                    systems::regen::run_regen_pulse(&mut w);
                    drop(w);
                }
                _ = maintenance_tick.tick() => {
                    let mut w = world.lock().await;
                    systems::corpse::run_corpse_pulse(&mut w);
                    if let Some(ref db) = db {
                        if let Ok(db_guard) = db.try_lock() {
                            save_online_players(&mut w, &db_guard);
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
            CombatOutcomeKind::Killed { xp_gained } => {
                if outcome.attacker_is_player {
                    if let Some(tx) = registry.sender(outcome.attacker) {
                        let _ = tx.send(
                            format!(
                                "You kill {}! You gain {} experience.\r\n",
                                outcome.target_name, xp_gained
                            )
                            .into_bytes(),
                        );
                    }
                } else if outcome.target_is_player {
                    if let Some(tx) = registry.sender(outcome.target) {
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

pub(crate) fn save_online_players(world: &mut World, db: &mud_data::Database) {
    save_player_positions(world, db);

    let players: Vec<Entity> = world
        .query::<(&Player, &DbId)>()
        .iter()
        .map(|(raw, _)| Entity::from(raw))
        .collect();

    for player in players {
        save_player_progress(world, player, db);
    }
}

pub(crate) fn save_player_progress(world: &World, player: Entity, db: &mud_data::Database) {
    let Some(db_id) = world
        .query_one::<&DbId>(player)
        .ok()
        .and_then(|mut q| q.get().copied())
        .map(|db_id| db_id.0)
    else {
        return;
    };

    let conn = db.conn();

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

    if let Some(health) = world
        .query_one::<&Health>(player)
        .ok()
        .and_then(|mut q| q.get().cloned())
    {
        let _ = mud_data::save_health_component(conn, db_id, health.current, health.max);
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
