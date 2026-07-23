use crate::context::CURRENT_SCRIPT_CONTEXT;
use oxide_core::Entity;
use rhai::Engine;

pub fn register(engine: &mut Engine) {
    // Messaging
    engine.register_fn("send_to", |entity: Entity, msg: String| {
        if let Some(bridge) = oxide_core::scripting::get_message_bridge() {
            bridge.send_to_entity(entity, &msg);
        }
    });
    engine.register_fn("send", |entity: Entity, msg: String| {
        if let Some(bridge) = oxide_core::scripting::get_message_bridge() {
            bridge.send_to_entity(entity, &msg);
        }
    });
    engine.register_fn("send", |msg: String| {
        if let Some(target_ent) = CURRENT_SCRIPT_CONTEXT
            .with(|c| c.borrow().and_then(|ctx| ctx.actor.or(Some(ctx.entity))))
        {
            if let Some(bridge) = oxide_core::scripting::get_message_bridge() {
                bridge.send_to_entity(target_ent, &msg);
            }
        }
    });

    // Scoped current room messaging (0 arguments needed)
    engine.register_fn("echo", |msg: String| {
        if let Some(room) = CURRENT_SCRIPT_CONTEXT.with(|c| c.borrow().and_then(|ctx| ctx.room))
        {
            if let Some(bridge) = oxide_core::scripting::get_message_bridge() {
                bridge.echo_to_room(room, &msg);
            }
        }
    });
    engine.register_fn("echo_except", |msg: String, exclude: rhai::Array| {
        if let Some(room) = CURRENT_SCRIPT_CONTEXT.with(|c| c.borrow().and_then(|ctx| ctx.room))
        {
            if let Some(bridge) = oxide_core::scripting::get_message_bridge() {
                let excluded_entities: Vec<Entity> = exclude
                    .into_iter()
                    .filter_map(|v| v.try_cast::<Entity>())
                    .collect();
                bridge.echo_to_room_except(room, &msg, &excluded_entities);
            }
        }
    });

    // Direct room messaging on Room Entity handle
    engine.register_fn("echo", |room: Entity, msg: String| {
        if let Some(bridge) = oxide_core::scripting::get_message_bridge() {
            bridge.echo_to_room(room, &msg);
        }
    });
    engine.register_fn(
        "echo_except",
        |room: Entity, msg: String, exclude: rhai::Array| {
            if let Some(bridge) = oxide_core::scripting::get_message_bridge() {
                let excluded_entities: Vec<Entity> = exclude
                    .into_iter()
                    .filter_map(|v| v.try_cast::<Entity>())
                    .collect();
                bridge.echo_to_room_except(room, &msg, &excluded_entities);
            }
        },
    );

    // Remote room messaging by room entity handle
    engine.register_fn("echo_to", |room: Entity, msg: String| {
        if let Some(bridge) = oxide_core::scripting::get_message_bridge() {
            bridge.echo_to_room(room, &msg);
        }
    });
    engine.register_fn(
        "echo_to_except",
        |room: Entity, msg: String, exclude: rhai::Array| {
            if let Some(bridge) = oxide_core::scripting::get_message_bridge() {
                let excluded_entities: Vec<Entity> = exclude
                    .into_iter()
                    .filter_map(|v| v.try_cast::<Entity>())
                    .collect();
                bridge.echo_to_room_except(room, &msg, &excluded_entities);
            }
        },
    );
}
