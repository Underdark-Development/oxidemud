pub mod combat;
pub mod effects;
pub mod entity_commands;
pub mod equipment;
pub mod messaging;
pub mod quests;
pub mod world;

use oxide_core::{DamageType, Entity, HitContext};
use rhai::Engine;

pub fn register_all(engine: &mut Engine) {
    // Security limits
    engine.set_max_operations(50_000);
    engine.set_max_call_levels(32);
    engine.set_max_string_size(10_000);

    // Register custom types
    engine.register_type_with_name::<Entity>("Entity");
    engine.register_type_with_name::<HitContext>("HitContext");
    engine.register_type_with_name::<DamageType>("DamageType");

    // Entity bindings
    engine.register_fn("id", |entity: Entity| entity.id() as i64);
    engine.register_fn("to_string", |entity: Entity| {
        format!("Entity({})", entity.id())
    });

    combat::register(engine);
    world::register(engine);
    messaging::register(engine);
    quests::register(engine);
    effects::register(engine);
    equipment::register(engine);
    entity_commands::register(engine);
}
