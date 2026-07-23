use crate::context::with_current_world;
use oxide_core::Entity;
use rhai::Engine;
use std::collections::HashMap;

pub fn register(engine: &mut Engine) {
    engine.register_fn(
        "set_cooldown",
        |entity: Entity, skill_id: String, secs: i64| {
            with_current_world(|w| {
                if let Ok(mut q) = w.query_one::<&mut oxide_core::SkillCooldowns>(entity) {
                    if let Some(cd) = q.get() {
                        cd.set_cooldown(skill_id, secs as u32);
                        return;
                    }
                }
                let mut cd = oxide_core::SkillCooldowns::default();
                cd.set_cooldown(skill_id, secs as u32);
                let _ = w.insert(entity, (cd,));
            });
        },
    );
    engine.register_fn(
        "is_on_cooldown",
        |entity: Entity, skill_id: String| -> bool {
            with_current_world(|w| {
                if let Ok(mut q) = w.query_one::<&oxide_core::SkillCooldowns>(entity) {
                    q.get()
                        .map(|cd| cd.is_on_cooldown(&skill_id))
                        .unwrap_or(false)
                } else {
                    false
                }
            })
            .unwrap_or(false)
        },
    );

    engine.register_fn(
        "apply_script_effect",
        |target: Entity,
         id: String,
         source: String,
         duration_secs: i64,
         affects_display: String,
         expire_msg: String| {
            with_current_world(|w| {
                let mut expire_conditions = Vec::new();
                if duration_secs > 0 {
                    expire_conditions.push(oxide_core::EffectExpireCondition::Timer);
                }
                let effect = oxide_core::ActiveScriptEffect {
                    id: id.clone(),
                    display_name: source.clone(),
                    source,
                    description: affects_display.clone(),
                    remaining_secs: duration_secs.max(0) as u32,
                    expire_message: if expire_msg.is_empty() {
                        None
                    } else {
                        Some(expire_msg)
                    },
                    affects_display: if affects_display.is_empty() {
                        None
                    } else {
                        Some(affects_display)
                    },
                    show_remaining_time: true,
                    visible_in_affects: true,
                    name_prefix: None,
                    name_suffix: None,
                    short_desc_override: None,
                    visible_on_look: false,
                    look_aura: None,
                    expire_conditions,
                    params: HashMap::new(),
                };
                if let Ok(mut q) = w.query_one::<&mut oxide_core::ActiveScriptEffects>(target) {
                    if let Some(active) = q.get() {
                        active.effects.retain(|e| e.id != id);
                        active.effects.push(effect);
                        return;
                    }
                }
                let mut active = oxide_core::ActiveScriptEffects::default();
                active.effects.push(effect);
                let _ = w.insert(target, (active,));
            });
        },
    );

    engine.register_fn(
        "apply_script_effect_full",
        |target: Entity,
         id: String,
         display_name: String,
         source: String,
         duration_secs: i64,
         affects_display: String,
         name_prefix: String,
         name_suffix: String,
         short_desc_override: String,
         look_aura: String,
         expire_msg: String,
         params: rhai::Map| {
            with_current_world(|w| {
                let mut expire_conditions = Vec::new();
                if let Some(cond_val) = params.get("expire_conditions") {
                    if let Ok(arr) = cond_val.clone().into_array() {
                        for v in arr {
                            if let Ok(s) = v.into_string() {
                                expire_conditions.push(oxide_core::EffectExpireCondition::parse(&s));
                            }
                        }
                    }
                }
                if expire_conditions.is_empty() && duration_secs > 0 {
                    expire_conditions.push(oxide_core::EffectExpireCondition::Timer);
                }

                let parsed_params: HashMap<String, String> = params
                    .into_iter()
                    .filter_map(|(k, v)| v.into_string().ok().map(|s| (k.to_string(), s)))
                    .collect();

                let effect = oxide_core::ActiveScriptEffect {
                    id: id.clone(),
                    display_name: display_name.clone(),
                    source,
                    description: affects_display.clone(),
                    remaining_secs: duration_secs.max(0) as u32,
                    expire_message: if expire_msg.is_empty() {
                        None
                    } else {
                        Some(expire_msg)
                    },
                    affects_display: if affects_display.is_empty() {
                        None
                    } else {
                        Some(affects_display)
                    },
                    show_remaining_time: true,
                    visible_in_affects: true,
                    name_prefix: if name_prefix.is_empty() {
                        None
                    } else {
                        Some(name_prefix)
                    },
                    name_suffix: if name_suffix.is_empty() {
                        None
                    } else {
                        Some(name_suffix)
                    },
                    short_desc_override: if short_desc_override.is_empty() {
                        None
                    } else {
                        Some(short_desc_override)
                    },
                    visible_on_look: !look_aura.is_empty(),
                    look_aura: if look_aura.is_empty() {
                        None
                    } else {
                        Some(look_aura)
                    },
                    expire_conditions,
                    params: parsed_params,
                };
                if let Ok(mut q) = w.query_one::<&mut oxide_core::ActiveScriptEffects>(target) {
                    if let Some(active) = q.get() {
                        active.effects.retain(|e| e.id != id);
                        active.effects.push(effect);
                        return;
                    }
                }
                let mut active = oxide_core::ActiveScriptEffects::default();
                active.effects.push(effect);
                let _ = w.insert(target, (active,));
            });
        },
    );

    engine.register_fn(
        "remove_script_effect",
        |target: Entity, id: String| -> bool {
            with_current_world(|w| {
                if let Ok(mut q) = w.query_one::<&mut oxide_core::ActiveScriptEffects>(target) {
                    if let Some(active) = q.get() {
                        let len_before = active.effects.len();
                        active.effects.retain(|e| e.id != id);
                        return active.effects.len() < len_before;
                    }
                }
                false
            })
            .unwrap_or(false)
        },
    );

    engine.register_fn("has_script_effect", |target: Entity, id: String| -> bool {
        with_current_world(|w| {
            if let Ok(mut q) = w.query_one::<&oxide_core::ActiveScriptEffects>(target) {
                if let Some(active) = q.get() {
                    return active.effects.iter().any(|e| e.id == id);
                }
            }
            false
        })
        .unwrap_or(false)
    });
}
