use std::sync::Arc;
use std::time::Duration;

use mud_core::systems;
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
        let mut maintenance_tick = interval(Duration::from_secs(5));

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
                _ = maintenance_tick.tick() => {
                    let mut w = world.lock().await;
                    systems::corpse::run_corpse_pulse(&mut w);
                    drop(w);
                }
            }
        }
    });
}
