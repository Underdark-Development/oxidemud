use crate::context::with_current_world;
use oxide_core::Entity;
use rhai::Engine;

pub fn register(engine: &mut Engine) {
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
    engine.register_fn(
        "get_skill_rank",
        |entity: Entity, skill_id: String| -> i64 {
            with_current_world(|w| {
                if let Ok(mut q) = w.query_one::<&oxide_core::LearnedSkills>(entity) {
                    q.get().map(|s| s.rank(&skill_id) as i64).unwrap_or(0)
                } else {
                    0
                }
            })
            .unwrap_or(0)
        },
    );
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
    engine.register_fn("name", |entity: Entity| -> String {
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
    engine.register_get("name", |entity: &mut Entity| -> String {
        let ent = *entity;
        with_current_world(|w| {
            if let Ok(mut q) = w.query_one::<&oxide_core::Name>(ent) {
                q.get()
                    .map(|n| n.to_string())
                    .unwrap_or_else(|| "Someone".to_string())
            } else {
                "Someone".to_string()
            }
        })
        .unwrap_or_else(|| "Someone".to_string())
    });

    // Follower control
    engine.register_fn("follow", |entity: Entity, target: Entity| {
        with_current_world(|w| {
            let _ = w.insert(
                entity,
                (oxide_core::Following {
                    target,
                    autofollow: true,
                },),
            );
        });
    });
    engine.register_fn("unfollow", |entity: Entity| {
        with_current_world(|w| {
            let _ = w.remove_one::<oxide_core::Following>(entity);
        });
    });

    // Exit controls & Room query
    engine.register_fn("is_exit_closed", |room: Entity, dir_str: String| -> bool {
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
    });
    engine.register_fn("is_exit_locked", |room: Entity, dir_str: String| -> bool {
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
    });
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
    engine.register_fn("room_exits", |room: Entity| -> rhai::Array {
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
    });
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
    engine.register_fn("move_entity", |entity: Entity, dest: Entity| {
        with_current_world(|w| {
            let _ = w.insert(entity, (oxide_core::Position::new(dest), oxide_core::Dirty));
        });
    });

    // Mob spawning & Template spawn
    engine.register_fn(
        "spawn_mob",
        |template_id: String, room_entity: Entity| -> Entity {
            with_current_world(|w| {
                let templates = match oxide_core::templates::get_global_templates() {
                    Some(t) => t,
                    None => return hecs::Entity::DANGLING,
                };
                if let Some(mob_tpl) = templates.mobs.get(&template_id) {
                    mob_tpl.spawn(w, room_entity, &templates)
                } else {
                    hecs::Entity::DANGLING
                }
            })
            .unwrap_or(hecs::Entity::DANGLING)
        },
    );
}
