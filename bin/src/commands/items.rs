use std::collections::HashMap;
use std::str::FromStr;

use oxide_core as core;
use oxide_core::templates::SetDef;
use oxide_core::{get_entity_name, get_pos_room, AccessLevel, World};
use oxide_server::{Command, CommandHelp, Connection, ConnectionRegistry, Server};

fn trigger_message(trigger: &core::TriggeredEffect, world: &World) -> String {
    let item_name = world
        .query_one::<&core::Name>(trigger.item)
        .ok()
        .and_then(|mut q| q.get().cloned())
        .map(|n| n.0)
        .unwrap_or_else(|| "something".to_owned());
    format!("Your {item_name} {}.", trigger.cast)
}

pub fn register(server: &mut Server) {
    server.register_command(Command {
        name: "inventory",
        aliases: &["inv", "i"],
        access: AccessLevel::Player,
        topic: "Items",
        help: CommandHelp {
            short: "List your carried items",
            body: None,
        },
        handler: cmd_inventory,
    });
    server.register_command(Command {
        name: "equipment",
        aliases: &["eq"],
        access: AccessLevel::Player,
        topic: "Items",
        help: CommandHelp {
            short: "Show what you are wearing and wielding",
            body: None,
        },
        handler: cmd_equipment,
    });
    server.register_command(Command {
        name: "wear",
        aliases: &[],
        access: AccessLevel::Player,
        topic: "Items",
        help: CommandHelp {
            short: "Wear a piece of armor",
            body: None,
        },
        handler: cmd_wear,
    });
    server.register_command(Command {
        name: "wield",
        aliases: &[],
        access: AccessLevel::Player,
        topic: "Items",
        help: CommandHelp {
            short: "Wield a weapon",
            body: None,
        },
        handler: cmd_wield,
    });
    server.register_command(Command {
        name: "remove",
        aliases: &[],
        access: AccessLevel::Player,
        topic: "Items",
        help: CommandHelp {
            short: "Remove an equipped item",
            body: None,
        },
        handler: cmd_remove,
    });
    server.register_command(Command {
        name: "examine",
        aliases: &["exa"],
        access: AccessLevel::Player,
        topic: "Items",
        help: CommandHelp {
            short: "Inspect an item or target",
            body: None,
        },
        handler: cmd_examine,
    });
    server.register_command(Command {
        name: "get",
        aliases: &["take"],
        access: AccessLevel::Player,
        topic: "Items",
        help: CommandHelp {
            short: "Pick up an item",
            body: None,
        },
        handler: cmd_get,
    });
    server.register_command(Command {
        name: "drop",
        aliases: &[],
        access: AccessLevel::Player,
        topic: "Items",
        help: CommandHelp {
            short: "Drop an item",
            body: None,
        },
        handler: cmd_drop,
    });
    server.register_command(Command {
        name: "put",
        aliases: &[],
        access: AccessLevel::Player,
        topic: "Items",
        help: CommandHelp {
            short: "Put an item into a container",
            body: None,
        },
        handler: cmd_put,
    });
    server.register_command(Command {
        name: "give",
        aliases: &[],
        access: AccessLevel::Player,
        topic: "Items",
        help: CommandHelp {
            short: "Give an item to someone",
            body: None,
        },
        handler: cmd_give,
    });
    server.register_command(Command {
        name: "loot",
        aliases: &[],
        access: AccessLevel::Player,
        topic: "Items",
        help: CommandHelp {
            short: "Take all items from a corpse",
            body: None,
        },
        handler: cmd_loot,
    });
    server.register_command(Command {
        name: "repair",
        aliases: &["fix"],
        access: AccessLevel::Player,
        topic: "Items",
        help: CommandHelp {
            short: "Repair damaged equipment at a blacksmith",
            body: None,
        },
        handler: cmd_repair,
    });
}

pub fn cmd_inventory(
    world: &mut World,
    conn: &mut dyn Connection,
    _name: &str,
    _args: &str,
    _registry: &ConnectionRegistry,
) {
    let entity = match conn.entity() {
        Some(e) => e,
        None => {
            conn.send_line("You have no form.");
            return;
        }
    };

    let inv = match world.query_one::<&core::Inventory>(entity) {
        Ok(mut q) => q.get().map(|inv| inv.0.clone()).unwrap_or_default(),
        Err(_) => {
            conn.send_line("You are carrying nothing.");
            return;
        }
    };

    if inv.is_empty() {
        conn.send_line("You are carrying nothing.");
        return;
    }

    conn.send_line("");
    conn.send_line("You are carrying:");

    for (i, item) in inv.iter().enumerate() {
        let name = world
            .query_one::<&core::Name>(*item)
            .ok()
            .and_then(|mut q| q.get().map(|n| n.to_string()))
            .unwrap_or_else(|| format!("item_{}", item.id()));

        conn.send_line(&format!("  {}. {name}", i + 1));
    }
}

pub fn cmd_equipment(
    world: &mut World,
    conn: &mut dyn Connection,
    _name: &str,
    _args: &str,
    _registry: &ConnectionRegistry,
) {
    let entity = match conn.entity() {
        Some(e) => e,
        None => {
            conn.send_line("You have no form.");
            return;
        }
    };

    let eq = match world.query_one::<&core::Equipment>(entity) {
        Ok(mut q) => q.get().map(|eq| eq.slots.clone()).unwrap_or_default(),
        Err(_) => {
            conn.send_line("You have no equipment.");
            return;
        }
    };

    if eq.is_empty() {
        conn.send_line("You are not wearing anything.");
        return;
    }

    conn.send_line("");
    conn.send_line("Equipment:");

    for (slot, item) in &eq {
        let name = world
            .query_one::<&core::Name>(*item)
            .ok()
            .and_then(|mut q| q.get().map(|n| n.to_string()))
            .unwrap_or_else(|| format!("item_{}", item.id()));

        conn.send_line(&format!("  {:?}: {name}", slot));
    }
}

fn find_item_in_inventory(
    world: &World,
    entity: core::Entity,
    query: &str,
) -> Option<core::Entity> {
    let inv = world
        .query_one::<&core::Inventory>(entity)
        .ok()?
        .get()?
        .0
        .clone();

    if let Ok(idx) = query.parse::<usize>() {
        if idx > 0 && idx <= inv.len() {
            return Some(inv[idx - 1]);
        }
    }

    let candidates: Vec<(String, core::Entity)> = inv
        .iter()
        .filter_map(|&item| get_entity_name(world, item).map(|name| (name, item)))
        .collect();

    match core::trie::trie_match(query, candidates) {
        core::trie::TrieMatch::One(e) => Some(e),
        core::trie::TrieMatch::Many(items) => items.into_iter().next(),
        core::trie::TrieMatch::None => None,
    }
}

fn evaluate_equipment_sets(world: &mut World, entity: core::Entity) -> Option<String> {
    if let Some(templates) = oxide_server::get_templates() {
        let set_defs: HashMap<String, SetDef> = templates.sets.clone();
        let changes = core::systems::set_bonus::evaluate_set_bonuses(world, entity, &set_defs);
        if changes.is_empty() {
            return None;
        }
        let mut msgs: Vec<String> = Vec::new();
        for change in &changes {
            if change.new_count > change.old_count {
                let tier_str = change
                    .active_tiers
                    .iter()
                    .map(|t| t.to_string())
                    .collect::<Vec<_>>()
                    .join(", ");
                if tier_str.is_empty() {
                    msgs.push(format!(
                        "Your {} pieces now number {}.",
                        change.set_name, change.new_count
                    ));
                } else {
                    msgs.push(format!(
                        "Your {} set grants tier bonuses at {} piece(s)!",
                        change.set_name, tier_str
                    ));
                }
            }
        }
        if msgs.is_empty() {
            None
        } else {
            Some(msgs.join(" "))
        }
    } else {
        None
    }
}

pub fn cmd_wear(
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

    let is_ghost = world
        .query_one::<&core::PlayerState>(entity)
        .ok()
        .and_then(|mut q| q.get().map(|s| matches!(s, core::PlayerState::Dead)))
        .unwrap_or(false);

    if is_ghost {
        conn.send_line("You are a ghost! You cannot wear items.");
        return;
    }

    let item_name = args.trim();
    if item_name.is_empty() {
        conn.send_line("Wear what?");
        return;
    }

    let item = match find_item_in_inventory(world, entity, item_name) {
        Some(i) => i,
        None => {
            conn.send_line("You don't have that item.");
            return;
        }
    };

    if let Some(req) = world
        .query_one::<&core::ItemSkillRequirement>(item)
        .ok()
        .and_then(|mut q| q.get().cloned())
    {
        let has_skill = world
            .query_one::<&core::LearnedSkills>(entity)
            .ok()
            .and_then(|mut q| q.get().map(|s| s.rank(&req.id) >= req.level))
            .unwrap_or(false);
        if !has_skill {
            conn.send_line("You lack the skill to use that.");
            return;
        }
    }

    let slot = if let Some(templates) = oxide_server::get_templates() {
        if let Ok(mut q) = world.query_one::<&core::Item>(item) {
            if let Some(item_comp) = q.get() {
                if let Some(item_tmpl) = templates.items.get(&item_comp.template_id) {
                    if let Some(eq_def) = &item_tmpl.equipment {
                        core::EquipmentSlot::from_str(&eq_def.slot)
                            .unwrap_or(core::EquipmentSlot::Torso)
                    } else {
                        conn.send_line("You can't wear that.");
                        return;
                    }
                } else {
                    conn.send_line("You can't wear that.");
                    return;
                }
            } else {
                conn.send_line("You can't wear that.");
                return;
            }
        } else {
            conn.send_line("You can't wear that.");
            return;
        }
    } else {
        conn.send_line("Server error: templates unavailable.");
        return;
    };

    if slot == core::EquipmentSlot::Shield {
        let wielding_two_handed = world
            .query_one::<&core::Equipment>(entity)
            .ok()
            .and_then(|mut q| {
                q.get().and_then(|eq| {
                    eq.equipped(&core::EquipmentSlot::Weapon)
                        .and_then(|w_entity| world.query_one::<&core::Weapon>(*w_entity).ok())
                        .and_then(|mut w_query| w_query.get().map(|w| w.is_two_handed()))
                })
            })
            .unwrap_or(false);

        if wielding_two_handed {
            conn.send_line(
                "You are wielding a two-handed weapon and cannot use a shield/off-hand.",
            );
            return;
        }
    }

    if let Ok(mut q) = world.query_one::<&mut core::Inventory>(entity) {
        if let Some(inv) = q.get() {
            inv.0.retain(|e| *e != item);
        }
    }

    if let Ok(mut q) = world.query_one::<&mut core::Equipment>(entity) {
        if let Some(eq) = q.get() {
            if let Some(old) = eq.unequip(&slot) {
                if let Ok(mut iq) = world.query_one::<&mut core::Inventory>(entity) {
                    if let Some(inv) = iq.get() {
                        inv.0.push(old);
                    }
                }
            }
            eq.equip(slot, item);
            conn.send_line("You wear it.");
        }
    }

    if let Some(msg) = evaluate_equipment_sets(world, entity) {
        conn.send_line(&msg);
    }

    for trigger in core::systems::trigger::process_triggers(world, entity, "on_wear") {
        conn.send_line(&trigger_message(&trigger, world));
    }
    notify_equip_commands(world, conn, item);
}

fn notify_equip_commands(world: &World, conn: &mut dyn Connection, item: core::Entity) {
    if let Ok(mut q) = world.query_one::<&core::EntityCommands>(item) {
        if let Some(cmds) = q.get() {
            for cmd in &cmds.commands {
                if cmd.restrictions.requires_equipped {
                    if let Some(ref msg) = cmd.equip_message {
                        conn.send_line(msg);
                    } else {
                        conn.send_line(&format!(
                            "Equipping this item bestows the ability to '{}'.",
                            cmd.command_name
                        ));
                    }
                }
            }
        }
    }
}

fn notify_unequip_commands(world: &World, conn: &mut dyn Connection, item: core::Entity) {
    if let Ok(mut q) = world.query_one::<&core::EntityCommands>(item) {
        if let Some(cmds) = q.get() {
            for cmd in &cmds.commands {
                if cmd.restrictions.requires_equipped {
                    if let Some(ref msg) = cmd.unequip_message {
                        conn.send_line(msg);
                    } else {
                        conn.send_line(&format!(
                            "Unequipping this item removes the ability to '{}'.",
                            cmd.command_name
                        ));
                    }
                }
            }
        }
    }
}

pub fn cmd_wield(
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

    let is_ghost = world
        .query_one::<&core::PlayerState>(entity)
        .ok()
        .and_then(|mut q| q.get().map(|s| matches!(s, core::PlayerState::Dead)))
        .unwrap_or(false);

    if is_ghost {
        conn.send_line("You are a ghost! You cannot wield items.");
        return;
    }

    let item_name = args.trim();
    if item_name.is_empty() {
        conn.send_line("Wield what?");
        return;
    }

    let item = match find_item_in_inventory(world, entity, item_name) {
        Some(i) => i,
        None => {
            conn.send_line("You don't have that item.");
            return;
        }
    };

    if let Some(req) = world
        .query_one::<&core::ItemSkillRequirement>(item)
        .ok()
        .and_then(|mut q| q.get().cloned())
    {
        let has_skill = world
            .query_one::<&core::LearnedSkills>(entity)
            .ok()
            .and_then(|mut q| q.get().map(|s| s.rank(&req.id) >= req.level))
            .unwrap_or(false);
        if !has_skill {
            conn.send_line("You lack the skill to use that.");
            return;
        }
    }

    let has_weapon = world
        .query_one::<&core::Weapon>(item)
        .is_ok_and(|mut q| q.get().is_some());
    if !has_weapon {
        conn.send_line("You can't wield that.");
        return;
    }

    if let Ok(mut q) = world.query_one::<&mut core::Inventory>(entity) {
        if let Some(inv) = q.get() {
            inv.0.retain(|e| *e != item);
        }
    }

    let is_two_handed = world
        .query_one::<&core::Weapon>(item)
        .ok()
        .and_then(|mut q| q.get().map(|w| w.is_two_handed()))
        .unwrap_or(false);

    if let Ok(mut q) = world.query_one::<&mut core::Equipment>(entity) {
        if let Some(eq) = q.get() {
            if let Some(old) = eq.unequip(&core::EquipmentSlot::Weapon) {
                if let Ok(mut iq) = world.query_one::<&mut core::Inventory>(entity) {
                    if let Some(inv) = iq.get() {
                        inv.0.push(old);
                    }
                }
            }

            if is_two_handed {
                if let Some(old_shield) = eq.unequip(&core::EquipmentSlot::Shield) {
                    if let Ok(mut name_q) = world.query_one::<&core::Name>(old_shield) {
                        if let Some(sname) = name_q.get() {
                            conn.send_line(&format!(
                                "You unequip {} to wield the two-handed weapon.",
                                sname.0
                            ));
                        }
                    }
                    if let Ok(mut iq) = world.query_one::<&mut core::Inventory>(entity) {
                        if let Some(inv) = iq.get() {
                            inv.0.push(old_shield);
                        }
                    }
                }
            }

            eq.equip(core::EquipmentSlot::Weapon, item);
            conn.send_line("You wield it.");
        }
    }

    if let Some(msg) = evaluate_equipment_sets(world, entity) {
        conn.send_line(&msg);
    }

    for trigger in core::systems::trigger::process_triggers(world, entity, "on_wear") {
        conn.send_line(&trigger_message(&trigger, world));
    }
    notify_equip_commands(world, conn, item);
}

pub fn cmd_remove(
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

    let is_ghost = world
        .query_one::<&core::PlayerState>(entity)
        .ok()
        .and_then(|mut q| q.get().map(|s| matches!(s, core::PlayerState::Dead)))
        .unwrap_or(false);

    if is_ghost {
        conn.send_line("You are a ghost! You cannot equip or remove items.");
        return;
    }

    let slot_name = args.trim().to_lowercase();
    if slot_name.is_empty() {
        conn.send_line("Remove what?");
        return;
    }

    let slot = match core::EquipmentSlot::from_str(&slot_name).ok() {
        Some(s) => s,
        None => {
            conn.send_line("Unknown slot. Try: head, neck, torso, arms, hands, finger, legs, feet, weapon, shield, back, waist.");
            return;
        }
    };

    if let Ok(mut q) = world.query_one::<&mut core::Equipment>(entity) {
        if let Some(eq) = q.get() {
            if let Some(item) = eq.unequip(&slot) {
                if let Ok(mut iq) = world.query_one::<&mut core::Inventory>(entity) {
                    if let Some(inv) = iq.get() {
                        inv.0.push(item);
                    }
                }
                conn.send_line("You remove it.");
                notify_unequip_commands(world, conn, item);
            } else {
                conn.send_line("You aren't wearing anything there.");
            }
        }
    }

    if let Some(msg) = evaluate_equipment_sets(world, entity) {
        conn.send_line(&msg);
    }

    for trigger in core::systems::trigger::process_triggers(world, entity, "on_remove") {
        conn.send_line(&trigger_message(&trigger, world));
    }
}

pub fn cmd_examine(
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
        conn.send_line("Examine what?");
        return;
    }

    let item = find_item_in_inventory(world, entity, item_name).or_else(|| {
        let room = get_pos_room(world, entity)?;
        let mut floor_q = world.query_one::<&core::FloorItems>(room).ok()?;
        let floor = floor_q.get()?;
        if let Ok(idx) = item_name.parse::<usize>() {
            if idx > 0 && idx <= floor.0.len() {
                return Some(floor.0[idx - 1]);
            }
        }
        let candidates: Vec<(String, core::Entity)> = floor
            .0
            .iter()
            .filter_map(|&item| get_entity_name(world, item).map(|name| (name, item)))
            .collect();
        match core::trie::trie_match(item_name, candidates) {
            core::trie::TrieMatch::One(e) => Some(e),
            core::trie::TrieMatch::Many(items) => items.into_iter().next(),
            core::trie::TrieMatch::None => None,
        }
    });

    let item = match item {
        Some(i) => i,
        None => {
            conn.send_line("You don't see that here.");
            return;
        }
    };

    conn.send_line("");

    let name = world
        .query_one::<&core::Name>(item)
        .ok()
        .and_then(|mut q| q.get().map(|n| n.to_string()))
        .unwrap_or_else(|| "Unknown item".to_string());

    let display_name = if let Ok(mut q) = world.query_one::<&core::AffixNames>(item) {
        if let Some(affixes) = q.get() {
            let quality = affixes.0.first().map(|s| s.as_str()).unwrap_or("");
            if !quality.is_empty() && quality != "Common" && affixes.0.len() <= 1 {
                format!("[{quality}] {name}")
            } else if !affixes.0.is_empty() {
                let rest: Vec<&str> = affixes.0.iter().map(|s| s.as_str()).collect();
                format!("[{}] {name}", rest.join(" "))
            } else {
                name.clone()
            }
        } else {
            name.clone()
        }
    } else {
        name.clone()
    };

    conn.send_line(&format!("--- {display_name} ---"));

    if let Ok(mut q) = world.query_one::<&core::AffixNames>(item) {
        if let Some(affixes) = q.get() {
            for affix_name in &affixes.0 {
                conn.send_line(&format!("  ~ {affix_name}"));
            }
        }
    }

    if let Ok(mut q) = world.query_one::<&core::AffixModifiers>(item) {
        if let Some(mods) = q.get() {
            for m in &mods.0 {
                conn.send_line(&format!("  * +{} {}", m.amount, m.stat));
            }
        }
    }

    if let Ok(mut q) = world.query_one::<&core::Item>(item) {
        if let Some(item_comp) = q.get() {
            conn.send_line(&format!("Template: {}", item_comp.template_id));
        }
    }

    if let Ok(mut q) = world.query_one::<&core::Weapon>(item) {
        if let Some(wep) = q.get() {
            conn.send_line(&format!(
                "Weapon: {} {:?} damage, Range: {:?}",
                wep.damage_dice, wep.damage_type, wep.range
            ));
        }
    }

    if let Ok(mut q) = world.query_one::<&core::Armor>(item) {
        if let Some(armor) = q.get() {
            conn.send_line(&format!("Armor: base {} bonus {}", armor.base, armor.bonus));
        }
    }

    if let Ok(mut q) = world.query_one::<&core::Durability>(item) {
        if let Some(dur) = q.get() {
            conn.send_line(&format!("Durability: {}/{}", dur.current, dur.max));
        }
    }

    if let Ok(mut q) = world.query_one::<&core::ActiveScriptEffects>(item) {
        if let Some(active) = q.get() {
            for effect in &active.effects {
                if effect.visible_on_look {
                    if let Some(ref aura) = effect.look_aura {
                        conn.send_line(&format!("Active Aura: {}", aura));
                    } else {
                        conn.send_line(&format!(
                            "Active Effect: [{}] {}",
                            effect.display_name, effect.description
                        ));
                    }
                }
            }
        }
    }

    if let Ok(mut q) = world.query_one::<&core::EntityCommands>(item) {
        if let Some(cmds) = q.get() {
            for cmd in &cmds.commands {
                if let Some(ref hint) = cmd.examine_hint {
                    conn.send_line(&format!("  Hint: {}", hint));
                }
            }
        }
    }
}

pub fn cmd_get(
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

    let is_ghost = world
        .query_one::<&core::PlayerState>(entity)
        .ok()
        .and_then(|mut q| q.get().map(|s| matches!(s, core::PlayerState::Dead)))
        .unwrap_or(false);

    if is_ghost {
        conn.send_line("You are a ghost! You cannot pick up items.");
        return;
    }

    let input = args.trim();
    if input.is_empty() {
        conn.send_line("Get what?");
        return;
    }

    let room = match get_pos_room(world, entity) {
        Some(r) => r,
        None => {
            conn.send_line("You are nowhere.");
            return;
        }
    };

    if let Some(pos) = input.to_lowercase().find(" from ") {
        let item_query = input[..pos].trim();
        let container_query = input[pos + 6..].trim();

        if item_query.is_empty() || container_query.is_empty() {
            conn.send_line("Get what from what?");
            return;
        }

        let container_ent =
            match super::common::find_item_in_inv_or_room(world, entity, room, container_query) {
                Some(c) => c,
                None => {
                    conn.send_line("You don't see that container.");
                    return;
                }
            };

        let container_name =
            get_entity_name(world, container_ent).unwrap_or_else(|| "container".to_string());

        let mut item_container =
            match world.query_one::<&mut core::components::ItemContainer>(container_ent) {
                Ok(mut q) => match q.get() {
                    Some(c) => c.clone(),
                    None => {
                        conn.send_line("That is not a container.");
                        return;
                    }
                },
                Err(_) => {
                    conn.send_line("That is not a container.");
                    return;
                }
            };

        if item_container.is_closed {
            conn.send_line(&format!("The {container_name} is closed."));
            return;
        }

        let items_to_get: Vec<core::Entity> = {
            let candidates: Vec<(String, core::Entity)> = item_container
                .contents
                .iter()
                .filter_map(|&e| get_entity_name(world, e).map(|n| (n, e)))
                .collect();
            match core::trie::trie_match(item_query, candidates) {
                core::trie::TrieMatch::One(e) => vec![e],
                core::trie::TrieMatch::Many(es) => es,
                core::trie::TrieMatch::None => Vec::new(),
            }
        };

        if items_to_get.is_empty() {
            conn.send_line(&format!("You don't find that in the {container_name}."));
            return;
        }

        {
            if let Ok(mut q) = world.query_one::<&mut core::Inventory>(entity) {
                if let Some(inv) = q.get() {
                    for item_ent in &items_to_get {
                        item_container.contents.retain(|e| e != item_ent);
                        inv.0.push(*item_ent);
                        let iname =
                            get_entity_name(world, *item_ent).unwrap_or_else(|| "item".to_string());
                        conn.send_line(&format!("You get {iname} from the {container_name}."));
                    }
                }
            }
        }

        let _ = world.insert(container_ent, (item_container,));
        return;
    }

    let item = {
        let mut q = world.query_one::<&mut core::FloorItems>(room);
        match q.as_mut().ok().and_then(|q| q.get()) {
            Some(floor) => {
                let candidates: Vec<(String, core::Entity)> = floor
                    .0
                    .iter()
                    .filter_map(|&e| get_entity_name(world, e).map(|name| (name, e)))
                    .collect();
                let matched = match core::trie::trie_match(input, candidates) {
                    core::trie::TrieMatch::One(e) => Some(e),
                    core::trie::TrieMatch::Many(items) => items.into_iter().next(),
                    core::trie::TrieMatch::None => None,
                };
                matched.and_then(|m| {
                    let idx = floor.0.iter().position(|&e| e == m)?;
                    Some(floor.0.remove(idx))
                })
            }
            None => None,
        }
    };

    let item = match item {
        Some(i) => i,
        None => {
            conn.send_line("You don't see that here.");
            return;
        }
    };

    let is_gettable = if let Ok(mut q) = world.query_one::<&core::components::ItemFlags>(item) {
        q.get().map(|f| f.is_gettable()).unwrap_or(true)
    } else {
        true
    };

    if !is_gettable {
        let iname = get_entity_name(world, item).unwrap_or_else(|| "item".to_string());
        conn.send_line(&format!(
            "The {iname} is fixed in place and cannot be taken."
        ));
        if let Ok(mut q) = world.query_one::<&mut core::FloorItems>(room) {
            if let Some(floor) = q.get() {
                floor.0.push(item);
            }
        }
        return;
    }

    if let Ok(mut q) = world.query_one::<&mut core::Inventory>(entity) {
        if let Some(inv) = q.get() {
            inv.0.push(item);
            let iname = get_entity_name(world, item).unwrap_or_else(|| "item".to_string());
            conn.send_line(&format!("You pick up {iname}."));
        }
    }

    if let Some(templates) = oxide_server::get_templates() {
        let msgs = core::reconcile_gather_objectives(world, entity, &templates);
        for msg in msgs {
            conn.send_line(&msg);
        }
    }
}

pub fn cmd_drop(
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
        conn.send_line("Drop what?");
        return;
    }

    let item = match super::common::find_item_in_inventory(world, entity, item_name) {
        Some(i) => i,
        None => {
            conn.send_line("You don't have that item.");
            return;
        }
    };

    let room = match get_pos_room(world, entity) {
        Some(r) => r,
        None => {
            conn.send_line("You are nowhere.");
            return;
        }
    };

    if let Ok(mut q) = world.query_one::<&mut core::Inventory>(entity) {
        if let Some(inv) = q.get() {
            inv.0.retain(|e| *e != item);
        }
    }

    if let Ok(mut q) = world.query_one::<&mut core::FloorItems>(room) {
        if let Some(floor) = q.get() {
            floor.0.push(item);
            conn.send_line("You drop it.");
        }
    }

    if let Some(templates) = oxide_server::get_templates() {
        let msgs = core::reconcile_gather_objectives(world, entity, &templates);
        for msg in msgs {
            conn.send_line(&msg);
        }
    }
}

pub fn cmd_put(
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
    let pos = match input
        .to_lowercase()
        .find(" in ")
        .or_else(|| input.to_lowercase().find(" into "))
    {
        Some(p) => p,
        None => {
            conn.send_line("Put what into what? (Syntax: put <item> in <container>)");
            return;
        }
    };

    let sep_len = if input[pos..].to_lowercase().starts_with(" into ") {
        6
    } else {
        4
    };
    let item_query = input[..pos].trim();
    let container_query = input[pos + sep_len..].trim();

    if item_query.is_empty() || container_query.is_empty() {
        conn.send_line("Put what into what?");
        return;
    }

    let room = match get_pos_room(world, entity) {
        Some(r) => r,
        None => {
            conn.send_line("You are nowhere.");
            return;
        }
    };

    let container_ent =
        match super::common::find_item_in_inv_or_room(world, entity, room, container_query) {
            Some(c) => c,
            None => {
                conn.send_line("You don't see that container.");
                return;
            }
        };

    let container_name =
        get_entity_name(world, container_ent).unwrap_or_else(|| "container".to_string());

    let mut item_container =
        match world.query_one::<&mut core::components::ItemContainer>(container_ent) {
            Ok(mut q) => match q.get() {
                Some(c) => c.clone(),
                None => {
                    conn.send_line("That is not a container.");
                    return;
                }
            },
            Err(_) => {
                conn.send_line("That is not a container.");
                return;
            }
        };

    if item_container.is_closed {
        conn.send_line(&format!("The {container_name} is closed."));
        return;
    }

    let player_inv = match world.query_one::<&core::Inventory>(entity) {
        Ok(mut q) => match q.get() {
            Some(inv) => inv.clone(),
            None => return,
        },
        Err(_) => return,
    };

    let candidates: Vec<(String, core::Entity)> = player_inv
        .0
        .iter()
        .filter(|&&e| e != container_ent)
        .filter_map(|&e| get_entity_name(world, e).map(|n| (n, e)))
        .collect();

    let items_to_put: Vec<core::Entity> = match core::trie::trie_match(item_query, candidates) {
        core::trie::TrieMatch::One(e) => vec![e],
        core::trie::TrieMatch::Many(es) => es,
        core::trie::TrieMatch::None => Vec::new(),
    };

    if items_to_put.is_empty() {
        conn.send_line("You don't have that item to put away.");
        return;
    }

    {
        let mut inv_q = world.query_one::<&mut core::Inventory>(entity).unwrap();
        if let Some(inv) = inv_q.get() {
            for item_ent in items_to_put {
                if item_container.max_items > 0
                    && item_container.contents.len() as u16 >= item_container.max_items
                {
                    conn.send_line(&format!("The {container_name} is full."));
                    break;
                }
                inv.0.retain(|&e| e != item_ent);
                item_container.contents.push(item_ent);
                let iname = get_entity_name(world, item_ent).unwrap_or_else(|| "item".to_string());
                conn.send_line(&format!("You put {iname} into the {container_name}."));
            }
        }
    }

    let _ = world.insert(container_ent, (item_container,));
}

pub fn cmd_give(
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

    let input = args.trim();
    let pos = match input.to_lowercase().find(" to ") {
        Some(p) => p,
        None => {
            conn.send_line("Give what to whom? (Syntax: give <item> to <player>)");
            return;
        }
    };

    let item_query = input[..pos].trim();
    let target_query = input[pos + 4..].trim();

    if item_query.is_empty() || target_query.is_empty() {
        conn.send_line("Give what to whom?");
        return;
    }

    let target_ent = match super::common::find_online_player(world, registry, target_query) {
        Some(p) => p,
        None => {
            conn.send_line("You don't see that person online.");
            return;
        }
    };

    if target_ent == entity {
        conn.send_line("You cannot give items to yourself.");
        return;
    }

    let item_ent = match super::common::find_item_in_inventory(world, entity, item_query) {
        Some(i) => i,
        None => {
            conn.send_line("You don't have that item.");
            return;
        }
    };

    if let Ok(mut q) = world.query_one::<&mut core::Inventory>(entity) {
        if let Some(inv) = q.get() {
            inv.0.retain(|&e| e != item_ent);
        }
    }
    if let Ok(mut q) = world.query_one::<&mut core::Inventory>(target_ent) {
        if let Some(inv) = q.get() {
            inv.0.push(item_ent);
        }
    }

    let item_name = get_entity_name(world, item_ent).unwrap_or_else(|| "item".to_string());
    let giver_name = core::get_name(world, entity)
        .map(|n| n.0.clone())
        .unwrap_or_else(|| "Someone".to_string());
    let recipient_name = core::get_name(world, target_ent)
        .map(|n| n.0.clone())
        .unwrap_or_else(|| "Someone".to_string());

    conn.send_line(&format!("You give {item_name} to {recipient_name}."));
    super::common::send_to_online_player(
        registry,
        target_ent,
        &format!("{giver_name} gives you {item_name}."),
    );
}

pub fn cmd_repair(
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
        conn.send_line("Repair what item?");
        return;
    }

    let room = match get_pos_room(world, entity) {
        Some(r) => r,
        None => {
            conn.send_line("You are nowhere.");
            return;
        }
    };

    let has_blacksmith = {
        let mut npcs = world.query::<(&core::Npc, &core::Position)>();
        npcs.iter().any(|(_, (npc, pos))| {
            pos.room == room
                && (npc.template_id.contains("blacksmith") || npc.template_id.contains("smith"))
        })
    };

    if !has_blacksmith {
        conn.send_line("There is no blacksmith or forge here to repair your equipment.");
        return;
    }

    let item_ent = match super::common::find_item_in_inventory(world, entity, item_name) {
        Some(i) => i,
        None => {
            conn.send_line("You don't have that item in your inventory.");
            return;
        }
    };

    let item_display_name = get_entity_name(world, item_ent).unwrap_or_else(|| "item".to_string());

    let missing_durability = {
        if let Ok(mut q) = world.query_one::<&core::components::Durability>(item_ent) {
            if let Some(dur) = q.get() {
                if dur.current >= dur.max {
                    conn.send_line(&format!(
                        "Your {item_display_name} is already in pristine condition."
                    ));
                    return;
                }
                dur.max - dur.current
            } else {
                conn.send_line("That item does not have durability.");
                return;
            }
        } else {
            conn.send_line("That item does not have durability.");
            return;
        }
    };

    let repair_cost_gold = (missing_durability as u64) * 2;

    let mut wallet = match world.query_one::<&mut core::Wallet>(entity) {
        Ok(mut q) => match q.get() {
            Some(w) => w.clone(),
            None => return,
        },
        Err(_) => return,
    };

    if wallet.gold < repair_cost_gold {
        conn.send_line(&format!(
            "Repairing your {item_display_name} costs {repair_cost_gold} gold, but you only have {} gold.",
            wallet.gold
        ));
        return;
    }

    wallet.gold -= repair_cost_gold;
    let _ = world.insert(entity, (wallet,));

    if let Ok(mut q) = world.query_one::<&mut core::components::Durability>(item_ent) {
        if let Some(dur) = q.get() {
            dur.repair(dur.max);
        }
    }

    conn.send_line(&format!(
        "The blacksmith repairs your {item_display_name} for {repair_cost_gold} gold. It is now good as new!"
    ));
}

pub fn cmd_loot(
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

    let corpse_name = args.trim();
    if corpse_name.is_empty() {
        conn.send_line("Loot what?");
        return;
    }

    let room = match get_pos_room(world, entity) {
        Some(r) => r,
        None => {
            conn.send_line("You are nowhere.");
            return;
        }
    };

    let corpse = {
        let mut q = world.query::<(&core::Corpse, &core::Position, &core::Name)>();
        let candidates: Vec<(String, core::Entity)> = q
            .iter()
            .filter(|(_, (_, pos, _))| pos.room == room)
            .map(|(raw, (_, _, name))| (name.as_str().to_lowercase(), raw))
            .collect();
        match core::trie::trie_match(corpse_name, candidates) {
            core::trie::TrieMatch::One(e) => Some(e),
            core::trie::TrieMatch::Many(items) => items.into_iter().next(),
            core::trie::TrieMatch::None => None,
        }
    };

    let corpse = match corpse {
        Some(c) => c,
        None => {
            conn.send_line("You don't see a corpse here.");
            return;
        }
    };

    let player_db_id = world
        .query_one::<&core::DbId>(entity)
        .ok()
        .and_then(|mut q| q.get().map(|d| d.0));

    if let Ok(mut q) = world.query_one::<&core::Corpse>(corpse) {
        if let Some(c_comp) = q.get() {
            match c_comp.lootable_by {
                core::LootRule::Public => {}
                core::LootRule::OwnerOnly | core::LootRule::Faction => {
                    let is_owner = c_comp.owner == Some(entity)
                        || (player_db_id.is_some() && c_comp.owner_db_id == player_db_id);
                    if !is_owner {
                        conn.send_line("This corpse does not belong to you.");
                        return;
                    }
                }
                core::LootRule::GroupOnly => {
                    let is_owner = c_comp.owner == Some(entity)
                        || (player_db_id.is_some() && c_comp.owner_db_id == player_db_id);
                    if !is_owner {
                        let mut in_same_group = false;
                        if let Ok(mut q_gm) = world.query_one::<&core::GroupMember>(entity) {
                            if let Some(gm) = q_gm.get() {
                                let group_entity = gm.group_id;
                                if let Ok(mut q_group) =
                                    world.query_one::<&core::Group>(group_entity)
                                {
                                    if let Some(group) = q_group.get() {
                                        in_same_group = group.members.iter().any(|m| {
                                            (c_comp.owner.is_some() && m.entity == c_comp.owner)
                                                || (c_comp.owner_db_id.is_some()
                                                    && Some(m.db_id) == c_comp.owner_db_id)
                                        });
                                    }
                                }
                            }
                        }
                        if !in_same_group {
                            conn.send_line("This corpse does not belong to you or your group.");
                            return;
                        }
                    }
                }
            }
        }
    }

    let items = world
        .query_one::<&core::Inventory>(corpse)
        .ok()
        .and_then(|mut q| q.get().map(|inv| inv.0.clone()))
        .unwrap_or_default();

    if items.is_empty() {
        conn.send_line("The corpse has nothing.");
        return;
    }

    if let Ok(mut q) = world.query_one::<&mut core::Inventory>(entity) {
        if let Some(inv) = q.get() {
            let count = items.len();
            inv.0.extend(items);
            conn.send_line(&format!("You loot {count} item(s) from the corpse."));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_helpers::*;
    use super::*;
    use oxide_core::Position;

    #[test]
    fn test_loot_rules_checks() {
        let (mut world, _void, room_a, _room_b) = test_world();
        let (player, mut conn, registry) = test_player(&mut world, room_a);

        world
            .insert(player, (core::DbId(42), core::Inventory::new()))
            .unwrap();

        let item = world.spawn((core::Item::new("sword"),));

        let _corpse_other = world.spawn((
            core::Corpse {
                owner: None,
                owner_db_id: Some(99),
                created_at: std::time::Instant::now(),
                decay_secs: 1800,
                lootable_by: core::LootRule::OwnerOnly,
            },
            Position::new(room_a),
            core::Name::new("corpse"),
            core::Inventory(vec![item]),
        ));

        cmd_loot(&mut world, &mut conn, "loot", "corpse", &registry);
        let lines = conn.take_lines();
        assert!(lines
            .iter()
            .any(|l| l.contains("This corpse does not belong to you.")));

        let _corpse_mine = world.spawn((
            core::Corpse {
                owner: None,
                owner_db_id: Some(42),
                created_at: std::time::Instant::now(),
                decay_secs: 1800,
                lootable_by: core::LootRule::OwnerOnly,
            },
            Position::new(room_a),
            core::Name::new("my_corpse"),
            core::Inventory(vec![item]),
        ));

        cmd_loot(&mut world, &mut conn, "loot", "my_corpse", &registry);
        let lines = conn.take_lines();
        assert!(!lines
            .iter()
            .any(|l| l.contains("This corpse does not belong to you.")));
    }

    #[test]
    fn test_two_handed_slot_restrictions() {
        let _guard = init_test_templates();

        let (mut world, _void, room_a, _room_b) = test_world();
        let (player, mut conn, conn_reg) = test_player(&mut world, room_a);

        let weapon_entity = world.spawn((
            core::Item::new("two_handed_sword"),
            core::Name::new("Greatsword"),
            core::Weapon {
                damage_dice: core::dice::DiceRoll::new(2, 6, 0),
                damage_type: core::DamageType::Slash,
                speed: 2.0,
                range: core::WeaponRange::Melee,
                hands: core::WeaponHands::TwoHand,
            },
        ));
        let shield_entity = world.spawn((
            core::Item::new("wooden_shield"),
            core::Name::new("Wooden Shield"),
            core::Armor { base: 2, bonus: 0 },
        ));

        world
            .insert(
                player,
                (
                    core::Inventory(vec![weapon_entity, shield_entity]),
                    core::Equipment::new(),
                ),
            )
            .unwrap();

        cmd_wield(&mut world, &mut conn, "wield", "greatsword", &conn_reg);
        let lines = conn.take_lines();
        assert!(lines.iter().any(|l| l.contains("You wield it.")));

        let has_weapon = world
            .query_one::<&core::Equipment>(player)
            .ok()
            .and_then(|mut q| {
                q.get()
                    .map(|eq| eq.equipped(&core::EquipmentSlot::Weapon).copied())
            })
            .flatten();
        assert_eq!(has_weapon, Some(weapon_entity));

        cmd_wear(&mut world, &mut conn, "wear", "wooden shield", &conn_reg);
        let lines = conn.take_lines();
        assert!(lines.iter().any(|l| l
            .contains("You are wielding a two-handed weapon and cannot use a shield/off-hand.")));

        let has_shield = world
            .query_one::<&core::Equipment>(player)
            .ok()
            .and_then(|mut q| {
                q.get()
                    .map(|eq| eq.equipped(&core::EquipmentSlot::Shield).copied())
            })
            .flatten();
        assert!(has_shield.is_none());

        cmd_remove(&mut world, &mut conn, "remove", "weapon", &conn_reg);
        let _ = conn.take_lines();

        cmd_wear(&mut world, &mut conn, "wear", "wooden shield", &conn_reg);
        let lines = conn.take_lines();
        assert!(lines.iter().any(|l| l.contains("You wear it.")));

        cmd_wield(&mut world, &mut conn, "wield", "greatsword", &conn_reg);
        let lines = conn.take_lines();
        assert!(lines
            .iter()
            .any(|l| l.contains("You unequip Wooden Shield to wield the two-handed weapon.")));

        let has_shield = world
            .query_one::<&core::Equipment>(player)
            .ok()
            .and_then(|mut q| {
                q.get()
                    .map(|eq| eq.equipped(&core::EquipmentSlot::Shield).copied())
            })
            .flatten();
        assert!(has_shield.is_none());

        let has_weapon = world
            .query_one::<&core::Equipment>(player)
            .ok()
            .and_then(|mut q| {
                q.get()
                    .map(|eq| eq.equipped(&core::EquipmentSlot::Weapon).copied())
            })
            .flatten();
        assert_eq!(has_weapon, Some(weapon_entity));
    }
}
