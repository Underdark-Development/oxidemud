use oxide_core as core;
use oxide_core::{get_pos_room, AccessLevel, World};
use oxide_server::{Command, CommandHelp, Connection, ConnectionRegistry, Server};

use super::common::*;

pub fn register(server: &mut Server) {
    server.register_command(Command {
        name: "craft",
        aliases: &[],
        access: AccessLevel::Player,
        topic: "Abilities",
        help: CommandHelp {
            short: "Craft an item using a known recipe",
            body: None,
        },
        handler: cmd_craft,
    });
    server.register_command(Command {
        name: "use",
        aliases: &[],
        access: AccessLevel::Player,
        topic: "Abilities",
        help: CommandHelp {
            short: "Use a skill, potion, or item",
            body: None,
        },
        handler: cmd_use,
    });
    server.register_command(Command {
        name: "cast",
        aliases: &[],
        access: AccessLevel::Player,
        topic: "Abilities",
        help: CommandHelp {
            short: "Cast a spell",
            body: None,
        },
        handler: cmd_cast,
    });
    server.register_command(Command {
        name: "quaff",
        aliases: &[],
        access: AccessLevel::Player,
        topic: "Abilities",
        help: CommandHelp {
            short: "Drink a potion",
            body: None,
        },
        handler: cmd_quaff,
    });
    server.register_command(Command {
        name: "recite",
        aliases: &["read"],
        access: AccessLevel::Player,
        topic: "Abilities",
        help: CommandHelp {
            short: "Recite a magical scroll",
            body: None,
        },
        handler: cmd_recite,
    });
    server.register_command(Command {
        name: "zap",
        aliases: &[],
        access: AccessLevel::Player,
        topic: "Abilities",
        help: CommandHelp {
            short: "Zap a magic wand",
            body: None,
        },
        handler: cmd_zap,
    });
    server.register_command(Command {
        name: "eat",
        aliases: &["consume"],
        access: AccessLevel::Player,
        topic: "Abilities",
        help: CommandHelp {
            short: "Eat a food item",
            body: None,
        },
        handler: cmd_eat,
    });
    server.register_command(Command {
        name: "drink",
        aliases: &["sip"],
        access: AccessLevel::Player,
        topic: "Abilities",
        help: CommandHelp {
            short: "Drink from a waterskin or fluid source",
            body: None,
        },
        handler: cmd_drink,
    });
    server.register_command(Command {
        name: "fill",
        aliases: &[],
        access: AccessLevel::Player,
        topic: "Abilities",
        help: CommandHelp {
            short: "Fill a waterskin or drink container",
            body: None,
        },
        handler: cmd_fill,
    });
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
    let remaining_args = args[skill_input.len()..].trim();

    let dynamic_spell = core::with_dynamic_skills(|reg| reg.find_spell(skill_input).cloned());
    if let Some(spell) = dynamic_spell {
        if let Some(entity) = conn.entity() {
            if let Some(bridge) = core::get_scripting_bridge() {
                let _ = bridge.execute_script_skill(&spell.script, entity, remaining_args, world);
                return;
            }
        }
    }

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

pub fn cmd_quaff(
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

    let item_name = args.trim();
    if item_name.is_empty() {
        conn.send_line("Quaff what potion?");
        return;
    }

    let item = match super::common::find_item_in_inventory(world, entity, item_name) {
        Some(i) => i,
        None => {
            conn.send_line("You don't have that potion.");
            return;
        }
    };

    let item_display_name =
        core::get_entity_name(world, item).unwrap_or_else(|| "potion".to_string());

    let mut consumable = match world.query_one::<&mut core::components::Consumable>(item) {
        Ok(mut q) => match q.get() {
            Some(c) => c.clone(),
            None => {
                conn.send_line(&format!("You cannot quaff the {item_display_name}."));
                return;
            }
        },
        Err(_) => {
            conn.send_line(&format!("You cannot quaff the {item_display_name}."));
            return;
        }
    };

    if consumable.kind != core::components::ConsumableKind::Potion
        && !matches!(consumable.kind, core::components::ConsumableKind::Other(_))
    {
        conn.send_line(&format!("The {item_display_name} is not a potion."));
        return;
    }

    if consumable.is_empty() {
        conn.send_line(&format!("The {item_display_name} is empty."));
        return;
    }

    consumable.charges = consumable.charges.saturating_sub(1);

    if consumable.restore_health > 0 {
        if let Ok(mut q) = world.query_one::<&mut core::Health>(entity) {
            if let Some(hp) = q.get() {
                hp.current = (hp.current + consumable.restore_health).min(hp.max);
                conn.send_line(&format!(
                    "You feel a wave of healing energy (+{} HP).",
                    consumable.restore_health
                ));
            }
        }
    }
    if consumable.restore_mana > 0 {
        if let Ok(mut q) = world.query_one::<&mut core::Mana>(entity) {
            if let Some(mp) = q.get() {
                mp.current = (mp.current + consumable.restore_mana as u16).min(mp.max);
                conn.send_line(&format!(
                    "Your mind clears and mana restores (+{} MP).",
                    consumable.restore_mana
                ));
            }
        }
    }
    if consumable.restore_stamina > 0 {
        if let Ok(mut q) = world.query_one::<&mut core::Stamina>(entity) {
            if let Some(st) = q.get() {
                st.current = (st.current + consumable.restore_stamina as u16).min(st.max);
                conn.send_line(&format!(
                    "Vigor returns to your limbs (+{} Stamina).",
                    consumable.restore_stamina
                ));
            }
        }
    }

    conn.send_line(&format!("You quaff the {item_display_name}."));

    if consumable.is_empty() {
        if let Ok(mut q) = world.query_one::<&mut core::Inventory>(entity) {
            if let Some(inv) = q.get() {
                inv.0.retain(|&e| e != item);
            }
        }
        if let Some(dep_tmpl_id) = &consumable.depleted_template {
            if let Some(templates) = oxide_server::get_templates() {
                if let Some(tmpl) = templates.items.get(dep_tmpl_id) {
                    let vial = tmpl.spawn(world);
                    if let Ok(mut q) = world.query_one::<&mut core::Inventory>(entity) {
                        if let Some(inv) = q.get() {
                            inv.0.push(vial);
                            conn.send_line("You are left holding an empty vial.");
                        }
                    }
                }
            }
        }
    } else {
        let _ = world.insert(item, (consumable,));
    }
}

pub fn cmd_recite(
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

    let input = args.trim();
    if input.is_empty() {
        conn.send_line("Recite what scroll?");
        return;
    }

    let item = match super::common::find_item_in_inventory(world, entity, input) {
        Some(i) => i,
        None => {
            conn.send_line("You don't have that scroll.");
            return;
        }
    };

    let item_display_name =
        core::get_entity_name(world, item).unwrap_or_else(|| "scroll".to_string());

    let mut consumable = match world.query_one::<&mut core::components::Consumable>(item) {
        Ok(mut q) => match q.get() {
            Some(c) => c.clone(),
            None => {
                conn.send_line(&format!("You cannot recite the {item_display_name}."));
                return;
            }
        },
        Err(_) => {
            conn.send_line(&format!("You cannot recite the {item_display_name}."));
            return;
        }
    };

    if consumable.kind != core::components::ConsumableKind::Scroll {
        conn.send_line(&format!("The {item_display_name} is not a scroll."));
        return;
    }

    if consumable.is_empty() {
        conn.send_line(&format!("The {item_display_name} has no magic left."));
        return;
    }

    consumable.charges = consumable.charges.saturating_sub(1);
    conn.send_line(&format!("You recite the mystic words upon the {item_display_name}. As you finish, the parchment bursts into glowing ash!"));

    if consumable.is_empty() {
        if let Ok(mut q) = world.query_one::<&mut core::Inventory>(entity) {
            if let Some(inv) = q.get() {
                inv.0.retain(|&e| e != item);
            }
        }
    } else {
        let _ = world.insert(item, (consumable,));
    }
}

pub fn cmd_zap(
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

    let input = args.trim();
    if input.is_empty() {
        conn.send_line("Zap what wand?");
        return;
    }

    let item = match super::common::find_item_in_inventory(world, entity, input) {
        Some(i) => i,
        None => {
            conn.send_line("You don't have that wand.");
            return;
        }
    };

    let item_display_name =
        core::get_entity_name(world, item).unwrap_or_else(|| "wand".to_string());

    let mut consumable = match world.query_one::<&mut core::components::Consumable>(item) {
        Ok(mut q) => match q.get() {
            Some(c) => c.clone(),
            None => {
                conn.send_line(&format!("You cannot zap the {item_display_name}."));
                return;
            }
        },
        Err(_) => {
            conn.send_line(&format!("You cannot zap the {item_display_name}."));
            return;
        }
    };

    if consumable.kind != core::components::ConsumableKind::Wand {
        conn.send_line(&format!("The {item_display_name} is not a wand."));
        return;
    }

    if consumable.is_empty() {
        conn.send_line("The wand sputters with a faint whisper of smoke and does nothing.");
        return;
    }

    consumable.charges = consumable.charges.saturating_sub(1);
    conn.send_line(&format!("You point the {item_display_name} and zap it! A streak of arcane energy shoots forth! ({}/{} charges remaining)", consumable.charges, consumable.max_charges));

    let _ = world.insert(item, (consumable,));
}

pub fn cmd_eat(
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

    let input = args.trim();
    if input.is_empty() {
        conn.send_line("Eat what?");
        return;
    }

    let item = match super::common::find_item_in_inventory(world, entity, input) {
        Some(i) => i,
        None => {
            conn.send_line("You don't have that food.");
            return;
        }
    };

    let item_display_name =
        core::get_entity_name(world, item).unwrap_or_else(|| "food".to_string());

    let mut consumable = match world.query_one::<&mut core::components::Consumable>(item) {
        Ok(mut q) => match q.get() {
            Some(c) => c.clone(),
            None => {
                conn.send_line(&format!("You cannot eat the {item_display_name}."));
                return;
            }
        },
        Err(_) => {
            conn.send_line(&format!("You cannot eat the {item_display_name}."));
            return;
        }
    };

    if consumable.kind != core::components::ConsumableKind::Food {
        conn.send_line(&format!("The {item_display_name} is not edible."));
        return;
    }

    if consumable.is_empty() {
        conn.send_line(&format!("The {item_display_name} is completely consumed."));
        return;
    }

    consumable.charges = consumable.charges.saturating_sub(1);

    if let Ok(mut q) = world.query_one::<&mut core::Stamina>(entity) {
        if let Some(st) = q.get() {
            let gain = if consumable.restore_stamina > 0 {
                consumable.restore_stamina as u16
            } else {
                15u16
            };
            st.current = (st.current + gain).min(st.max);
            conn.send_line(&format!(
                "You eat the {item_display_name} and feel nourished (+{} Stamina).",
                gain
            ));
        }
    } else {
        conn.send_line(&format!("You eat the {item_display_name}."));
    }

    if consumable.is_empty() {
        if let Ok(mut q) = world.query_one::<&mut core::Inventory>(entity) {
            if let Some(inv) = q.get() {
                inv.0.retain(|&e| e != item);
            }
        }
    } else {
        let _ = world.insert(item, (consumable,));
    }
}

pub fn cmd_drink(
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

    let input = args.trim();
    let room = match get_pos_room(world, entity) {
        Some(r) => r,
        None => {
            conn.send_line("You are nowhere.");
            return;
        }
    };

    let target_item = if input.is_empty() {
        super::common::find_item_in_inventory(world, entity, "waterskin")
            .or_else(|| super::common::find_item_in_room(world, room, "fountain"))
    } else {
        super::common::find_item_in_inv_or_room(world, entity, room, input)
    };

    let item = match target_item {
        Some(i) => i,
        None => {
            conn.send_line("Drink from what?");
            return;
        }
    };

    let item_name = core::get_entity_name(world, item).unwrap_or_else(|| "source".to_string());

    if let Ok(mut q) = world.query_one::<&mut core::components::DrinkContainer>(item) {
        if let Some(dc) = q.get() {
            if dc.charges == 0 {
                conn.send_line(&format!("The {item_name} is empty."));
                return;
            }
            dc.charges = dc.charges.saturating_sub(1);
            let liquid = dc.liquid_type.clone();
            conn.send_line(&format!("You drink a refreshing draught of {liquid} from the {item_name}. ({}/{} charges remaining)", dc.charges, dc.max_charges));
            return;
        }
    }

    if let Ok(mut q) = world.query_one::<&mut core::components::Consumable>(item) {
        if let Some(c) = q.get() {
            if c.kind == core::components::ConsumableKind::Drink {
                if c.is_empty() {
                    conn.send_line(&format!("The {item_name} is empty."));
                    return;
                }
                c.charges = c.charges.saturating_sub(1);
                let liquid = c.liquid_type.as_deref().unwrap_or("beverage");
                conn.send_line(&format!("You drink the {liquid} from the {item_name}."));
                if c.is_empty() {
                    if let Ok(mut inv_q) = world.query_one::<&mut core::Inventory>(entity) {
                        if let Some(inv) = inv_q.get() {
                            inv.0.retain(|&e| e != item);
                        }
                    }
                }
                return;
            }
        }
    }

    conn.send_line(&format!("You cannot drink from the {item_name}."));
}

pub fn cmd_fill(
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

    let input = args.trim();
    if input.is_empty() {
        conn.send_line("Fill what container?");
        return;
    }

    let room = match get_pos_room(world, entity) {
        Some(r) => r,
        None => {
            conn.send_line("You are nowhere.");
            return;
        }
    };

    let pos = input.to_lowercase().find(" from ");
    let (cont_query, source_query) = match pos {
        Some(p) => (input[..p].trim(), Some(input[p + 6..].trim())),
        None => (input, None),
    };

    let container_item = match super::common::find_item_in_inventory(world, entity, cont_query) {
        Some(i) => i,
        None => {
            conn.send_line("You don't have that container in your inventory.");
            return;
        }
    };

    let container_name =
        core::get_entity_name(world, container_item).unwrap_or_else(|| "container".to_string());

    let source_item = if let Some(sq) = source_query {
        super::common::find_item_in_room(world, room, sq)
    } else {
        super::common::find_item_in_room(world, room, "fountain")
            .or_else(|| super::common::find_item_in_room(world, room, "well"))
            .or_else(|| super::common::find_item_in_room(world, room, "water"))
    };

    let source_item = match source_item {
        Some(s) => s,
        None => {
            conn.send_line("There is no fluid source here to fill from.");
            return;
        }
    };

    let source_name =
        core::get_entity_name(world, source_item).unwrap_or_else(|| "fluid source".to_string());

    if let Ok(mut q) = world.query_one::<&mut core::components::DrinkContainer>(container_item) {
        if let Some(dc) = q.get() {
            if dc.charges >= dc.max_charges {
                conn.send_line(&format!("The {container_name} is already completely full."));
                return;
            }
            dc.charges = dc.max_charges;
            conn.send_line(&format!("You submerge the {container_name} into the {source_name} and fill it to the brim with clear water!"));
            return;
        }
    }

    conn.send_line(&format!("The {container_name} cannot be refilled."));
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
