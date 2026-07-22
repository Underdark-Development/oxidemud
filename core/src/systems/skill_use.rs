use crate::{
    EffectTemplate, Energy, Entity, Health, LearnedSkills, Mana, PlayerState, Position, Psi,
    ResourceCost, RestState, SkillCooldowns, SkillDef, Stamina, Targeting, TemporaryEffect, World,
};

/// Verify if the actor is allowed and able to use the skill.
pub fn can_use_skill(
    world: &World,
    entity: Entity,
    skill: &SkillDef,
    target: Option<Entity>,
) -> Result<(), String> {
    // 1. Level check
    if let Ok(mut q_lvl) = world.query_one::<&crate::components::Level>(entity) {
        if let Some(level) = q_lvl.get() {
            if level.0 < skill.level_requirement {
                return Err(format!(
                    "You must be at least level {} to use this skill.",
                    skill.level_requirement
                ));
            }
        }
    }

    // 2. Class/Race restrictions
    if !skill.allowed_classes.is_empty() {
        let is_allowed = world
            .query_one::<&crate::components::Class>(entity)
            .ok()
            .and_then(|mut q| {
                q.get().map(|c| {
                    skill
                        .allowed_classes
                        .iter()
                        .any(|ac| ac.to_lowercase() == c.0.to_lowercase())
                })
            })
            .unwrap_or(false);
        if !is_allowed {
            return Err("Your class cannot use this skill.".to_string());
        }
    }
    if !skill.allowed_races.is_empty() {
        let is_allowed = world
            .query_one::<&crate::components::Race>(entity)
            .ok()
            .and_then(|mut q| {
                q.get().map(|r| {
                    skill
                        .allowed_races
                        .iter()
                        .any(|ar| ar.to_lowercase() == r.0.to_lowercase())
                })
            })
            .unwrap_or(false);
        if !is_allowed {
            return Err("Your race cannot use this skill.".to_string());
        }
    }

    // 3. Check learned rank
    let rank = world
        .query_one::<&LearnedSkills>(entity)
        .ok()
        .and_then(|mut q| q.get().map(|ls| ls.rank(&skill.id)))
        .unwrap_or(0);
    if rank == 0 {
        return Err("You have not learned this skill.".to_string());
    }

    // 4. Cooldown check
    if let Ok(mut q_cd) = world.query_one::<&SkillCooldowns>(entity) {
        if let Some(sc) = q_cd.get() {
            if let Some(&rem) = sc.cooldowns.get(&skill.id) {
                if rem > 0 {
                    return Err(format!("That skill is on cooldown ({}s remaining).", rem));
                }
            }
        }
    }

    // 5. Combat / rest state constraints
    let in_combat = world
        .query_one::<&crate::components::CombatState>(entity)
        .ok()
        .and_then(|mut q| q.get().map(|cs| cs.is_in_combat()))
        .unwrap_or(false);
    if in_combat && !skill.use_while_fighting {
        return Err("You cannot use that skill while fighting!".to_string());
    }

    let rest_state = world
        .query_one::<&PlayerState>(entity)
        .ok()
        .and_then(|mut q| q.get().map(|ps| ps.rest()))
        .unwrap_or(RestState::Standing);
    if rest_state == RestState::Sitting && !skill.use_while_sitting {
        return Err("You must stand up first.".to_string());
    }
    if rest_state == RestState::Sleeping || rest_state == RestState::Unconscious {
        return Err("You cannot do that while sleeping or unconscious.".to_string());
    }

    // 6. Target validation
    match skill.targeting {
        Targeting::SelfTarget => {
            if target.is_some() && target != Some(entity) {
                return Err("This skill can only target yourself.".to_string());
            }
        }
        Targeting::Single { .. } => {
            let t = target.ok_or_else(|| "You must specify a target.".to_string())?;
            let actor_room = world
                .query_one::<&Position>(entity)
                .ok()
                .and_then(|mut q| q.get().map(|p| p.room))
                .ok_or_else(|| "You are nowhere.".to_string())?;
            let target_room = world
                .query_one::<&Position>(t)
                .ok()
                .and_then(|mut q| q.get().map(|p| p.room))
                .ok_or_else(|| "Your target is nowhere.".to_string())?;
            if actor_room != target_room {
                return Err("Your target is not in the same room.".to_string());
            }
            if let Ok(mut q_hp) = world.query_one::<&Health>(t) {
                if let Some(hp) = q_hp.get() {
                    if hp.is_dead() {
                        return Err("Your target is already dead.".to_string());
                    }
                }
            }
        }
        Targeting::Room | Targeting::Area { .. } => {}
    }

    // 7. Resource check
    match skill.cost {
        ResourceCost::None => {}
        ResourceCost::Stamina(cost) => {
            let pool = world
                .query_one::<&Stamina>(entity)
                .ok()
                .and_then(|mut q| q.get().map(|p| p.current))
                .unwrap_or(0);
            if pool < cost {
                return Err("You are too exhausted.".to_string());
            }
        }
        ResourceCost::Mana(cost) => {
            let pool = world
                .query_one::<&Mana>(entity)
                .ok()
                .and_then(|mut q| q.get().map(|p| p.current))
                .unwrap_or(0);
            if pool < cost {
                return Err("You do not have enough mana.".to_string());
            }
        }
        ResourceCost::Energy(cost) => {
            let pool = world
                .query_one::<&Energy>(entity)
                .ok()
                .and_then(|mut q| q.get().map(|p| p.current))
                .unwrap_or(0);
            if pool < cost {
                return Err("You do not have enough energy.".to_string());
            }
        }
        ResourceCost::Psi(cost) => {
            let pool = world
                .query_one::<&Psi>(entity)
                .ok()
                .and_then(|mut q| q.get().map(|p| p.current))
                .unwrap_or(0);
            if pool < cost {
                return Err("You do not have enough focus/psi.".to_string());
            }
        }
        ResourceCost::Gold(cost) => {
            let pool = world
                .query_one::<&crate::components::Wallet>(entity)
                .ok()
                .and_then(|mut q| q.get().map(|w| w.total_copper()))
                .unwrap_or(0);
            let copper_cost = cost
                * crate::components::Wallet::COPPER_PER_SILVER
                * crate::components::Wallet::SILVER_PER_GOLD;
            if pool < copper_cost {
                return Err("You do not have enough gold.".to_string());
            }
        }
        ResourceCost::Xp(cost) => {
            let pool = world
                .query_one::<&crate::components::Experience>(entity)
                .ok()
                .and_then(|mut q| q.get().map(|xp| xp.0))
                .unwrap_or(0);
            if pool < cost {
                return Err("You do not have enough experience.".to_string());
            }
        }
    }

    Ok(())
}

/// Deduct resource cost from the player entity.
pub fn deduct_resource_cost(
    world: &mut World,
    entity: Entity,
    cost: &ResourceCost,
) -> Result<(), String> {
    match cost {
        ResourceCost::None => {}
        ResourceCost::Stamina(amount) => {
            if let Ok(mut q) = world.query_one::<&mut Stamina>(entity) {
                if let Some(pool) = q.get() {
                    pool.current = pool.current.saturating_sub(*amount);
                }
            }
        }
        ResourceCost::Mana(amount) => {
            if let Ok(mut q) = world.query_one::<&mut Mana>(entity) {
                if let Some(pool) = q.get() {
                    pool.current = pool.current.saturating_sub(*amount);
                }
            }
        }
        ResourceCost::Energy(amount) => {
            if let Ok(mut q) = world.query_one::<&mut Energy>(entity) {
                if let Some(pool) = q.get() {
                    pool.current = pool.current.saturating_sub(*amount);
                }
            }
        }
        ResourceCost::Psi(amount) => {
            if let Ok(mut q) = world.query_one::<&mut Psi>(entity) {
                if let Some(pool) = q.get() {
                    pool.current = pool.current.saturating_sub(*amount);
                }
            }
        }
        ResourceCost::Gold(amount) => {
            if let Ok(mut q) = world.query_one::<&mut crate::components::Wallet>(entity) {
                if let Some(wallet) = q.get() {
                    let copper_cost = *amount
                        * crate::components::Wallet::COPPER_PER_SILVER
                        * crate::components::Wallet::SILVER_PER_GOLD;
                    let _ = wallet.deduct_copper(copper_cost);
                }
            }
        }
        ResourceCost::Xp(amount) => {
            if let Ok(mut q) = world.query_one::<&mut crate::components::Experience>(entity) {
                if let Some(xp) = q.get() {
                    xp.0 = xp.0.saturating_sub(*amount);
                }
            }
        }
    }
    let _ = world.insert(entity, (crate::Dirty,));
    Ok(())
}

/// Apply the skill's specific effects onto target.
pub fn apply_skill_effect(
    world: &mut World,
    actor: Entity,
    target: Option<Entity>,
    effect: &EffectTemplate,
    source_name: &str,
    templates: &crate::templates::TemplateRegistry,
) -> Vec<String> {
    let mut msgs = Vec::new();
    let resolved_target = target.unwrap_or(actor);

    match effect {
        EffectTemplate::Damage { dice } => {
            if let Ok(roll) = dice.parse::<crate::dice::DiceRoll>() {
                let damage = roll.roll();
                let mut target_name = "target".to_string();
                if let Ok(mut q_name) = world.query_one::<&crate::components::Name>(resolved_target)
                {
                    if let Some(name) = q_name.get() {
                        target_name = name.0.clone();
                    }
                }

                let mut target_dead = false;
                if let Ok(mut q_hp) = world.query_one::<&mut Health>(resolved_target) {
                    if let Some(hp) = q_hp.get() {
                        hp.damage(damage);
                        msgs.push(format!(
                            "You hit {} with {} for {} damage!",
                            target_name, source_name, damage
                        ));
                        if hp.is_dead() {
                            target_dead = true;
                        }
                    }
                }

                if target_dead {
                    msgs.push(format!("{} has been defeated!", target_name));
                    crate::systems::combat::handle_death(world, resolved_target);
                } else {
                    let _ = world.insert(resolved_target, (crate::Dirty,));
                }
            }
        }
        EffectTemplate::Heal { dice } => {
            if let Ok(roll) = dice.parse::<crate::dice::DiceRoll>() {
                let heal = roll.roll();
                let mut target_name = "target".to_string();
                if let Ok(mut q_name) = world.query_one::<&crate::components::Name>(resolved_target)
                {
                    if let Some(name) = q_name.get() {
                        target_name = name.0.clone();
                    }
                }

                if let Ok(mut q_hp) = world.query_one::<&mut Health>(resolved_target) {
                    if let Some(hp) = q_hp.get() {
                        hp.heal(heal);
                        msgs.push(format!(
                            "You heal {} with {} for {} points!",
                            target_name, source_name, heal
                        ));
                    }
                }
                let _ = world.insert(resolved_target, (crate::Dirty,));
            }
        }
        EffectTemplate::Buff {
            stat,
            amount: _,
            duration,
        } => {
            let temp_effect = TemporaryEffect {
                effect: effect.clone(),
                remaining_secs: *duration,
                source: source_name.to_string(),
            };

            let mut effects_list: Vec<TemporaryEffect> = world
                .query_one::<&Vec<TemporaryEffect>>(resolved_target)
                .ok()
                .and_then(|mut q| q.get().cloned())
                .unwrap_or_default();

            effects_list.retain(|e| e.source != *source_name);
            effects_list.push(temp_effect);

            let _ = world.insert(resolved_target, (effects_list, crate::Dirty));
            msgs.push(format!(
                "You feel a surge of {} from {}!",
                stat, source_name
            ));
        }
        EffectTemplate::Debuff {
            stat,
            amount: _,
            duration,
        } => {
            let temp_effect = TemporaryEffect {
                effect: effect.clone(),
                remaining_secs: *duration,
                source: source_name.to_string(),
            };

            let mut effects_list: Vec<TemporaryEffect> = world
                .query_one::<&Vec<TemporaryEffect>>(resolved_target)
                .ok()
                .and_then(|mut q| q.get().cloned())
                .unwrap_or_default();

            effects_list.retain(|e| e.source != *source_name);
            effects_list.push(temp_effect);

            let _ = world.insert(resolved_target, (effects_list, crate::Dirty));
            msgs.push(format!("You feel weakened in {} by {}!", stat, source_name));
        }
        EffectTemplate::Teleport { room } => {
            let mut resolved_room = None;
            for (raw_entity, r_key) in world.query::<&crate::components::RoomKey>().iter() {
                if r_key.0 == *room {
                    resolved_room = Some(crate::Entity::from(raw_entity));
                    break;
                }
            }

            if let Some(r) = resolved_room {
                let _ = world.insert(resolved_target, (Position::new(r), crate::Dirty));
                msgs.push(format!("You are instantly teleported to {}!", room));
            } else {
                msgs.push(format!("Teleport failed: room '{}' not found.", room));
            }
        }
        EffectTemplate::Script { id } => {
            if let Some(bridge) = crate::scripting::get_scripting_bridge() {
                if let Err(e) = bridge.execute_use_skill(id, actor, target, world) {
                    msgs.push(format!("Script error: {}", e));
                }
            } else {
                msgs.push("Scripting bridge not available.".to_string());
            }
        }
        EffectTemplate::Spawn { mob_id, count } => {
            if let Some(actor_room) = world
                .query_one::<&Position>(actor)
                .ok()
                .and_then(|mut q| q.get().map(|p| p.room))
            {
                if let Some(mob_tmpl) = templates.mobs.get(mob_id) {
                    for _ in 0..*count {
                        let spawned_mob = mob_tmpl.spawn(world, actor_room, templates);
                        let _ = world.insert(spawned_mob, (crate::Dirty,));
                    }
                    let mob_name = &mob_tmpl.name;
                    msgs.push(format!("Spawned {} x{}.", mob_name, count));
                } else {
                    msgs.push(format!(
                        "Spawn failed: mob template '{}' not found.",
                        mob_id
                    ));
                }
            }
        }
    }

    msgs
}

/// Decay cooldowns for all entities.
pub fn run_cooldown_decay(world: &mut World, elapsed_secs: u32) {
    let mut updates = Vec::new();

    for (raw_entity, cd) in world.query::<&SkillCooldowns>().iter() {
        let entity = crate::Entity::from(raw_entity);
        let mut new_cds = cd.cooldowns.clone();
        let mut changed = false;

        for (_, rem) in new_cds.iter_mut() {
            if *rem > 0 {
                *rem = rem.saturating_sub(elapsed_secs);
                changed = true;
            }
        }

        new_cds.retain(|_, &mut rem| rem > 0);

        if changed {
            updates.push((entity, SkillCooldowns { cooldowns: new_cds }));
        }
    }

    for (entity, new_cd) in updates {
        let _ = world.insert(entity, (new_cd, crate::Dirty));
    }
}

/// Decay active/temporary effects on all entities.
pub fn run_temporary_effect_decay(
    world: &mut World,
    elapsed_secs: u32,
) -> Vec<(Entity, String, String)> {
    let mut updates = Vec::new();
    let mut expired = Vec::new();

    for (raw_entity, effects) in world.query::<&Vec<TemporaryEffect>>().iter() {
        let entity = crate::Entity::from(raw_entity);
        let mut new_effects = effects.clone();
        let mut changed = false;

        for idx in (0..new_effects.len()).rev() {
            let effect = &mut new_effects[idx];
            if effect.remaining_secs > 0 {
                effect.remaining_secs = effect.remaining_secs.saturating_sub(elapsed_secs);
                changed = true;
            }

            if effect.remaining_secs == 0 {
                let exp = new_effects.remove(idx);
                let stat_name = match &exp.effect {
                    EffectTemplate::Buff { stat, .. } => stat.clone(),
                    EffectTemplate::Debuff { stat, .. } => stat.clone(),
                    _ => String::new(),
                };
                expired.push((entity, exp.source, stat_name));
                changed = true;
            }
        }

        if changed {
            updates.push((entity, new_effects));
        }
    }

    for (entity, new_effects) in updates {
        if new_effects.is_empty() {
            let _ = world.remove_one::<Vec<TemporaryEffect>>(entity);
        } else {
            let _ = world.insert(entity, (new_effects,));
        }
        let _ = world.insert(entity, (crate::Dirty,));
    }

    expired
}

/// Calculate the entity's attributes modified by active passive and temporary effects.
pub fn get_modified_attributes(world: &World, entity: Entity) -> crate::components::Attributes {
    let mut attrs = world
        .query_one::<&crate::components::Attributes>(entity)
        .ok()
        .and_then(|mut q| q.get().cloned())
        .unwrap_or_default();

    // 1. Apply ActiveEffects (passives, set bonuses)
    if let Ok(mut q_effs) = world.query_one::<&Vec<crate::components::ActiveEffect>>(entity) {
        if let Some(effects) = q_effs.get() {
            for effect in effects {
                if let (Some(stat), Some(amount)) = (&effect.stat, effect.amount) {
                    match stat.as_str() {
                        "strength" => {
                            attrs.strength = (attrs.strength as i32 + amount).clamp(
                                crate::components::Attributes::MIN as i32,
                                crate::components::Attributes::MAX as i32,
                            ) as u8
                        }
                        "dexterity" => {
                            attrs.dexterity = (attrs.dexterity as i32 + amount).clamp(
                                crate::components::Attributes::MIN as i32,
                                crate::components::Attributes::MAX as i32,
                            ) as u8
                        }
                        "intelligence" => {
                            attrs.intelligence = (attrs.intelligence as i32 + amount).clamp(
                                crate::components::Attributes::MIN as i32,
                                crate::components::Attributes::MAX as i32,
                            ) as u8
                        }
                        "wisdom" => {
                            attrs.wisdom = (attrs.wisdom as i32 + amount).clamp(
                                crate::components::Attributes::MIN as i32,
                                crate::components::Attributes::MAX as i32,
                            ) as u8
                        }
                        "constitution" => {
                            attrs.constitution = (attrs.constitution as i32 + amount).clamp(
                                crate::components::Attributes::MIN as i32,
                                crate::components::Attributes::MAX as i32,
                            ) as u8
                        }
                        "charisma" => {
                            attrs.charisma = (attrs.charisma as i32 + amount).clamp(
                                crate::components::Attributes::MIN as i32,
                                crate::components::Attributes::MAX as i32,
                            ) as u8
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    // 2. Apply TemporaryEffects (buffs, debuffs)
    if let Ok(mut q_temps) = world.query_one::<&Vec<TemporaryEffect>>(entity) {
        if let Some(temps) = q_temps.get() {
            for temp in temps {
                match &temp.effect {
                    EffectTemplate::Buff { stat, amount, .. } => match stat.as_str() {
                        "strength" => {
                            attrs.strength = (attrs.strength as i32 + amount).clamp(
                                crate::components::Attributes::MIN as i32,
                                crate::components::Attributes::MAX as i32,
                            ) as u8
                        }
                        "dexterity" => {
                            attrs.dexterity = (attrs.dexterity as i32 + amount).clamp(
                                crate::components::Attributes::MIN as i32,
                                crate::components::Attributes::MAX as i32,
                            ) as u8
                        }
                        "intelligence" => {
                            attrs.intelligence = (attrs.intelligence as i32 + amount).clamp(
                                crate::components::Attributes::MIN as i32,
                                crate::components::Attributes::MAX as i32,
                            ) as u8
                        }
                        "wisdom" => {
                            attrs.wisdom = (attrs.wisdom as i32 + amount).clamp(
                                crate::components::Attributes::MIN as i32,
                                crate::components::Attributes::MAX as i32,
                            ) as u8
                        }
                        "constitution" => {
                            attrs.constitution = (attrs.constitution as i32 + amount).clamp(
                                crate::components::Attributes::MIN as i32,
                                crate::components::Attributes::MAX as i32,
                            ) as u8
                        }
                        "charisma" => {
                            attrs.charisma = (attrs.charisma as i32 + amount).clamp(
                                crate::components::Attributes::MIN as i32,
                                crate::components::Attributes::MAX as i32,
                            ) as u8
                        }
                        _ => {}
                    },
                    EffectTemplate::Debuff { stat, amount, .. } => match stat.as_str() {
                        "strength" => {
                            attrs.strength = (attrs.strength as i32 - amount).clamp(
                                crate::components::Attributes::MIN as i32,
                                crate::components::Attributes::MAX as i32,
                            ) as u8
                        }
                        "dexterity" => {
                            attrs.dexterity = (attrs.dexterity as i32 - amount).clamp(
                                crate::components::Attributes::MIN as i32,
                                crate::components::Attributes::MAX as i32,
                            ) as u8
                        }
                        "intelligence" => {
                            attrs.intelligence = (attrs.intelligence as i32 - amount).clamp(
                                crate::components::Attributes::MIN as i32,
                                crate::components::Attributes::MAX as i32,
                            ) as u8
                        }
                        "wisdom" => {
                            attrs.wisdom = (attrs.wisdom as i32 - amount).clamp(
                                crate::components::Attributes::MIN as i32,
                                crate::components::Attributes::MAX as i32,
                            ) as u8
                        }
                        "constitution" => {
                            attrs.constitution = (attrs.constitution as i32 - amount).clamp(
                                crate::components::Attributes::MIN as i32,
                                crate::components::Attributes::MAX as i32,
                            ) as u8
                        }
                        "charisma" => {
                            attrs.charisma = (attrs.charisma as i32 - amount).clamp(
                                crate::components::Attributes::MIN as i32,
                                crate::components::Attributes::MAX as i32,
                            ) as u8
                        }
                        _ => {}
                    },
                    _ => {}
                }
            }
        }
    }

    attrs
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::*;
    use crate::templates::TemplateRegistry;

    fn setup_test_world() -> (World, Entity, TemplateRegistry) {
        let mut world = World::new();
        let templates = TemplateRegistry::new();

        let actor = world.spawn((
            Name("TestActor".to_string()),
            Level(10),
            Attributes::new(12, 12, 12, 12, 12, 12),
            Health::new(100),
            Mana::new(100),
            Stamina::new(100),
            Energy::new(100),
            Psi::new(100),
            Wallet::new(0, 0, 10, 0), // 10 gold
            Experience(1000),
            LearnedSkills::new(),
            PlayerState::Resting(RestState::Standing),
            Position::new(Entity::from(hecs::Entity::DANGLING)),
        ));

        (world, actor, templates)
    }

    #[test]
    fn test_can_use_skill_level_gate() {
        let (world, actor, _templates) = setup_test_world();
        let mut skill = SkillDef::new("fireball", "Fireball", "A fire spell", SkillType::Magic);
        skill.level_requirement = 15;

        if let Ok(mut q) = world.query_one::<&mut LearnedSkills>(actor) {
            if let Some(ls) = q.get() {
                ls.skills.insert("fireball".to_string(), 1);
            }
        }

        let res = can_use_skill(&world, actor, &skill, None);
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("You must be at least level 15"));
    }

    #[test]
    fn test_can_use_skill_cost_mana() {
        let (mut world, actor, _templates) = setup_test_world();
        let mut skill = SkillDef::new("fireball", "Fireball", "A fire spell", SkillType::Magic);
        skill.cost = ResourceCost::Mana(40);

        if let Ok(mut q) = world.query_one::<&mut LearnedSkills>(actor) {
            if let Some(ls) = q.get() {
                ls.skills.insert("fireball".to_string(), 1);
            }
        }

        assert!(can_use_skill(&world, actor, &skill, None).is_ok());

        assert!(deduct_resource_cost(&mut world, actor, &skill.cost).is_ok());
        let mana_left = world
            .query_one::<&Mana>(actor)
            .ok()
            .and_then(|mut q| q.get().map(|m| m.current))
            .unwrap();
        assert_eq!(mana_left, 60);

        assert!(deduct_resource_cost(&mut world, actor, &ResourceCost::Mana(70)).is_ok());
        let res = can_use_skill(&world, actor, &skill, None);
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("mana"));
    }

    #[test]
    fn test_cooldown_decay() {
        let (mut world, actor, _templates) = setup_test_world();
        let mut cds = SkillCooldowns::default();
        cds.cooldowns.insert("power_strike".to_string(), 5);
        let _ = world.insert(actor, (cds,));

        run_cooldown_decay(&mut world, 2);

        let rem = world
            .query_one::<&SkillCooldowns>(actor)
            .ok()
            .and_then(|mut q| {
                q.get()
                    .and_then(|c| c.cooldowns.get("power_strike").copied())
            })
            .unwrap_or(0);
        assert_eq!(rem, 3);

        run_cooldown_decay(&mut world, 4);

        let rem = world
            .query_one::<&SkillCooldowns>(actor)
            .ok()
            .and_then(|mut q| {
                q.get()
                    .and_then(|c| c.cooldowns.get("power_strike").copied())
            })
            .unwrap_or(0);
        assert_eq!(rem, 0);
    }

    #[test]
    fn test_apply_effects_damage_heal_buff() {
        let (mut world, actor, templates) = setup_test_world();

        let damage_effect = EffectTemplate::Damage {
            dice: "2d6+4".to_string(),
        };
        let msgs = apply_skill_effect(
            &mut world,
            actor,
            Some(actor),
            &damage_effect,
            "Backstab",
            &templates,
        );
        assert!(!msgs.is_empty());
        assert!(msgs[0].contains("Backstab"));

        let hp_left = world
            .query_one::<&Health>(actor)
            .ok()
            .and_then(|mut q| q.get().map(|h| h.current))
            .unwrap();
        assert!(hp_left < 100);

        let heal_effect = EffectTemplate::Heal {
            dice: "20d1".to_string(),
        };
        let _ = apply_skill_effect(
            &mut world,
            actor,
            Some(actor),
            &heal_effect,
            "Heal Spell",
            &templates,
        );
        let hp_healed = world
            .query_one::<&Health>(actor)
            .ok()
            .and_then(|mut q| q.get().map(|h| h.current))
            .unwrap();
        assert_eq!(hp_healed, 100);

        let buff_effect = EffectTemplate::Buff {
            stat: "strength".to_string(),
            amount: 5,
            duration: 10,
        };
        let _ = apply_skill_effect(
            &mut world,
            actor,
            Some(actor),
            &buff_effect,
            "Giant Strength",
            &templates,
        );

        let modified_strength = get_modified_attributes(&world, actor).strength;
        assert_eq!(modified_strength, 17);

        let expired = run_temporary_effect_decay(&mut world, 10);
        assert_eq!(expired.len(), 1);
        assert_eq!(expired[0].1, "Giant Strength");

        let strength_after = get_modified_attributes(&world, actor).strength;
        assert_eq!(strength_after, 12);
    }
}
