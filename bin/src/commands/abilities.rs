use oxide_core as core;
use oxide_core::{get_pos_room, AccessLevel, World};
use oxide_server::{Connection, ConnectionRegistry, Server};

use super::common::*;

pub fn register(server: &mut Server) {
    server.register_command(
        "craft",
        &[],
        AccessLevel::Player,
        "Abilities",
        "Craft an item using a known recipe",
        cmd_craft,
    );
    server.register_command(
        "use",
        &[],
        AccessLevel::Player,
        "Abilities",
        "Use a skill, potion, or item",
        cmd_use,
    );
    server.register_command(
        "cast",
        &[],
        AccessLevel::Player,
        "Abilities",
        "Cast a spell",
        cmd_cast,
    );
}

pub fn cmd_craft(
    world: &mut World,
    conn: &mut dyn Connection,
    _name: &str,
    args: &str,
    _registry: &ConnectionRegistry,
) {
    let entity = match conn.entity() {
        Some(e) => e,
        None => {
            conn.send_line("You have no form.");
            return;
        }
    };

    let templates = match oxide_server::get_templates() {
        Some(t) => t,
        None => {
            conn.send_line("Error: Template registry not loaded.");
            return;
        }
    };

    let recipe_input = args.trim();
    if recipe_input.is_empty() {
        conn.send_line("Craft what recipe?");
        return;
    }

    let recipe_input_lower = recipe_input.to_lowercase();
    let lr = match world.query_one::<&core::LearnedRecipes>(entity) {
        Ok(mut q) => q.get().cloned().unwrap_or_default(),
        Err(_) => core::LearnedRecipes::new(),
    };

    let mut candidates = Vec::new();
    for recipe_id in &lr.recipes {
        if let Some(recipe) = templates.recipes.get(recipe_id) {
            if recipe.name.to_lowercase().starts_with(&recipe_input_lower)
                || recipe_id.to_lowercase().starts_with(&recipe_input_lower)
            {
                candidates.push((recipe_id.clone(), recipe.name.clone()));
            }
        }
    }

    if candidates.is_empty() {
        conn.send_line(&format!(
            "You don't know any recipe matching '{}'.",
            recipe_input
        ));
        return;
    }

    if candidates.len() > 1 {
        let names: Vec<String> = candidates.iter().map(|(_, name)| name.clone()).collect();
        conn.send_line(&format!("Which recipe did you mean? {}", names.join(", ")));
        return;
    }

    let (recipe_id, _) = &candidates[0];
    match core::craft_recipe(world, entity, recipe_id, &templates) {
        Ok(msg) => conn.send_line(&msg),
        Err(err) => conn.send_line(&format!("Error: {}", err)),
    }
}

pub fn cmd_use(
    world: &mut World,
    conn: &mut dyn Connection,
    _name: &str,
    args: &str,
    registry: &ConnectionRegistry,
) {
    let entity = match conn.entity() {
        Some(e) => e,
        None => {
            conn.send_line("You have no form.");
            return;
        }
    };

    let templates = match oxide_server::get_templates() {
        Some(t) => t,
        None => {
            conn.send_line("Error: Template registry not loaded.");
            return;
        }
    };

    let mut parts = args.split_whitespace();
    let skill_input = match parts.next() {
        Some(s) => s,
        None => {
            conn.send_line("Use what skill?");
            return;
        }
    };

    let target_arg = parts.next();

    let skill_id = match templates.resolve_skill(skill_input, None) {
        Ok(id) => id,
        Err(core::templates::SkillResolveError::NotFound) => {
            conn.send_line(&format!(
                "You don't know any skill named '{}'.",
                skill_input
            ));
            return;
        }
        Err(core::templates::SkillResolveError::Multiple(candidates)) => {
            let names: Vec<&str> = candidates.iter().map(|(id, _)| id.as_str()).collect();
            conn.send_line(&format!("Which skill did you mean? {}", names.join(", ")));
            return;
        }
    };

    let skill_def = match templates.skills.get(&skill_id) {
        Some(def) => def,
        None => {
            conn.send_line("Error: Skill definition not found in registry.");
            return;
        }
    };

    let target_entity = if let Some(target_name) = target_arg {
        let room = match get_pos_room(world, entity) {
            Some(r) => r,
            None => {
                conn.send_line("You are nowhere.");
                return;
            }
        };

        let occupants = core::util::entities_in_room(world, room);
        let mut matched = None;
        for occupant in occupants {
            if occupant == entity {
                continue;
            }
            if let Ok(mut q_name) = world.query_one::<&core::Name>(occupant) {
                if let Some(name) = q_name.get() {
                    if name
                        .0
                        .to_lowercase()
                        .starts_with(&target_name.to_lowercase())
                    {
                        matched = Some(occupant);
                        break;
                    }
                }
            }
        }
        if matched.is_none() {
            conn.send_line(&format!("You don't see '{}' here.", target_name));
            return;
        }
        matched
    } else {
        None
    };

    if let Err(err) = core::can_use_skill(world, entity, skill_def, target_entity) {
        conn.send_line(&err);
        return;
    }

    let _ = core::deduct_resource_cost(world, entity, &skill_def.cost);

    if skill_def.cooldown_secs > 0 {
        let mut cd_comp = world
            .query_one::<&core::SkillCooldowns>(entity)
            .ok()
            .and_then(|mut q| q.get().cloned())
            .unwrap_or_default();
        cd_comp
            .cooldowns
            .insert(skill_def.id.clone(), skill_def.cooldown_secs);
        let _ = world.insert(entity, (cd_comp, core::Dirty));
    }

    conn.send_line(&format!("You use {}!", skill_def.name));

    if let Ok(mut q_name) = world.query_one::<&core::Name>(entity) {
        if let Some(name) = q_name.get() {
            if let Some(room) = get_pos_room(world, entity) {
                let msg = format!("{} uses {}!", name.0, skill_def.name);
                broadcast_to_room_except(world, registry, room, entity, &msg);
            }
        }
    }

    if let Some(ref effect) = skill_def.effect {
        let msgs = core::apply_skill_effect(
            world,
            entity,
            target_entity,
            effect,
            &skill_def.name,
            &templates,
        );
        for msg in msgs {
            conn.send_line(&msg);
        }
    }
}

pub fn cmd_cast(
    world: &mut World,
    conn: &mut dyn Connection,
    name: &str,
    args: &str,
    registry: &ConnectionRegistry,
) {
    let templates = match oxide_server::get_templates() {
        Some(t) => t,
        None => {
            conn.send_line("Error: Template registry not loaded.");
            return;
        }
    };

    let mut parts = args.split_whitespace();
    let skill_input = match parts.next() {
        Some(s) => s,
        None => {
            conn.send_line("Cast what spell?");
            return;
        }
    };

    let skill_id = match templates.resolve_skill(skill_input, None) {
        Ok(id) => id,
        Err(core::templates::SkillResolveError::NotFound) => {
            conn.send_line(&format!(
                "You don't know any spell named '{}'.",
                skill_input
            ));
            return;
        }
        Err(core::templates::SkillResolveError::Multiple(candidates)) => {
            let names: Vec<&str> = candidates.iter().map(|(id, _)| id.as_str()).collect();
            conn.send_line(&format!("Which spell did you mean? {}", names.join(", ")));
            return;
        }
    };

    let skill_def = match templates.skills.get(&skill_id) {
        Some(def) => def,
        None => {
            conn.send_line("Error: Spell definition not found in registry.");
            return;
        }
    };

    if !matches!(skill_def.skill_type, core::SkillType::Magic) {
        conn.send_line("That is not a magic spell! Use 'use' instead.");
        return;
    }

    cmd_use(world, conn, name, args, registry);
}

#[cfg(test)]
mod tests {
    use super::super::test_helpers::*;
    use super::*;

    #[test]
    fn test_use_potion_success() {
        let _guard = init_test_templates();

        let (mut world, _void, room_a, _room_b) = test_world();
        let (player, mut conn, conn_reg) = test_player(&mut world, room_a);

        world
            .insert(
                player,
                (
                    core::Health {
                        current: 10,
                        max: 100,
                    },
                    core::Mana::new(50),
                    core::LearnedSkills {
                        skills: [("potion_minor_heal".to_string(), 1)].into_iter().collect(),
                    },
                ),
            )
            .unwrap();

        cmd_use(&mut world, &mut conn, "use", "potion_minor_heal", &conn_reg);

        let lines = conn.take_lines();
        assert!(lines
            .iter()
            .any(|l| l.contains("You use Minor Healing Potion!")));
        assert!(lines.iter().any(|l| l.contains("heal")));

        let mut hp = world.query_one::<&core::Health>(player).unwrap();
        assert!(hp.get().unwrap().current > 10);
    }

    #[test]
    fn test_use_scroll_success() {
        let _guard = init_test_templates();

        let (mut world, _void, room_a, _room_b) = test_world();
        let (player, mut conn, conn_reg) = test_player(&mut world, room_a);

        world
            .insert(
                player,
                (
                    core::Mana::new(50),
                    core::LearnedSkills {
                        skills: [("scroll_fireball".to_string(), 1)].into_iter().collect(),
                    },
                ),
            )
            .unwrap();

        let mob = world.spawn((
            core::Position::new(room_a),
            core::Name::new("Goblin"),
            core::Health::new(50),
            core::Npc::new("goblin"),
        ));

        cmd_use(
            &mut world,
            &mut conn,
            "use",
            "scroll_fireball Goblin",
            &conn_reg,
        );

        let lines = conn.take_lines();
        assert!(lines
            .iter()
            .any(|l| l.contains("You use Scroll of Fireball!")));

        let mut mob_hp = world.query_one::<&core::Health>(mob).unwrap();
        assert!(mob_hp.get().unwrap().current < 50);
    }

    #[test]
    fn test_cast_offensive_spell_valid_target() {
        let _guard = init_test_templates();

        let (mut world, _void, room_a, _room_b) = test_world();
        let (player, mut conn, conn_reg) = test_player(&mut world, room_a);

        world
            .insert(
                player,
                (
                    core::Mana::new(50),
                    core::LearnedSkills {
                        skills: [("spell_fireball".to_string(), 1)].into_iter().collect(),
                    },
                ),
            )
            .unwrap();

        let mob = world.spawn((
            core::Position::new(room_a),
            core::Name::new("Orc"),
            core::Health::new(100),
            core::Npc::new("orc"),
        ));

        cmd_cast(
            &mut world,
            &mut conn,
            "cast",
            "spell_fireball Orc",
            &conn_reg,
        );

        let lines = conn.take_lines();
        assert!(lines.iter().any(|l| l.contains("You use Fireball!")));

        let mut mob_hp = world.query_one::<&core::Health>(mob).unwrap();
        assert!(mob_hp.get().unwrap().current < 100);
    }

    #[test]
    fn test_cast_offensive_spell_dead_target() {
        let _guard = init_test_templates();

        let (mut world, _void, room_a, _room_b) = test_world();
        let (player, mut conn, conn_reg) = test_player(&mut world, room_a);

        world
            .insert(
                player,
                (
                    core::Mana::new(50),
                    core::LearnedSkills {
                        skills: [("spell_fireball".to_string(), 1)].into_iter().collect(),
                    },
                ),
            )
            .unwrap();

        let _dead_mob = world.spawn((
            core::Position::new(room_a),
            core::Name::new("Orc"),
            core::Health {
                current: 0,
                max: 100,
            },
            core::Npc::new("orc"),
        ));

        cmd_cast(
            &mut world,
            &mut conn,
            "cast",
            "spell_fireball Orc",
            &conn_reg,
        );

        let lines = conn.take_lines();
        assert!(lines
            .iter()
            .any(|l| l.contains("Your target is already dead.")));
    }
}
