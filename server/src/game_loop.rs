use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use oxide_core::systems;
use oxide_core::templates::SetDef;
use oxide_core::{run_player_state_decay, World};

use tokio::sync::Mutex;
use tokio::time::interval;

use crate::dispatch::process_combat_pulse_results;
use crate::persistence::save_online_players;
use crate::registry::ConnectionRegistry;

/// Spawn a background task that runs game systems on fixed intervals.
pub fn spawn_game_loop(
    world: Arc<Mutex<World>>,
    db: Option<Arc<Mutex<oxide_data::Database>>>,
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
        let mut player_state_tick = interval(Duration::from_millis(250));
        let mut last_player_state_tick = Instant::now();
        let mut skill_decay_tick = interval(Duration::from_secs(1));
        let mut last_backup = Instant::now();
        let real_mins = crate::config::get().time.real_minutes_per_game_hour.max(1);
        let mut time_tick = interval(Duration::from_secs(real_mins));
        let mut weather_tick = interval(Duration::from_secs(300));

        loop {
            tokio::select! {
                biased;
                _ = shutdown.changed() => {
                    tracing::info!("Game loop preparing for shutdown");
                    break;
                }
                _ = weather_tick.tick() => {
                    let mut w = world.lock().await;
                    let reg = registry.lock().await;

                    let current_season = {
                        let mut q = w.query::<&oxide_core::GameTime>();
                        q.into_iter().next().map(|(_, gt)| gt.season).unwrap_or(oxide_core::Season::Spring)
                    };

                    if let Some(templates) = crate::get_templates() {
                        if let Some(ref weather_config) = templates.weather {
                            let room_keys: Vec<(oxide_core::Entity, String)> = w
                                .query::<&oxide_core::RoomKey>()
                                .iter()
                                .map(|(e, rk)| (oxide_core::Entity::from(e), rk.0.clone()))
                                .collect();

                            let empty_map = std::collections::HashMap::new();

                            for (room_ent, room_key_str) in room_keys {
                                let mut parts = room_key_str.split(':');
                                let area_id = parts.next().unwrap_or("");
                                let room_id = parts.next().unwrap_or("");

                                let area_tmpl = templates.areas.get(area_id);
                                let room_tmpl = area_tmpl.and_then(|a| a.rooms.get(room_id));

                                let (room_no_weather, room_exclude, room_add) = if let Some(rt) = room_tmpl {
                                    (rt.no_weather, rt.exclude_weather.as_slice(), &rt.additional_weather)
                                } else {
                                    (false, [].as_slice(), &empty_map)
                                };

                                let (area_no_weather, area_zone, area_matrix) = if let Some(at) = area_tmpl {
                                    (
                                        at.no_weather,
                                        at.weather_zone.as_deref(),
                                        if at.weather_matrix.is_empty() {
                                            None
                                        } else {
                                            Some(&at.weather_matrix)
                                        },
                                    )
                                } else {
                                    (false, None, None)
                                };

                                let params = oxide_core::ResolutionParams {
                                    season: current_season,
                                    area_no_weather,
                                    area_weather_zone: area_zone,
                                    area_weather_matrix: area_matrix,
                                    room_no_weather,
                                    room_exclude_weather: room_exclude,
                                    room_additional_weather: room_add,
                                };

                                let base_weights = oxide_core::resolve_weather_weights(&params, weather_config, oxide_core::templates::weather::ConditionType::Base);
                                let mod_weights = oxide_core::resolve_weather_weights(&params, weather_config, oxide_core::templates::weather::ConditionType::Modifier);

                                let rolled_base = oxide_core::roll_weather(&base_weights);
                                let rolled_mod = oxide_core::roll_modifier(&mod_weights);

                                let old_state = w.query_one::<&oxide_core::WeatherState>(room_ent)
                                    .ok()
                                    .and_then(|mut q| q.get().cloned())
                                    .unwrap_or_default();

                                let mut new_state = oxide_core::WeatherState::new(Some(rolled_base.clone()), rolled_mod.clone());
                                new_state.effects = oxide_core::get_effective_weather_effects(&new_state, weather_config);

                                if old_state != new_state {
                                    let _ = w.insert(room_ent, (new_state.clone(),));

                                    if let Some(def) = weather_config.conditions.get(&rolled_base) {
                                        if def.severity == oxide_core::templates::weather::WeatherSeverity::Severe {
                                            let broadcast_msg = format!("\r\n[Weather] {}\r\n", def.description);
                                            reg.broadcast_to_room(&w, room_ent, &broadcast_msg, None);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                _ = time_tick.tick() => {
                    let w = world.lock().await;
                    let time_config = crate::config::get().time.clone();
                    let events = {
                        let mut q = w.query::<&mut oxide_core::GameTime>();
                        if let Some((_, gt)) = q.into_iter().next() {
                            oxide_core::advance_time(gt, 1, &time_config)
                        } else {
                            Vec::new()
                        }
                    };

                    for event in events {
                        match event {
                            oxide_core::TimeEvent::PeriodChanged { old_period, new_period } => {
                                tracing::info!("Time period changed from {} to {}", old_period.name(), new_period.name());
                            }
                            oxide_core::TimeEvent::SeasonChanged { old_season, new_season } => {
                                tracing::info!("Season changed from {} to {}", old_season.name(), new_season.name());
                            }
                            _ => {}
                        }
                    }
                }
                _ = combat_tick.tick() => {
                    let mut w = world.lock().await;
                    let reg = registry.lock().await;
                    let outcomes = systems::combat::run_combat_pulse(&mut w);

                    process_combat_pulse_results(&mut w, &reg, &db, outcomes);

                    systems::ai::run_ai_pulse(&mut w);
                    systems::stance::run_stance_pulse(&mut w);
                    oxide_core::run_formation_effects(&mut w);
                    drop(reg);
                    drop(w);
                }
                _ = tokio::time::sleep_until(next_big_tick) => {
                    let now = Instant::now();
                    let tick_duration = now - last_tick_time;
                    last_tick_time = now;

                    let mut w = world.lock().await;
                    let reg = registry.lock().await;

                    let dead_entities = systems::regen::run_regen_pulse(&mut w, tick_duration);
                    for (player, room_entity) in dead_entities {
                        let is_player = w
                            .query_one::<&oxide_core::Player>(player)
                            .is_ok_and(|mut q| q.get().is_some());
                        if is_player {
                            if let Some(tx) = reg.sender(player) {
                                let _ = tx.send(
                                    b"You bleed to death...\r\nAlas, you are dead! You are a ghost now...\r\n".to_vec()
                                );
                            }

                            let name = w
                                .query_one::<&oxide_core::Name>(player)
                                .ok()
                                .and_then(|mut q| q.get().map(|n| n.as_str().to_string()))
                                .unwrap_or_else(|| "Someone".to_string());
                            let room_msg = format!("{} bleeds to death.\r\n{} is dead! R.I.P.\r\n", name, name);
                            let room_msg_bytes = room_msg.into_bytes();
                            for &other in &reg.occupants(&w, room_entity) {
                                if other != player {
                                    if let Some(other_tx) = reg.sender(other) {
                                        let _ = other_tx.send(room_msg_bytes.clone());
                                    }
                                }
                            }
                        }
                    }
                    crate::prompt::broadcast_prompts(&w, &reg);

                    drop(reg);
                    drop(w);

                    next_big_tick = tokio::time::Instant::now()
                        + Duration::from_secs(fastrand::u64(30..=90));
                }
                _ = maintenance_tick.tick() => {
                    {
                        let mut w = world.lock().await;
                        systems::corpse::run_corpse_pulse(&mut w);
                        oxide_core::run_skill_gate_pulse(&mut w);
                        oxide_core::run_group_cleanup(&mut w, Instant::now());
                        if let Some(ref db) = db {
                            if let Ok(db_guard) = db.try_lock() {
                                save_online_players(&mut w, &db_guard, false);
                            }
                        }
                    }

                    let retention = crate::config::get().logging.retention_days;
                    crate::config::prune_old_logs(retention);

                    if last_backup.elapsed() >= Duration::from_secs(3600) {
                        last_backup = Instant::now();
                        if let Some(ref db_mutex) = db {
                            let db_clone = db_mutex.clone();
                            tokio::spawn(async move {
                                let backup_dir = "data/backups";
                                match tokio::task::spawn_blocking(move || {
                                    let db_guard = db_clone.blocking_lock();
                                    db_guard.run_backup(backup_dir)
                                })
                                .await
                                {
                                    Ok(Ok(())) => {
                                        tracing::info!("Database backup completed successfully.");
                                    }
                                    Ok(Err(e)) => {
                                        tracing::error!("Backup failed: {e}");
                                    }
                                    Err(e) => {
                                        tracing::error!("Backup thread panicked: {e}");
                                    }
                                }
                            });
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
                _ = player_state_tick.tick() => {
                    let now = Instant::now();
                    let elapsed = now - last_player_state_tick;
                    last_player_state_tick = now;

                    let mut w = world.lock().await;
                    let reg = registry.lock().await;

                    let transitions = run_player_state_decay(&mut w, elapsed);
                    for (entity, old_state, new_state) in transitions {
                        let name = w
                            .query_one::<&oxide_core::Name>(entity)
                            .ok()
                            .and_then(|mut q| q.get().map(|n| n.as_str().to_string()));

                        let room = w
                            .query_one::<&oxide_core::Position>(entity)
                            .ok()
                            .and_then(|mut q| q.get().map(|p| p.room));

                        if let Some(tx) = reg.sender(entity) {
                            match (old_state, new_state) {
                                (oxide_core::PlayerState::Stunned { .. }, oxide_core::PlayerState::Resting(oxide_core::RestState::Standing)) => {
                                    let _ = tx.send(b"You recover from your stun and stand up.\r\n".to_vec());
                                    if let (Some(name), Some(room)) = (&name, room) {
                                        let msg = format!("{} recovers from their stun and stands up.\r\n", name);
                                        reg.broadcast_to_room(&w, room, &msg, Some(entity));
                                    }
                                }
                                (oxide_core::PlayerState::Casting { .. }, oxide_core::PlayerState::Resting(oxide_core::RestState::Standing)) => {
                                    let _ = tx.send(b"You finish casting your spell.\r\n".to_vec());
                                    if let (Some(name), Some(room)) = (&name, room) {
                                        let msg = format!("{} finishes casting their spell.\r\n", name);
                                        reg.broadcast_to_room(&w, room, &msg, Some(entity));
                                    }
                                }
                                _ => {}
                            }
                            crate::prompt::send_player_prompt(&w, entity, &reg);
                        }
                    }
                }
                _ = skill_decay_tick.tick() => {
                    let mut w = world.lock().await;
                    let reg = registry.lock().await;

                    oxide_core::run_cooldown_decay(&mut w, 1);

                    let expired = oxide_core::run_temporary_effect_decay(&mut w, 1);
                    for (entity, source, stat) in expired {
                        if let Some(tx) = reg.sender(entity) {
                            let _ = tx.send(format!("Your {} buff/debuff from {} has worn off.\r\n", stat, source).into_bytes());
                        }
                        crate::prompt::send_player_prompt(&w, entity, &reg);
                    }
                }

            }
        }
    });
}
