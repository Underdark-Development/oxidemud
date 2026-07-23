use oxide_core as core;
use oxide_core::templates::SkillResolveError;
use oxide_core::{get_pos_room, AccessLevel, FloorItems, Item, Name, World};
use oxide_server::{Command, CommandHelp, Connection, ConnectionRegistry, Server};

use super::common::*;

pub const HELP_TRAIN: &str =
    "Usage: train [attribute]\n  show attributes and cost, or train an attribute";

pub const HELP_PRACTICE: &str = "Usage: practice [list|skill]\n  show skills and points, list trainable skills, or train a skill";

pub const HELP_PRAY: &str =
    "Usage: pray\n  offer a prayer to your deity for a blessing (cooldown applies)";

pub fn register(server: &mut Server) {
    server.register_command(Command {
        name: "affects",
        aliases: &["effects", "spells"],
        access: AccessLevel::Player,
        topic: "Character",
        help: CommandHelp {
            short: "Display active spells, effects, and passives",
            body: None,
        },
        handler: cmd_affects,
    });
    server.register_command(Command {
        name: "score",
        aliases: &["stats"],
        access: AccessLevel::Player,
        topic: "Character",
        help: CommandHelp {
            short: "Display your character stats",
            body: None,
        },
        handler: cmd_score,
    });
    server.register_command(Command {
        name: "quest",
        aliases: &["quests"],
        access: AccessLevel::Player,
        topic: "Character",
        help: CommandHelp {
            short: "Manage your quests",
            body: None,
        },
        handler: cmd_quest,
    });
    server.register_command(Command {
        name: "faction",
        aliases: &["factions"],
        access: AccessLevel::Player,
        topic: "Character",
        help: CommandHelp {
            short: "Display faction standings and ranks",
            body: None,
        },
        handler: cmd_faction,
    });
    server.register_command(Command {
        name: "recipes",
        aliases: &[],
        access: AccessLevel::Player,
        topic: "Character",
        help: CommandHelp {
            short: "Display learned crafting recipes",
            body: None,
        },
        handler: cmd_recipes,
    });
    server.register_command(Command {
        name: "train",
        aliases: &[],
        access: AccessLevel::Player,
        topic: "Character",
        help: CommandHelp {
            short: "Spend practice points to increase attributes",
            body: Some(HELP_TRAIN),
        },
        handler: cmd_train,
    });
    server.register_command(Command {
        name: "practice",
        aliases: &[],
        access: AccessLevel::Player,
        topic: "Character",
        help: CommandHelp {
            short: "View skills and spend practice points",
            body: Some(HELP_PRACTICE),
        },
        handler: cmd_practice,
    });
    server.register_command(Command {
        name: "pray",
        aliases: &[],
        access: AccessLevel::Player,
        topic: "Character",
        help: CommandHelp {
            short: "Pray to your deity for a blessing",
            body: Some(HELP_PRAY),
        },
        handler: cmd_pray,
    });
    server.register_command(Command {
        name: "sit",
        aliases: &[],
        access: AccessLevel::Player,
        topic: "Character",
        help: CommandHelp {
            short: "Sit down to rest or look around",
            body: None,
        },
        handler: cmd_sit,
    });
    server.register_command(Command {
        name: "rest",
        aliases: &[],
        access: AccessLevel::Player,
        topic: "Character",
        help: CommandHelp {
            short: "Rest and recover health/mana/stamina faster",
            body: None,
        },
        handler: cmd_rest,
    });
    server.register_command(Command {
        name: "sleep",
        aliases: &[],
        access: AccessLevel::Player,
        topic: "Character",
        help: CommandHelp {
            short: "Go to sleep for maximum recovery rate",
            body: None,
        },
        handler: cmd_sleep,
    });
    server.register_command(Command {
        name: "wake",
        aliases: &[],
        access: AccessLevel::Player,
        topic: "Character",
        help: CommandHelp {
            short: "Wake up from sleep",
            body: None,
        },
        handler: cmd_wake,
    });
    server.register_command(Command {
        name: "stand",
        aliases: &[],
        access: AccessLevel::Player,
        topic: "Character",
        help: CommandHelp {
            short: "Stand up to allow movement and combat",
            body: None,
        },
        handler: cmd_stand,
    });
    server.register_command(Command {
        name: "die",
        aliases: &[],
        access: AccessLevel::Player,
        topic: "Character",
        help: CommandHelp {
            short: "Submit to death when unconscious to respawn as a ghost",
            body: None,
        },
        handler: cmd_die,
    });
    server.register_command(Command {
        name: "reclaim",
        aliases: &[],
        access: AccessLevel::Player,
        topic: "Character",
        help: CommandHelp {
            short: "Reclaim your corpse to return to life with your items",
            body: None,
        },
        handler: cmd_reclaim,
    });
    server.register_command(Command {
        name: "revive",
        aliases: &[],
        access: AccessLevel::Player,
        topic: "Character",
        help: CommandHelp {
            short: "Pray at an altar or reclaim your corpse to revive",
            body: None,
        },
        handler: cmd_revive,
    });
    server.register_command(Command {
        name: "toggle",
        aliases: &[],
        access: AccessLevel::Player,
        topic: "Character",
        help: CommandHelp {
            short: "Toggle player settings",
            body: None,
        },
        handler: cmd_toggle,
    });
    server.register_command(Command {
        name: "@advance",
        aliases: &[],
        access: AccessLevel::Player,
        topic: "Progression",
        help: CommandHelp {
            short: "Spend a pending level to advance a class level",
            body: None,
        },
        handler: cmd_advance,
    });
    server.register_command(Command {
        name: "@multi_class",
        aliases: &[],
        access: AccessLevel::Player,
        topic: "Progression",
        help: CommandHelp {
            short: "Adopt a new base class at level 1",
            body: None,
        },
        handler: cmd_multi_class,
    });
    server.register_command(Command {
        name: "@prestige",
        aliases: &[],
        access: AccessLevel::Player,
        topic: "Progression",
        help: CommandHelp {
            short: "Adopt a prestige class at level 1",
            body: None,
        },
        handler: cmd_prestige,
    });
}

pub fn cmd_score(
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

    let name = world
        .query_one::<&core::Name>(entity)
        .ok()
        .and_then(|mut q| q.get().map(|n| n.to_string()))
        .unwrap_or_else(|| "Unknown".to_string());

    let level = world
        .query_one::<&core::Level>(entity)
        .ok()
        .and_then(|mut q| q.get().copied())
        .unwrap_or(core::Level(1));

    let xp = world
        .query_one::<&core::Experience>(entity)
        .ok()
        .and_then(|mut q| q.get().copied())
        .unwrap_or(core::Experience(0));

    let attrs = world
        .query_one::<&core::Attributes>(entity)
        .ok()
        .and_then(|mut q| q.get().cloned())
        .unwrap_or_default();

    let hp = world
        .query_one::<&core::Health>(entity)
        .ok()
        .and_then(|mut q| q.get().cloned())
        .unwrap_or(core::Health::new(20));

    let practice_pts = world
        .query_one::<&core::PracticePoints>(entity)
        .ok()
        .and_then(|mut q| q.get().copied())
        .unwrap_or(core::PracticePoints(0));

    let combat_stats = world
        .query_one::<&core::CombatStats>(entity)
        .ok()
        .and_then(|mut q| q.get().cloned())
        .unwrap_or_default();

    let mana = world
        .query_one::<&core::Mana>(entity)
        .ok()
        .and_then(|mut q| q.get().copied());

    let stamina = world
        .query_one::<&core::Stamina>(entity)
        .ok()
        .and_then(|mut q| q.get().copied());

    let energy = world
        .query_one::<&core::Energy>(entity)
        .ok()
        .and_then(|mut q| q.get().copied());

    let psi = world
        .query_one::<&core::Psi>(entity)
        .ok()
        .and_then(|mut q| q.get().copied());

    let deity = world
        .query_one::<&core::Deity>(entity)
        .ok()
        .and_then(|mut q| q.get().cloned())
        .map(|d| d.0.unwrap_or_else(|| "none".to_string()))
        .unwrap_or_else(|| "none".to_string());

    let age = world
        .query_one::<&core::Age>(entity)
        .ok()
        .and_then(|mut q| q.get().copied())
        .map(|a| a.0)
        .unwrap_or(20);

    let appearance = world
        .query_one::<&core::Appearance>(entity)
        .ok()
        .and_then(|mut q| q.get().cloned());

    let xp_to_next = xp.to_next_level(level.0);

    conn.send_line("");
    conn.send_line("--- Character Score ---");
    conn.send_line(&format!("  Name:            {}", name));
    conn.send_line(&format!("  Level:           {}", level.0));
    conn.send_line(&format!(
        "  Experience:      {} / {} ({} to next level)",
        xp.0,
        core::Experience::for_level(level.0 + 1),
        xp_to_next
    ));
    conn.send_line(&format!("  Practice Points: {}", practice_pts.0));
    conn.send_line(&format!("  HP:              {} / {}", hp.current, hp.max));
    conn.send_line(&format!("  Deity:           {}", deity));
    conn.send_line(&format!("  Age:             {} years", age));
    if let Some(app) = appearance {
        conn.send_line(&format!(
            "  Appearance:      {}in, {}lbs, {} build",
            app.height, app.weight, app.build
        ));
        conn.send_line(&format!(
            "                   Hair: {} ({}), Eyes: {}, Skin: {}",
            app.hair_style, app.hair_color, app.eye_color, app.skin_tone
        ));
    }

    if let Some(m) = mana {
        conn.send_line(&format!("  Mana:            {} / {}", m.current, m.max));
    }
    if let Some(s) = stamina {
        conn.send_line(&format!("  Stamina:         {} / {}", s.current, s.max));
    }
    if let Some(e) = energy {
        conn.send_line(&format!("  Energy:          {} / {}", e.current, e.max));
    }
    if let Some(p) = psi {
        conn.send_line(&format!("  Psi:             {} / {}", p.current, p.max));
    }

    let format_modifier = |val: i32| -> String {
        if val >= 0 {
            format!("+{}", val)
        } else {
            val.to_string()
        }
    };

    conn.send_line("");
    conn.send_line(&format!(
        "  BAB:             {}",
        format_modifier(combat_stats.base_attack_bonus)
    ));
    conn.send_line(&format!(
        "  Saves:           Fort: {}, Ref: {}, Will: {}",
        format_modifier(combat_stats.fort_save),
        format_modifier(combat_stats.ref_save),
        format_modifier(combat_stats.will_save)
    ));

    conn.send_line("");
    conn.send_line("  Attributes:");
    conn.send_line(&format!("    Strength:     {}", attrs.strength));
    conn.send_line(&format!("    Dexterity:    {}", attrs.dexterity));
    conn.send_line(&format!("    Intelligence: {}", attrs.intelligence));
    conn.send_line(&format!("    Wisdom:       {}", attrs.wisdom));
    conn.send_line(&format!("    Constitution: {}", attrs.constitution));
    conn.send_line(&format!("    Charisma:     {}", attrs.charisma));
    conn.send_line("");
}

fn max_rank_for_level(level: u8) -> u16 {
    (level as u16 * 5) + 5
}

fn resolve_skill_name_for_practicing(
    input: &str,
    world: &World,
    entity: core::Entity,
) -> Result<String, String> {
    let templates = match oxide_server::get_templates() {
        Some(t) => t,
        None => return Ok(input.to_lowercase()),
    };

    let raw = input.to_lowercase();
    match templates.resolve_skill(&raw, None) {
        Ok(id) => {
            let known = world
                .query_one::<&core::LearnedSkills>(entity)
                .ok()
                .and_then(|mut q| q.get().cloned())
                .is_some_and(|s| s.has(&id));
            if known {
                Ok(id)
            } else {
                Ok(raw)
            }
        }
        Err(SkillResolveError::NotFound) => Ok(raw),
        Err(SkillResolveError::Multiple(candidates)) => {
            let names: Vec<String> = candidates
                .into_iter()
                .map(|(id, name)| format!("{id} ({name})"))
                .collect();
            Err(format!("Which skill did you mean? {}", names.join(", ")))
        }
    }
}

pub fn cmd_practice(
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

    let level = match world.query_one::<&core::Level>(entity) {
        Ok(mut q) => match q.get() {
            Some(l) => *l,
            None => {
                conn.send_line("You have no level component.");
                return;
            }
        },
        Err(_) => {
            conn.send_line("You have no level component.");
            return;
        }
    };

    let args = args.trim();

    if args.is_empty() || args.eq_ignore_ascii_case("list") {
        let skills = match world.query_one::<&core::LearnedSkills>(entity) {
            Ok(mut q) => match q.get() {
                Some(s) => s.clone(),
                None => {
                    conn.send_line("You have no skills component.");
                    return;
                }
            },
            Err(_) => {
                conn.send_line("You have no skills component.");
                return;
            }
        };

        let practice_pts = match world.query_one::<&core::PracticePoints>(entity) {
            Ok(mut q) => match q.get() {
                Some(p) => *p,
                None => {
                    conn.send_line("You have no practice points.");
                    return;
                }
            },
            Err(_) => {
                conn.send_line("You have no practice points.");
                return;
            }
        };

        conn.send_line("");
        conn.send_line("--- Practice & Skills ---");
        conn.send_line(&format!("Practice points: {}", practice_pts.0));
        conn.send_line("");

        if skills.skills.is_empty() {
            conn.send_line("You know no skills.");
        } else {
            conn.send_line("Known skills:");
            let max_rank = max_rank_for_level(level.0);
            for (id, rank) in &skills.skills {
                conn.send_line(&format!("  {:<20} rank {:>3} / {:>3}", id, rank, max_rank));
            }
        }
        conn.send_line("");
        conn.send_line("  Skills are granted through race/class selection.");
        conn.send_line("  Use 'practice <skill>' to increase a known skill's rank.");
        conn.send_line("");
        return;
    }

    let skill_id = match resolve_skill_name_for_practicing(args, world, entity) {
        Ok(id) => id,
        Err(msg) => {
            conn.send_line(&msg);
            return;
        }
    };

    let mut skills = match world.query_one::<&mut core::LearnedSkills>(entity) {
        Ok(mut q) => match q.get() {
            Some(s) => s.clone(),
            None => {
                conn.send_line("You have no skills component.");
                return;
            }
        },
        Err(_) => {
            conn.send_line("You have no skills component.");
            return;
        }
    };

    let mut practice_pts = match world.query_one::<&mut core::PracticePoints>(entity) {
        Ok(mut q) => match q.get() {
            Some(p) => *p,
            None => {
                conn.send_line("You have no practice points.");
                return;
            }
        },
        Err(_) => {
            conn.send_line("You have no practice points.");
            return;
        }
    };

    let current_rank = skills.rank(&skill_id);
    if current_rank == 0 {
        conn.send_line(&format!("You don't know the skill '{skill_id}'."));
        return;
    }

    let room = match get_pos_room(world, entity) {
        Some(r) => r,
        None => {
            conn.send_line("You can't do that here. Seek out a trainer.");
            return;
        }
    };

    let room_entities = core::util::entities_in_room(world, room);
    let mut has_trainer = false;
    for e in &room_entities {
        if let Ok(mut q) = world.query_one::<&core::Trainer>(*e) {
            if q.get().is_some() {
                has_trainer = true;
                break;
            }
        }
    }

    if !has_trainer {
        conn.send_line("You can't do that here. Seek out a trainer.");
        return;
    }

    let templates = oxide_server::get_templates();
    let skill_def = templates.as_ref().and_then(|t| t.get_skill(&skill_id));
    let skill_type_str = skill_def
        .map(|def| format!("{:?}", def.skill_type).to_lowercase())
        .unwrap_or_else(|| "combat".to_string());

    let mut can_train_skill = false;
    for e in room_entities {
        if let Ok(mut q) = world.query_one::<&core::Trainer>(e) {
            if let Some(t) = q.get() {
                if t.can_train(&skill_type_str) {
                    can_train_skill = true;
                    break;
                }
            }
        }
    }

    if !can_train_skill {
        conn.send_line("You can't practice that here.");
        return;
    }

    let max_rank = max_rank_for_level(level.0);
    if current_rank >= max_rank {
        conn.send_line(&format!(
            "You cannot practice '{skill_id}' beyond rank {max_rank} at your level."
        ));
        return;
    }

    let cost = 1;
    if practice_pts.0 < cost {
        conn.send_line(&format!(
            "Practicing '{skill_id}' costs {cost} point(s), but you only have {}.",
            practice_pts.0
        ));
        return;
    }

    practice_pts.0 -= cost;
    let new_rank = current_rank + 1;
    skills.set_rank(&skill_id, new_rank);
    let remaining = practice_pts.0;

    let _ = world.insert(entity, (skills, practice_pts, core::Dirty));
    conn.send_line(&format!(
        "You practice '{skill_id}' to rank {new_rank}. ({remaining} point(s) remaining)",
    ));
}

pub fn cmd_train(
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

    let args = args.trim();

    let mut attrs = match world.query_one::<&mut core::Attributes>(entity) {
        Ok(mut q) => match q.get() {
            Some(a) => a.clone(),
            None => {
                conn.send_line("You have no attributes component.");
                return;
            }
        },
        Err(_) => {
            conn.send_line("You have no attributes component.");
            return;
        }
    };

    let mut practice_pts = match world.query_one::<&mut core::PracticePoints>(entity) {
        Ok(mut q) => match q.get() {
            Some(p) => *p,
            None => {
                conn.send_line("You have no practice points.");
                return;
            }
        },
        Err(_) => {
            conn.send_line("You have no practice points.");
            return;
        }
    };

    let class_id = world
        .query_one::<&core::Class>(entity)
        .ok()
        .and_then(|mut q| q.get().map(|c| c.0.clone()));

    let templates = oxide_server::get_templates();

    let get_attr_cost = |attr_name: &str| -> u32 {
        if let (Some(c_id), Some(t)) = (&class_id, &templates) {
            if let Some(class_template) = t.get_class(c_id) {
                let c = &class_template.attribute_mods;
                let mods = [
                    c.strength,
                    c.dexterity,
                    c.intelligence,
                    c.wisdom,
                    c.constitution,
                    c.charisma,
                ];
                let max_val = mods.into_iter().max().unwrap_or(0);
                let is_prime = match attr_name {
                    "strength" => c.strength == max_val,
                    "dexterity" => c.dexterity == max_val,
                    "intelligence" => c.intelligence == max_val,
                    "wisdom" => c.wisdom == max_val,
                    "constitution" => c.constitution == max_val,
                    "charisma" => c.charisma == max_val,
                    _ => false,
                };
                if is_prime {
                    return 3;
                }
            }
        }
        5
    };

    if args.is_empty() {
        conn.send_line("");
        conn.send_line("--- Attribute Training ---");
        conn.send_line(&format!("Practice points: {}", practice_pts.0));
        conn.send_line("");
        conn.send_line("Attributes:");
        conn.send_line(&format!(
            "  Strength:     {} (cost: {} pts)",
            attrs.strength,
            get_attr_cost("strength")
        ));
        conn.send_line(&format!(
            "  Dexterity:    {} (cost: {} pts)",
            attrs.dexterity,
            get_attr_cost("dexterity")
        ));
        conn.send_line(&format!(
            "  Intelligence: {} (cost: {} pts)",
            attrs.intelligence,
            get_attr_cost("intelligence")
        ));
        conn.send_line(&format!(
            "  Wisdom:       {} (cost: {} pts)",
            attrs.wisdom,
            get_attr_cost("wisdom")
        ));
        conn.send_line(&format!(
            "  Constitution: {} (cost: {} pts)",
            attrs.constitution,
            get_attr_cost("constitution")
        ));
        conn.send_line(&format!(
            "  Charisma:     {} (cost: {} pts)",
            attrs.charisma,
            get_attr_cost("charisma")
        ));
        conn.send_line("");
        return;
    }

    let input = args.to_lowercase();
    let (target_attr, attr_name_cap) = match input.as_str() {
        "str" | "strength" => ("strength", "Strength"),
        "dex" | "dexterity" => ("dexterity", "Dexterity"),
        "int" | "intelligence" => ("intelligence", "Intelligence"),
        "wis" | "wisdom" => ("wisdom", "Wisdom"),
        "con" | "constitution" => ("constitution", "Constitution"),
        "cha" | "charisma" => ("charisma", "Charisma"),
        _ => {
            conn.send_line("Invalid attribute. Choose from: Strength, Dexterity, Intelligence, Wisdom, Constitution, Charisma.");
            return;
        }
    };

    let room = match get_pos_room(world, entity) {
        Some(r) => r,
        None => {
            conn.send_line("You can't do that here. Seek out a trainer.");
            return;
        }
    };

    let room_entities = core::util::entities_in_room(world, room);
    let mut has_trainer = false;
    for e in &room_entities {
        if let Ok(mut q) = world.query_one::<&core::Trainer>(*e) {
            if q.get().is_some() {
                has_trainer = true;
                break;
            }
        }
    }

    if !has_trainer {
        conn.send_line("You can't do that here. Seek out a trainer.");
        return;
    }

    let mut can_train_attr = false;
    for e in room_entities {
        if let Ok(mut q) = world.query_one::<&core::Trainer>(e) {
            if let Some(t) = q.get() {
                if t.can_train("attributes") {
                    can_train_attr = true;
                    break;
                }
            }
        }
    }

    if !can_train_attr {
        conn.send_line("You can't train that here.");
        return;
    }

    let current_val = match target_attr {
        "strength" => attrs.strength,
        "dexterity" => attrs.dexterity,
        "intelligence" => attrs.intelligence,
        "wisdom" => attrs.wisdom,
        "constitution" => attrs.constitution,
        "charisma" => attrs.charisma,
        _ => unreachable!(),
    };

    if current_val >= core::Attributes::MAX {
        conn.send_line(&format!(
            "Your {} is already at the maximum of {}.",
            attr_name_cap,
            core::Attributes::MAX
        ));
        return;
    }

    let cost = get_attr_cost(target_attr);
    if practice_pts.0 < cost {
        conn.send_line(&format!(
            "Training {} costs {} practice points, but you only have {}.",
            attr_name_cap, cost, practice_pts.0
        ));
        return;
    }

    practice_pts.0 -= cost;
    let new_val = current_val + 1;
    match target_attr {
        "strength" => attrs.strength = new_val,
        "dexterity" => attrs.dexterity = new_val,
        "intelligence" => attrs.intelligence = new_val,
        "wisdom" => attrs.wisdom = new_val,
        "constitution" => attrs.constitution = new_val,
        "charisma" => attrs.charisma = new_val,
        _ => unreachable!(),
    }

    let _ = world.insert(entity, (attrs, practice_pts, core::Dirty));
    conn.send_line(&format!(
        "You train {} to {}. ({} practice points remaining)",
        attr_name_cap, new_val, practice_pts.0
    ));
}

fn room_has_shrine_for_deity(world: &World, player: core::Entity, deity_id: &str) -> bool {
    let Some(room) = get_pos_room(world, player) else {
        return false;
    };
    let Ok(mut q) = world.query_one::<&FloorItems>(room) else {
        return false;
    };
    let Some(floor_items) = q.get() else {
        return false;
    };
    for &item_entity in &floor_items.0 {
        if let Ok(mut item_q) = world.query_one::<(&Name, &Item)>(item_entity) {
            if let Some((name, item)) = item_q.get() {
                let name_lower = name.0.to_lowercase();
                if name_lower.contains("shrine") && name_lower.contains(&deity_id.to_lowercase()) {
                    return true;
                }
                if item.template_id.to_lowercase().contains("shrine")
                    && item
                        .template_id
                        .to_lowercase()
                        .contains(&deity_id.to_lowercase())
                {
                    return true;
                }
            }
        }
    }
    false
}

pub fn cmd_pray(
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

    let target = args.trim();
    let player_deity = world
        .query_one::<&core::Deity>(entity)
        .ok()
        .and_then(|mut q| q.get().cloned())
        .and_then(|d| d.0);

    let templates = match oxide_server::get_templates() {
        Some(t) => t,
        None => {
            conn.send_line("Server error: templates unavailable.");
            return;
        }
    };

    let deity_id = if target.is_empty() {
        match player_deity {
            Some(d) => d,
            None => {
                conn.send_line("You do not follow a deity. Who do you wish to pray to?");
                return;
            }
        }
    } else {
        let resolved = templates
            .deities
            .keys()
            .find(|k| k.to_lowercase() == target.to_lowercase())
            .cloned();
        let deity_id = match resolved {
            Some(d) => d,
            None => {
                conn.send_line(&format!("There is no deity named '{}'.", target));
                return;
            }
        };

        if Some(&deity_id) != player_deity.as_ref() {
            let room_has_shrine = room_has_shrine_for_deity(world, entity, &deity_id);
            if !room_has_shrine {
                conn.send_line(&format!(
                    "You do not follow {}, and there is no shrine to them here.",
                    deity_id
                ));
                return;
            }
        }
        deity_id
    };

    let deity_tmpl = match templates.deities.get(&deity_id) {
        Some(d) => d,
        None => {
            conn.send_line(&format!(
                "The deity '{}' does not exist in our archives.",
                deity_id
            ));
            return;
        }
    };

    let effect = match &deity_tmpl.prayer_effect {
        Some(e) => e,
        None => {
            conn.send_line(&format!(
                "You pray to {}, but feel no response.",
                deity_tmpl.name
            ));
            return;
        }
    };

    let now = std::time::Instant::now();
    let on_cooldown = if let Ok(mut q) = world.query_one::<&core::PrayerCooldown>(entity) {
        if let Some(cooldown) = q.get() {
            let elapsed = now.duration_since(cooldown.last_prayed).as_secs();
            if elapsed < effect.cooldown_secs {
                let wait = effect.cooldown_secs - elapsed;
                conn.send_line(&format!("Your prayers to {} have been answered too recently. You must wait {} more seconds.", deity_tmpl.name, wait));
                true
            } else {
                false
            }
        } else {
            false
        }
    } else {
        false
    };

    if on_cooldown {
        return;
    }

    let _ = world.remove_one::<core::PrayerCooldown>(entity);
    let _ = world.insert(entity, (core::PrayerCooldown { last_prayed: now },));

    conn.send_line(&format!(
        "You bow your head and pray to {}.",
        deity_tmpl.name
    ));
    conn.send_line(&format!("You feel a response: {}", effect.description));

    if deity_id == "solaris" {
        if let Ok(mut q) = world.query_one::<&mut core::Health>(entity) {
            if let Some(h) = q.get() {
                let restore = h.max / 4;
                h.current = (h.current + restore).min(h.max);
                conn.send_line("Your wounds are knit by Solaris' light.");
            }
        }
        if let Ok(mut q) = world.query_one::<&mut core::Stamina>(entity) {
            if let Some(s) = q.get() {
                let restore = s.max / 2;
                s.current = (s.current + restore).min(s.max);
                conn.send_line("You feel a surge of solar stamina.");
            }
        }
    } else if deity_id == "luna" {
        if let Ok(mut q) = world.query_one::<&mut core::Mana>(entity) {
            if let Some(m) = q.get() {
                let restore = m.max / 3;
                m.current = (m.current + restore).min(m.max);
                conn.send_line("Moonlight replenishes your magical energy.");
            }
        }
    } else if deity_id == "astra" {
        if let Ok(mut q) = world.query_one::<&mut core::Mana>(entity) {
            if let Some(m) = q.get() {
                let restore = m.max / 4;
                m.current = (m.current + restore).min(m.max);
                conn.send_line("Starlight clears your mind and replenishes your mana.");
            }
        }
        if let Ok(mut q) = world.query_one::<&mut core::Stamina>(entity) {
            if let Some(s) = q.get() {
                let restore = s.max / 4;
                s.current = (s.current + restore).min(s.max);
                conn.send_line("Starlight fills your limbs with light energy.");
            }
        }
    } else if deity_id == "kronos" {
        if let Ok(mut q) = world.query_one::<&mut core::Stamina>(entity) {
            if let Some(s) = q.get() {
                let restore = s.max / 2;
                s.current = (s.current + restore).min(s.max);
                conn.send_line("You feel a moment of stillness, and your stamina is restored.");
            }
        }
    } else if deity_id == "vulgath" {
        if let Ok(mut q) = world.query_one::<&mut core::Health>(entity) {
            if let Some(h) = q.get() {
                let restore = h.max / 6;
                h.current = (h.current + restore).min(h.max);
                conn.send_line("Sinuous shadows envelope you, drawing away pain.");
            }
        }
        if let Ok(mut q) = world.query_one::<&mut core::Mana>(entity) {
            if let Some(m) = q.get() {
                let restore = m.max / 6;
                m.current = (m.current + restore).min(m.max);
                conn.send_line("Sinuous shadows seep into your mind, replenishing your mana.");
            }
        }
    } else if deity_id == "karrgath" {
        if let Ok(mut q) = world.query_one::<&mut core::Health>(entity) {
            if let Some(h) = q.get() {
                let restore = h.max / 8;
                h.current = (h.current + restore).min(h.max);
                conn.send_line("A surge of iron fury knit your minor wounds.");
            }
        }
        if let Ok(mut q) = world.query_one::<&mut core::Stamina>(entity) {
            if let Some(s) = q.get() {
                let restore = s.max / 3;
                s.current = (s.current + restore).min(s.max);
                conn.send_line("A surge of war-fury restores your stamina.");
            }
        }
    }

    let active_effect = core::ActiveEffect {
        source: format!("prayer:{}", deity_id),
        stat: Some(effect.buff_id.clone()),
        amount: Some(1),
        aura_id: None,
        radius: None,
    };
    let mut effects = world
        .query_one::<&Vec<core::ActiveEffect>>(entity)
        .ok()
        .and_then(|mut q| q.get().cloned())
        .unwrap_or_default();
    effects.retain(|e| !e.source.starts_with("prayer:"));
    effects.push(active_effect);
    let _ = world.remove_one::<Vec<core::ActiveEffect>>(entity);
    let _ = world.insert(entity, (effects,));

    let _ = world.insert(entity, (core::Dirty,));
}

pub fn cmd_sit(
    world: &mut World,
    conn: &mut dyn Connection,
    _name: &str,
    _args: &str,
    registry: &ConnectionRegistry,
) {
    let entity = match conn.entity() {
        Some(e) => e,
        None => return,
    };

    match core::try_transition_player_state(world, entity, core::PlayerStateTrigger::Sit) {
        Ok(_) => {
            conn.send_line("You sit down.");
            if let Ok(mut name_q) = world.query_one::<&core::Name>(entity) {
                if let Some(name) = name_q.get() {
                    if let Some(room) = get_pos_room(world, entity) {
                        broadcast_to_room_except(
                            world,
                            registry,
                            room,
                            entity,
                            &format!("{} sits down.", name.0),
                        );
                    }
                }
            }
        }
        Err(err) => {
            let msg = match err {
                "You are stunned and cannot move." => "You are stunned and cannot move.",
                "You are too busy casting to do that." => "You are too busy casting to sit down.",
                "You must wake up first." => "You must wake up first.",
                "You are unconscious." => "You are unconscious.",
                "You are already sitting." => "You are already sitting.",
                "You are a ghost and cannot do that." => "You are a ghost! Ghosts do not sit down.",
                _ => err,
            };
            conn.send_line(msg);
        }
    }
}

pub fn cmd_rest(
    world: &mut World,
    conn: &mut dyn Connection,
    _name: &str,
    _args: &str,
    registry: &ConnectionRegistry,
) {
    let entity = match conn.entity() {
        Some(e) => e,
        None => return,
    };

    match core::try_transition_player_state(world, entity, core::PlayerStateTrigger::Rest) {
        Ok(_) => {
            conn.send_line("You lean back and rest.");
            if let Ok(mut name_q) = world.query_one::<&core::Name>(entity) {
                if let Some(name) = name_q.get() {
                    if let Some(room) = get_pos_room(world, entity) {
                        broadcast_to_room_except(
                            world,
                            registry,
                            room,
                            entity,
                            &format!("{} rests.", name.0),
                        );
                    }
                }
            }
        }
        Err(err) => {
            let msg = match err {
                "You are stunned and cannot move." => "You are stunned and cannot rest.",
                "You are too busy casting to do that." => "You are too busy casting to rest.",
                "You must wake up first." => "You must wake up first.",
                "You are unconscious." => "You are unconscious.",
                "You are already resting." => "You are already resting.",
                "You are a ghost and cannot do that." => "You are a ghost! Ghosts do not rest.",
                _ => err,
            };
            conn.send_line(msg);
        }
    }
}

pub fn cmd_sleep(
    world: &mut World,
    conn: &mut dyn Connection,
    _name: &str,
    _args: &str,
    registry: &ConnectionRegistry,
) {
    let entity = match conn.entity() {
        Some(e) => e,
        None => return,
    };

    match core::try_transition_player_state(world, entity, core::PlayerStateTrigger::Sleep) {
        Ok(_) => {
            conn.send_line("You lie down and go to sleep.");
            if let Ok(mut name_q) = world.query_one::<&core::Name>(entity) {
                if let Some(name) = name_q.get() {
                    if let Some(room) = get_pos_room(world, entity) {
                        broadcast_to_room_except(
                            world,
                            registry,
                            room,
                            entity,
                            &format!("{} goes to sleep.", name.0),
                        );
                    }
                }
            }
        }
        Err(err) => {
            let msg = match err {
                "You are stunned and cannot move." => "You are stunned and cannot sleep.",
                "You are too busy casting to do that." => "You are too busy casting to sleep.",
                "You must wake up first." => "You must wake up first.",
                "You are unconscious." => "You are unconscious.",
                "You are already sleeping." => "You are already sleeping.",
                "You are a ghost and cannot do that." => "You are a ghost! Ghosts do not sleep.",
                _ => err,
            };
            conn.send_line(msg);
        }
    }
}

pub fn cmd_wake(
    world: &mut World,
    conn: &mut dyn Connection,
    _name: &str,
    _args: &str,
    registry: &ConnectionRegistry,
) {
    let entity = match conn.entity() {
        Some(e) => e,
        None => return,
    };

    match core::try_transition_player_state(world, entity, core::PlayerStateTrigger::Wake) {
        Ok(_) => {
            conn.send_line("You wake up.");
            if let Ok(mut name_q) = world.query_one::<&core::Name>(entity) {
                if let Some(name) = name_q.get() {
                    if let Some(room) = get_pos_room(world, entity) {
                        broadcast_to_room_except(
                            world,
                            registry,
                            room,
                            entity,
                            &format!("{} wakes up.", name.0),
                        );
                    }
                }
            }
        }
        Err(err) => {
            let msg = match err {
                "You are stunned and cannot move." => "You are stunned and cannot wake up.",
                "You are unconscious." => "You are unconscious.",
                "You are already awake." => "You are already awake.",
                "You are a ghost and cannot do that." => "You are a ghost! You cannot wake up.",
                _ => err,
            };
            conn.send_line(msg);
        }
    }
}

pub fn cmd_stand(
    world: &mut World,
    conn: &mut dyn Connection,
    _name: &str,
    _args: &str,
    registry: &ConnectionRegistry,
) {
    let entity = match conn.entity() {
        Some(e) => e,
        None => return,
    };

    match core::try_transition_player_state(world, entity, core::PlayerStateTrigger::Stand) {
        Ok(_) => {
            conn.send_line("You stand up.");
            if let Ok(mut name_q) = world.query_one::<&core::Name>(entity) {
                if let Some(name) = name_q.get() {
                    if let Some(room) = get_pos_room(world, entity) {
                        broadcast_to_room_except(
                            world,
                            registry,
                            room,
                            entity,
                            &format!("{} stands up.", name.0),
                        );
                    }
                }
            }
        }
        Err(err) => {
            let msg = match err {
                "You are stunned and cannot move." => "You are stunned and cannot stand up.",
                "You are unconscious." => "You are unconscious.",
                "You are already standing." => "You are already standing.",
                "You are too busy casting to do that." => "You stand up (you were already awake).",
                "You are a ghost and cannot do that." => {
                    "You are a ghost! Ghosts stand in ethereal form."
                }
                _ => err,
            };
            conn.send_line(msg);
        }
    }
}

pub fn cmd_die(
    world: &mut World,
    conn: &mut dyn Connection,
    _name: &str,
    _args: &str,
    registry: &ConnectionRegistry,
) {
    let entity = match conn.entity() {
        Some(e) => e,
        None => return,
    };

    let mut is_unconscious = false;
    if let Ok(mut q) = world.query_one::<&core::Health>(entity) {
        if let Some(hp) = q.get() {
            if hp.current <= 0 {
                is_unconscious = true;
            }
        }
    }
    if !is_unconscious {
        if let Ok(mut q) = world.query_one::<&core::PlayerState>(entity) {
            if let Some(core::PlayerState::Resting(core::RestState::Unconscious)) = q.get() {
                is_unconscious = true;
            }
        }
    }

    if !is_unconscious {
        conn.send_line("You can only choose to die when you are unconscious.");
        return;
    }

    let name = world
        .query_one::<&core::Name>(entity)
        .ok()
        .and_then(|mut q| q.get().map(|n| n.0.clone()))
        .unwrap_or_else(|| "Someone".to_string());

    let room = get_pos_room(world, entity);

    core::systems::combat::handle_death(world, entity);

    conn.send_line(
        "You choose to submit to death...\r\nAlas, you are dead! You are a ghost now...",
    );

    oxide_server::prompt::send_player_prompt(world, entity, registry);

    if let Some(r) = room {
        broadcast_to_room_except(
            world,
            registry,
            r,
            entity,
            &format!("{} has died.\r\n{} is dead! R.I.P.", name, name),
        );
    }
}

pub fn cmd_reclaim(
    world: &mut World,
    conn: &mut dyn Connection,
    _name: &str,
    _args: &str,
    registry: &ConnectionRegistry,
) {
    let entity = match conn.entity() {
        Some(e) => e,
        None => return,
    };

    let is_ghost = world
        .query_one::<&core::PlayerState>(entity)
        .ok()
        .and_then(|mut q| q.get().map(|s| matches!(s, core::PlayerState::Dead)))
        .unwrap_or(false);

    if !is_ghost {
        conn.send_line("You are already alive.");
        return;
    }

    let player_db_id = world
        .query_one::<&core::DbId>(entity)
        .ok()
        .and_then(|mut q| q.get().map(|d| d.0));

    let corpse_entity = if let Some(room) = get_pos_room(world, entity) {
        let mut found = None;
        let mut to_update = None;
        {
            let mut q = world.query::<(&core::Corpse, &core::Position)>();
            for (raw, (corpse, pos)) in q.iter() {
                if pos.room == room {
                    let is_owner = corpse.owner == Some(entity)
                        || (player_db_id.is_some() && corpse.owner_db_id == player_db_id);
                    if is_owner {
                        found = Some(core::Entity::from(raw));
                        if corpse.owner != Some(entity) {
                            to_update = Some(core::Entity::from(raw));
                        }
                        break;
                    }
                }
            }
        }
        if let Some(c_entity) = to_update {
            if let Ok(mut q) = world.query_one::<&mut core::Corpse>(c_entity) {
                if let Some(corpse) = q.get() {
                    corpse.owner = Some(entity);
                }
            }
        }
        found
    } else {
        None
    };

    let corpse_entity = match corpse_entity {
        Some(c) => c,
        None => {
            conn.send_line("Your corpse is not in this room. You cannot reclaim your body here.");
            return;
        }
    };

    let corpse_eq = world
        .query_one::<&core::Equipment>(corpse_entity)
        .ok()
        .and_then(|mut q| q.get().map(|eq| eq.slots.clone()))
        .unwrap_or_default();

    if let Ok(mut q) = world.query_one::<&mut core::Equipment>(entity) {
        if let Some(player_eq) = q.get() {
            player_eq.slots = corpse_eq;
        }
    }

    let corpse_inv = world
        .query_one::<&core::Inventory>(corpse_entity)
        .ok()
        .and_then(|mut q| q.get().map(|inv| inv.0.clone()))
        .unwrap_or_default();

    if let Ok(mut q) = world.query_one::<&mut core::Inventory>(entity) {
        if let Some(player_inv) = q.get() {
            player_inv.0 = corpse_inv;
        }
    }

    if let Ok(mut q) = world.query_one::<&mut core::Health>(entity) {
        if let Some(hp) = q.get() {
            hp.current = hp.max;
        }
    }

    let _ = core::try_transition_player_state(world, entity, core::PlayerStateTrigger::Revive);

    let _ = world.despawn(corpse_entity);

    conn.send_line("You reclaim your body and return to the land of the living!");
    if let Ok(mut name_q) = world.query_one::<&core::Name>(entity) {
        if let Some(name) = name_q.get() {
            if let Some(room) = get_pos_room(world, entity) {
                broadcast_to_room_except(
                    world,
                    registry,
                    room,
                    entity,
                    &format!("{} returns to life!", name.0),
                );
            }
        }
    }
}

pub fn cmd_revive(
    world: &mut World,
    conn: &mut dyn Connection,
    name: &str,
    args: &str,
    registry: &ConnectionRegistry,
) {
    let entity = match conn.entity() {
        Some(e) => e,
        None => return,
    };

    let is_ghost = world
        .query_one::<&core::PlayerState>(entity)
        .ok()
        .and_then(|mut q| q.get().map(|s| matches!(s, core::PlayerState::Dead)))
        .unwrap_or(false);

    if !is_ghost {
        conn.send_line("You are already alive.");
        return;
    }

    let room = match get_pos_room(world, entity) {
        Some(r) => r,
        None => {
            conn.send_line("You are nowhere.");
            return;
        }
    };

    let player_db_id = world
        .query_one::<&core::DbId>(entity)
        .ok()
        .and_then(|mut q| q.get().map(|d| d.0));

    let corpse_entity = {
        let mut found = None;
        let mut q = world.query::<(&core::Corpse, &core::Position)>();
        for (raw, (corpse, pos)) in q.iter() {
            if pos.room == room {
                let is_owner = corpse.owner == Some(entity)
                    || (player_db_id.is_some() && corpse.owner_db_id == player_db_id);
                if is_owner {
                    found = Some(core::Entity::from(raw));
                    break;
                }
            }
        }
        found
    };

    if corpse_entity.is_some() {
        cmd_reclaim(world, conn, name, args, registry);
        return;
    }

    let can_revive = world.query_one::<&core::RoomAllowRevive>(room).is_ok();

    if !can_revive {
        conn.send_line("You cannot revive here. You must find your corpse or pray at a temple.");
        return;
    }

    if let Ok(mut q) = world.query_one::<&mut core::Health>(entity) {
        if let Some(hp) = q.get() {
            hp.current = hp.max;
        }
    }

    let _ = core::try_transition_player_state(world, entity, core::PlayerStateTrigger::Revive);

    conn.send_line("You pray at the altar and are restored to life! (Your equipment remains with your corpse.)");
    if let Ok(mut name_q) = world.query_one::<&core::Name>(entity) {
        if let Some(name) = name_q.get() {
            broadcast_to_room_except(
                world,
                registry,
                room,
                entity,
                &format!("{} returns to life!", name.0),
            );
        }
    }
}

pub fn cmd_toggle(
    world: &mut World,
    conn: &mut dyn Connection,
    _name: &str,
    args: &str,
    _registry: &ConnectionRegistry,
) {
    let entity = match conn.entity() {
        Some(e) => e,
        None => return,
    };

    let arg = args.trim().to_lowercase();
    if arg == "resurrect" || arg == "res" {
        let mut current_val = false;
        if let Ok(mut q) = world.query_one::<&mut core::Player>(entity) {
            if let Some(player) = q.get() {
                player.no_resurrect = !player.no_resurrect;
                current_val = player.no_resurrect;
            }
        }
        let _ = world.insert(entity, (core::Dirty,));
        if current_val {
            conn.send_line("You will now prevent unwanted resurrections (no_resurrect is ON).");
        } else {
            conn.send_line("You now allow resurrections (no_resurrect is OFF).");
        }
    } else {
        conn.send_line("Toggle options: resurrect");
    }
}

pub fn cmd_quest(
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

    let parts: Vec<&str> = args.split_whitespace().collect();
    if parts.is_empty() {
        list_quests(world, conn, entity, &templates);
        return;
    }

    match parts[0] {
        "list" => {
            list_quests(world, conn, entity, &templates);
        }
        "show" | "info" => {
            if parts.len() < 2 {
                conn.send_line("Usage: quest show <quest_id>");
                return;
            }
            show_quest(world, conn, entity, parts[1], &templates);
        }
        "accept" => {
            if parts.len() < 2 {
                conn.send_line("Usage: quest accept <quest_id>");
                return;
            }
            accept_quest_command(world, conn, entity, parts[1], &templates);
        }
        "complete" | "turnin" => {
            if parts.len() < 2 {
                conn.send_line("Usage: quest complete <quest_id>");
                return;
            }
            complete_quest_command(world, conn, entity, parts[1], &templates);
        }
        "abandon" => {
            if parts.len() < 2 {
                conn.send_line("Usage: quest abandon <quest_id>");
                return;
            }
            abandon_quest_command(world, conn, entity, parts[1]);
        }
        _ => {
            conn.send_line("Unknown quest subcommand. Try: list, show, accept, complete, abandon");
        }
    }
}

fn list_quests(
    world: &World,
    conn: &mut dyn Connection,
    player: core::Entity,
    templates: &core::templates::TemplateRegistry,
) {
    let mut q_log = match world.query_one::<&core::QuestLog>(player) {
        Ok(q) => q,
        Err(_) => {
            conn.send_line("You have no quest log.");
            return;
        }
    };
    let quest_log = match q_log.get() {
        Some(log) => log,
        None => {
            conn.send_line("You have no quest log.");
            return;
        }
    };

    if quest_log.active.is_empty() && quest_log.completed.is_empty() {
        conn.send_line("You have no quests.");
        return;
    }

    if !quest_log.active.is_empty() {
        conn.send_line("Active Quests:");
        for (quest_id, progress) in &quest_log.active {
            if let Some(quest_def) = templates.quests.get(quest_id) {
                let all_done = progress.objectives.iter().all(|o| o.completed);
                let status = if all_done { " (Ready to turn in)" } else { "" };
                conn.send_line(&format!(
                    "  {} - {}{}",
                    quest_def.id, quest_def.name, status
                ));
            } else {
                conn.send_line(&format!("  {} (Unknown Quest definition)", quest_id));
            }
        }
    }

    if !quest_log.completed.is_empty() {
        conn.send_line("Completed Quests:");
        for quest_id in &quest_log.completed {
            if let Some(quest_def) = templates.quests.get(quest_id) {
                conn.send_line(&format!("  {} - {}", quest_def.id, quest_def.name));
            } else {
                conn.send_line(&format!("  {}", quest_id));
            }
        }
    }
}

fn show_quest(
    world: &World,
    conn: &mut dyn Connection,
    player: core::Entity,
    quest_id: &str,
    templates: &core::templates::TemplateRegistry,
) {
    let mut q_log = match world.query_one::<&core::QuestLog>(player) {
        Ok(q) => q,
        Err(_) => {
            conn.send_line("You have no quest log.");
            return;
        }
    };
    let quest_log = match q_log.get() {
        Some(log) => log,
        None => {
            conn.send_line("You have no quest log.");
            return;
        }
    };

    let progress = match quest_log.active.get(quest_id) {
        Some(p) => p,
        None => {
            if quest_log.completed.contains(quest_id) {
                if let Some(quest_def) = templates.quests.get(quest_id) {
                    conn.send_line(&format!(
                        "Quest: {}\r\nStatus: Completed\r\nDescription: {}",
                        quest_def.name, quest_def.description
                    ));
                } else {
                    conn.send_line(&format!("Quest '{}' (Completed)", quest_id));
                }
            } else {
                conn.send_line("You are not on that quest.");
            }
            return;
        }
    };

    let quest_def = match templates.quests.get(quest_id) {
        Some(d) => d,
        None => {
            conn.send_line("Error: Quest definition not found.");
            return;
        }
    };

    conn.send_line(&format!("Quest: {}", quest_def.name));
    conn.send_line(&format!("Description: {}", quest_def.description));
    conn.send_line("Objectives:");
    for (objective, obj_progress) in quest_def.objectives.iter().zip(&progress.objectives) {
        let status = if obj_progress.completed { "[x]" } else { "[ ]" };
        match objective {
            core::templates::QuestObjective::Kill { mob, count } => {
                let mob_name = templates
                    .mobs
                    .get(mob)
                    .map(|m| m.name.as_str())
                    .unwrap_or(mob);
                conn.send_line(&format!(
                    "  {} Kill {}: {}/{}",
                    status, mob_name, obj_progress.current, count
                ));
            }
            core::templates::QuestObjective::Gather { item, count } => {
                let item_name = templates
                    .items
                    .get(item)
                    .map(|i| i.name.as_str())
                    .unwrap_or(item);
                conn.send_line(&format!(
                    "  {} Gather {}: {}/{}",
                    status, item_name, obj_progress.current, count
                ));
            }
            core::templates::QuestObjective::Deliver { item, npc } => {
                let item_name = templates
                    .items
                    .get(item)
                    .map(|i| i.name.as_str())
                    .unwrap_or(item);
                let npc_name = templates
                    .mobs
                    .get(npc)
                    .map(|m| m.name.as_str())
                    .unwrap_or(npc);
                conn.send_line(&format!(
                    "  {} Deliver {} to {}",
                    status, item_name, npc_name
                ));
            }
            core::templates::QuestObjective::Explore { room } => {
                conn.send_line(&format!("  {} Explore room: {}", status, room));
            }
            core::templates::QuestObjective::Talk { npc } => {
                let npc_name = templates
                    .mobs
                    .get(npc)
                    .map(|m| m.name.as_str())
                    .unwrap_or(npc);
                conn.send_line(&format!("  {} Talk to {}", status, npc_name));
            }
        }
    }
}

fn accept_quest_command(
    world: &mut World,
    conn: &mut dyn Connection,
    player: core::Entity,
    quest_id: &str,
    templates: &core::templates::TemplateRegistry,
) {
    let quest_def = match templates.quests.get(quest_id) {
        Some(qd) => qd,
        None => {
            conn.send_line(&format!("Quest '{}' does not exist.", quest_id));
            return;
        }
    };

    let room = match get_pos_room(world, player) {
        Some(r) => r,
        None => {
            conn.send_line("You are nowhere.");
            return;
        }
    };

    if let Some(ref giver_id) = quest_def.giver_npc {
        let occupants = core::util::entities_in_room(world, room);
        let mut giver_present = false;
        for occupant in occupants {
            if let Ok(mut q_npc) = world.query_one::<&core::Npc>(occupant) {
                if let Some(npc) = q_npc.get() {
                    if npc.template_id == *giver_id {
                        giver_present = true;
                        break;
                    }
                }
            }
        }
        if !giver_present {
            let giver_name = templates
                .mobs
                .get(giver_id)
                .map(|m| m.name.as_str())
                .unwrap_or(giver_id);
            conn.send_line(&format!(
                "You must be near {} to accept this quest.",
                giver_name
            ));
            return;
        }
    }

    match core::accept_quest(world, player, quest_id, templates) {
        Ok(msgs) => {
            for msg in msgs {
                conn.send_line(&msg);
            }
        }
        Err(err) => {
            conn.send_line(&err);
        }
    }
}

fn complete_quest_command(
    world: &mut World,
    conn: &mut dyn Connection,
    player: core::Entity,
    quest_id: &str,
    templates: &core::templates::TemplateRegistry,
) {
    let quest_def = match templates.quests.get(quest_id) {
        Some(qd) => qd,
        None => {
            conn.send_line(&format!("Quest '{}' does not exist.", quest_id));
            return;
        }
    };

    let room = match get_pos_room(world, player) {
        Some(r) => r,
        None => {
            conn.send_line("You are nowhere.");
            return;
        }
    };

    if let Some(ref turn_in_id) = quest_def.turn_in_npc {
        let occupants = core::util::entities_in_room(world, room);
        let mut turn_in_present = false;
        for occupant in occupants {
            if let Ok(mut q_npc) = world.query_one::<&core::Npc>(occupant) {
                if let Some(npc) = q_npc.get() {
                    if npc.template_id == *turn_in_id {
                        turn_in_present = true;
                        break;
                    }
                }
            }
        }
        if !turn_in_present {
            let turn_in_name = templates
                .mobs
                .get(turn_in_id)
                .map(|m| m.name.as_str())
                .unwrap_or(turn_in_id);
            conn.send_line(&format!(
                "You must be near {} to complete this quest.",
                turn_in_name
            ));
            return;
        }
    }

    match core::complete_quest(world, player, quest_id, templates) {
        Ok(msgs) => {
            for msg in msgs {
                conn.send_line(&msg);
            }
            let level_up_msgs = oxide_server::award_xp(world, player);
            for msg in level_up_msgs {
                conn.send_line(&msg);
            }
        }
        Err(err) => {
            conn.send_line(&err);
        }
    }
}

fn abandon_quest_command(
    world: &mut World,
    conn: &mut dyn Connection,
    player: core::Entity,
    quest_id: &str,
) {
    match core::abandon_quest(world, player, quest_id) {
        Ok(msgs) => {
            for msg in msgs {
                conn.send_line(&msg);
            }
        }
        Err(err) => {
            conn.send_line(&err);
        }
    }
}

pub fn cmd_faction(
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

    let templates = match oxide_server::get_templates() {
        Some(t) => t,
        None => {
            conn.send_line("Error: Template registry not loaded.");
            return;
        }
    };

    let fs = match world.query_one::<&core::FactionStanding>(entity) {
        Ok(mut q) => q.get().cloned().unwrap_or_default(),
        Err(_) => core::FactionStanding::new(),
    };

    if fs.standings.is_empty() {
        conn.send_line("You have no faction standings.");
        return;
    }

    conn.send_line("Your faction standings:");
    conn.send_line("------------------------------------------------");
    for (faction_id, standing) in &fs.standings {
        let faction_name = templates
            .factions
            .get(faction_id)
            .map(|f| f.name.as_str())
            .unwrap_or(faction_id);

        let rank = templates
            .factions
            .get(faction_id)
            .map(|f| f.get_rank(*standing))
            .unwrap_or_else(|| "Neutral".to_string());

        conn.send_line(&format!(
            "{:<24} : {:>6} ({})",
            faction_name, standing, rank
        ));
    }
}

pub fn cmd_recipes(
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

    let templates = match oxide_server::get_templates() {
        Some(t) => t,
        None => {
            conn.send_line("Error: Template registry not loaded.");
            return;
        }
    };

    let lr = match world.query_one::<&core::LearnedRecipes>(entity) {
        Ok(mut q) => q.get().cloned().unwrap_or_default(),
        Err(_) => core::LearnedRecipes::new(),
    };

    if lr.recipes.is_empty() {
        conn.send_line("You do not know any crafting recipes.");
        return;
    }

    conn.send_line("You know the following recipes:");
    conn.send_line("------------------------------------------------");
    for recipe_id in &lr.recipes {
        if let Some(recipe) = templates.recipes.get(recipe_id) {
            conn.send_line(&format!(
                "{} (Difficulty {}, Success Chance {}%):",
                recipe.name, recipe.difficulty, recipe.success_chance
            ));
            conn.send_line(&format!("  Description: {}", recipe.description));
            if let Some(ref station) = recipe.station {
                conn.send_line(&format!("  Station: {}", station.replace("station:", "")));
            }
            if let Some(ref skill_req) = recipe.skill_requirement {
                conn.send_line(&format!(
                    "  Skill: {} (Rank {})",
                    skill_req.id, skill_req.rank
                ));
            }
            conn.send_line("  Materials:");
            for material in &recipe.materials {
                let mat_name = templates
                    .items
                    .get(&material.template_id)
                    .map(|i| i.name.as_str())
                    .unwrap_or(&material.template_id);
                conn.send_line(&format!("    - {} x {}", mat_name, material.quantity));
            }
            let res_name = templates
                .items
                .get(&recipe.result.template_id)
                .map(|i| i.name.as_str())
                .unwrap_or(&recipe.result.template_id);
            conn.send_line("  Result:");
            conn.send_line(&format!("    - {} x {}", res_name, recipe.result.quantity));
            conn.send_line("");
        }
    }
}

pub fn cmd_advance(
    world: &mut World,
    conn: &mut dyn Connection,
    _name: &str,
    args: &str,
    _registry: &ConnectionRegistry,
) {
    let player = match conn.entity() {
        Some(p) => p,
        None => return,
    };
    let class_id = args.trim();
    if class_id.is_empty() {
        conn.send_line("Usage: @advance <class_id>");
        return;
    }
    match oxide_server::advance_player_class(world, player, class_id) {
        Ok(msgs) => {
            for m in msgs {
                conn.send_line(&m);
            }
        }
        Err(e) => conn.send_line(&format!("Error: {}", e)),
    }
}

pub fn cmd_multi_class(
    world: &mut World,
    conn: &mut dyn Connection,
    _name: &str,
    args: &str,
    _registry: &ConnectionRegistry,
) {
    let player = match conn.entity() {
        Some(p) => p,
        None => return,
    };
    let class_id = args.trim();
    if class_id.is_empty() {
        conn.send_line("Usage: @multi_class <class_id>");
        return;
    }

    let templates = match oxide_server::get_templates() {
        Some(t) => t,
        None => {
            conn.send_line("Error: Template registry not loaded.");
            return;
        }
    };

    let class_template = match templates.get_class(class_id) {
        Some(c) => c,
        None => {
            conn.send_line(&format!("Error: Class '{}' not found.", class_id));
            return;
        }
    };

    if class_template.prestige {
        conn.send_line("Error: That is a prestige class. Use '@prestige' instead.");
        return;
    }

    let mut mc_info = match world.query_one::<&mut core::MultiClassInfo>(player) {
        Ok(mut q) => q.get().cloned().unwrap_or_default(),
        Err(_) => {
            conn.send_line("Error: You do not have class information.");
            return;
        }
    };

    if mc_info.has_class(class_id) {
        conn.send_line(&format!(
            "Error: You already have the class '{}'. Use '@advance' to level it up.",
            class_id
        ));
        return;
    }

    if !class_template.allowed_races.is_empty() {
        let race = world
            .query_one::<&core::Race>(player)
            .ok()
            .and_then(|mut q| q.get().map(|r| r.0.clone()))
            .unwrap_or_default();
        if !class_template
            .allowed_races
            .iter()
            .any(|r| r.to_lowercase() == race.to_lowercase())
        {
            conn.send_line(&format!(
                "Error: Your race '{}' is not allowed for this class.",
                race
            ));
            return;
        }
    }

    if !class_template.allowed_alignments.is_empty() {
        let align = world
            .query_one::<&core::Alignment>(player)
            .ok()
            .and_then(|mut q| q.get().map(|a| a.0.clone()))
            .unwrap_or_default();
        if !class_template
            .allowed_alignments
            .iter()
            .any(|a| a.to_lowercase() == align.to_lowercase())
        {
            conn.send_line(&format!(
                "Error: Your alignment '{}' is not allowed for this class.",
                align
            ));
            return;
        }
    }

    let current_level = mc_info.total_level();
    let next_level = current_level + 1;
    let xp = world
        .query_one::<&core::Experience>(player)
        .ok()
        .and_then(|mut q| q.get().map(|x| x.0))
        .unwrap_or(0);
    let threshold = core::Experience::for_level(next_level);

    if xp < threshold {
        conn.send_line(&format!(
            "Error: You need {} XP to level up/multi-class, but you only have {}.",
            threshold, xp
        ));
        return;
    }

    mc_info.add_class(class_id.to_string(), 1, false);

    let attrs = world
        .query_one::<&core::Attributes>(player)
        .ok()
        .and_then(|mut q| q.get().cloned())
        .unwrap_or_default();
    let con_mod = (attrs.constitution as i32 - 10) / 2;
    let hp_gain = (class_template.hit_die as i32 + con_mod).max(1);

    if let Ok(mut q) = world.query_one::<&mut core::Health>(player) {
        if let Some(health) = q.get() {
            health.max += hp_gain;
            health.current = health.max;
        }
    }

    if let Ok(mut q) = world.query_one::<&mut core::Level>(player) {
        if let Some(level) = q.get() {
            level.0 = next_level;
        }
    }

    if let Ok(mut q) = world.query_one::<&mut core::Experience>(player) {
        if let Some(xp_comp) = q.get() {
            xp_comp.0 = xp.saturating_sub(threshold);
            let db_arc = oxide_server::get_db();
            let db = db_arc.as_ref().and_then(|d| d.try_lock().ok());
            let conn_db = db.as_ref().map(|g| g.conn());
            if let Some(conn_db) = conn_db {
                if let Ok(mut q_db) = world.query_one::<&core::DbId>(player) {
                    if let Some(db_id) = q_db.get() {
                        let _ =
                            oxide_data::save_level_component(conn_db, db_id.0, next_level as i64);
                        let _ = oxide_data::save_experience_component(
                            conn_db,
                            db_id.0,
                            xp_comp.0 as i64,
                        );
                    }
                }
            }
        }
    }

    if let Ok(mut q) = world.query_one::<&mut core::Mana>(player) {
        if let Some(mana) = q.get() {
            let formula_mana = core::Mana::from_formula(
                next_level as u16,
                attrs.intelligence as u16,
                attrs.wisdom as u16,
            );
            mana.max = formula_mana.max;
            mana.current = mana.max;
        }
    }

    if let Ok(mut q) = world.query_one::<&mut core::Stamina>(player) {
        if let Some(stamina) = q.get() {
            let formula_stamina = core::Stamina::from_formula(
                next_level as u16,
                attrs.strength as u16,
                attrs.dexterity as u16,
            );
            stamina.max = formula_stamina.max;
            stamina.current = stamina.max;
        }
    }

    let new_combat_stats = core::calculate_multiclass_combat_stats(&mc_info, &templates);
    let _ = world.insert(player, (new_combat_stats,));

    let _ = world.insert(player, (mc_info,));
    let _ = world.insert(player, (core::Dirty,));

    conn.send_line(&format!(
        "You successfully multi-classed into {}!",
        class_template.name
    ));
    conn.send_line(&format!(
        "You are now level {} ({} level 1).",
        next_level, class_template.name
    ));
    conn.send_line(&format!("You gained {} max HP.", hp_gain));
}

pub fn cmd_prestige(
    world: &mut World,
    conn: &mut dyn Connection,
    _name: &str,
    args: &str,
    _registry: &ConnectionRegistry,
) {
    let player = match conn.entity() {
        Some(p) => p,
        None => return,
    };
    let class_id = args.trim();
    if class_id.is_empty() {
        conn.send_line("Usage: @prestige <prestige_class_id>");
        return;
    }

    let templates = match oxide_server::get_templates() {
        Some(t) => t,
        None => {
            conn.send_line("Error: Template registry not loaded.");
            return;
        }
    };

    let class_template = match templates.get_class(class_id) {
        Some(c) => c,
        None => {
            conn.send_line(&format!("Error: Prestige Class '{}' not found.", class_id));
            return;
        }
    };

    if !class_template.prestige {
        conn.send_line("Error: That is not a prestige class. Use '@multi_class' instead.");
        return;
    }

    if let Some(ref gate) = class_template.prestige_gate {
        if let Err(gate_err) = core::satisfies_prestige_gate(world, player, gate, &templates) {
            conn.send_line(&format!(
                "Error: You do not satisfy requirements for prestige class '{}': {}",
                class_template.name, gate_err
            ));
            return;
        }
    }

    let mut mc_info = match world.query_one::<&mut core::MultiClassInfo>(player) {
        Ok(mut q) => q.get().cloned().unwrap_or_default(),
        Err(_) => {
            conn.send_line("Error: You do not have class information.");
            return;
        }
    };

    if mc_info.has_class(class_id) {
        conn.send_line(&format!(
            "Error: You already have prestige class '{}'. Use '@advance' to level it up.",
            class_id
        ));
        return;
    }

    let current_level = mc_info.total_level();
    let next_level = current_level + 1;
    let xp = world
        .query_one::<&core::Experience>(player)
        .ok()
        .and_then(|mut q| q.get().map(|x| x.0))
        .unwrap_or(0);
    let threshold = core::Experience::for_level(next_level);

    if xp < threshold {
        conn.send_line(&format!(
            "Error: You need {} XP to level up/adopt prestige class, but you only have {}.",
            threshold, xp
        ));
        return;
    }

    mc_info.add_class(class_id.to_string(), 1, false);

    let attrs = world
        .query_one::<&core::Attributes>(player)
        .ok()
        .and_then(|mut q| q.get().cloned())
        .unwrap_or_default();
    let con_mod = (attrs.constitution as i32 - 10) / 2;
    let hp_gain = (class_template.hit_die as i32 + con_mod).max(1);

    if let Ok(mut q) = world.query_one::<&mut core::Health>(player) {
        if let Some(health) = q.get() {
            health.max += hp_gain;
            health.current = health.max;
        }
    }

    if let Ok(mut q) = world.query_one::<&mut core::Level>(player) {
        if let Some(level) = q.get() {
            level.0 = next_level;
        }
    }

    if let Ok(mut q) = world.query_one::<&mut core::Experience>(player) {
        if let Some(xp_comp) = q.get() {
            xp_comp.0 = xp.saturating_sub(threshold);
            let db_arc = oxide_server::get_db();
            let db = db_arc.as_ref().and_then(|d| d.try_lock().ok());
            let conn_db = db.as_ref().map(|g| g.conn());
            if let Some(conn_db) = conn_db {
                if let Ok(mut q_db) = world.query_one::<&core::DbId>(player) {
                    if let Some(db_id) = q_db.get() {
                        let _ =
                            oxide_data::save_level_component(conn_db, db_id.0, next_level as i64);
                        let _ = oxide_data::save_experience_component(
                            conn_db,
                            db_id.0,
                            xp_comp.0 as i64,
                        );
                    }
                }
            }
        }
    }

    if let Ok(mut q) = world.query_one::<&mut core::Mana>(player) {
        if let Some(mana) = q.get() {
            let formula_mana = core::Mana::from_formula(
                next_level as u16,
                attrs.intelligence as u16,
                attrs.wisdom as u16,
            );
            mana.max = formula_mana.max;
            mana.current = mana.max;
        }
    }

    if let Ok(mut q) = world.query_one::<&mut core::Stamina>(player) {
        if let Some(stamina) = q.get() {
            let formula_stamina = core::Stamina::from_formula(
                next_level as u16,
                attrs.strength as u16,
                attrs.dexterity as u16,
            );
            stamina.max = formula_stamina.max;
            stamina.current = stamina.max;
        }
    }

    let new_combat_stats = core::calculate_multiclass_combat_stats(&mc_info, &templates);
    let _ = world.insert(player, (new_combat_stats,));

    let _ = world.insert(player, (mc_info,));
    let _ = world.insert(player, (core::Dirty,));

    conn.send_line(&format!(
        "You successfully unlocked prestige class {}!",
        class_template.name
    ));
    conn.send_line(&format!(
        "You are now level {} ({} level 1).",
        next_level, class_template.name
    ));
    conn.send_line(&format!("You gained {} max HP.", hp_gain));
}

pub fn cmd_affects(
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

    conn.send_line("");
    conn.send_line("Active Spells & Temporary Effects:");

    let mut temp_count = 0;
    if let Ok(mut q) = world.query_one::<&core::ActiveScriptEffects>(entity) {
        if let Some(active) = q.get() {
            for effect in &active.effects {
                if !effect.visible_in_affects {
                    continue;
                }
                temp_count += 1;
                let display = if let Some(ref custom) = effect.affects_display {
                    if effect.show_remaining_time && effect.remaining_secs > 0 {
                        format!("  {} ({}s remaining)", custom, effect.remaining_secs)
                    } else {
                        format!("  {}", custom)
                    }
                } else if effect.show_remaining_time && effect.remaining_secs > 0 {
                    format!(
                        "  [{}] {} ({}s remaining)",
                        effect.display_name, effect.description, effect.remaining_secs
                    )
                } else {
                    format!("  [{}] {}", effect.display_name, effect.description)
                };
                conn.send_line(&display);
            }
        }
    }

    if temp_count == 0 {
        conn.send_line("  None");
    }

    conn.send_line("");
    conn.send_line("Permanent Passives & Blessings:");
    let mut perm_count = 0;

    let mut equipment_items = Vec::new();
    if let Ok(mut q) = world.query_one::<&core::Equipment>(entity) {
        if let Some(eq) = q.get() {
            for (_slot, item_entity) in &eq.slots {
                equipment_items.push(*item_entity);
            }
        }
    }

    for item_entity in equipment_items {
        if let Ok(mut q) = world.query_one::<&core::PermanentItemAffects>(item_entity) {
            if let Some(perm) = q.get() {
                let item_name =
                    core::get_short_desc(world, item_entity).unwrap_or_else(|| "Item".to_string());
                for affect in &perm.affects {
                    perm_count += 1;
                    let display = if let Some(ref custom) = affect.affects_display {
                        format!("  [{item_name}] {custom}")
                    } else {
                        format!("  [{item_name}] {} ({:+})", affect.name, affect.amount)
                    };
                    conn.send_line(&display);
                }
            }
        }
    }

    if perm_count == 0 {
        conn.send_line("  None");
    }
    conn.send_line("");
}

#[cfg(test)]
mod tests {
    use super::super::test_helpers::*;
    use super::*;
    use oxide_core::Position;

    #[test]
    fn test_practice_success() {
        let (mut world, _void, room_a, _room_b) = test_world();
        let (player, mut conn, registry) = test_player(&mut world, room_a);

        let _trainer = world.spawn((
            Position::new(room_a),
            core::Trainer {
                trainer_types: vec!["combat".to_string()],
            },
        ));

        let mut skills = core::LearnedSkills::new();
        skills.set_rank("swords", 1);
        world
            .insert(player, (skills, core::PracticePoints(5), core::Level(1)))
            .unwrap();

        cmd_practice(&mut world, &mut conn, "practice", "swords", &registry);

        let lines = conn.take_lines();
        assert!(lines
            .iter()
            .any(|l| l.contains("You practice 'swords' to rank 2")));

        let mut pts = world.query_one::<&core::PracticePoints>(player).unwrap();
        assert_eq!(pts.get().unwrap().0, 4);
    }

    #[test]
    fn test_practice_no_trainer() {
        let (mut world, _void, room_a, _room_b) = test_world();
        let (player, mut conn, registry) = test_player(&mut world, room_a);

        let mut skills = core::LearnedSkills::new();
        skills.set_rank("swords", 1);
        world
            .insert(player, (skills, core::PracticePoints(5), core::Level(1)))
            .unwrap();

        cmd_practice(&mut world, &mut conn, "practice", "swords", &registry);

        let lines = conn.take_lines();
        assert!(lines.iter().any(|l| l.contains("Seek out a trainer")));
    }

    #[test]
    fn test_practice_wrong_trainer() {
        let (mut world, _void, room_a, _room_b) = test_world();
        let (player, mut conn, registry) = test_player(&mut world, room_a);

        let _trainer = world.spawn((
            Position::new(room_a),
            core::Trainer {
                trainer_types: vec!["magic".to_string()],
            },
        ));

        let mut skills = core::LearnedSkills::new();
        skills.set_rank("swords", 1);
        world
            .insert(player, (skills, core::PracticePoints(5), core::Level(1)))
            .unwrap();

        cmd_practice(&mut world, &mut conn, "practice", "swords", &registry);

        let lines = conn.take_lines();
        assert!(lines
            .iter()
            .any(|l| l.contains("You can't practice that here.")));
    }

    #[test]
    fn test_practice_not_known_skill() {
        let (mut world, _void, room_a, _room_b) = test_world();
        let (player, mut conn, registry) = test_player(&mut world, room_a);

        let _trainer = world.spawn((
            Position::new(room_a),
            core::Trainer {
                trainer_types: vec!["combat".to_string()],
            },
        ));

        world
            .insert(
                player,
                (
                    core::LearnedSkills::new(),
                    core::PracticePoints(5),
                    core::Level(1),
                ),
            )
            .unwrap();

        cmd_practice(&mut world, &mut conn, "practice", "swords", &registry);

        let lines = conn.take_lines();
        assert!(lines.iter().any(|l| l.contains("You don't know the skill")));
    }

    #[test]
    fn test_practice_max_rank() {
        let (mut world, _void, room_a, _room_b) = test_world();
        let (player, mut conn, registry) = test_player(&mut world, room_a);

        let _trainer = world.spawn((
            Position::new(room_a),
            core::Trainer {
                trainer_types: vec!["combat".to_string()],
            },
        ));

        let mut skills = core::LearnedSkills::new();
        skills.set_rank("swords", 10); // max for level 1 (1*5+5 = 10)
        world
            .insert(player, (skills, core::PracticePoints(5), core::Level(1)))
            .unwrap();

        cmd_practice(&mut world, &mut conn, "practice", "swords", &registry);

        let lines = conn.take_lines();
        assert!(lines
            .iter()
            .any(|l| l.contains("cannot practice 'swords' beyond rank 10")));
    }

    #[test]
    fn test_practice_no_points() {
        let (mut world, _void, room_a, _room_b) = test_world();
        let (player, mut conn, registry) = test_player(&mut world, room_a);

        let _trainer = world.spawn((
            Position::new(room_a),
            core::Trainer {
                trainer_types: vec!["combat".to_string()],
            },
        ));

        let mut skills = core::LearnedSkills::new();
        skills.set_rank("swords", 1);
        world
            .insert(player, (skills, core::PracticePoints(0), core::Level(1)))
            .unwrap();

        cmd_practice(&mut world, &mut conn, "practice", "swords", &registry);

        let lines = conn.take_lines();
        assert!(lines
            .iter()
            .any(|l| l.contains("costs 1 point(s), but you only have 0")));
    }

    #[test]
    fn test_train_success() {
        let (mut world, _void, room_a, _room_b) = test_world();
        let (player, mut conn, registry) = test_player(&mut world, room_a);

        let _trainer = world.spawn((
            Position::new(room_a),
            core::Trainer {
                trainer_types: vec!["attributes".to_string()],
            },
        ));

        let attrs = core::Attributes {
            strength: 10,
            dexterity: 10,
            intelligence: 10,
            wisdom: 10,
            constitution: 10,
            charisma: 10,
        };
        world
            .insert(player, (attrs, core::PracticePoints(10)))
            .unwrap();

        cmd_train(&mut world, &mut conn, "train", "strength", &registry);

        let lines = conn.take_lines();
        assert!(lines.iter().any(|l| l.contains("You train Strength to 11")));

        let mut a = world.query_one::<&core::Attributes>(player).unwrap();
        assert_eq!(a.get().unwrap().strength, 11);
    }

    #[test]
    fn test_train_no_trainer() {
        let (mut world, _void, room_a, _room_b) = test_world();
        let (player, mut conn, registry) = test_player(&mut world, room_a);

        world
            .insert(
                player,
                (core::Attributes::default(), core::PracticePoints(10)),
            )
            .unwrap();

        cmd_train(&mut world, &mut conn, "train", "strength", &registry);

        let lines = conn.take_lines();
        assert!(lines.iter().any(|l| l.contains("Seek out a trainer")));
    }

    #[test]
    fn test_train_wrong_trainer() {
        let (mut world, _void, room_a, _room_b) = test_world();
        let (player, mut conn, registry) = test_player(&mut world, room_a);

        let _trainer = world.spawn((
            Position::new(room_a),
            core::Trainer {
                trainer_types: vec!["combat".to_string()],
            },
        ));

        world
            .insert(
                player,
                (core::Attributes::default(), core::PracticePoints(10)),
            )
            .unwrap();

        cmd_train(&mut world, &mut conn, "train", "strength", &registry);

        let lines = conn.take_lines();
        assert!(lines
            .iter()
            .any(|l| l.contains("You can't train that here")));
    }

    #[test]
    fn test_train_already_max() {
        let (mut world, _void, room_a, _room_b) = test_world();
        let (player, mut conn, registry) = test_player(&mut world, room_a);

        let _trainer = world.spawn((
            Position::new(room_a),
            core::Trainer {
                trainer_types: vec!["attributes".to_string()],
            },
        ));

        let attrs = core::Attributes {
            strength: core::Attributes::MAX,
            ..Default::default()
        };
        world
            .insert(player, (attrs, core::PracticePoints(10)))
            .unwrap();

        cmd_train(&mut world, &mut conn, "train", "strength", &registry);

        let lines = conn.take_lines();
        assert!(lines.iter().any(|l| l.contains("already at the maximum")));
    }

    #[test]
    fn test_train_no_points() {
        let (mut world, _void, room_a, _room_b) = test_world();
        let (player, mut conn, registry) = test_player(&mut world, room_a);

        let _trainer = world.spawn((
            Position::new(room_a),
            core::Trainer {
                trainer_types: vec!["attributes".to_string()],
            },
        ));

        world
            .insert(
                player,
                (core::Attributes::default(), core::PracticePoints(0)),
            )
            .unwrap();

        cmd_train(&mut world, &mut conn, "train", "strength", &registry);

        let lines = conn.take_lines();
        assert!(lines.iter().any(|l| l.contains("costs")));
    }

    #[test]
    fn test_score_displays_stats() {
        let (mut world, _void, room_a, _room_b) = test_world();
        let (player, mut conn, registry) = test_player(&mut world, room_a);

        world
            .insert(
                player,
                (
                    core::Level(5),
                    core::Experience(1200),
                    core::Health::new(50),
                    core::Attributes {
                        strength: 14,
                        dexterity: 12,
                        intelligence: 10,
                        wisdom: 8,
                        constitution: 14,
                        charisma: 10,
                    },
                    core::PracticePoints(3),
                    core::CombatStats {
                        base_attack_bonus: 3,
                        fort_save: 4,
                        ref_save: 1,
                        will_save: 0,
                    },
                ),
            )
            .unwrap();

        cmd_score(&mut world, &mut conn, "score", "", &registry);

        let lines = conn.take_lines();
        assert!(lines.iter().any(|l| l.contains("TestPlayer")));
        assert!(lines.iter().any(|l| l.contains("Level:           5")));
        assert!(lines.iter().any(|l| l.contains("1200")));
        assert!(lines.iter().any(|l| l.contains("50 / 50")));
        assert!(lines.iter().any(|l| l.contains("Strength:     14")));
        assert!(lines.iter().any(|l| l.contains("BAB:             +3")));
    }

    #[test]
    fn test_pray_command() {
        let _guard = init_test_templates();

        let (mut world, _void, room_a, _room_b) = test_world();
        let (player, mut conn, conn_reg) = test_player(&mut world, room_a);

        world
            .insert(
                player,
                (
                    core::Deity(Some("solaris".into())),
                    core::Health::new(100),
                    core::Stamina::new(100),
                ),
            )
            .unwrap();

        cmd_pray(&mut world, &mut conn, "pray", "", &conn_reg);

        let lines = conn.take_lines();
        assert!(lines
            .iter()
            .any(|l| l.contains("You bow your head and pray to Solaris.")));

        cmd_pray(&mut world, &mut conn, "pray", "", &conn_reg);
        let lines2 = conn.take_lines();
        assert!(lines2
            .iter()
            .any(|l| l.contains("have been answered too recently")));
    }

    #[test]
    fn test_toggle_resurrect() {
        let (mut world, _void, room_a, _room_b) = test_world();
        let (player, mut conn, conn_reg) = test_player(&mut world, room_a);

        world.insert(player, (core::Player::new(1),)).unwrap();

        cmd_toggle(&mut world, &mut conn, "toggle", "resurrect", &conn_reg);
        let lines = conn.take_lines();
        assert!(lines.iter().any(|l| l.contains("ON")));

        cmd_toggle(&mut world, &mut conn, "toggle", "resurrect", &conn_reg);
        let lines = conn.take_lines();
        assert!(lines.iter().any(|l| l.contains("OFF")));
    }

    #[test]
    fn test_reclaim_corpse() {
        let (mut world, _void, room_a, _room_b) = test_world();
        let (player, mut conn, conn_reg) = test_player(&mut world, room_a);

        world
            .insert(
                player,
                (
                    core::PlayerState::Dead,
                    core::DbId(42),
                    core::Inventory::new(),
                    core::Equipment::new(),
                    core::Health::new(100),
                ),
            )
            .unwrap();

        let item = world.spawn((core::Item::new("sword"),));

        let _corpse = world.spawn((
            core::Corpse {
                owner: None,
                owner_db_id: Some(42),
                created_at: std::time::Instant::now(),
                decay_secs: 1800,
                lootable_by: core::LootRule::OwnerOnly,
            },
            Position::new(room_a),
            core::Name::new("corpse"),
            core::Inventory(vec![item]),
        ));

        cmd_reclaim(&mut world, &mut conn, "reclaim", "", &conn_reg);
        let lines = conn.take_lines();
        assert!(lines
            .iter()
            .any(|l| l.contains("reclaim your body and return to the land of the living")));
    }

    #[test]
    fn test_die_command() {
        let (mut world, _void, room_a, _room_b) = test_world();
        let (player, mut conn, conn_reg) = test_player(&mut world, room_a);

        world
            .insert(
                player,
                (
                    core::Health {
                        current: 0,
                        max: 100,
                    },
                    core::PlayerState::Resting(core::RestState::Unconscious),
                ),
            )
            .unwrap();

        cmd_die(&mut world, &mut conn, "die", "", &conn_reg);
        let lines = conn.take_lines();
        assert!(lines
            .iter()
            .any(|l| l.contains("You choose to submit to death")));
    }

    #[test]
    fn test_revive_at_altar() {
        let (mut world, _void, room_a, _room_b) = test_world();

        world.insert(room_a, (core::RoomAllowRevive,)).unwrap();

        let (player, mut conn, conn_reg) = test_player(&mut world, room_a);

        world
            .insert(
                player,
                (
                    core::PlayerState::Dead,
                    core::Health {
                        current: 0,
                        max: 100,
                    },
                ),
            )
            .unwrap();

        cmd_revive(&mut world, &mut conn, "revive", "", &conn_reg);
        let lines = conn.take_lines();
        assert!(lines
            .iter()
            .any(|l| l.contains("You pray at the altar and are restored to life")));
    }

    #[test]
    fn test_ghost_command_restrictions() {
        let (mut world, _void, room_a, _room_b) = test_world();
        let (player, mut conn, conn_reg) = test_player(&mut world, room_a);

        world
            .insert(
                player,
                (
                    core::PlayerState::Dead,
                    core::Inventory::new(),
                    core::Equipment::new(),
                ),
            )
            .unwrap();

        cmd_sit(&mut world, &mut conn, "sit", "", &conn_reg);
        let lines = conn.take_lines();
        assert!(lines
            .iter()
            .any(|l| l.contains("You are a ghost! Ghosts do not sit down.")));

        cmd_rest(&mut world, &mut conn, "rest", "", &conn_reg);
        let lines = conn.take_lines();
        assert!(lines
            .iter()
            .any(|l| l.contains("You are a ghost! Ghosts do not rest.")));

        cmd_sleep(&mut world, &mut conn, "sleep", "", &conn_reg);
        let lines = conn.take_lines();
        assert!(lines
            .iter()
            .any(|l| l.contains("You are a ghost! Ghosts do not sleep.")));
    }
}
