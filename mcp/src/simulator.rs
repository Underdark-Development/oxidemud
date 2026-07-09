use oxide_core::systems::combat::{calculate_damage, calculate_hit};
use oxide_core::systems::loot::roll_loot;
use oxide_core::systems::passive::apply_all_passives;
use oxide_core::systems::set_bonus::evaluate_set_bonuses;
use oxide_core::templates::{DeityPolicy, TemplateRegistry};
use oxide_core::{
    apply_skill_effect, can_use_skill, deduct_resource_cost, ActiveEffect, Armor, Attributes,
    Energy, Equipment, EquipmentSlot, Health, LearnedSkills, Level, Mana, PlayerState, Position,
    Psi, RestState, SetMembership, SetTracker, SkillCooldowns, Stamina, Weapon, World,
};
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;
use std::str::FromStr;

// --- Loot Simulator ---
pub fn simulate_loot(
    registry: &TemplateRegistry,
    mob_id: &str,
    iterations: u32,
    detailed: bool,
) -> Result<String, String> {
    let mob = registry
        .mobs
        .get(mob_id)
        .ok_or_else(|| format!("Mob template '{}' not found", mob_id))?;

    if detailed {
        let mut out = format!(
            "### Detailed Loot Simulation for `{}` ({} Corpse rolls)\n\n",
            mob_id, iterations
        );
        for i in 1..=iterations {
            let spawns = roll_loot(&mob.loot, mob.level, registry);
            out.push_str(&format!("*   **Corpse #{}**:\n", i));
            if spawns.is_empty() {
                out.push_str("    *   *(No loot drops)*\n");
            } else {
                for spawn in spawns {
                    let name = registry
                        .items
                        .get(&spawn.template_id)
                        .map(|item| item.name.as_str())
                        .unwrap_or("Unknown Item");

                    let affixes = if spawn.prefix_ids.is_empty() && spawn.suffix_ids.is_empty() {
                        "None".to_string()
                    } else {
                        let mut parts = Vec::new();
                        for p in &spawn.prefix_ids {
                            parts.push(format!("prefix: {}", p));
                        }
                        for s in &spawn.suffix_ids {
                            parts.push(format!("suffix: {}", s));
                        }
                        parts.join(", ")
                    };

                    out.push_str(&format!(
                        "    *   `{}` ({}) x{} | Quality: `{:?}` | Affixes: `{}`\n",
                        spawn.template_id, name, spawn.count, spawn.quality, affixes
                    ));
                }
            }
        }
        return Ok(out);
    }

    let mut drop_counts: HashMap<String, u32> = HashMap::new();
    let mut total_drops = 0;

    for _ in 0..iterations {
        let spawns = roll_loot(&mob.loot, mob.level, registry);
        for spawn in spawns {
            *drop_counts.entry(spawn.template_id.clone()).or_insert(0) += 1;
            total_drops += 1;
        }
    }

    let mut out = format!("### Loot Simulation Results for `{}`\n", mob_id);
    out.push_str(&format!("*   **Iterations (Corpses)**: {}\n", iterations));
    out.push_str(&format!("*   **Total Items Dropped**: {}\n\n", total_drops));
    out.push_str("| Item ID | Name | Times Dropped | Observed Drop Rate | Expected (Template) |\n");
    out.push_str("|---|---|---|---|---|\n");

    let mut entries: Vec<(&String, &u32)> = drop_counts.iter().collect();
    entries.sort_by(|a, b| b.1.cmp(a.1));

    for (item_id, count) in entries {
        let name = registry
            .items
            .get(item_id)
            .map(|i| i.name.as_str())
            .unwrap_or("Unknown Item");

        let observed_rate = (*count as f64 / iterations as f64) * 100.0;

        let expected_chance = mob
            .loot
            .entries
            .iter()
            .find(|e| e.item == *item_id)
            .map(|e| e.chance as f64)
            .unwrap_or(0.0);

        out.push_str(&format!(
            "| `{}` | {} | {} | {:.2}% | {:.2}% |\n",
            item_id, name, count, observed_rate, expected_chance
        ));
    }

    Ok(out)
}

// --- Combat Simulator ---
#[allow(clippy::too_many_arguments)]
pub fn simulate_combat(
    registry: &TemplateRegistry,
    attacker_template: Option<&str>,
    attacker_weapon: Option<&str>,
    attacker_level: Option<u8>,
    defender_template: Option<&str>,
    defender_level: Option<u8>,
    defender_ac_override: Option<i32>,
    rounds: u32,
) -> Result<String, String> {
    let mut world = World::new();
    let room = world.spawn(());

    // 1. Attacker Setup
    let (atk_level, atk_attrs) = if let Some(tmpl_id) = attacker_template {
        let mob = registry
            .mobs
            .get(tmpl_id)
            .ok_or_else(|| format!("Attacker template '{}' not found", tmpl_id))?;
        (
            attacker_level.unwrap_or(mob.level),
            Attributes::new(
                mob.attributes.strength,
                mob.attributes.dexterity,
                mob.attributes.intelligence,
                mob.attributes.wisdom,
                mob.attributes.constitution,
                mob.attributes.charisma,
            ),
        )
    } else {
        (
            attacker_level.unwrap_or(1),
            Attributes::new(10, 10, 10, 10, 10, 10),
        )
    };

    let mut attacker_eq = Equipment::new();
    if let Some(wep_id) = attacker_weapon {
        let wep_tmpl = registry
            .items
            .get(wep_id)
            .ok_or_else(|| format!("Weapon template '{}' not found", wep_id))?;
        if let Some(wpn_def) = &wep_tmpl.weapon {
            let dice = wpn_def.damage.as_str().parse().map_err(|_| {
                format!(
                    "Invalid weapon damage notation '{}'",
                    wpn_def.damage.as_str()
                )
            })?;
            let damage_type = wpn_def
                .damage_type
                .parse()
                .map_err(|_| format!("Invalid damage type '{}'", wpn_def.damage_type))?;

            let hands = match wpn_def.hands.to_lowercase().as_str() {
                "twohand" | "twohanded" | "two_hand" | "two_handed" => {
                    oxide_core::WeaponHands::TwoHand
                }
                _ => {
                    if wep_tmpl
                        .requires_skill
                        .as_ref()
                        .map(|s| s.id == "two_handed")
                        .unwrap_or(false)
                    {
                        oxide_core::WeaponHands::TwoHand
                    } else {
                        oxide_core::WeaponHands::OneHand
                    }
                }
            };

            let wep_entity = world.spawn((Weapon {
                damage_dice: dice,
                damage_type,
                speed: wpn_def.speed,
                range: oxide_core::WeaponRange::Melee,
                hands,
            },));
            attacker_eq.equip(EquipmentSlot::Weapon, wep_entity);
        }
    }

    let attacker = world.spawn((
        Level(atk_level),
        Health::new(1000),
        atk_attrs,
        Position::new(room),
        attacker_eq,
    ));

    // 2. Defender Setup
    let (def_level, def_attrs, def_armor) = if let Some(tmpl_id) = defender_template {
        let mob = registry
            .mobs
            .get(tmpl_id)
            .ok_or_else(|| format!("Defender template '{}' not found", tmpl_id))?;
        (
            defender_level.unwrap_or(mob.level),
            Attributes::new(
                mob.attributes.strength,
                mob.attributes.dexterity,
                mob.attributes.intelligence,
                mob.attributes.wisdom,
                mob.attributes.constitution,
                mob.attributes.charisma,
            ),
            Armor {
                base: defender_ac_override.unwrap_or(mob.armor),
                bonus: 0,
            },
        )
    } else {
        (
            defender_level.unwrap_or(1),
            Attributes::new(10, 10, 10, 10, 10, 10),
            Armor {
                base: defender_ac_override.unwrap_or(0),
                bonus: 0,
            },
        )
    };

    let defender = world.spawn((
        Level(def_level),
        Health::new(1000),
        def_attrs,
        Position::new(room),
        def_armor,
        Equipment::new(),
    ));

    // 3. Run Rounds
    let mut hits = 0;
    let mut crits = 0;
    let mut misses = 0;
    let mut total_damage = 0;
    let mut log_lines = Vec::new();

    for r in 1..=rounds {
        let hit_result = calculate_hit(&mut world, attacker, defender, false);
        if hit_result == oxide_core::HitResult::Hit {
            let (mut damage, damage_type) = calculate_damage(&mut world, attacker, defender, false);

            // Auto-crit check (1 in 20 chance to simulate Natural 20 roll, or if calculating hit rolled 20)
            let is_crit = fastrand::u8(1..=20) == 20;
            if is_crit {
                damage *= 2;
                crits += 1;
            } else {
                hits += 1;
            }
            total_damage += damage;

            log_lines.push(format!(
                "| Round {} | HIT{} | {} | {:?} |",
                r,
                if is_crit { " (CRIT)" } else { "" },
                damage,
                damage_type
            ));
        } else {
            misses += 1;
            log_lines.push(format!("| Round {} | MISS | - | - |", r));
        }
    }

    let mut out = "### Combat Simulation Summary\n\n".to_string();
    out.push_str(&format!("*   **Rounds Simulated**: {}\n", rounds));
    out.push_str(&format!(
        "*   **Total Hits**: {} ({:.2}%)\n",
        hits + crits,
        ((hits + crits) as f64 / rounds as f64) * 100.0
    ));
    out.push_str(&format!("*   **Critical Hits**: {}\n", crits));
    out.push_str(&format!(
        "*   **Misses**: {} ({:.2}%)\n",
        misses,
        (misses as f64 / rounds as f64) * 100.0
    ));
    out.push_str(&format!("*   **Total Damage**: {}\n", total_damage));
    out.push_str(&format!(
        "*   **Average Damage/Round**: {:.2}\n\n",
        total_damage as f64 / rounds as f64
    ));

    out.push_str("#### Combat Log\n\n");
    out.push_str("| Round | Outcome | Damage | Damage Type |\n");
    out.push_str("|---|---|---|---|\n");
    for line in log_lines {
        out.push_str(&line);
        out.push('\n');
    }

    Ok(out)
}

// --- Progression Simulator ---
pub fn simulate_progression(
    registry: &TemplateRegistry,
    race_id: &str,
    class_id: &str,
    start_level: u8,
    end_level: u8,
) -> Result<String, String> {
    let race = registry
        .races
        .get(race_id)
        .ok_or_else(|| format!("Race template '{}' not found", race_id))?;
    let class = registry
        .classes
        .get(class_id)
        .ok_or_else(|| format!("Class template '{}' not found", class_id))?;

    let base_str = (10 + race.attributes.strength as i16 + class.attribute_mods.strength as i16 - 8)
        .clamp(3, 50) as u8;
    let base_dex = (10 + race.attributes.dexterity as i16 + class.attribute_mods.dexterity as i16
        - 8)
    .clamp(3, 50) as u8;
    let base_int =
        (10 + race.attributes.intelligence as i16 + class.attribute_mods.intelligence as i16 - 8)
            .clamp(3, 50) as u8;
    let base_wis = (10 + race.attributes.wisdom as i16 + class.attribute_mods.wisdom as i16 - 8)
        .clamp(3, 50) as u8;
    let base_con =
        (10 + race.attributes.constitution as i16 + class.attribute_mods.constitution as i16 - 8)
            .clamp(3, 50) as u8;
    let base_cha = (10 + race.attributes.charisma as i16 + class.attribute_mods.charisma as i16 - 8)
        .clamp(3, 50) as u8;

    let mut out = format!(
        "### Progression Simulation for `{} {}`\n\n",
        race.name, class.name
    );
    out.push_str("| Level | Max HP | Max Mana | Max Stamina | Strength | Dexterity | Intel | Wisdom | Const | Charisma | BAB | Fort/Ref/Will Saves |\n");
    out.push_str("|---|---|---|---|---|---|---|---|---|---|---|---|\n");

    let con_mod = (base_con as i32 - 10) / 2;
    let hit_die = class.hit_die;

    let mut current_hp = hit_die as i32 + con_mod;
    current_hp = current_hp.max(1);

    for level in 1..=end_level {
        if level > 1 {
            let hp_gain = (hit_die as i32 + con_mod).max(1);
            current_hp += hp_gain;
        }

        if level >= start_level {
            let max_mana = level as u16 * 4 + base_int as u16 * 2 + base_wis as u16 * 2;
            let max_stamina = level as u16 * 4 + base_str as u16 * 2 + base_dex as u16 * 2;

            let combat_stats = class.calculate_combat_stats(level);

            out.push_str(&format!(
                "| {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | +{} | +{}/+{}/+{} |\n",
                level,
                current_hp,
                max_mana,
                max_stamina,
                base_str,
                base_dex,
                base_int,
                base_wis,
                base_con,
                base_cha,
                combat_stats.base_attack_bonus,
                combat_stats.fort_save,
                combat_stats.ref_save,
                combat_stats.will_save
            ));
        }
    }

    Ok(out)
}

// --- Gear Loadout Simulator ---
pub fn simulate_gear_loadout(
    registry: &TemplateRegistry,
    race_id: &str,
    class_id: &str,
    level: u8,
    equipped_items: &[String],
) -> Result<String, String> {
    let mut world = World::new();
    let room = world.spawn(());

    let race = registry
        .races
        .get(race_id)
        .ok_or_else(|| format!("Race template '{}' not found", race_id))?;
    let class = registry
        .classes
        .get(class_id)
        .ok_or_else(|| format!("Class template '{}' not found", class_id))?;

    let base_str = (10 + race.attributes.strength as i16 + class.attribute_mods.strength as i16 - 8)
        .clamp(3, 50) as u8;
    let base_dex = (10 + race.attributes.dexterity as i16 + class.attribute_mods.dexterity as i16
        - 8)
    .clamp(3, 50) as u8;
    let base_int =
        (10 + race.attributes.intelligence as i16 + class.attribute_mods.intelligence as i16 - 8)
            .clamp(3, 50) as u8;
    let base_wis = (10 + race.attributes.wisdom as i16 + class.attribute_mods.wisdom as i16 - 8)
        .clamp(3, 50) as u8;
    let base_con =
        (10 + race.attributes.constitution as i16 + class.attribute_mods.constitution as i16 - 8)
            .clamp(3, 50) as u8;
    let base_cha = (10 + race.attributes.charisma as i16 + class.attribute_mods.charisma as i16 - 8)
        .clamp(3, 50) as u8;

    let attrs = Attributes::new(base_str, base_dex, base_int, base_wis, base_con, base_cha);

    let con_mod = (base_con as i32 - 10) / 2;
    let mut hp_val = class.hit_die as i32 + con_mod;
    for _ in 2..=level {
        hp_val += (class.hit_die as i32 + con_mod).max(1);
    }
    let hp = Health::new(hp_val.max(1));
    let mana = oxide_core::Mana::from_formula(level as u16, base_int as u16, base_wis as u16);
    let stamina = oxide_core::Stamina::from_formula(level as u16, base_str as u16, base_dex as u16);

    let mut equipment = Equipment::new();

    for item_id in equipped_items {
        let item_tmpl = registry
            .items
            .get(item_id)
            .ok_or_else(|| format!("Item template '{}' not found", item_id))?;

        let item_entity = world.spawn((
            oxide_core::Item::new(item_id),
            oxide_core::Name::new(&item_tmpl.name),
        ));

        if let Some(set) = &item_tmpl.set {
            world
                .insert(item_entity, (SetMembership::from(set.clone()),))
                .unwrap();
        }

        if !item_tmpl.triggers.is_empty() {
            world
                .insert(
                    item_entity,
                    (oxide_core::ItemTriggers(item_tmpl.triggers.clone()),),
                )
                .unwrap();
        }

        if let Some(wep_def) = &item_tmpl.weapon {
            let dice = wep_def
                .damage
                .as_str()
                .parse()
                .map_err(|_| format!("Invalid damage dice '{}'", wep_def.damage.as_str()))?;
            let damage_type = wep_def
                .damage_type
                .parse()
                .map_err(|_| format!("Invalid damage type '{}'", wep_def.damage_type))?;
            let hands = match wep_def.hands.to_lowercase().as_str() {
                "twohand" | "twohanded" | "two_hand" | "two_handed" => {
                    oxide_core::WeaponHands::TwoHand
                }
                _ => {
                    if item_tmpl
                        .requires_skill
                        .as_ref()
                        .map(|s| s.id == "two_handed")
                        .unwrap_or(false)
                    {
                        oxide_core::WeaponHands::TwoHand
                    } else {
                        oxide_core::WeaponHands::OneHand
                    }
                }
            };
            world
                .insert(
                    item_entity,
                    (Weapon {
                        damage_dice: dice,
                        damage_type,
                        speed: wep_def.speed,
                        range: oxide_core::WeaponRange::Melee,
                        hands,
                    },),
                )
                .unwrap();
        }

        let slot = if let Some(eq_def) = &item_tmpl.equipment {
            EquipmentSlot::from_str(&eq_def.slot)
                .map_err(|_| format!("Invalid slot '{}'", eq_def.slot))?
        } else if item_tmpl.item_type == "weapon" {
            EquipmentSlot::Weapon
        } else {
            EquipmentSlot::Torso
        };

        equipment.equip(slot, item_entity);
    }

    let char_entity = world.spawn((
        oxide_core::Race(race_id.to_string()),
        oxide_core::Class(class_id.to_string()),
        Level(level),
        hp,
        mana,
        stamina,
        attrs,
        Position::new(room),
        equipment,
        Vec::<ActiveEffect>::new(),
    ));

    apply_all_passives(&mut world, char_entity, registry);
    evaluate_set_bonuses(&mut world, char_entity, &registry.sets);

    let mut final_str = base_str as i32;
    let mut final_dex = base_dex as i32;
    let mut final_int = base_int as i32;
    let mut final_wis = base_wis as i32;
    let mut final_con = base_con as i32;
    let mut final_cha = base_cha as i32;
    let mut bonus_ac = 0;

    let active_effects = world
        .query_one::<&Vec<ActiveEffect>>(char_entity)
        .ok()
        .and_then(|mut q| q.get().cloned())
        .unwrap_or_default();

    for eff in &active_effects {
        if let (Some(stat), Some(amount)) = (&eff.stat, eff.amount) {
            match stat.to_lowercase().as_str() {
                "strength" | "str" => final_str += amount,
                "dexterity" | "dex" => final_dex += amount,
                "intelligence" | "int" => final_int += amount,
                "wisdom" | "wis" => final_wis += amount,
                "constitution" | "con" => final_con += amount,
                "charisma" | "cha" => final_cha += amount,
                "armor" | "ac" => bonus_ac += amount,
                _ => {}
            }
        }
    }

    let _ = world.insert(
        char_entity,
        (Armor {
            base: 0,
            bonus: bonus_ac,
        },),
    );
    let final_ac = oxide_core::systems::combat::calculate_ac(&world, char_entity);

    let mut out = format!(
        "### Gear Loadout Simulation for `{} {}` (Level {})\n\n",
        race.name, class.name, level
    );
    out.push_str("**Equipped Items**:\n");
    for item_id in equipped_items {
        let name = registry
            .items
            .get(item_id)
            .map(|i| i.name.as_str())
            .unwrap_or("Unknown Item");
        out.push_str(&format!("*   `{}` ({})\n", item_id, name));
    }

    out.push_str("\n**Final Stats**:\n");
    out.push_str(&format!("*   **Armor Class (AC)**: {}\n", final_ac));
    out.push_str(&format!(
        "*   **Max Health**: {}\n",
        world
            .query_one::<&Health>(char_entity)
            .unwrap()
            .get()
            .unwrap()
            .max
    ));
    out.push_str(&format!(
        "*   **Max Mana**: {}\n",
        world
            .query_one::<&oxide_core::Mana>(char_entity)
            .unwrap()
            .get()
            .unwrap()
            .max
    ));
    out.push_str(&format!(
        "*   **Max Stamina**: {}\n",
        world
            .query_one::<&oxide_core::Stamina>(char_entity)
            .unwrap()
            .get()
            .unwrap()
            .max
    ));
    out.push_str(&format!(
        "*   **Attributes**: STR: {}, DEX: {}, INT: {}, WIS: {}, CON: {}, CHA: {}\n",
        final_str, final_dex, final_int, final_wis, final_con, final_cha
    ));

    out.push_str("\n**Active Set Bonuses**:\n");
    if let Ok(mut q) = world.query_one::<&SetTracker>(char_entity) {
        if let Some(tracker) = q.get() {
            if tracker.0.is_empty() {
                out.push_str("None\n");
            } else {
                for (set_id, count) in &tracker.0 {
                    let set_name = registry
                        .sets
                        .get(set_id)
                        .map(|s| s.name.as_str())
                        .unwrap_or(set_id.as_str());
                    out.push_str(&format!(
                        "*   **{}**: {} piece(s) equipped\n",
                        set_name, count
                    ));
                }
            }
        }
    }

    out.push_str("\n**Active Triggers**:\n");
    let mut has_triggers = false;
    if let Ok(mut eq) = world.query_one::<&Equipment>(char_entity) {
        if let Some(eq_val) = eq.get() {
            for (_slot, item_entity) in &eq_val.slots {
                if let Ok(mut t_q) = world.query_one::<&oxide_core::ItemTriggers>(*item_entity) {
                    if let Some(triggers) = t_q.get() {
                        for trig in &triggers.0 {
                            has_triggers = true;
                            let item_name = world
                                .query_one::<&oxide_core::Name>(*item_entity)
                                .ok()
                                .and_then(|mut q| q.get().map(|n| n.to_string()))
                                .unwrap_or_else(|| "Item".to_string());
                            out.push_str(&format!(
                                "*   `{}`: on `{}`: {}% chance to cast `{}` on `{}`\n",
                                item_name, trig.event, trig.chance, trig.cast, trig.target
                            ));
                        }
                    }
                }
            }
        }
    }
    if !has_triggers {
        out.push_str("None\n");
    }

    Ok(out)
}

// --- AI Wander Simulator ---
pub fn simulate_ai_wander(
    registry: &TemplateRegistry,
    mob_id: &str,
    start_room_str: &str,
    ticks: u32,
) -> Result<String, String> {
    let mob = registry
        .mobs
        .get(mob_id)
        .ok_or_else(|| format!("Mob template '{}' not found", mob_id))?;

    let (mut current_area, mut current_room) = if let Some((a, r)) = start_room_str.split_once(':')
    {
        (a.to_string(), r.to_string())
    } else if let Some((a, r)) = start_room_str.split_once('.') {
        (a.to_string(), r.to_string())
    } else {
        return Err("Start room must be in area_id:room_id or area_id.room_id format".to_string());
    };

    if !registry.room_exists(&current_area, &current_room) {
        return Err(format!(
            "Start room '{}:{}' not found in registry",
            current_area, current_room
        ));
    }

    let mut path = Vec::new();
    path.push(format!("{}:{}", current_area, current_room));

    let mut visitation: HashMap<String, u32> = HashMap::new();
    *visitation
        .entry(format!("{}:{}", current_area, current_room))
        .or_insert(0) += 1;

    for _ in 0..ticks {
        let room_tmpl = registry.get_room(&current_area, &current_room).unwrap();

        let mut valid_destinations = Vec::new();
        for dest_tpl in room_tmpl.exits.values() {
            let dest_str = dest_tpl.dest();
            let (dest_area, dest_room) = if let Some((a, r)) = dest_str.split_once(':') {
                (a.to_string(), r.to_string())
            } else if let Some((a, r)) = dest_str.split_once('.') {
                (a.to_string(), r.to_string())
            } else {
                (current_area.clone(), dest_str.to_string())
            };

            if !registry.room_exists(&dest_area, &dest_room) {
                continue;
            }

            if !mob.wander_rooms.is_empty() {
                let matches_wander_list = mob.wander_rooms.iter().any(|wr| {
                    if let Some((wa, wr_room)) = wr.split_once(':') {
                        wa == dest_area && wr_room == dest_room
                    } else if let Some((wa, wr_room)) = wr.split_once('.') {
                        wa == dest_area && wr_room == dest_room
                    } else {
                        wr == &dest_room
                    }
                });
                if !matches_wander_list {
                    continue;
                }
            }

            if mob.wander_area && dest_area != current_area {
                continue;
            }

            valid_destinations.push((dest_area, dest_room));
        }

        if valid_destinations.is_empty() {
            *visitation
                .entry(format!("{}:{}", current_area, current_room))
                .or_insert(0) += 1;
        } else {
            let idx = fastrand::usize(0..valid_destinations.len());
            let (next_area, next_room) = valid_destinations[idx].clone();
            current_area = next_area;
            current_room = next_room;
            let room_key = format!("{}:{}", current_area, current_room);
            path.push(room_key.clone());
            *visitation.entry(room_key).or_insert(0) += 1;
        }
    }

    let mut out = format!("### AI Wander Simulation for `{}`\n", mob_id);
    out.push_str(&format!("*   **Start Room**: `{}`\n", start_room_str));
    out.push_str(&format!("*   **Total Ticks**: {}\n\n", ticks));

    out.push_str("**Path Visited**:\n");
    let truncate_path = if path.len() > 15 {
        let mut p_str = path[0..10].join(" ➔ ");
        p_str.push_str(" ➔ ... ➔ ");
        p_str.push_str(&path[path.len() - 5..].join(" ➔ "));
        p_str
    } else {
        path.join(" ➔ ")
    };
    out.push_str(&format!("*   {}\n\n", truncate_path));

    out.push_str("**Room Visit Frequencies**:\n\n");
    out.push_str("| Room | Visits | Visitation Rate |\n");
    out.push_str("|---|---|---|\n");

    let mut visit_entries: Vec<(&String, &u32)> = visitation.iter().collect();
    visit_entries.sort_by(|a, b| b.1.cmp(a.1));
    for (room_key, count) in visit_entries {
        let rate = (*count as f64 / (ticks + 1) as f64) * 100.0;
        out.push_str(&format!("| `{}` | {} | {:.2}% |\n", room_key, count, rate));
    }

    Ok(out)
}

// --- Shop Pricing Simulator ---
pub fn simulate_shop_transaction(
    registry: &TemplateRegistry,
    shop_id: &str,
    item_id: &str,
) -> Result<String, String> {
    let shop = registry
        .shops
        .get(shop_id)
        .ok_or_else(|| format!("Shop template '{}' not found", shop_id))?;
    let item = registry
        .items
        .get(item_id)
        .ok_or_else(|| format!("Item template '{}' not found", item_id))?;

    let base_val = item.value as f64;

    fn fmt_coins(copper: f64) -> String {
        let total_cp = copper.round() as u64;
        let gp = total_cp / 10000;
        let rem = total_cp % 10000;
        let sp = rem / 100;
        let cp = rem % 100;

        let mut parts = Vec::new();
        if gp > 0 {
            parts.push(format!("{}gp", gp));
        }
        if sp > 0 || gp > 0 {
            parts.push(format!("{}sp", sp));
        }
        parts.push(format!("{}cp", cp));
        parts.join(" ")
    }

    let mut out = format!(
        "### Shop Pricing Simulation: Item `{}` at `{}`\n",
        item.name, shop.name
    );
    out.push_str(&format!(
        "*   **Base Value**: {} ({:.0} cp)\n",
        fmt_coins(base_val),
        base_val
    ));
    out.push_str(&format!(
        "*   **Shop Buy Markup (Sell Rate)**: {:.2}x\n",
        shop.sell_rate
    ));
    out.push_str(&format!(
        "*   **Shop Sell Markdown (Buy Rate)**: {:.2}x\n\n",
        shop.buy_rate
    ));

    out.push_str(
        "| Reputation Level | Buying from Shop (Player Cost) | Selling to Shop (Player Gain) |\n",
    );
    out.push_str("|---|---|---|\n");

    let reps = [
        ("Adored", 0.80, 1.20),
        ("Friendly", 0.90, 1.10),
        ("Neutral", 1.00, 1.00),
        ("Unfriendly", 1.25, 0.75),
        ("Hostile", 1.50, 0.50),
    ];

    for (name, buy_mult, sell_mult) in &reps {
        let player_buy = base_val * shop.sell_rate * buy_mult;
        let player_sell = base_val * shop.buy_rate * sell_mult;

        out.push_str(&format!(
            "| **{}** | {} ({:.0} cp) | {} ({:.0} cp) |\n",
            name,
            fmt_coins(player_buy),
            player_buy.round(),
            fmt_coins(player_sell),
            player_sell.round()
        ));
    }

    Ok(out)
}

// --- Content Cycle Validator ---
#[derive(Deserialize)]
struct RawSkillTemplate {
    id: String,
    requires_skill: Option<String>,
}

pub fn validate_content_dag(content_path: &Path) -> Result<String, String> {
    let mut skill_deps = HashMap::new();

    let skills_dir = content_path.join("skills");
    if skills_dir.exists() && skills_dir.is_dir() {
        if let Ok(entries) = fs::read_dir(skills_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().is_some_and(|e| e == "toml") {
                    if let Ok(content) = fs::read_to_string(&path) {
                        if let Ok(skill) = toml::from_str::<RawSkillTemplate>(&content) {
                            if let Some(ref req) = skill.requires_skill {
                                if !req.trim().is_empty() {
                                    skill_deps.insert(skill.id.clone(), req.trim().to_string());
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    let mut visited = HashSet::new();
    let mut stack = HashSet::new();
    let mut cycles = Vec::new();

    fn has_cycle(
        node: &str,
        graph: &HashMap<String, String>,
        visited: &mut HashSet<String>,
        stack: &mut HashSet<String>,
        path: &mut Vec<String>,
        cycles: &mut Vec<String>,
    ) -> bool {
        visited.insert(node.to_string());
        stack.insert(node.to_string());
        path.push(node.to_string());

        if let Some(next) = graph.get(node) {
            if !visited.contains(next) {
                if has_cycle(next, graph, visited, stack, path, cycles) {
                    return true;
                }
            } else if stack.contains(next) {
                let start_idx = path.iter().position(|x| x == next).unwrap_or(0);
                let cycle = path[start_idx..].join(" ➔ ") + " ➔ " + next;
                cycles.push(cycle);
                return true;
            }
        }

        path.pop();
        stack.remove(node);
        false
    }

    for skill_id in skill_deps.keys() {
        if !visited.contains(skill_id) {
            let mut path = Vec::new();
            has_cycle(
                skill_id,
                &skill_deps,
                &mut visited,
                &mut stack,
                &mut path,
                &mut cycles,
            );
        }
    }

    let mut out = "### Content DAG Integrity Report\n\n".to_string();
    if cycles.is_empty() {
        out.push_str("✅ **Success**: No circular skill dependency loops detected.\n");
    } else {
        out.push_str(
            "❌ **Error**: Circular dependency loops detected in skill prerequisites:\n\n",
        );
        for cycle in cycles {
            out.push_str(&format!("*   `{}`\n", cycle));
        }
    }

    let mut broken = Vec::new();
    for (skill, req) in &skill_deps {
        let req_file = content_path.join("skills").join(format!("{}.toml", req));
        if !req_file.exists() {
            broken.push(format!(
                "Skill `{}` requires non-existent skill `{}`",
                skill, req
            ));
        }
    }

    if !broken.is_empty() {
        out.push_str("\n⚠️ **Broken References**:\n\n");
        for brk in broken {
            out.push_str(&format!("*   {}\n", brk));
        }
    }

    Ok(out)
}

#[allow(clippy::too_many_arguments)]
pub fn simulate_character_creation(
    registry: &TemplateRegistry,
    race_id: &str,
    class_id: &str,
    strength: u8,
    dexterity: u8,
    intelligence: u8,
    wisdom: u8,
    constitution: u8,
    charisma: u8,
    selected_skills: &[String],
) -> Result<String, String> {
    let race = registry
        .races
        .get(race_id)
        .ok_or_else(|| format!("Race '{}' not found", race_id))?;

    let class = registry
        .classes
        .get(class_id)
        .ok_or_else(|| format!("Class '{}' not found", class_id))?;

    // Validate attributes (Standard Array or Point-Buy)
    let base_attrs = [
        strength,
        dexterity,
        intelligence,
        wisdom,
        constitution,
        charisma,
    ];
    let mut sorted_attrs = base_attrs;
    sorted_attrs.sort();
    let expected_array = [8, 10, 12, 13, 14, 15];

    let mut is_valid = sorted_attrs == expected_array;
    if !is_valid {
        // Try point buy check
        let mut total_cost = 0;
        let mut point_buy_ok = true;
        for &v in &base_attrs {
            if !(8..=18).contains(&v) {
                point_buy_ok = false;
                break;
            }
            let mut current = 8;
            let mut cost = 0;
            const PB_COSTS: [u8; 11] = [1, 1, 1, 1, 1, 2, 2, 3, 3, 4, 4];
            while current < v {
                if !(8..18).contains(&current) {
                    point_buy_ok = false;
                    break;
                }
                cost += PB_COSTS[(current - 8) as usize];
                current += 1;
            }
            total_cost += cost;
        }
        if point_buy_ok && total_cost == 27 {
            is_valid = true;
        }
    }

    if !is_valid {
        return Err("Attributes must match either Standard Array [15, 14, 13, 12, 10, 8] or Point-Buy (27 points spent, max 18 per stat)".to_string());
    }

    // Compute final attributes (race base + class mods + chosen - 8)
    let base_str = race.attributes.strength as i16;
    let base_dex = race.attributes.dexterity as i16;
    let base_int = race.attributes.intelligence as i16;
    let base_wis = race.attributes.wisdom as i16;
    let base_con = race.attributes.constitution as i16;
    let base_cha = race.attributes.charisma as i16;

    let mod_str = class.attribute_mods.strength as i16;
    let mod_dex = class.attribute_mods.dexterity as i16;
    let mod_int = class.attribute_mods.intelligence as i16;
    let mod_wis = class.attribute_mods.wisdom as i16;
    let mod_con = class.attribute_mods.constitution as i16;
    let mod_cha = class.attribute_mods.charisma as i16;

    let final_str = (base_str + mod_str + strength as i16 - 8).clamp(3, 50) as u8;
    let final_dex = (base_dex + mod_dex + dexterity as i16 - 8).clamp(3, 50) as u8;
    let final_int = (base_int + mod_int + intelligence as i16 - 8).clamp(3, 50) as u8;
    let final_wis = (base_wis + mod_wis + wisdom as i16 - 8).clamp(3, 50) as u8;
    let final_con = (base_con + mod_con + constitution as i16 - 8).clamp(3, 50) as u8;
    let final_cha = (base_cha + mod_cha + charisma as i16 - 8).clamp(3, 50) as u8;

    let hp = class.hit_die as i32 + (final_con as i32 - 10) / 2;
    let hp = hp.max(1);

    let mana = oxide_core::Mana::from_formula(1, final_int as u16, final_wis as u16);
    let stamina = oxide_core::Stamina::from_formula(1, final_str as u16, final_dex as u16);

    let mut auto_skills = HashSet::new();
    for ability in &race.racial_abilities {
        auto_skills.insert(ability.clone());
    }
    for skill_id in &class.auto_skills {
        auto_skills.insert(skill_id.clone());
    }
    for skill_id in selected_skills {
        auto_skills.insert(skill_id.clone());
    }

    let mut auto_skills_sorted: Vec<String> = auto_skills.into_iter().collect();
    auto_skills_sorted.sort();

    let gold = &class.starting_gold;

    let mut out = format!(
        "### Character Creation Simulation: Race = `{}`, Class = `{}`\n\n",
        race_id, class_id
    );
    out.push_str("#### Base Attributes (Chosen):\n");
    out.push_str(&format!(
        "*   Str: {}, Dex: {}, Int: {}, Wis: {}, Con: {}, Cha: {}\n\n",
        strength, dexterity, intelligence, wisdom, constitution, charisma
    ));
    out.push_str("#### Final Attributes:\n");
    out.push_str(&format!(
        "*   Str: {} (Racial Base: {}, Class Mod: {})\n",
        final_str, race.attributes.strength, class.attribute_mods.strength
    ));
    out.push_str(&format!(
        "*   Dex: {} (Racial Base: {}, Class Mod: {})\n",
        final_dex, race.attributes.dexterity, class.attribute_mods.dexterity
    ));
    out.push_str(&format!(
        "*   Int: {} (Racial Base: {}, Class Mod: {})\n",
        final_int, race.attributes.intelligence, class.attribute_mods.intelligence
    ));
    out.push_str(&format!(
        "*   Wis: {} (Racial Base: {}, Class Mod: {})\n",
        final_wis, race.attributes.wisdom, class.attribute_mods.wisdom
    ));
    out.push_str(&format!(
        "*   Con: {} (Racial Base: {}, Class Mod: {})\n",
        final_con, race.attributes.constitution, class.attribute_mods.constitution
    ));
    out.push_str(&format!(
        "*   Cha: {} (Racial Base: {}, Class Mod: {})\n\n",
        final_cha, race.attributes.charisma, class.attribute_mods.charisma
    ));
    out.push_str("#### Derived Resources:\n");
    out.push_str(&format!(
        "*   **Hit Points (HP)**: {} (Hit Die: d{})\n",
        hp, class.hit_die
    ));
    out.push_str(&format!("*   **Mana**: {}\n", mana.max));
    out.push_str(&format!("*   **Stamina**: {}\n\n", stamina.max));
    out.push_str("#### Starting Gold:\n");
    out.push_str(&format!(
        "*   Copper: {}, Silver: {}, Gold: {}, Platinum: {}\n\n",
        gold.copper, gold.silver, gold.gold, gold.platinum
    ));
    out.push_str("#### Auto-Granted Skills:\n");
    if auto_skills_sorted.is_empty() {
        out.push_str("*   *(None)*\n");
    } else {
        for s in auto_skills_sorted {
            out.push_str(&format!("*   `{}`\n", s));
        }
    }

    Ok(out)
}

pub fn simulate_crafting(
    registry: &TemplateRegistry,
    recipe_id: &str,
    _player_level: u8,
    dexterity: u8,
    intelligence: u8,
    skill_rank: u16,
    has_station: bool,
) -> Result<String, String> {
    let recipe = registry
        .recipes
        .get(recipe_id)
        .ok_or_else(|| format!("Recipe '{}' not found", recipe_id))?;

    let item_res = registry
        .items
        .get(&recipe.result.template_id)
        .ok_or_else(|| {
            format!(
                "Resulting item template '{}' not found",
                recipe.result.template_id
            )
        })?;

    let station_ok = if let Some(_req_station) = &recipe.station {
        has_station
    } else {
        true
    };

    if !station_ok {
        return Ok(format!(
            "### Crafting Simulation Failed\n\n*   **Reason**: Missing required crafting station: `{}`\n",
            recipe.station.as_deref().unwrap_or("")
        ));
    }

    if let Some(req) = &recipe.skill_requirement {
        if skill_rank < req.rank as u16 {
            return Ok(format!(
                "### Crafting Simulation Failed\n\n*   **Reason**: Crafting skill too low. Required: `{}`, Current: `{}`\n",
                req.rank, skill_rank
            ));
        }
    }

    let base_chance = recipe.success_chance as i32;
    let stat_bonus = ((dexterity as i32 + intelligence as i32) / 2 - 10) / 2;
    let final_chance = (base_chance + stat_bonus).clamp(5, 95);

    let roll = fastrand::i32(1..=100);
    let success = roll <= final_chance;
    let is_critical_failure = roll == 100;

    let mut out = format!("### Crafting Simulation: Recipe = `{}`\n\n", recipe.name);
    out.push_str(&format!(
        "*   **Result Item**: `{}` ({})\n",
        recipe.result.template_id, item_res.name
    ));
    out.push_str(&format!("*   **Skill Rank**: {}\n", skill_rank));
    out.push_str(&format!(
        "*   **Success Chance**: {}% (Base: {}%, Stat Bonus: {})\n",
        final_chance, base_chance, stat_bonus
    ));
    out.push_str(&format!(
        "*   **Roll**: {} ➔ {}\n\n",
        roll,
        if success {
            "SUCCESS"
        } else if is_critical_failure {
            "CRITICAL FAILURE"
        } else {
            "FAILURE"
        }
    ));

    if success {
        let diff = recipe.difficulty as i32;
        let margin = (skill_rank as i32 - diff).max(0);
        let quality_roll = fastrand::i32(1..=100) + margin;

        let quality = if quality_roll >= 120 {
            "legendary"
        } else if quality_roll >= 100 {
            "epic"
        } else if quality_roll >= 80 {
            "rare"
        } else if quality_roll >= 50 {
            "uncommon"
        } else {
            "common"
        };

        let xp_gained = recipe.difficulty.pow(2) * 10;

        out.push_str("#### Success Details:\n");
        out.push_str(&format!(
            "*   **Quantity Crafted**: {}\n",
            recipe.result.quantity
        ));
        out.push_str(&format!(
            "*   **Result Quality Tier**: `{:?}` (Quality Roll: {})\n",
            quality, quality_roll
        ));
        out.push_str(&format!("*   **Experience Gained**: {} XP\n", xp_gained));
        out.push_str("*   **Materials Consumed**: All materials consumed successfully.\n");
    } else if is_critical_failure {
        out.push_str("#### Failure Details:\n");
        out.push_str(
            "*   **Outcome**: Critical Failure! All crafting materials were completely lost.\n",
        );
    } else {
        out.push_str("#### Failure Details:\n");
        out.push_str(
            "*   **Outcome**: Failed. 50% of the crafting materials were salvaged/recovered.\n",
        );
    }

    Ok(out)
}

#[allow(clippy::too_many_arguments)]
pub fn simulate_skill_use(
    registry: &TemplateRegistry,
    skill_id: &str,
    actor_level: u8,
    actor_class: Option<&str>,
    actor_race: Option<&str>,
    strength: Option<u8>,
    dexterity: Option<u8>,
    intelligence: Option<u8>,
    wisdom: Option<u8>,
    constitution: Option<u8>,
    charisma: Option<u8>,
    skill_rank: Option<u16>,
    target_level: Option<u8>,
) -> Result<String, String> {
    let skill = registry
        .skills
        .get(skill_id)
        .ok_or_else(|| format!("Skill template '{}' not found", skill_id))?;

    let mut world = World::new();
    let room = world.spawn(());

    let actor_attrs = Attributes::new(
        strength.unwrap_or(10),
        dexterity.unwrap_or(10),
        intelligence.unwrap_or(10),
        wisdom.unwrap_or(10),
        constitution.unwrap_or(10),
        charisma.unwrap_or(10),
    );
    let mut actor_skills = LearnedSkills::new();
    let rank = skill_rank.unwrap_or(1);
    actor_skills.skills.insert(skill_id.to_string(), rank);

    let actor = world.spawn((
        oxide_core::Name("Actor".to_string()),
        Level(actor_level),
        Health::new(100),
        Mana::new(100),
        Stamina::new(100),
        Energy::new(100),
        Psi::new(100),
        oxide_core::Wallet::new(0, 0, 10, 0),
        oxide_core::Experience(1000),
        actor_skills,
        PlayerState::Resting(RestState::Standing),
        Position::new(room),
        SkillCooldowns::default(),
        oxide_core::CombatState::NotInCombat,
        actor_attrs,
    ));

    if let Some(c) = actor_class {
        let _ = world.insert(actor, (oxide_core::Class(c.to_string()),));
    }
    if let Some(r) = actor_race {
        let _ = world.insert(actor, (oxide_core::Race(r.to_string()),));
    }

    let target = world.spawn((
        oxide_core::Name("Target".to_string()),
        Level(target_level.unwrap_or(1)),
        Health::new(100),
        Position::new(room),
    ));

    let mut out = format!(
        "### Skill/Spell Use Simulation: Skill = `{}`\n\n",
        skill.name
    );

    match can_use_skill(&world, actor, skill, Some(target)) {
        Ok(_) => {
            out.push_str("*   **Validation Check**: PASSED\n");
            let _ = deduct_resource_cost(&mut world, actor, &skill.cost);
            out.push_str(&format!("*   **Resource Cost**: `{:?}`\n", skill.cost));

            if let Some(effect) = &skill.effect {
                let messages = apply_skill_effect(
                    &mut world,
                    actor,
                    Some(target),
                    effect,
                    &skill.name,
                    registry,
                );

                out.push_str("\n#### Resolution Log:\n");
                if messages.is_empty() {
                    out.push_str("*   *(No output messages generated)*\n");
                } else {
                    for m in messages {
                        out.push_str(&format!("*   {}\n", m));
                    }
                }
            } else {
                out.push_str("\n*   *This skill has no effect defined in its template.*\n");
            }

            let final_mana = world
                .query_one::<&Mana>(actor)
                .map(|mut q| q.get().map(|m| m.current).unwrap_or(0))
                .unwrap_or(0);
            let final_stamina = world
                .query_one::<&Stamina>(actor)
                .map(|mut q| q.get().map(|m| m.current).unwrap_or(0))
                .unwrap_or(0);
            let final_hp = world
                .query_one::<&Health>(actor)
                .map(|mut q| q.get().map(|m| m.current).unwrap_or(0))
                .unwrap_or(0);
            let target_hp = world
                .query_one::<&Health>(target)
                .map(|mut q| q.get().map(|m| m.current).unwrap_or(0))
                .unwrap_or(0);

            out.push_str("\n#### Post-execution States:\n");
            out.push_str(&format!("*   Actor HP: {}\n", final_hp));
            out.push_str(&format!("*   Actor Mana: {}\n", final_mana));
            out.push_str(&format!("*   Actor Stamina: {}\n", final_stamina));
            out.push_str(&format!("*   Target HP: {}\n", target_hp));
        }
        Err(err_msg) => {
            out.push_str("*   **Validation Check**: FAILED\n");
            out.push_str(&format!("*   **Error Reason**: {}\n", err_msg));
        }
    }

    Ok(out)
}

pub fn simulate_prayer(
    registry: &TemplateRegistry,
    deity_id: &str,
    player_race: &str,
    player_class: &str,
    player_alignment: &str,
    cleric_level: Option<u8>,
    wisdom: u8,
) -> Result<String, String> {
    let deity = registry
        .deities
        .get(deity_id)
        .ok_or_else(|| format!("Deity '{}' not found", deity_id))?;

    let _race = registry
        .races
        .get(player_race)
        .ok_or_else(|| format!("Race '{}' not found", player_race))?;

    let class = registry
        .classes
        .get(player_class)
        .ok_or_else(|| format!("Class '{}' not found", player_class))?;

    let policy_ok = match &class.deity_policy {
        DeityPolicy::None => false,
        DeityPolicy::Any => true,
        DeityPolicy::Required => true,
        DeityPolicy::Subset(allowed_list) => allowed_list
            .iter()
            .any(|id| id.to_lowercase() == deity_id.to_lowercase()),
    };

    let race_ok = if deity.allowed_races.is_empty() {
        true
    } else {
        deity
            .allowed_races
            .iter()
            .any(|r| r.to_lowercase() == player_race.to_lowercase())
    };

    let class_ok = if deity.allowed_classes.is_empty() {
        true
    } else {
        deity
            .allowed_classes
            .iter()
            .any(|c| c.to_lowercase() == player_class.to_lowercase())
    };

    let align_ok = if deity.allowed_alignments.is_empty() {
        true
    } else {
        deity
            .allowed_alignments
            .iter()
            .any(|a| a.to_lowercase() == player_alignment.to_lowercase())
    };

    let eligible = policy_ok && race_ok && class_ok && align_ok;

    let mut out = format!(
        "### Deity Adoption & Prayer Simulation: Deity = `{}`\n\n",
        deity.name
    );
    out.push_str("#### Eligibility Matrix:\n");
    out.push_str(&format!(
        "*   Class Deity Policy check: {}\n",
        if policy_ok { "PASSED" } else { "FAILED" }
    ));
    out.push_str(&format!(
        "*   Race alignment check: {}\n",
        if race_ok { "PASSED" } else { "FAILED" }
    ));
    out.push_str(&format!(
        "*   Class restrictions check: {}\n",
        if class_ok { "PASSED" } else { "FAILED" }
    ));
    out.push_str(&format!(
        "*   Alignment restrictions check: {}\n\n",
        if align_ok { "PASSED" } else { "FAILED" }
    ));
    out.push_str(&format!(
        "*   **Overall Eligibility**: {}\n\n",
        if eligible {
            "**ELIGIBLE**"
        } else {
            "**INELIGIBLE**"
        }
    ));

    if eligible {
        if let Some(effect) = &deity.prayer_effect {
            let base_duration = effect.duration_secs;
            let cooldown = effect.cooldown_secs;

            let wis_mod = ((wisdom as i32 - 10) / 2).max(0) as u64;
            let level_mult = cleric_level.unwrap_or(1) as u64;
            let final_duration = base_duration + (wis_mod * 10) + (level_mult * 5);

            out.push_str("#### Prayer Simulation (Cast Outcomes):\n");
            out.push_str(&format!("*   **Buff / Effect ID**: `{}`\n", effect.buff_id));
            out.push_str(&format!(
                "*   **Base Duration**: {} seconds\n",
                base_duration
            ));
            out.push_str(&format!("*   **Final Duration (Scaled)**: {} seconds (Wis bonus: +{}s, Level bonus: +{}s)\n", final_duration, wis_mod * 10, level_mult * 5));
            out.push_str(&format!("*   **Cooldown**: {} seconds\n", cooldown));
            out.push_str(&format!(
                "*   **Flavor Description**: *\"{}\"*\n",
                effect.description
            ));
        } else {
            out.push_str("#### Prayer Simulation:\n");
            out.push_str("*   *This deity does not have an active prayer effect defined in their template.*\n");
        }
    }

    Ok(out)
}

pub fn simulate_prestige_eligibility(
    registry: &TemplateRegistry,
    prestige_class_id: &str,
    base_classes: &HashMap<String, u8>,
    skill_ranks: &HashMap<String, u16>,
    completed_quests: &[String],
    faction_standings: &HashMap<String, i32>,
) -> Result<String, String> {
    let p_class = registry
        .classes
        .get(prestige_class_id)
        .ok_or_else(|| format!("Prestige class '{}' not found", prestige_class_id))?;

    let is_prestige = p_class
        .params
        .get("prestige")
        .map(|v| v.to_lowercase() == "true")
        .unwrap_or(false);

    if !is_prestige {
        return Err(format!(
            "Class '{}' is not configured as a prestige class (params.prestige is not true)",
            prestige_class_id
        ));
    }

    let mut out = format!("### Prestige Class Eligibility: `{}`\n\n", p_class.name);
    out.push_str("#### Requirements Checklist:\n");

    let mut eligible = true;

    if let Some(req_lvl_str) = p_class.params.get("requires_level") {
        if let Ok(req_lvl) = req_lvl_str.parse::<u8>() {
            let total_level: u8 = base_classes.values().sum();
            let met = total_level >= req_lvl;
            if !met {
                eligible = false;
            }
            out.push_str(&format!(
                "*   [{}] Total character level >= {} (Current: {})\n",
                if met { "x" } else { " " },
                req_lvl,
                total_level
            ));
        }
    }

    if let Some(req_class_str) = p_class.params.get("requires_class") {
        let parts: Vec<&str> = req_class_str.split(':').collect();
        if parts.len() == 2 {
            if let Ok(req_lvl) = parts[1].trim().parse::<u8>() {
                let class_id = parts[0].trim().to_lowercase();
                let current_lvl = base_classes.get(&class_id).copied().unwrap_or(0);
                let met = current_lvl >= req_lvl;
                if !met {
                    eligible = false;
                }
                out.push_str(&format!(
                    "*   [{}] Level in class `{}` >= {} (Current: {})\n",
                    if met { "x" } else { " " },
                    class_id,
                    req_lvl,
                    current_lvl
                ));
            }
        }
    }

    if let Some(req_skill_str) = p_class.params.get("requires_skills") {
        let parts: Vec<&str> = req_skill_str.split(':').collect();
        if parts.len() == 2 {
            if let Ok(req_rank) = parts[1].trim().parse::<u16>() {
                let skill_id = parts[0].trim().to_lowercase();
                let current_rank = skill_ranks.get(&skill_id).copied().unwrap_or(0);
                let met = current_rank >= req_rank;
                if !met {
                    eligible = false;
                }
                out.push_str(&format!(
                    "*   [{}] Rank in skill `{}` >= {} (Current: {})\n",
                    if met { "x" } else { " " },
                    skill_id,
                    req_rank,
                    current_rank
                ));
            }
        }
    }

    if let Some(req_race) = p_class.params.get("requires_race") {
        let allowed = p_class.allowed_races.is_empty()
            || p_class
                .allowed_races
                .iter()
                .any(|r| r.to_lowercase() == req_race.to_lowercase());
        if !allowed {
            eligible = false;
        }
        out.push_str(&format!(
            "*   [{}] Race allowed: `{}`\n",
            if allowed { "x" } else { " " },
            req_race
        ));
    }

    if let Some(req_align) = p_class.params.get("requires_alignment") {
        let allowed = p_class.allowed_alignments.is_empty()
            || p_class
                .allowed_alignments
                .iter()
                .any(|a| a.to_lowercase() == req_align.to_lowercase());
        if !allowed {
            eligible = false;
        }
        out.push_str(&format!(
            "*   [{}] Alignment allowed: `{}`\n",
            if allowed { "x" } else { " " },
            req_align
        ));
    }

    if let Some(req_quest) = p_class.params.get("requires_quest") {
        let met = completed_quests
            .iter()
            .any(|q| q.to_lowercase() == req_quest.to_lowercase());
        if !met {
            eligible = false;
        }
        out.push_str(&format!(
            "*   [{}] Completed quest: `{}`\n",
            if met { "x" } else { " " },
            req_quest
        ));
    }

    if let Some(req_faction_str) = p_class.params.get("requires_faction") {
        let parts: Vec<&str> = req_faction_str.split(':').collect();
        if parts.len() == 2 {
            if let Ok(req_val) = parts[1].trim().parse::<i32>() {
                let faction_id = parts[0].trim().to_lowercase();
                let current_val = faction_standings.get(&faction_id).copied().unwrap_or(0);
                let met = current_val >= req_val;
                if !met {
                    eligible = false;
                }
                out.push_str(&format!(
                    "*   [{}] Standing with faction `{}` >= {} (Current: {})\n",
                    if met { "x" } else { " " },
                    faction_id,
                    req_val,
                    current_val
                ));
            }
        }
    }

    out.push_str(&format!(
        "\n*   **Final Status**: {}\n",
        if eligible {
            "**ELIGIBLE TO ADOPT**"
        } else {
            "**NOT ELIGIBLE**"
        }
    ));

    Ok(out)
}

pub struct MockMember {
    pub class_id: String,
    pub has_shield: bool,
    pub is_front_row: bool,
}

pub fn simulate_group_formation(formation: &str, members: &[MockMember]) -> Result<String, String> {
    let size = members.len();
    let fmt_lower = formation.to_lowercase();

    let mut out = format!("### Group Formation Simulation: `{}`\n\n", formation);
    out.push_str(&format!("*   **Total Members**: {}\n", size));

    let (min_size, detail) = match fmt_lower.as_str() {
        "line" => (2, "Line: requires min 2 members. Modifiers: +1 AC front, -1 AC back."),
        "scattered" => (2, "Scattered: requires min 2 members. Modifiers: -2 AC, +10% dodge."),
        "column" => (3, "Column: requires min 3 members. Modifiers: +1 damage first hit."),
        "wedge" => (3, "Wedge: requires min 3 members. Modifiers: +2 attack, -4 AC leader."),
        "shield wall" => (2, "Shield Wall: requires min 2 members. Modifiers: +2 AC, -2 attack. Shields required for all members."),
        _ => return Err(format!("Unknown formation '{}'. Valid options: Line, Scattered, Column, Wedge, Shield Wall", formation)),
    };

    out.push_str(&format!("*   **Rules**: {}\n\n", detail));

    if size < min_size {
        out.push_str(&format!("*   **Status**: **INVALID FORMATION** (Requires at least {} members, but only has {})\n", min_size, size));
        return Ok(out);
    }

    if fmt_lower == "shield wall" {
        let missing_shield = members.iter().any(|m| !m.has_shield);
        if missing_shield {
            out.push_str("*   **Status**: **INVALID FORMATION** (All members must equip a shield for Shield Wall)\n");
            return Ok(out);
        }
    }

    out.push_str("| Member # | Class | Position | Shield? | Modifiers Applied |\n");
    out.push_str("|---|---|---|---|---|\n");

    for (idx, m) in members.iter().enumerate() {
        let is_leader = idx == 0;
        let pos_str = if is_leader { "Leader" } else { "Member" };
        let shield_str = if m.has_shield { "Yes" } else { "No" };

        let modifiers = match fmt_lower.as_str() {
            "line" => {
                if m.is_front_row {
                    "+1 AC (Front Row)"
                } else {
                    "-1 AC (Back Row)"
                }
            }
            "scattered" => "-2 AC, +10% Dodge",
            "column" if is_leader => "+1 Damage on First Hit (Lead)",
            "column" => "None",
            "wedge" => {
                if is_leader {
                    "+2 Attack, -4 AC (Leader Penalty)"
                } else {
                    "+2 Attack"
                }
            }
            "shield wall" => "+2 AC, -2 Attack",
            _ => "None",
        };

        out.push_str(&format!(
            "| #{} | {} | {} | {} | {} |\n",
            idx + 1,
            m.class_id,
            pos_str,
            shield_str,
            modifiers
        ));
    }

    out.push_str("\n*   **Status**: **ACTIVE & VALID**\n");

    Ok(out)
}

pub fn simulate_death_penalty(
    current_level: u8,
    current_xp: u64,
    allow_revive_room: bool,
) -> Result<String, String> {
    if !(1..=100).contains(&current_level) {
        return Err("Level must be between 1 and 100".to_string());
    }

    let _xp_floor = (current_level as u64).pow(3) * 100;
    let penalty = (current_xp as f64 * 0.10) as u64;

    let min_level = current_level.saturating_sub(5).max(1);
    let min_xp_floor = (min_level as u64).pow(3) * 100;

    let final_xp = current_xp.saturating_sub(penalty);
    let capped = final_xp < min_xp_floor;
    let actual_xp = if capped { min_xp_floor } else { final_xp };
    let actual_penalty = current_xp - actual_xp;

    let mut out = format!(
        "### Player Death & Ghost Simulation: Level {}\n\n",
        current_level
    );
    out.push_str("#### Experience Penalty:\n");
    out.push_str(&format!("*   **Current Experience**: {} XP\n", current_xp));
    out.push_str(&format!("*   **Raw 10% Penalty**: -{} XP\n", penalty));
    out.push_str(&format!(
        "*   **De-level Protection Floor (Level {} Floor)**: {} XP\n",
        min_level, min_xp_floor
    ));
    out.push_str(&format!(
        "*   **Actual Penalty Applied**: -{} XP {}\n",
        actual_penalty,
        if capped {
            "(CAPPED by de-level protection)"
        } else {
            ""
        }
    ));
    out.push_str(&format!(
        "*   **Post-Death Experience**: {} XP\n\n",
        actual_xp
    ));

    out.push_str("#### State & Resurrect Changes:\n");
    out.push_str("*   **Health Restored**: `1` HP (Unconscious threshold is -10 HP)\n");
    out.push_str("*   **Rest State**: `PlayerState::Dead` (Ghost State)\n");
    out.push_str("*   **Inventory & Equipment**: Completely cleared (dropped into a player corpse entity in room)\n");
    out.push_str("*   **Corpse Decay Duration**: 30 minutes (1800 seconds)\n");
    out.push_str(&format!(
        "*   **Revive Room allowed**: {}\n",
        if allow_revive_room {
            "Yes (Naked revive allowed without corpse)"
        } else {
            "No (Must walk back to corpse to reclaim)"
        }
    ));

    out.push_str("\n#### Ghost Speech Constraints:\n");
    out.push_str("*   All normal speech is filtered through `format_ghost_text()`.\n");
    out.push_str("*   **Example translation**: `\"hello world\"` ➔ rendered as alternating cyan and blue characters.\n");

    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use oxide_core::templates::{DeityTemplate, PrayerEffect, *};
    use oxide_core::{EffectTemplate, ResourceCost, SkillDef, SkillType, Targeting};

    #[test]
    fn test_simulate_loot() {
        let mut registry = TemplateRegistry::new();
        let mob = MobTemplate {
            id: "goblin".to_string(),
            name: "Goblin".to_string(),
            description: "A small goblin".to_string(),
            short_desc: "a small goblin".to_string(),
            level: 1,
            attributes: RaceAttributes::default(),
            health: HealthBounds {
                current: 12,
                max: 12,
            },
            armor: 5,
            damage: None,
            damage_type: None,
            race: None,
            size: "medium".to_string(),
            equipment: vec![],
            xp_value: 10,
            loot: LootTable {
                entries: vec![LootEntry {
                    item: "copper_coin".to_string(),
                    treasure_class: None,
                    count: Some(CountRange { min: 1, max: 5 }),
                    chance: 100,
                }],
            },
            ai_mode: "wander".to_string(),
            patrol_route: vec![],
            wander_rooms: vec![],
            wander_area: false,
            aggro_range: 0,
            aggro_players: false,
            aggro_mobs: false,
            aggro_race: vec![],
            faction: None,
            faction_standing: 0,
            trainer_types: vec![],
            languages: vec![],
            skills: vec![],
            shop: None,
            friendly: false,
            scripts: vec![],
            params: HashMap::new(),
        };
        registry.mobs.insert("goblin".to_string(), mob);

        let res = simulate_loot(&registry, "goblin", 10, false).unwrap();
        assert!(res.contains("Loot Simulation Results for `goblin`"));
        assert!(res.contains("Iterations (Corpses)"));

        let res_det = simulate_loot(&registry, "goblin", 10, true).unwrap();
        assert!(res_det.contains("Detailed Loot Simulation for `goblin`"));
    }

    #[test]
    fn test_simulate_combat() {
        let registry = TemplateRegistry::new();
        let res =
            simulate_combat(&registry, None, None, Some(5), None, Some(5), Some(10), 5).unwrap();
        assert!(res.contains("Combat Simulation Summary"));
        assert!(res.contains("Rounds Simulated"));
    }

    #[test]
    fn test_simulate_progression() {
        let mut registry = TemplateRegistry::new();
        let race = RaceTemplate {
            id: "human".to_string(),
            name: "Human".to_string(),
            description: "A versatile human".to_string(),
            attributes: RaceAttributes::default(),
            allowed_classes: vec![],
            allowed_alignments: vec![],
            racial_abilities: vec![],
            allowed_genders: HashMap::new(),
            appearance_bounds: AppearanceBounds::default(),
            age_default: 20,
            age_max: 100,
            params: HashMap::new(),
        };
        let class = ClassTemplate {
            id: "warrior".to_string(),
            name: "Warrior".to_string(),
            description: "A battle-hardened warrior".to_string(),
            hit_die: 10,
            attribute_mods: ClassAttributeMods::default(),
            bab: "good".to_string(),
            fort_save: "good".to_string(),
            ref_save: "poor".to_string(),
            will_save: "poor".to_string(),
            allowed_races: vec![],
            allowed_alignments: vec![],
            auto_skills: vec![],
            skill_pool: vec![],
            starting_skill_slots: 2,
            starting_items: vec![],
            starting_gold: WalletAmount::default(),
            deity_policy: DeityPolicy::Any,
            params: HashMap::new(),
        };
        registry.races.insert("human".to_string(), race);
        registry.classes.insert("warrior".to_string(), class);

        let res = simulate_progression(&registry, "human", "warrior", 1, 5).unwrap();
        assert!(res.contains("Progression Simulation for `Human Warrior`"));
    }

    #[test]
    fn test_simulate_character_creation() {
        let mut registry = TemplateRegistry::new();
        let race = RaceTemplate {
            id: "human".to_string(),
            name: "Human".to_string(),
            description: "A versatile human".to_string(),
            attributes: RaceAttributes {
                strength: 10,
                dexterity: 10,
                intelligence: 10,
                wisdom: 10,
                constitution: 10,
                charisma: 10,
            },
            allowed_classes: vec![],
            allowed_alignments: vec![],
            racial_abilities: vec!["human_versatility".to_string()],
            allowed_genders: HashMap::new(),
            appearance_bounds: AppearanceBounds::default(),
            age_default: 20,
            age_max: 100,
            params: HashMap::new(),
        };
        let class = ClassTemplate {
            id: "warrior".to_string(),
            name: "Warrior".to_string(),
            description: "A battle-hardened warrior".to_string(),
            hit_die: 10,
            attribute_mods: ClassAttributeMods {
                strength: 2,
                dexterity: 0,
                intelligence: 0,
                wisdom: 0,
                constitution: 1,
                charisma: 0,
            },
            bab: "good".to_string(),
            fort_save: "good".to_string(),
            ref_save: "poor".to_string(),
            will_save: "poor".to_string(),
            allowed_races: vec![],
            allowed_alignments: vec![],
            auto_skills: vec!["swordplay".to_string()],
            skill_pool: vec![],
            starting_skill_slots: 2,
            starting_items: vec![],
            starting_gold: WalletAmount {
                copper: 100,
                silver: 10,
                gold: 5,
                platinum: 1,
            },
            deity_policy: DeityPolicy::Any,
            params: HashMap::new(),
        };
        registry.races.insert("human".to_string(), race);
        registry.classes.insert("warrior".to_string(), class);

        // Test with Standard Array: [15, 14, 13, 12, 10, 8]
        let res = simulate_character_creation(
            &registry,
            "human",
            "warrior",
            15, // str
            14, // dex
            13, // int
            12, // wis
            10, // con
            8,  // cha
            &["shield_bash".to_string()],
        )
        .unwrap();

        assert!(res.contains("Character Creation Simulation: Race = `human`, Class = `warrior`"));
        assert!(res.contains("Final Attributes:"));
        assert!(res.contains("Str: 19")); // 10 (race) + 2 (class) + 15 (chosen) - 8 = 19
        assert!(res.contains("Con: 13")); // 10 (race) + 1 (class) + 10 (chosen) - 8 = 13
        assert!(res.contains("Hit Points (HP)**: 11")); // 10 (hit die) + (13-10)/2 = 11
        assert!(res.contains("swordplay"));
        assert!(res.contains("human_versatility"));
        assert!(res.contains("shield_bash"));
        assert!(res.contains("Platinum: 1"));

        // Test with invalid stats (should fail)
        let fail_res =
            simulate_character_creation(&registry, "human", "warrior", 18, 18, 18, 18, 18, 18, &[]);
        assert!(fail_res.is_err());
    }

    #[test]
    fn test_simulate_gear_loadout() {
        let mut registry = TemplateRegistry::new();
        let race = RaceTemplate {
            id: "human".to_string(),
            name: "Human".to_string(),
            description: "A versatile human".to_string(),
            attributes: RaceAttributes::default(),
            allowed_classes: vec![],
            allowed_alignments: vec![],
            racial_abilities: vec![],
            allowed_genders: HashMap::new(),
            appearance_bounds: AppearanceBounds::default(),
            age_default: 20,
            age_max: 100,
            params: HashMap::new(),
        };
        let class = ClassTemplate {
            id: "warrior".to_string(),
            name: "Warrior".to_string(),
            description: "A battle-hardened warrior".to_string(),
            hit_die: 10,
            attribute_mods: ClassAttributeMods::default(),
            bab: "good".to_string(),
            fort_save: "good".to_string(),
            ref_save: "poor".to_string(),
            will_save: "poor".to_string(),
            allowed_races: vec![],
            allowed_alignments: vec![],
            auto_skills: vec![],
            skill_pool: vec![],
            starting_skill_slots: 2,
            starting_items: vec![],
            starting_gold: WalletAmount::default(),
            deity_policy: DeityPolicy::Any,
            params: HashMap::new(),
        };
        let item = ItemTemplate {
            id: "sword".to_string(),
            name: "Iron Sword".to_string(),
            description: "A standard sword".to_string(),
            item_type: "weapon".to_string(),
            subtype: "sword".to_string(),
            quality: "common".to_string(),
            level_requirement: 1,
            weight: 3.0,
            value: 10,
            flags: vec![],
            allowed_classes: vec![],
            allowed_races: vec![],
            allowed_alignments: vec![],
            requires_skill: None,
            weapon: Some(WeaponDef {
                damage: DiceString("1d6".to_string()),
                damage_type: "slash".to_string(),
                speed: 1.5,
                range: "melee".to_string(),
                hands: "OneHand".to_string(),
            }),
            equipment: None,
            set: None,
            triggers: vec![],
            params: HashMap::new(),
        };
        registry.races.insert("human".to_string(), race);
        registry.classes.insert("warrior".to_string(), class);
        registry.items.insert("sword".to_string(), item);

        let res = simulate_gear_loadout(&registry, "human", "warrior", 5, &["sword".to_string()])
            .unwrap();
        assert!(res.contains("Gear Loadout Simulation for `Human Warrior`"));
        assert!(res.contains("Iron Sword"));
    }

    #[test]
    fn test_simulate_ai_wander() {
        let mut registry = TemplateRegistry::new();
        let mob = MobTemplate {
            id: "goblin".to_string(),
            name: "Goblin".to_string(),
            description: "A goblin".to_string(),
            short_desc: "".to_string(),
            level: 1,
            attributes: RaceAttributes::default(),
            health: HealthBounds {
                current: 12,
                max: 12,
            },
            armor: 5,
            damage: None,
            damage_type: None,
            race: None,
            size: "medium".to_string(),
            equipment: vec![],
            xp_value: 10,
            loot: LootTable::default(),
            ai_mode: "wander".to_string(),
            patrol_route: vec![],
            wander_rooms: vec![],
            wander_area: false,
            aggro_range: 0,
            aggro_players: false,
            aggro_mobs: false,
            aggro_race: vec![],
            faction: None,
            faction_standing: 0,
            trainer_types: vec![],
            languages: vec![],
            skills: vec![],
            shop: None,
            friendly: false,
            scripts: vec![],
            params: HashMap::new(),
        };
        registry.mobs.insert("goblin".to_string(), mob);

        let mut area = AreaTemplate {
            id: "forest".to_string(),
            name: "The Forest".to_string(),
            description: "".to_string(),
            level_range: None,
            flags: vec![],
            weather_zone: None,
            reset_interval: None,
            credits: None,
            spawns: vec![],
            rooms: HashMap::new(),
        };

        let r1 = RoomTemplate {
            id: "room1".to_string(),
            area: "forest".to_string(),
            name: "Room 1".to_string(),
            description: "".to_string(),
            exits: HashMap::from([(
                "east".to_string(),
                ExitTemplate::Simple("forest:room2".to_string()),
            )]),
            portals: vec![],
            flags: vec![],
            content: RoomContent::default(),
            allow_revive: false,
            script: None,
            params: HashMap::new(),
        };

        let r2 = RoomTemplate {
            id: "room2".to_string(),
            area: "forest".to_string(),
            name: "Room 2".to_string(),
            description: "".to_string(),
            exits: HashMap::from([(
                "west".to_string(),
                ExitTemplate::Simple("forest:room1".to_string()),
            )]),
            portals: vec![],
            flags: vec![],
            content: RoomContent::default(),
            allow_revive: false,
            script: None,
            params: HashMap::new(),
        };

        area.rooms.insert("room1".to_string(), r1);
        area.rooms.insert("room2".to_string(), r2);
        registry.areas.insert("forest".to_string(), area);

        let res = simulate_ai_wander(&registry, "goblin", "forest:room1", 5).unwrap();
        assert!(res.contains("AI Wander Simulation for `goblin`"));
        assert!(res.contains("Room Visit Frequencies"));
    }

    #[test]
    fn test_simulate_shop_transaction() {
        let mut registry = TemplateRegistry::new();
        let shop = ShopTemplate {
            id: "shop".to_string(),
            name: "General Store".to_string(),
            buy_rate: 0.5,
            sell_rate: 1.2,
            restock_secs: 300,
            inventory: vec![],
            params: HashMap::new(),
        };
        let item = ItemTemplate {
            id: "sword".to_string(),
            name: "Iron Sword".to_string(),
            description: "A standard sword".to_string(),
            item_type: "weapon".to_string(),
            subtype: "sword".to_string(),
            quality: "common".to_string(),
            level_requirement: 1,
            weight: 3.0,
            value: 100,
            flags: vec![],
            allowed_classes: vec![],
            allowed_races: vec![],
            allowed_alignments: vec![],
            requires_skill: None,
            weapon: None,
            equipment: None,
            set: None,
            triggers: vec![],
            params: HashMap::new(),
        };
        registry.shops.insert("shop".to_string(), shop);
        registry.items.insert("sword".to_string(), item);

        let res = simulate_shop_transaction(&registry, "shop", "sword").unwrap();
        assert!(res.contains("Shop Pricing Simulation: Item `Iron Sword` at `General Store`"));
        assert!(res.contains("Reputation Level"));
    }

    #[test]
    fn test_validate_content_dag() {
        let temp_dir = std::env::temp_dir().join(format!("mud_test_skills_{}", fastrand::u64(..)));
        let skills_dir = temp_dir.join("skills");
        std::fs::create_dir_all(&skills_dir).unwrap();

        std::fs::write(
            skills_dir.join("skill1.toml"),
            r#"
            id = "skill1"
            requires_skill = "skill2"
            "#,
        )
        .unwrap();
        std::fs::write(
            skills_dir.join("skill2.toml"),
            r#"
            id = "skill2"
            requires_skill = "skill1"
            "#,
        )
        .unwrap();

        let res = validate_content_dag(&temp_dir).unwrap();
        assert!(res.contains("Circular dependency loops detected"));
        assert!(
            res.contains("skill1 ➔ skill2 ➔ skill1") || res.contains("skill2 ➔ skill1 ➔ skill2")
        );

        std::fs::remove_dir_all(&temp_dir).unwrap();
    }

    #[test]
    fn test_simulate_crafting() {
        let mut registry = TemplateRegistry::new();
        let recipe = RecipeDef {
            id: "potion".to_string(),
            name: "Healing Potion".to_string(),
            description: "Restores health".to_string(),
            station: Some("alchemy_lab".to_string()),
            skill_requirement: Some(RecipeSkillReq {
                id: "alchemy".to_string(),
                rank: 5,
            }),
            difficulty: 5,
            materials: vec![],
            result: RecipeResult {
                template_id: "red_potion".to_string(),
                quantity: 1,
            },
            success_chance: 80,
            quality_scaling: false,
            script: None,
        };
        let item = ItemTemplate {
            id: "red_potion".to_string(),
            name: "Red Potion".to_string(),
            description: "Restores 50 HP".to_string(),
            item_type: "usable".to_string(),
            subtype: "potion".to_string(),
            quality: "common".to_string(),
            level_requirement: 1,
            weight: 0.5,
            value: 10,
            flags: vec![],
            allowed_classes: vec![],
            allowed_races: vec![],
            allowed_alignments: vec![],
            requires_skill: None,
            weapon: None,
            equipment: None,
            set: None,
            triggers: vec![],
            params: HashMap::new(),
        };
        registry.recipes.insert("potion".to_string(), recipe);
        registry.items.insert("red_potion".to_string(), item);

        let res_no_station = simulate_crafting(&registry, "potion", 1, 10, 10, 5, false).unwrap();
        assert!(res_no_station.contains("Missing required crafting station"));

        let res_low_skill = simulate_crafting(&registry, "potion", 1, 10, 10, 2, true).unwrap();
        assert!(res_low_skill.contains("Crafting skill too low"));

        let res_success = simulate_crafting(&registry, "potion", 1, 10, 10, 6, true).unwrap();
        assert!(res_success.contains("Crafting Simulation: Recipe = `Healing Potion`"));
    }

    #[test]
    fn test_simulate_skill_use() {
        let mut registry = TemplateRegistry::new();
        let mut skill = SkillDef::new("heal", "Lesser Heal", "Heals target", SkillType::Magic);
        skill.max_rank = 5;
        skill.level_requirement = 1;
        skill.cooldown_secs = 5;
        skill.targeting = Targeting::Single { range: 10 };
        skill.cost = ResourceCost::Mana(10);
        skill.effect = Some(EffectTemplate::Heal {
            dice: "1d8+2".to_string(),
        });
        registry.skills.insert("heal".to_string(), skill);

        let res = simulate_skill_use(
            &registry,
            "heal",
            1,
            None,
            None,
            Some(10),
            Some(10),
            Some(10),
            Some(10),
            Some(10),
            Some(10),
            Some(1),
            Some(1),
        )
        .unwrap();

        assert!(res.contains("Skill/Spell Use Simulation: Skill = `Lesser Heal`"));
        assert!(res.contains("Validation Check**: PASSED"));
        assert!(res.contains("Resource Cost"));
    }

    #[test]
    fn test_simulate_prayer() {
        let mut registry = TemplateRegistry::new();
        let race = RaceTemplate {
            id: "human".to_string(),
            name: "Human".to_string(),
            description: "".to_string(),
            attributes: RaceAttributes::default(),
            allowed_classes: vec![],
            allowed_alignments: vec![],
            racial_abilities: vec![],
            allowed_genders: HashMap::new(),
            appearance_bounds: AppearanceBounds::default(),
            age_default: 20,
            age_max: 100,
            params: HashMap::new(),
        };
        let class = ClassTemplate {
            id: "cleric".to_string(),
            name: "Cleric".to_string(),
            description: "".to_string(),
            hit_die: 8,
            attribute_mods: ClassAttributeMods::default(),
            bab: "medium".to_string(),
            fort_save: "good".to_string(),
            ref_save: "poor".to_string(),
            will_save: "good".to_string(),
            allowed_races: vec![],
            allowed_alignments: vec![],
            auto_skills: vec![],
            params: HashMap::new(),
            skill_pool: vec![],
            starting_skill_slots: 2,
            starting_items: vec![],
            starting_gold: WalletAmount::default(),
            deity_policy: DeityPolicy::Any,
        };
        let deity = DeityTemplate {
            id: "sol".to_string(),
            name: "Sol".to_string(),
            description: "".to_string(),
            alignment: None,
            symbol: "".to_string(),
            favored_weapon: None,
            tenets: vec![],
            domains: vec![],
            allowed_alignments: vec!["lawful good".to_string()],
            allowed_classes: vec![],
            allowed_races: vec![],
            prayer_effect: Some(PrayerEffect {
                buff_id: "sol_blessing".to_string(),
                description: "Warm light".to_string(),
                duration_secs: 60,
                cooldown_secs: 300,
            }),
            params: HashMap::new(),
        };
        registry.races.insert("human".to_string(), race);
        registry.classes.insert("cleric".to_string(), class);
        registry.deities.insert("sol".to_string(), deity);

        let res = simulate_prayer(
            &registry,
            "sol",
            "human",
            "cleric",
            "lawful good",
            Some(5),
            14,
        )
        .unwrap();
        assert!(res.contains("Deity Adoption & Prayer Simulation: Deity = `Sol`"));
        assert!(res.contains("Overall Eligibility**: **ELIGIBLE**"));
        assert!(res.contains("Buff / Effect ID**: `sol_blessing`"));

        let res_fail = simulate_prayer(
            &registry,
            "sol",
            "human",
            "cleric",
            "chaotic evil",
            Some(5),
            14,
        )
        .unwrap();
        assert!(res_fail.contains("Overall Eligibility**: **INELIGIBLE**"));
    }

    #[test]
    fn test_simulate_prestige_eligibility() {
        let mut registry = TemplateRegistry::new();
        let class = ClassTemplate {
            id: "paladin".to_string(),
            name: "Paladin".to_string(),
            description: "".to_string(),
            hit_die: 10,
            attribute_mods: ClassAttributeMods::default(),
            bab: "full".to_string(),
            fort_save: "good".to_string(),
            ref_save: "poor".to_string(),
            will_save: "poor".to_string(),
            allowed_races: vec![],
            allowed_alignments: vec![],
            auto_skills: vec![],
            params: HashMap::from([
                ("prestige".to_string(), "true".to_string()),
                ("requires_class".to_string(), "warrior:5".to_string()),
                ("requires_skills".to_string(), "swordplay:5".to_string()),
            ]),
            skill_pool: vec![],
            starting_skill_slots: 2,
            starting_items: vec![],
            starting_gold: WalletAmount::default(),
            deity_policy: DeityPolicy::Any,
        };
        registry.classes.insert("paladin".to_string(), class);

        let mut base_classes = HashMap::new();
        base_classes.insert("warrior".to_string(), 5);

        let mut skill_ranks = HashMap::new();
        skill_ranks.insert("swordplay".to_string(), 5);

        let res = simulate_prestige_eligibility(
            &registry,
            "paladin",
            &base_classes,
            &skill_ranks,
            &[],
            &HashMap::new(),
        )
        .unwrap();

        assert!(res.contains("Prestige Class Eligibility: `Paladin`"));
        assert!(res.contains("Final Status**: **ELIGIBLE TO ADOPT**"));
    }

    #[test]
    fn test_simulate_group_formation() {
        let members = vec![
            MockMember {
                class_id: "warrior".to_string(),
                has_shield: true,
                is_front_row: true,
            },
            MockMember {
                class_id: "cleric".to_string(),
                has_shield: true,
                is_front_row: false,
            },
        ];

        let res = simulate_group_formation("shield wall", &members).unwrap();
        assert!(res.contains("Group Formation Simulation: `shield wall`"));
        assert!(res.contains("Status**: **ACTIVE & VALID**"));
    }

    #[test]
    fn test_simulate_death_penalty() {
        let res = simulate_death_penalty(10, 20000, true).unwrap();
        assert!(res.contains("Player Death & Ghost Simulation: Level 10"));
        assert!(res.contains("Actual Penalty Applied**: -2000 XP"));
    }
}
