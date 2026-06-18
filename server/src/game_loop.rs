use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use mud_core::systems;
use mud_core::templates::SetDef;
use mud_core::{DbId, Entity, Player, Position, World};
use tokio::sync::Mutex;
use tokio::time::interval;

/// Spawn a background task that runs game systems on fixed intervals.
pub fn spawn_game_loop(
    world: Arc<Mutex<World>>,
    db: Option<Arc<Mutex<mud_data::Database>>>,
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
                    systems::combat::run_combat_pulse(&mut w);
                    systems::ai::run_ai_pulse(&mut w);
                    systems::stance::run_stance_pulse(&mut w);
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
                    drop(w);
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
