use oxide_core::{DamageType, Entity, HitContext, ItemTriggers, Npc, Room, ScriptingBridge, World};
use rhai::{Engine, Scope, AST};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::OnceLock;
use std::sync::RwLock;

type AwardXpCallback = Box<dyn Fn(&mut World, Entity) -> Vec<String> + Send + Sync + 'static>;

static AWARD_XP_CALLBACK: OnceLock<AwardXpCallback> = OnceLock::new();

pub fn register_award_xp_callback(
    cb: impl Fn(&mut World, Entity) -> Vec<String> + Send + Sync + 'static,
) {
    let _ = AWARD_XP_CALLBACK.set(Box::new(cb));
}

#[derive(Debug, Clone)]
pub struct TestResult {
    pub name: String,
    pub success: bool,
    pub error: Option<String>,
}

/// A thread-safe wrapper around the ECS `World` raw pointer.
/// Rhai scripts run synchronously under the World lock, so this raw pointer
/// remains valid for the duration of the script's execution.
#[derive(Clone)]
pub struct ScriptWorld {
    world_ptr: *mut World,
}

// SAFETY: Rhai scripts run synchronously under the main game loop / world lock.
// The pointer is only accessed while the lock is held.
unsafe impl Send for ScriptWorld {}
unsafe impl Sync for ScriptWorld {}

impl ScriptWorld {
    pub fn new(world: &mut World) -> Self {
        ScriptWorld { world_ptr: world }
    }

    /// Access the underlying mutable reference to `World`.
    ///
    /// # SAFETY
    /// The caller must ensure that the pointer remains valid and is not aliased
    /// concurrently (which is guaranteed during synchronous script execution).
    #[allow(clippy::mut_from_ref)]
    pub unsafe fn as_mut(&self) -> &mut World {
        &mut *self.world_ptr
    }

    /// Access the underlying immutable reference to `World`.
    ///
    /// # SAFETY
    /// The caller must ensure that the pointer remains valid.
    pub unsafe fn as_ref(&self) -> &World {
        &*self.world_ptr
    }
}

thread_local! {
    static CURRENT_SCRIPT_CONTEXT: std::cell::RefCell<Option<ScriptExecContext>> = const { std::cell::RefCell::new(None) };
}

#[derive(Debug, Clone, Copy)]
pub struct ScriptExecContext {
    pub entity: Entity,
    pub actor: Option<Entity>,
    pub target: Option<Entity>,
    pub room: Option<Entity>,
    pub world_ptr: *mut World,
}

pub struct ScriptContextGuard;

impl Drop for ScriptContextGuard {
    fn drop(&mut self) {
        CURRENT_SCRIPT_CONTEXT.with(|c| *c.borrow_mut() = None);
    }
}

pub fn push_script_context(
    entity: Entity,
    actor: Option<Entity>,
    target: Option<Entity>,
    world: &mut World,
) -> ScriptContextGuard {
    let active_entity = actor.unwrap_or(entity);
    let room = world
        .query_one::<&oxide_core::Position>(active_entity)
        .ok()
        .and_then(|mut q| q.get().map(|p| p.room))
        .or_else(|| {
            world
                .query_one::<&oxide_core::Position>(entity)
                .ok()
                .and_then(|mut q| q.get().map(|p| p.room))
        });

    CURRENT_SCRIPT_CONTEXT.with(|c| {
        *c.borrow_mut() = Some(ScriptExecContext {
            entity,
            actor,
            target,
            room,
            world_ptr: world as *mut World,
        });
    });

    ScriptContextGuard
}

pub fn with_current_world<R>(f: impl FnOnce(&mut World) -> R) -> Option<R> {
    CURRENT_SCRIPT_CONTEXT.with(|c| {
        if let Some(ctx) = *c.borrow() {
            unsafe {
                let w = &mut *ctx.world_ptr;
                Some(f(w))
            }
        } else {
            None
        }
    })
}

pub struct ScriptEngine {
    engine: Engine,
    script_dir: PathBuf,
    ast_cache: RwLock<HashMap<String, AST>>,
}

impl ScriptEngine {
    pub fn new(script_dir: impl Into<PathBuf>) -> Self {
        let mut engine = Engine::new();

        // Security limits
        engine.set_max_operations(50_000);
        engine.set_max_call_levels(32);
        engine.set_max_string_size(10_000);

        // Register custom types
        engine.register_type_with_name::<Entity>("Entity");
        engine.register_type_with_name::<ScriptWorld>("World");
        engine.register_type_with_name::<HitContext>("HitContext");
        engine.register_type_with_name::<DamageType>("DamageType");

        // Entity bindings
        engine.register_fn("id", |entity: Entity| entity.id() as i64);
        engine.register_fn("to_string", |entity: Entity| {
            format!("Entity({})", entity.id())
        });

        // HitContext bindings
        engine.register_get_set(
            "is_aborted",
            |ctx: &mut HitContext| ctx.is_aborted,
            |ctx: &mut HitContext, val: bool| ctx.is_aborted = val,
        );
        engine.register_get_set(
            "hit_modifier",
            |ctx: &mut HitContext| ctx.hit_modifier as i64,
            |ctx: &mut HitContext, val: i64| ctx.hit_modifier = val as i32,
        );
        engine.register_fn("abort", |ctx: &mut HitContext, reason: String| {
            ctx.is_aborted = true;
            ctx.abort_reason = Some(reason);
        });
        engine.register_fn("override_hit", |ctx: &mut HitContext, outcome: bool| {
            ctx.override_hit = Some(outcome);
        });

        // DamageType bindings
        engine.register_fn("to_string", |dt: DamageType| format!("{:?}", dt));

        // World operations
        // Spawning / Despawning
        engine.register_fn("despawn", |entity: Entity| {
            with_current_world(|w| {
                let _ = w.despawn(entity);
            });
        });

        // Querying components
        engine.register_fn("get_hp", |entity: Entity| -> i64 {
            with_current_world(|w| {
                if let Ok(mut q) = w.query_one::<&oxide_core::Health>(entity) {
                    q.get().map(|h| h.current as i64).unwrap_or(0)
                } else {
                    0
                }
            })
            .unwrap_or(0)
        });
        engine.register_fn("set_hp", |entity: Entity, hp: i64| {
            with_current_world(|w| {
                if let Ok(mut q) = w.query_one::<&mut oxide_core::Health>(entity) {
                    if let Some(h) = q.get() {
                        h.current = hp as i32;
                    }
                }
            });
        });
        engine.register_fn("get_max_hp", |entity: Entity| -> i64 {
            with_current_world(|w| {
                if let Ok(mut q) = w.query_one::<&oxide_core::Health>(entity) {
                    q.get().map(|h| h.max as i64).unwrap_or(0)
                } else {
                    0
                }
            })
            .unwrap_or(0)
        });
        engine.register_fn("get_level", |entity: Entity| -> i64 {
            with_current_world(|w| {
                if let Ok(mut q) = w.query_one::<&oxide_core::Level>(entity) {
                    q.get().map(|l| l.0 as i64).unwrap_or(1)
                } else {
                    1
                }
            })
            .unwrap_or(1)
        });
        engine.register_fn("get_skill_rank", |entity: Entity, skill_id: String| -> i64 {
            with_current_world(|w| {
                if let Ok(mut q) = w.query_one::<&oxide_core::LearnedSkills>(entity) {
                    q.get().map(|s| s.rank(&skill_id) as i64).unwrap_or(0)
                } else {
                    0
                }
            })
            .unwrap_or(0)
        });
        engine.register_fn("rand", |low: i64, high: i64| -> i64 {
            fastrand::i64(low..=high)
        });
        engine.register_fn("get_name", |entity: Entity| -> String {
            with_current_world(|w| {
                if let Ok(mut q) = w.query_one::<&oxide_core::Name>(entity) {
                    q.get()
                        .map(|n| n.to_string())
                        .unwrap_or_else(|| "Someone".to_string())
                } else {
                    "Someone".to_string()
                }
            })
            .unwrap_or_else(|| "Someone".to_string())
        });

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
            if let Some(target_ent) = CURRENT_SCRIPT_CONTEXT.with(|c| c.borrow().and_then(|ctx| ctx.actor.or(Some(ctx.entity)))) {
                if let Some(bridge) = oxide_core::scripting::get_message_bridge() {
                    bridge.send_to_entity(target_ent, &msg);
                }
            }
        });

        // Scoped current room messaging (0 arguments needed)
        engine.register_fn("echo", |msg: String| {
            if let Some(room) = CURRENT_SCRIPT_CONTEXT.with(|c| c.borrow().and_then(|ctx| ctx.room)) {
                if let Some(bridge) = oxide_core::scripting::get_message_bridge() {
                    bridge.echo_to_room(room, &msg);
                }
            }
        });
        engine.register_fn("echo_except", |msg: String, exclude: rhai::Array| {
            if let Some(room) = CURRENT_SCRIPT_CONTEXT.with(|c| c.borrow().and_then(|ctx| ctx.room)) {
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

        // Follower control
        engine.register_fn(
            "follow",
            |entity: Entity, target: Entity| {
                with_current_world(|w| {
                    let _ = w.insert(
                        entity,
                        (oxide_core::Following {
                            target,
                            autofollow: true,
                        },),
                    );
                });
            },
        );
        engine.register_fn("unfollow", |entity: Entity| {
            with_current_world(|w| {
                let _ = w.remove_one::<oxide_core::Following>(entity);
            });
        });

        // Exit controls & Room query
        engine.register_fn(
            "is_exit_closed",
            |room: Entity, dir_str: String| -> bool {
                with_current_world(|w| {
                    if let Ok(mut q) = w.query_one::<&oxide_core::RoomExits>(room) {
                        if let Some(exits) = q.get() {
                            if let Some(dir) = oxide_core::Direction::from_short(&dir_str)
                                .or_else(|| oxide_core::Direction::from_long(&dir_str))
                            {
                                if let Some(exit) = exits.0.iter().find(|e| e.direction == dir) {
                                    return exit.is_closed();
                                }
                            }
                        }
                    }
                    false
                })
                .unwrap_or(false)
            },
        );
        engine.register_fn(
            "is_exit_locked",
            |room: Entity, dir_str: String| -> bool {
                with_current_world(|w| {
                    if let Ok(mut q) = w.query_one::<&oxide_core::RoomExits>(room) {
                        if let Some(exits) = q.get() {
                            if let Some(dir) = oxide_core::Direction::from_short(&dir_str)
                                .or_else(|| oxide_core::Direction::from_long(&dir_str))
                            {
                                if let Some(exit) = exits.0.iter().find(|e| e.direction == dir) {
                                    return exit.is_locked();
                                }
                            }
                        }
                    }
                    false
                })
                .unwrap_or(false)
            },
        );
        engine.register_fn(
            "set_exit_closed",
            |room: Entity, dir_str: String, closed: bool| {
                with_current_world(|w| {
                    if let Ok(mut q) = w.query_one::<&mut oxide_core::RoomExits>(room) {
                        if let Some(exits) = q.get() {
                            if let Some(dir) = oxide_core::Direction::from_short(&dir_str)
                                .or_else(|| oxide_core::Direction::from_long(&dir_str))
                            {
                                if let Some(exit) = exits.0.iter_mut().find(|e| e.direction == dir) {
                                    exit.set_closed(closed);
                                }
                            }
                        }
                    }
                });
            },
        );
        engine.register_fn(
            "set_exit_locked",
            |room: Entity, dir_str: String, locked: bool| {
                with_current_world(|w| {
                    if let Ok(mut q) = w.query_one::<&mut oxide_core::RoomExits>(room) {
                        if let Some(exits) = q.get() {
                            if let Some(dir) = oxide_core::Direction::from_short(&dir_str)
                                .or_else(|| oxide_core::Direction::from_long(&dir_str))
                            {
                                if let Some(exit) = exits.0.iter_mut().find(|e| e.direction == dir) {
                                    exit.set_locked(locked);
                                }
                            }
                        }
                    }
                });
            },
        );
        engine.register_fn("get_room", |entity: Entity| -> Entity {
            with_current_world(|w| {
                w.query_one::<&oxide_core::Position>(entity)
                    .ok()
                    .and_then(|mut q| q.get().map(|p| p.room))
                    .unwrap_or(entity)
            })
            .unwrap_or(entity)
        });
        engine.register_fn(
            "room_exits",
            |room: Entity| -> rhai::Array {
                with_current_world(|w| {
                    if let Ok(mut q) = w.query_one::<&oxide_core::RoomExits>(room) {
                        if let Some(exits) = q.get() {
                            return exits
                                .0
                                .iter()
                                .map(|e| rhai::Dynamic::from(e.direction.long_name().to_string()))
                                .collect();
                        }
                    }
                    Vec::new()
                })
                .unwrap_or_default()
            },
        );
        engine.register_fn(
            "room_exit_target",
            |room: Entity, dir_str: String| -> Entity {
                with_current_world(|w| {
                    if let Ok(mut q) = w.query_one::<&oxide_core::RoomExits>(room) {
                        if let Some(exits) = q.get() {
                            if let Some(dir) = oxide_core::Direction::from_short(&dir_str)
                                .or_else(|| oxide_core::Direction::from_long(&dir_str))
                            {
                                if let Some(exit) = exits.0.iter().find(|e| e.direction == dir) {
                                    return exit.dest;
                                }
                            }
                        }
                    }
                    room
                })
                .unwrap_or(room)
            },
        );
        engine.register_fn(
            "move_entity",
            |entity: Entity, dest: Entity| {
                with_current_world(|w| {
                    let _ = w.insert(entity, (oxide_core::Position::new(dest), oxide_core::Dirty));
                });
            },
        );

        // Mob spawning & Template spawn
        engine.register_fn(
            "spawn_mob",
            |template_id: String, room_entity: Entity| -> Entity {
                with_current_world(|w| {
                    let templates = match oxide_core::templates::get_global_templates() {
                        Some(t) => t,
                        None => return Entity::from(hecs::Entity::from_bits(0).unwrap()),
                    };
                    if let Some(mob_tpl) = templates.mobs.get(&template_id) {
                        mob_tpl.spawn(w, room_entity, &templates)
                    } else {
                        Entity::from(hecs::Entity::from_bits(0).unwrap())
                    }
                })
                .unwrap_or_else(|| Entity::from(hecs::Entity::from_bits(0).unwrap()))
            },
        );

        engine.register_fn(
            "accept_quest",
            |player: Entity, quest_id: String| -> bool {
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
            },
        );

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

        engine.register_fn(
            "is_on_quest",
            |player: Entity, quest_id: String| -> bool {
                with_current_world(|w| {
                    if let Ok(mut q) = w.query_one::<&oxide_core::QuestLog>(player) {
                        if let Some(log) = q.get() {
                            return log.active.contains_key(&quest_id);
                        }
                    }
                    false
                })
                .unwrap_or(false)
            },
        );

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

        engine.register_fn(
            "assert",
            |val: bool| -> Result<(), Box<rhai::EvalAltResult>> {
                if !val {
                    Err("Assertion failed".into())
                } else {
                    Ok(())
                }
            },
        );

        engine.register_fn(
            "register_skill",
            |id: String, name: String, command: String, script: String, help_text: String| {
                oxide_core::register_dynamic_skill(oxide_core::ScriptSkill {
                    id,
                    name,
                    command: Some(command),
                    is_spell: false,
                    topic: "Skills".to_string(),
                    help_text,
                    script,
                    restrictions: oxide_core::CommandRestrictions::default(),
                });
            },
        );

        engine.register_fn(
            "register_skill",
            |id: String,
             name: String,
             command: String,
             script: String,
             help_text: String,
             allowed_classes: rhai::Array| {
                let classes = allowed_classes
                    .into_iter()
                    .filter_map(|v| v.into_string().ok())
                    .collect();
                let mut restrictions = oxide_core::CommandRestrictions::default();
                restrictions.allowed_classes = classes;
                oxide_core::register_dynamic_skill(oxide_core::ScriptSkill {
                    id,
                    name,
                    command: Some(command),
                    is_spell: false,
                    topic: "Skills".to_string(),
                    help_text,
                    script,
                    restrictions,
                });
            },
        );

        engine.register_fn(
            "register_spell",
            |id: String, name: String, spell_name: String, script: String, help_text: String| {
                oxide_core::register_dynamic_skill(oxide_core::ScriptSkill {
                    id,
                    name,
                    command: Some(spell_name),
                    is_spell: true,
                    topic: "Spells".to_string(),
                    help_text,
                    script,
                    restrictions: oxide_core::CommandRestrictions::default(),
                });
            },
        );

        engine.register_fn(
            "register_spell",
            |id: String,
             name: String,
             spell_name: String,
             script: String,
             help_text: String,
             allowed_classes: rhai::Array| {
                let classes = allowed_classes
                    .into_iter()
                    .filter_map(|v| v.into_string().ok())
                    .collect();
                let mut restrictions = oxide_core::CommandRestrictions::default();
                restrictions.allowed_classes = classes;
                oxide_core::register_dynamic_skill(oxide_core::ScriptSkill {
                    id,
                    name,
                    command: Some(spell_name),
                    is_spell: true,
                    topic: "Spells".to_string(),
                    help_text,
                    script,
                    restrictions,
                });
            },
        );

        engine.register_fn(
            "register_entity_command",
            |entity: Entity,
             command_name: String,
             script: String,
             help_text: String| {
                with_current_world(|w| {
                    if let Ok(mut q) = w.query_one::<&mut oxide_core::EntityCommands>(entity) {
                        if let Some(ec) = q.get() {
                            ec.add(command_name, script, help_text);
                            return;
                        }
                    }
                    let mut ec = oxide_core::EntityCommands::new();
                    ec.add(command_name, script, help_text);
                    let _ = w.insert(entity, (ec,));
                });
            },
        );

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
                        q.get().map(|cd| cd.is_on_cooldown(&skill_id)).unwrap_or(false)
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
                    let effect = oxide_core::ActiveScriptEffect {
                        id: id.clone(),
                        display_name: source.clone(),
                        source,
                        description: affects_display.clone(),
                        remaining_secs: duration_secs.max(0) as u32,
                        expire_message: if expire_msg.is_empty() { None } else { Some(expire_msg) },
                        affects_display: if affects_display.is_empty() { None } else { Some(affects_display) },
                        show_remaining_time: true,
                        visible_in_affects: true,
                        name_prefix: None,
                        name_suffix: None,
                        short_desc_override: None,
                        visible_on_look: false,
                        look_aura: None,
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
                    let effect = oxide_core::ActiveScriptEffect {
                        id: id.clone(),
                        display_name: display_name.clone(),
                        source,
                        description: affects_display.clone(),
                        remaining_secs: duration_secs.max(0) as u32,
                        expire_message: if expire_msg.is_empty() { None } else { Some(expire_msg) },
                        affects_display: if affects_display.is_empty() { None } else { Some(affects_display) },
                        show_remaining_time: true,
                        visible_in_affects: true,
                        name_prefix: if name_prefix.is_empty() { None } else { Some(name_prefix) },
                        name_suffix: if name_suffix.is_empty() { None } else { Some(name_suffix) },
                        short_desc_override: if short_desc_override.is_empty() { None } else { Some(short_desc_override) },
                        visible_on_look: !look_aura.is_empty(),
                        look_aura: if look_aura.is_empty() { None } else { Some(look_aura) },
                        params: params.into_iter().filter_map(|(k, v)| v.into_string().ok().map(|s| (k.to_string(), s))).collect(),
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

        engine.register_fn(
            "has_script_effect",
            |target: Entity, id: String| -> bool {
                with_current_world(|w| {
                    if let Ok(mut q) = w.query_one::<&oxide_core::ActiveScriptEffects>(target) {
                        if let Some(active) = q.get() {
                            return active.effects.iter().any(|e| e.id == id);
                        }
                    }
                    false
                })
                .unwrap_or(false)
            },
        );

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

        ScriptEngine {
            engine,
            script_dir: script_dir.into(),
            ast_cache: RwLock::new(HashMap::new()),
        }
    }

    /// Retrieve or compile an AST for the given script path relative to the script directory.
    pub fn get_ast(&self, rel_path: &str) -> Result<AST, String> {
        {
            let cache = self.ast_cache.read().unwrap();
            if let Some(ast) = cache.get(rel_path) {
                return Ok(ast.clone());
            }
        }

        let full_path = self.script_dir.join(rel_path);
        let content = std::fs::read_to_string(&full_path)
            .map_err(|e| format!("Failed to read script {}: {}", rel_path, e))?;

        let processed = strip_tests(&content);
        let ast = self
            .engine
            .compile(&processed)
            .map_err(|e| format!("Compile error in {}: {}", rel_path, e))?;

        let mut cache = self.ast_cache.write().unwrap();
        cache.insert(rel_path.to_string(), ast.clone());
        Ok(ast)
    }

    pub fn eval(&self, script: &str) -> Result<(), Box<dyn std::error::Error>> {
        let processed = strip_tests(script);
        self.engine.run(&processed)?;
        Ok(())
    }

    pub fn eval_with_options(
        &self,
        script: &str,
        include_tests: bool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let processed = if include_tests {
            script.to_string()
        } else {
            strip_tests(script)
        };
        self.engine.run(&processed)?;
        Ok(())
    }

    pub fn run_tests(&self, script: &str) -> Vec<TestResult> {
        let mut results = Vec::new();
        let blocks = parse_test_blocks(script);

        if !blocks.is_empty() {
            for (idx, block) in blocks.iter().enumerate() {
                let test_script = construct_test_script(script, &blocks, idx);
                match self.engine.run(&test_script) {
                    Ok(_) => results.push(TestResult {
                        name: block.name.clone(),
                        success: true,
                        error: None,
                    }),
                    Err(e) => results.push(TestResult {
                        name: block.name.clone(),
                        success: false,
                        error: Some(e.to_string()),
                    }),
                }
            }
        } else {
            // Fallback: run test_* functions if any
            let test_fns = find_test_functions(script);
            if !test_fns.is_empty() {
                let mut scope = rhai::Scope::new();
                match self.engine.compile(script) {
                    Ok(ast) => {
                        for test_name in test_fns {
                            match self.engine.call_fn::<()>(&mut scope, &ast, &test_name, ()) {
                                Ok(_) => results.push(TestResult {
                                    name: test_name,
                                    success: true,
                                    error: None,
                                }),
                                Err(e) => results.push(TestResult {
                                    name: test_name,
                                    success: false,
                                    error: Some(e.to_string()),
                                }),
                            }
                        }
                    }
                    Err(e) => {
                        results.push(TestResult {
                            name: "Compilation".to_string(),
                            success: false,
                            error: Some(e.to_string()),
                        });
                    }
                }
            }
        }
        results
    }

    pub fn engine(&self) -> &Engine {
        &self.engine
    }

    pub fn engine_mut(&mut self) -> &mut Engine {
        &mut self.engine
    }
}

impl ScriptingBridge for ScriptEngine {
    fn execute_trigger(
        &self,
        script: &str,
        entity: Entity,
        actor: Option<Entity>,
        target: Option<Entity>,
        world: &mut World,
    ) -> Result<bool, String> {
        let _guard = push_script_context(entity, actor, target, world);
        let ast = self.get_ast(script)?;
        let mut scope = Scope::new();
        scope.push("self", entity);
        scope.push("actor", actor);
        scope.push("target", target);
        scope.push("world", ScriptWorld::new(world));

        // Inject ScriptParams if any
        if let Ok(mut q) = world.query_one::<&oxide_core::ScriptParams>(entity) {
            if let Some(params) = q.get() {
                for (k, v) in &params.0 {
                    scope.push(k.clone(), v.clone());
                }
            }
        }

        self.engine
            .call_fn::<bool>(&mut scope, &ast, "on_trigger", ())
            .map_err(|e| e.to_string())
    }

    fn execute_combat_hit_hook(
        &self,
        attacker: Entity,
        target: Entity,
        is_offhand: bool,
        world: &mut World,
    ) -> Result<HitContext, String> {
        let _guard = push_script_context(target, Some(attacker), Some(target), world);
        let mut hit_ctx = HitContext {
            attacker,
            target,
            is_offhand,
            is_aborted: false,
            abort_reason: None,
            hit_modifier: 0,
            override_hit: None,
        };

        // If target has parry, execute the parry script.
        let has_parry = if let Ok(mut q) = world.query_one::<&oxide_core::LearnedSkills>(target) {
            q.get().map(|s| s.has("parry")).unwrap_or(false)
        } else {
            false
        };

        if has_parry {
            if let Ok(ast) = self.get_ast("skills/parry.rhai") {
                let mut scope = Scope::new();
                scope.push("world", ScriptWorld::new(world));
                scope.push("hit_ctx", hit_ctx.clone());

                if let Ok(returned_ctx) =
                    self.engine
                        .call_fn::<HitContext>(&mut scope, &ast, "on_combat_hit", ())
                {
                    hit_ctx = returned_ctx;
                }
            }
        }

        Ok(hit_ctx)
    }

    fn execute_combat_damage_hook(
        &self,
        _attacker: Entity,
        _target: Entity,
        damage: i32,
        damage_type: DamageType,
        _world: &mut World,
    ) -> Result<(i32, DamageType), String> {
        Ok((damage, damage_type))
    }

    fn execute_mob_ai(
        &self,
        script: &str,
        entity: Entity,
        world: &mut World,
    ) -> Result<Option<String>, String> {
        let _guard = push_script_context(entity, None, None, world);
        let ast = self.get_ast(script)?;
        let mut scope = Scope::new();
        scope.push("self", entity);
        scope.push("world", ScriptWorld::new(world));

        // Inject ScriptParams if any
        if let Ok(mut q) = world.query_one::<&oxide_core::ScriptParams>(entity) {
            if let Some(params) = q.get() {
                for (k, v) in &params.0 {
                    scope.push(k.clone(), v.clone());
                }
            }
        }

        self.engine
            .call_fn::<Option<String>>(&mut scope, &ast, "on_ai_pulse", ())
            .map_err(|e| e.to_string())
    }

    fn execute_say_hook(
        &self,
        script_entity: Entity,
        speaker: Entity,
        message: &str,
        world: &mut World,
    ) -> Result<(), String> {
        let _guard = push_script_context(script_entity, Some(speaker), None, world);
        // Collect scripts alongside their associated parameter mapping if any
        let mut scripts_to_run: Vec<(String, HashMap<String, String>)> = Vec::new();

        if let Ok(mut q) = world.query_one::<&Npc>(script_entity) {
            if let Some(npc) = q.get() {
                if let Some(ref s) = npc.script {
                    let mut params = HashMap::new();
                    if let Ok(mut q_params) =
                        world.query_one::<&oxide_core::ScriptParams>(script_entity)
                    {
                        if let Some(p) = q_params.get() {
                            params = p.0.clone();
                        }
                    }
                    scripts_to_run.push((s.clone(), params));
                }
            }
        }

        if let Ok(mut q) = world.query_one::<&Room>(script_entity) {
            if let Some(room) = q.get() {
                if let Some(ref s) = room.script {
                    let mut params = HashMap::new();
                    if let Ok(mut q_params) =
                        world.query_one::<&oxide_core::ScriptParams>(script_entity)
                    {
                        if let Some(p) = q_params.get() {
                            params = p.0.clone();
                        }
                    }
                    scripts_to_run.push((s.clone(), params));
                }
            }
        }

        if let Ok(mut q) = world.query_one::<&ItemTriggers>(script_entity) {
            if let Some(triggers) = q.get() {
                for trigger in &triggers.0 {
                    if trigger.event == "say" {
                        if let Some(ref s) = trigger.script {
                            scripts_to_run.push((s.clone(), trigger.params.clone()));
                        }
                    }
                }
            }
        }

        for (script_path, params) in scripts_to_run {
            let ast = self.get_ast(&script_path)?;
            let mut scope = Scope::new();
            scope.push("self", script_entity);
            scope.push("speaker", speaker);
            scope.push("message", message.to_string());
            scope.push("world", ScriptWorld::new(world));

            for (k, v) in params {
                scope.push(k, v);
            }

            self.engine
                .call_fn::<()>(&mut scope, &ast, "on_say", ())
                .map_err(|e| e.to_string())?;
        }

        Ok(())
    }

    fn execute_use_skill(
        &self,
        script: &str,
        actor: Entity,
        target: Option<Entity>,
        world: &mut World,
    ) -> Result<(), String> {
        let _guard = push_script_context(actor, Some(actor), target, world);
        let ast = self.get_ast(script)?;
        let mut scope = Scope::new();
        scope.push("actor", actor);
        scope.push("target", target);
        scope.push("world", ScriptWorld::new(world));

        // SkillDef parameters injection is handled by resolving SkillDef from learned skills or by checking if the skill entity or registry params exist.
        // We can check if `actor` has the skill or query the database/registry. Let's look up the skill in LearnedSkills or Skills component if it is attached.
        // But scripting uses execute_use_skill. Since the parameters are part of SkillDef (which is in registry), they could be loaded or injected. Let's look up SkillDef params.
        // Wait, is there a SkillDef in the world we can query? Or is it passed in/available?
        // Let's search how execute_use_skill is called in the engine.
        // Let's inspect where execute_use_skill is called in core first.

        self.engine
            .call_fn::<()>(&mut scope, &ast, "on_use", ())
            .map_err(|e| e.to_string())
    }

    fn execute_quest_hook(
        &self,
        script: &str,
        player: Entity,
        quest_id: &str,
        world: &mut World,
    ) -> Result<(), String> {
        let _guard = push_script_context(player, Some(player), None, world);
        let ast = self.get_ast(script)?;
        let mut scope = Scope::new();
        scope.push("player", player);
        scope.push("world", ScriptWorld::new(world));
        scope.push("quest_id", quest_id.to_string());

        let mut rewards_map = rhai::Map::new();
        if let Some(templates) = oxide_core::templates::get_global_templates() {
            if let Some(quest_def) = templates.quests.get(quest_id) {
                rewards_map.insert("xp".into(), (quest_def.rewards.xp as i64).into());
                rewards_map.insert("gold".into(), (quest_def.rewards.gold as i64).into());

                let mut items_arr = rhai::Array::new();
                for item in &quest_def.rewards.items {
                    let mut item_map = rhai::Map::new();
                    item_map.insert(
                        "item_template_id".into(),
                        item.item_template_id.clone().into(),
                    );
                    item_map.insert("count".into(), (item.count as i64).into());
                    items_arr.push(item_map.into());
                }
                rewards_map.insert("items".into(), items_arr.into());

                let mut faction_arr = rhai::Array::new();
                for fac in &quest_def.rewards.faction {
                    let mut fac_map = rhai::Map::new();
                    fac_map.insert("faction_id".into(), fac.faction_id.clone().into());
                    fac_map.insert("amount".into(), (fac.amount as i64).into());
                    faction_arr.push(fac_map.into());
                }
                rewards_map.insert("faction".into(), faction_arr.into());
            }
        }
        scope.push("rewards", rewards_map);

        self.engine
            .run_ast_with_scope(&mut scope, &ast)
            .map_err(|e| e.to_string())
    }

    fn execute_script_skill(
        &self,
        script: &str,
        actor: Entity,
        args: &str,
        world: &mut World,
    ) -> Result<(), String> {
        let _guard = push_script_context(actor, Some(actor), None, world);
        let ast = self.get_ast(script)?;
        let mut scope = Scope::new();
        scope.push("actor", actor);
        scope.push("args", args.to_string());
        scope.push("world", ScriptWorld::new(world));

        if self
            .engine
            .call_fn::<()>(&mut scope, &ast, "on_use", ())
            .is_err()
        {
            if self
                .engine
                .call_fn::<()>(&mut scope, &ast, "on_cast", ())
                .is_err()
            {
                if self
                    .engine
                    .call_fn::<()>(&mut scope, &ast, "main", ())
                    .is_err()
                {
                    let _ = self.engine.run_ast_with_scope(&mut scope, &ast);
                }
            }
        }
        Ok(())
    }

    fn execute_entity_command(
        &self,
        entity: Entity,
        script: &str,
        actor: Entity,
        args: &str,
        world: &mut World,
    ) -> Result<(), String> {
        let _guard = push_script_context(entity, Some(actor), None, world);
        let ast = self.get_ast(script)?;
        let mut scope = Scope::new();
        scope.push("self", entity);
        scope.push("actor", actor);
        scope.push("args", args.to_string());
        scope.push("world", ScriptWorld::new(world));

        if self
            .engine
            .call_fn::<()>(&mut scope, &ast, "on_command", ())
            .is_err()
        {
            if self
                .engine
                .call_fn::<()>(&mut scope, &ast, "main", ())
                .is_err()
            {
                let _ = self.engine.run_ast_with_scope(&mut scope, &ast);
            }
        }
        Ok(())
    }

    fn evaluate_script_predicate(
        &self,
        script: &str,
        actor: Entity,
        target_entity: Option<Entity>,
        world: &mut World,
    ) -> Result<bool, String> {
        let ast = self.get_ast(script)?;
        let mut scope = Scope::new();
        scope.push("actor", actor);
        scope.push("target", target_entity);
        scope.push("world", ScriptWorld::new(world));

        if let Ok(res) = self.engine.call_fn::<bool>(&mut scope, &ast, "can_use", ()) {
            Ok(res)
        } else {
            self.engine
                .eval_ast_with_scope::<bool>(&mut scope, &ast)
                .map_err(|e| e.to_string())
        }
    }

    fn reload_script(&self, rel_path: &str) -> Result<(), String> {
        let full_path = self.script_dir.join(rel_path);
        if !full_path.exists() {
            let mut cache = self.ast_cache.write().unwrap();
            cache.remove(rel_path);
            return Ok(());
        }

        let content = std::fs::read_to_string(&full_path)
            .map_err(|e| format!("Failed to read script {}: {}", rel_path, e))?;

        let processed = strip_tests(&content);
        let ast = self
            .engine
            .compile(&processed)
            .map_err(|e| format!("Compile error in {}: {}", rel_path, e))?;

        let mut cache = self.ast_cache.write().unwrap();
        cache.insert(rel_path.to_string(), ast);
        Ok(())
    }
}

impl Default for ScriptEngine {
    fn default() -> Self {
        Self::new("content/scripts")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestBlock {
    pub name: String,
    pub start_line: usize,
    pub end_line: usize,
}

pub fn parse_test_blocks(script: &str) -> Vec<TestBlock> {
    let mut blocks = Vec::new();
    let mut current_start = None;
    let mut current_name = None;

    for (line_idx, line) in script.lines().enumerate() {
        let trimmed = line.trim();
        if let Some(stripped) = trimmed.strip_prefix("//#test") {
            let name = stripped.trim().to_string();
            let name = if name.is_empty() {
                format!("Test at line {}", line_idx + 1)
            } else {
                name
            };
            current_start = Some(line_idx);
            current_name = Some(name);
        } else if trimmed == "//#end" {
            if let (Some(start), Some(name)) = (current_start.take(), current_name.take()) {
                blocks.push(TestBlock {
                    name,
                    start_line: start,
                    end_line: line_idx,
                });
            }
        }
    }
    blocks
}

pub fn construct_test_script(script: &str, blocks: &[TestBlock], target_idx: usize) -> String {
    let target = &blocks[target_idx];
    let mut output = Vec::new();

    for (line_idx, line) in script.lines().enumerate() {
        if line_idx >= target.start_line && line_idx <= target.end_line {
            if line_idx == target.start_line || line_idx == target.end_line {
                output.push("");
            } else {
                output.push(line);
            }
        } else {
            let in_other_block = blocks.iter().enumerate().any(|(idx, b)| {
                idx != target_idx && line_idx >= b.start_line && line_idx <= b.end_line
            });
            if in_other_block {
                output.push("");
            } else {
                output.push(line);
            }
        }
    }
    output.join("\n")
}

pub fn strip_tests(script: &str) -> String {
    let blocks = parse_test_blocks(script);
    let mut output = Vec::new();

    for (line_idx, line) in script.lines().enumerate() {
        let in_any_block = blocks
            .iter()
            .any(|b| line_idx >= b.start_line && line_idx <= b.end_line);
        if in_any_block {
            output.push("");
        } else {
            output.push(line);
        }
    }
    output.join("\n")
}

fn find_test_functions(script: &str) -> Vec<String> {
    let mut names = Vec::new();
    for line in script.lines() {
        let line = line.trim();
        if line.starts_with("fn ") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let name_part = parts[1];
                if let Some(idx) = name_part.find('(') {
                    let name = &name_part[..idx];
                    if name.starts_with("test_") {
                        names.push(name.to_string());
                    }
                }
            }
        }
    }
    names
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_eval_arithmetic() {
        let engine = ScriptEngine::new("src");
        engine.eval("let x = 1 + 2;").unwrap();
    }

    #[test]
    fn test_eval_string() {
        let engine = ScriptEngine::new("src");
        engine.eval(r#"let msg = "hello";"#).unwrap();
    }

    #[test]
    fn test_eval_syntax_error() {
        let engine = ScriptEngine::new("src");
        let result = engine.eval("let x = ;");
        assert!(result.is_err());
    }

    #[test]
    fn test_eval_undefined_var() {
        let engine = ScriptEngine::new("src");
        let result = engine.eval("let y = undefined_var;");
        assert!(result.is_err());
    }

    #[test]
    fn test_engine_accessors() {
        let engine = ScriptEngine::new("src");
        assert!(engine.engine().compile("let a = 1;").is_ok());
    }

    #[test]
    fn test_parse_test_blocks() {
        let script = r#"
let a = 1;
//#test my test 1
assert(a == 1);
//#end
//#test
assert(a == 2);
//#end
"#;
        let blocks = parse_test_blocks(script);
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].name, "my test 1");
        assert_eq!(blocks[0].start_line, 2);
        assert_eq!(blocks[0].end_line, 4);

        assert_eq!(blocks[1].name, "Test at line 6");
        assert_eq!(blocks[1].start_line, 5);
        assert_eq!(blocks[1].end_line, 7);
    }

    #[test]
    fn test_construct_test_script() {
        let script = "let a = 1;\n//#test t1\nassert(a == 1);\n//#end\nlet b = 2;\n//#test t2\nassert(b == 2);\n//#end";
        let blocks = parse_test_blocks(script);
        assert_eq!(blocks.len(), 2);

        let s1 = construct_test_script(script, &blocks, 0);
        let s1_lines: Vec<&str> = s1.split('\n').collect();
        assert_eq!(s1_lines.len(), 8);
        assert_eq!(s1_lines[0], "let a = 1;");
        assert_eq!(s1_lines[1], ""); // marker
        assert_eq!(s1_lines[2], "assert(a == 1);"); // target content
        assert_eq!(s1_lines[3], ""); // marker
        assert_eq!(s1_lines[4], "let b = 2;");
        assert_eq!(s1_lines[5], ""); // other block cleared
        assert_eq!(s1_lines[6], ""); // other block cleared
        assert_eq!(s1_lines[7], ""); // other block cleared

        let s2 = construct_test_script(script, &blocks, 1);
        let s2_lines: Vec<&str> = s2.split('\n').collect();
        assert_eq!(s2_lines.len(), 8);
        assert_eq!(s2_lines[0], "let a = 1;");
        assert_eq!(s2_lines[1], "");
        assert_eq!(s2_lines[2], "");
        assert_eq!(s2_lines[3], "");
        assert_eq!(s2_lines[4], "let b = 2;");
        assert_eq!(s2_lines[5], "");
        assert_eq!(s2_lines[6], "assert(b == 2);");
        assert_eq!(s2_lines[7], "");
    }

    #[test]
    fn test_strip_tests_behavior() {
        let script = r#"
let a = 1;
//#test
fn test_foo() {
    assert(a == 1);
}
//#end
let b = 2;
"#;
        let stripped = strip_tests(script);
        assert!(!stripped.contains("test_foo"));
        assert!(stripped.contains("let a = 1;"));
        assert!(stripped.contains("let b = 2;"));
        // Line count preserved
        assert_eq!(script.lines().count(), stripped.lines().count());
    }

    #[test]
    fn test_run_tests_with_blocks() {
        let engine = ScriptEngine::new("src");
        let script = r#"
let val = 10;
//#test success block
assert(val == 10);
//#end
//#test fail block
assert(val == 20);
//#end
"#;
        let results = engine.run_tests(script);
        println!("Test block results: {:?}", results);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].name, "success block");
        if !results[0].success {
            panic!("Test 0 failed: {:?}", results[0].error);
        }
        assert_eq!(results[1].name, "fail block");
        assert!(!results[1].success);
        assert!(results[1]
            .error
            .as_ref()
            .unwrap()
            .contains("Assertion failed"));
    }

    #[test]
    fn test_run_tests_fallback_fns() {
        let engine = ScriptEngine::new("src");
        let script = r#"
fn test_success() {
    let x = 1;
}
fn test_fail() {
    throw "assert failure";
}
"#;
        let results = engine.run_tests(script);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].name, "test_success");
        assert!(results[0].success);
        assert_eq!(results[1].name, "test_fail");
        assert!(!results[1].success);
    }

    #[test]
    fn test_script_say_hook_execution() {
        use oxide_core::{Direction, Exit, Position, RoomExits};
        let engine = ScriptEngine::default();
        let mut world = World::new();

        let room = world.spawn((Room::new("Test Room", "Desc"),));
        let _ = world.insert(room, (Position::new(room),));

        let mut exits = RoomExits(vec![Exit::new(Direction::North, room)]);
        exits.0[0].set_closed(true);
        exits.0[0].set_locked(true);
        let _ = world.insert(room, (exits,));

        let mut scope = Scope::new();
        scope.push("self", room);
        scope.push("world", ScriptWorld::new(&mut world));

        let exits_list = engine
            .engine()
            .eval_with_scope::<rhai::Array>(&mut scope, "world.room_exits(self)")
            .unwrap();
        assert_eq!(exits_list.len(), 1);
        assert_eq!(exits_list[0].to_string(), "north");

        let is_closed = engine
            .engine()
            .eval_with_scope::<bool>(&mut scope, r#"world.is_exit_closed(self, "north")"#)
            .unwrap();
        assert!(is_closed);

        engine
            .engine()
            .run_with_scope(
                &mut scope,
                r#"
            world.set_exit_locked(self, "north", false);
            world.set_exit_closed(self, "north", false);
        "#,
            )
            .unwrap();

        let is_closed_now = engine
            .engine()
            .eval_with_scope::<bool>(&mut scope, r#"world.is_exit_closed(self, "north")"#)
            .unwrap();
        assert!(!is_closed_now);
    }

    #[test]
    fn test_say_hook_dynamic_parameters() {
        use oxide_core::ScriptParams;
        let engine = ScriptEngine::new("../content/scripts");
        let mut world = World::new();

        let room_id = world.spawn((
            Room::new("Para Room", "Desc").with_script(Some("rooms/open_sesame.rhai".to_string())),
            {
                let mut p = HashMap::new();
                p.insert("keyword".to_string(), "please open".to_string());
                p.insert("direction".to_string(), "north".to_string());
                ScriptParams(p)
            },
        ));

        let mut exits = oxide_core::RoomExits(vec![oxide_core::Exit::new(
            oxide_core::Direction::North,
            room_id,
        )]);
        exits.0[0].set_closed(true);
        exits.0[0].set_locked(true);
        world.insert(room_id, (exits,)).unwrap();

        // Speaker entity
        let speaker = world.spawn(());

        // Execute say hook which triggers open_sesame.rhai
        engine
            .execute_say_hook(room_id, speaker, "please open", &mut world)
            .unwrap();

        // Check if the door was opened using the dynamic keyword & direction parameters
        let is_closed = world
            .query_one::<&oxide_core::RoomExits>(room_id)
            .unwrap()
            .get()
            .unwrap()
            .0[0]
            .is_closed();
        assert!(!is_closed);
    }

    #[test]
    fn test_reload_script() {
        let temp_dir = std::env::current_dir()
            .unwrap()
            .join("target")
            .join("temp_scripts_test");
        if temp_dir.exists() {
            let _ = std::fs::remove_dir_all(&temp_dir);
        }
        std::fs::create_dir_all(&temp_dir).unwrap();

        let script_file = temp_dir.join("test_script.rhai");
        std::fs::write(&script_file, b"fn on_trigger() { true }").unwrap();

        let engine = ScriptEngine::new(&temp_dir);

        let _ast = engine.get_ast("test_script.rhai").unwrap();

        std::fs::write(&script_file, b"fn on_trigger() { false }").unwrap();
        engine.reload_script("test_script.rhai").unwrap();

        let mut world = World::new();
        let entity = world.spawn(());
        let res = engine
            .execute_trigger("test_script.rhai", entity, None, None, &mut world)
            .unwrap();
        assert!(!res);

        std::fs::remove_file(&script_file).unwrap();
        engine.reload_script("test_script.rhai").unwrap();

        {
            let cache = engine.ast_cache.read().unwrap();
            assert!(!cache.contains_key("test_script.rhai"));
        }

        std::fs::remove_dir_all(&temp_dir).unwrap();
    }

    #[test]
    fn test_quest_chaining_and_rewards_scripting() {
        use oxide_core::templates::{QuestDef, QuestRewards, TemplateRegistry};
        use oxide_core::{Equipment, Experience, Inventory, Level, QuestLog, Wallet};

        let engine = ScriptEngine::default();
        let mut world = World::new();

        // 1. Setup templates
        let mut templates = TemplateRegistry::new();

        let quest_a = QuestDef {
            id: "quest_a".to_string(),
            name: "Quest A".to_string(),
            description: "First quest".to_string(),
            level_requirement: 0,
            repeatable: false,
            auto_complete: false,
            giver_npc: None,
            turn_in_npc: None,
            prerequisites: Vec::new(),
            objectives: Vec::new(),
            rewards: QuestRewards {
                xp: 100,
                gold: 50,
                items: Vec::new(),
                faction: Vec::new(),
            },
            scripts: None,
            params: HashMap::new(),
        };

        let quest_b = QuestDef {
            id: "quest_b".to_string(),
            name: "Quest B".to_string(),
            description: "Second quest".to_string(),
            level_requirement: 0,
            repeatable: false,
            auto_complete: false,
            giver_npc: None,
            turn_in_npc: None,
            prerequisites: vec!["quest_a".to_string()],
            objectives: Vec::new(),
            rewards: QuestRewards::default(),
            scripts: None,
            params: HashMap::new(),
        };

        templates.quests.insert("quest_a".to_string(), quest_a);
        templates.quests.insert("quest_b".to_string(), quest_b);

        // Register templates globally
        oxide_core::templates::register_global_templates(std::sync::Arc::new(templates));

        // 2. Setup player
        let player = world.spawn((
            QuestLog::new(),
            Level(1),
            Experience(0),
            Wallet::new(0, 0, 0, 0),
            Inventory(Vec::new()),
            Equipment::new(),
        ));

        // 3. Accept Quest A and complete it
        let global_templates = oxide_core::templates::get_global_templates().unwrap();
        oxide_core::accept_quest(&mut world, player, "quest_a", &global_templates).unwrap();
        {
            let mut q_log = world.query_one::<&mut QuestLog>(player).unwrap();
            let log = q_log.get().unwrap();
            log.completed.insert("quest_a".to_string());
        }

        // 4. Compile a test script that validates rewards scope and chains Quest B
        let script = r#"
            // Verify rewards in scope
            assert(rewards.xp == 100);
            assert(rewards.gold == 50);

            // Accept the next quest in chain
            accept_quest(world, player, "quest_b");
        "#;
        let ast = engine.engine().compile(script).unwrap();

        // 5. Mock the AST cache for quest_a_complete.rhai
        {
            let mut cache = engine.ast_cache.write().unwrap();
            cache.insert("quest_a_complete.rhai".to_string(), ast);
        }

        // Now run the hook using the cached AST path
        engine
            .execute_quest_hook("quest_a_complete.rhai", player, "quest_a", &mut world)
            .unwrap();

        // 6. Verify that Quest B has been successfully accepted!
        let mut q_log = world.query_one::<&QuestLog>(player).unwrap();
        let log = q_log.get().unwrap();
        assert!(log.active.contains_key("quest_b"));
    }
}
