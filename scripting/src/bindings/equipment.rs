use crate::context::with_current_world;
use oxide_core::Entity;
use rhai::Engine;

pub fn register(engine: &mut Engine) {
    engine.register_fn("is_equipped", |item: Entity| -> bool {
        with_current_world(|w| {
            for (_, eq) in w.query::<&oxide_core::Equipment>().iter() {
                if eq.slots.iter().any(|(_, e)| *e == item) {
                    return true;
                }
            }
            false
        })
        .unwrap_or(false)
    });
}
