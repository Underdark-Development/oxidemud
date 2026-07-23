use crate::context::with_current_world;
use oxide_core::Entity;
use rhai::Engine;

pub fn register(engine: &mut Engine) {
    engine.register_fn("accept_quest", |player: Entity, quest_id: String| -> bool {
        with_current_world(|w| {
            let templates = match oxide_core::templates::get_global_templates() {
                Some(t) => t,
                None => return false,
            };
            let res = oxide_core::accept_quest(w, player, &quest_id, &templates);
            if let Ok(msgs) = res {
                if let Some(msg_bridge) = oxide_core::scripting::get_message_bridge() {
                    for msg in msgs {
                        msg_bridge.send_to_entity(player, &msg);
                    }
                }
                true
            } else {
                false
            }
        })
        .unwrap_or(false)
    });

    engine.register_fn(
        "complete_quest",
        |player: Entity, quest_id: String| -> bool {
            with_current_world(|w| {
                let templates = match oxide_core::templates::get_global_templates() {
                    Some(t) => t,
                    None => return false,
                };
                let res = oxide_core::complete_quest(w, player, &quest_id, &templates);
                if let Ok(msgs) = res {
                    if let Some(msg_bridge) = oxide_core::scripting::get_message_bridge() {
                        for msg in msgs {
                            msg_bridge.send_to_entity(player, &msg);
                        }
                    }
                    true
                } else {
                    false
                }
            })
            .unwrap_or(false)
        },
    );

    engine.register_fn("is_on_quest", |player: Entity, quest_id: String| -> bool {
        with_current_world(|w| {
            if let Ok(mut q) = w.query_one::<&oxide_core::QuestLog>(player) {
                if let Some(log) = q.get() {
                    return log.active.contains_key(&quest_id);
                }
            }
            false
        })
        .unwrap_or(false)
    });

    engine.register_fn(
        "has_completed_quest",
        |player: Entity, quest_id: String| -> bool {
            with_current_world(|w| {
                if let Ok(mut q) = w.query_one::<&oxide_core::QuestLog>(player) {
                    if let Some(log) = q.get() {
                        return log.completed.contains(&quest_id);
                    }
                }
                false
            })
            .unwrap_or(false)
        },
    );
}
