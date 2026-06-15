use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use mud_core::systems;
use mud_core::templates::SetDef;
use mud_core::World;
use tokio::sync::Mutex;
use tokio::time::interval;

/// Spawn a background task that runs game systems on fixed intervals.
pub fn spawn_game_loop(
    world: Arc<Mutex<World>>,
    _db: Option<Arc<Mutex<mud_data::Database>>>,
    mut shutdown: tokio::sync::watch::Receiver<bool>,
) {
    tokio::spawn(async move {
        let mut combat_tick = interval(Duration::from_secs(2));
        let mut regen_tick = interval(Duration::from_secs(6));
        let mut maintenance_tick = interval(Duration::from_secs(5));
        let mut set_bonus_tick = interval(Duration::from_secs(10));

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
            }
        }
    });
}
