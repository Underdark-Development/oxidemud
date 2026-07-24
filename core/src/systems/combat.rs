use crate::dice::DiceRoll;
use crate::systems::trigger::process_triggers;
use crate::{
    Armor, CombatState, Corpse, DamageType, Entity, Equipment, EquipmentSlot, Friendly, Health,
    Inventory, LastMessenger, Level, LootRule, Name, Npc, Player, PlayerState, Position,
    RecallRoom, Resistance, RoomExits, Weapon, WeaponHands, World,
};

// ---------------------------------------------------------------------------
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HitResult {
    Hit,
    Miss,
    Aborted,
}

#[derive(Debug, Clone)]
pub struct CombatOutcome {
    pub attacker: Entity,
    /// May be despawned (killed) by the time the caller processes this.
    pub target: Entity,
    pub room: Entity,
    pub attacker_name: String,
    pub target_name: String,
    pub attacker_is_player: bool,
    pub target_is_player: bool,
    pub kind: CombatOutcomeKind,
}

#[derive(Debug, Clone)]
pub enum CombatOutcomeKind {
    Hit {
        damage: i32,
        damage_type: DamageType,
        unconscious: bool,
    },
    Miss,
    Killed {
        damage: i32,
        damage_type: DamageType,
        xp_gained: u64,
        corpse: Entity,
        mob_template_id: Option<String>,
        mob_level: u8,
    },
    FleeSuccess {
        dest: Entity,
        moved: bool,
    },
    FleeFail {
        attempts: u8,
    },
}

/// Calculate ability modifier: (stat - 10) / 2
fn ability_mod(stat: u8) -> i32 {
    (stat as i32 - 10) / 2
}

/// Returns the attacker's shield entity if they have one equipped.
fn get_shield(world: &World, entity: Entity) -> Option<Entity> {
    world
        .query_one::<&Equipment>(entity)
        .ok()
        .and_then(|mut q| {
            q.get()
                .and_then(|eq| eq.equipped(&EquipmentSlot::Shield).copied())
        })
}

/// Check if attacker is dual-wielding (weapon in both Weapon and Shield slots).
fn is_dual_wielding(world: &World, entity: Entity) -> bool {
    let has_weapon = world
        .query_one::<&Equipment>(entity)
        .ok()
        .and_then(|mut q| {
            q.get().and_then(|eq| {
                eq.equipped(&EquipmentSlot::Weapon)
                    .and_then(|e| world.query_one::<&Weapon>(*e).ok())
                    .and_then(|mut wq| wq.get().map(|w| w.hands == WeaponHands::OneHand))
            })
        })
        .unwrap_or(false);

    if !has_weapon {
        return false;
    }

    // Check if Shield slot has a weapon (not an actual shield)
    get_shield(world, entity).is_some_and(|s| {
        world
            .query_one::<&Weapon>(s)
            .is_ok_and(|mut q| q.get().is_some())
    })
}

/// Get the equipped weapon entity and data for a given slot.
fn get_weapon_data(world: &World, entity: Entity, slot: EquipmentSlot) -> Option<(Entity, Weapon)> {
    let mut eq = world.query_one::<&Equipment>(entity).ok()?;
    let weapon_entity = *eq.get()?.equipped(&slot)?;
    let weapon = world
        .query_one::<&Weapon>(weapon_entity)
        .ok()?
        .get()?
        .clone();
    Some((weapon_entity, weapon))
}

/// Calculate AC for an entity.
pub fn calculate_ac(world: &World, entity: Entity) -> i32 {
    let level = world
        .query_one::<&Level>(entity)
        .ok()
        .and_then(|mut q| q.get().copied())
        .unwrap_or(Level(1));

    let dex = crate::get_modified_attributes(world, entity).dexterity;

    let armor = world
        .query_one::<&Armor>(entity)
        .ok()
        .and_then(|mut q| q.get().cloned())
        .unwrap_or(Armor { base: 0, bonus: 0 });

    // Shield AC bonus
    let shield_bonus = get_shield(world, entity).is_some_and(|s| {
        world
            .query_one::<&Armor>(s)
            .is_ok_and(|mut q| q.get().is_some())
    }) as i32
        * 2;

    let mut ac_modifier = 0;
    if let Ok(mut q) = world.query_one::<&Vec<crate::ActiveEffect>>(entity) {
        if let Some(effects) = q.get() {
            for effect in effects {
                if let (Some(stat), Some(amount)) = (&effect.stat, effect.amount) {
                    if stat.eq_ignore_ascii_case("ac") || stat.eq_ignore_ascii_case("armor_class") {
                        ac_modifier += amount;
                    }
                }
            }
        }
    }

    10 + level.0 as i32 + ability_mod(dex) + armor.total() + shield_bonus + ac_modifier
}

/// Calculate to-hit penalty for dual-wielding.
/// Main hand: -2, offhand: -4.
fn dual_wield_penalty(is_offhand: bool) -> i32 {
    if is_offhand {
        -4
    } else {
        -2
    }
}

/// Calculate damage for an attack from `attacker` against `target`.
/// `is_offhand` should be true for the offhand weapon when dual-wielding.
/// Returns (damage_amount, damage_type).
pub fn calculate_damage(
    world: &mut World,
    attacker: Entity,
    target: Entity,
    is_offhand: bool,
) -> (i32, DamageType) {
    let str = crate::get_modified_attributes(world, attacker).strength;

    let level = world
        .query_one::<&Level>(attacker)
        .ok()
        .and_then(|mut q| q.get().copied())
        .unwrap_or(Level(1));

    let str_mod = ability_mod(str);

    // Check for equipped weapon
    let slot = if is_offhand {
        EquipmentSlot::Shield
    } else {
        EquipmentSlot::Weapon
    };

    let weapon_damage = get_weapon_data(world, attacker, slot);

    let (mut final_dmg, mut final_type) = if let Some((_, ref wep)) = weapon_damage {
        let dice_damage = wep.damage_dice.roll();
        let str_bonus = if wep.is_two_handed() {
            // Two-handed: 1.5x str mod
            (str_mod as f32 * 1.5).round() as i32
        } else if is_offhand {
            // Offhand: 0.5x str mod
            (str_mod as f32 * 0.5).round() as i32
        } else {
            str_mod
        };
        let base = dice_damage + str_bonus;
        (base.max(1), wep.damage_type)
    } else {
        // Unarmed: 1d4 + str mod
        let dice = DiceRoll::new(1, 4, 0);
        let base = dice.roll() + str_mod + level.0 as i32 / 5;
        (base.max(1), DamageType::Bludgeon)
    };

    if let Some(bridge) = crate::scripting::get_scripting_bridge() {
        if let Ok((dmg, dtype)) =
            bridge.execute_combat_damage_hook(attacker, target, final_dmg, final_type, world)
        {
            final_dmg = dmg;
            final_type = dtype;
        }
    }

    let mut dmg_modifier = 0;
    if let Ok(mut q) = world.query_one::<&Vec<crate::ActiveEffect>>(attacker) {
        if let Some(effects) = q.get() {
            for effect in effects {
                if let (Some(stat), Some(amount)) = (&effect.stat, effect.amount) {
                    if stat.eq_ignore_ascii_case("damage") {
                        dmg_modifier += amount;
                    }
                }
            }
        }
    }

    // Weather damage modifiers (damage_fire, damage_lightning, ranged_attack)
    let room_opt = world
        .query_one::<&Position>(attacker)
        .ok()
        .and_then(|mut q| q.get().map(|p| p.room));

    if let Some(room) = room_opt {
        if let Ok(mut q_ws) = world.query_one::<&crate::WeatherState>(room) {
            if let Some(ws) = q_ws.get() {
                let weather_dmg_mod = ws.effects.get_damage_modifier(final_type.as_str());
                dmg_modifier += weather_dmg_mod;

                if let Some((_, ref wep)) = weapon_damage {
                    if wep.is_ranged() {
                        if let Some(val) = ws.effects.ranged_attack {
                            dmg_modifier += val;
                        }
                    }
                }
            }
        }
    }

    final_dmg = (final_dmg + dmg_modifier).max(1);

    (final_dmg, final_type)
}

/// Calculate to-hit value for attacker vs target. Returns HitResult.
/// `is_offhand` should be true for the offhand weapon when dual-wielding.
pub fn calculate_hit(
    world: &mut World,
    attacker: Entity,
    target: Entity,
    is_offhand: bool,
) -> HitResult {
    let mut hit_ctx = crate::scripting::HitContext {
        attacker,
        target,
        is_offhand,
        is_aborted: false,
        abort_reason: None,
        hit_modifier: 0,
        override_hit: None,
    };

    if let Some(bridge) = crate::scripting::get_scripting_bridge() {
        if let Ok(ctx) = bridge.execute_combat_hit_hook(attacker, target, is_offhand, world) {
            hit_ctx = ctx;
        }
    }

    if hit_ctx.is_aborted {
        if let Some(ref msg) = hit_ctx.abort_reason {
            if let Some(msg_bridge) = crate::scripting::get_message_bridge() {
                let room = world
                    .query_one::<&Position>(target)
                    .ok()
                    .and_then(|mut q| q.get().map(|p| p.room));
                if let Some(r) = room {
                    msg_bridge.echo_to_room(r, msg);
                }
            }
        }
        return HitResult::Aborted;
    }

    if let Some(override_val) = hit_ctx.override_hit {
        return if override_val {
            HitResult::Hit
        } else {
            HitResult::Miss
        };
    }

    let atk_level = world
        .query_one::<&Level>(attacker)
        .ok()
        .and_then(|mut q| q.get().copied())
        .unwrap_or(Level(1))
        .0 as i32;

    let str = crate::get_modified_attributes(world, attacker).strength;

    let ac = calculate_ac(world, target);

    let has_ambidexterity = world
        .query_one::<&crate::components::LearnedSkills>(attacker)
        .ok()
        .and_then(|mut q| q.get().map(|s| s.has("ambidexterity")))
        .unwrap_or(false);

    let dw_penalty = if is_dual_wielding(world, attacker) {
        let base_penalty = dual_wield_penalty(is_offhand);
        if has_ambidexterity {
            base_penalty / 2
        } else {
            base_penalty
        }
    } else {
        0
    };

    let roll = fastrand::i32(1..=20);
    if roll == 1 {
        return HitResult::Miss; // auto-miss
    }
    if roll == 20 {
        return HitResult::Hit; // auto-crit
    }

    let mut atk_modifier = 0;
    if let Ok(mut q) = world.query_one::<&Vec<crate::ActiveEffect>>(attacker) {
        if let Some(effects) = q.get() {
            for effect in effects {
                if let (Some(stat), Some(amount)) = (&effect.stat, effect.amount) {
                    if stat.eq_ignore_ascii_case("attack") || stat.eq_ignore_ascii_case("hit") {
                        atk_modifier += amount;
                    }
                }
            }
        }
    }

    // Weather ranged accuracy modifiers
    let room_opt = world
        .query_one::<&Position>(attacker)
        .ok()
        .and_then(|mut q| q.get().map(|p| p.room));

    if let Some(room) = room_opt {
        if let Ok(mut q_ws) = world.query_one::<&crate::WeatherState>(room) {
            if let Some(ws) = q_ws.get() {
                let slot = if is_offhand {
                    EquipmentSlot::Shield
                } else {
                    EquipmentSlot::Weapon
                };
                let is_ranged_wep = get_weapon_data(world, attacker, slot)
                    .map(|(_, w)| w.is_ranged())
                    .unwrap_or(false);

                if is_ranged_wep {
                    if let Some(val) = ws.effects.ranged_accuracy {
                        atk_modifier += val;
                    }
                    if let Some(pct) = ws.effects.ranged_accuracy_pct {
                        atk_modifier += (roll * pct) / 100;
                    }
                }
            }
        }
    }

    let mut dodge_modifier = 0;
    if let Ok(mut q) = world.query_one::<&Vec<crate::ActiveEffect>>(target) {
        if let Some(effects) = q.get() {
            for effect in effects {
                if let (Some(stat), Some(amount)) = (&effect.stat, effect.amount) {
                    if stat.eq_ignore_ascii_case("dodge") {
                        dodge_modifier += amount;
                    }
                }
            }
        }
    }

    if roll + atk_level + ability_mod(str) + dw_penalty + hit_ctx.hit_modifier + atk_modifier
        >= ac + dodge_modifier
    {
        HitResult::Hit
    } else {
        HitResult::Miss
    }
}

/// Expire all active script effects on an entity matching a specified expiry condition.
pub fn expire_effects_by_condition(
    world: &mut World,
    entity: Entity,
    condition: crate::EffectExpireCondition,
) {
    let mut expired_msgs = Vec::new();
    if let Ok(mut q) = world.query_one::<&mut crate::ActiveScriptEffects>(entity) {
        if let Some(active) = q.get() {
            let mut remaining = Vec::new();
            for effect in active.effects.drain(..) {
                if effect.expire_conditions.contains(&condition) {
                    if let Some(msg) = effect.expire_message {
                        expired_msgs.push(msg);
                    }
                } else {
                    remaining.push(effect);
                }
            }
            active.effects = remaining;
        }
    }

    if let Some(bridge) = crate::scripting::get_message_bridge() {
        for msg in expired_msgs {
            bridge.send_to_entity(entity, &msg);
        }
    }
}

pub fn transition_combat_state(world: &mut World, entity: Entity, new_state: CombatState) {
    let old_state = world
        .query_one::<&CombatState>(entity)
        .ok()
        .and_then(|mut q| q.get().cloned())
        .unwrap_or(CombatState::NotInCombat);

    if old_state != new_state {
        let _ = world.insert(entity, (new_state.clone(),));

        if new_state == CombatState::NotInCombat {
            // Automatically expire any active script effects configured to expire on ExitCombat
            expire_effects_by_condition(world, entity, crate::EffectExpireCondition::ExitCombat);
        }
    }
}

/// Run one combat pulse for all entities with CombatState.
/// Returns one outcome per attack swing for the server layer to dispatch as messages.
pub fn run_combat_pulse(world: &mut World) -> Vec<CombatOutcome> {
    let attackers: Vec<(Entity, CombatState, Entity)> = {
        let mut q = world.query::<(&CombatState, &Health, &Position)>();
        q.iter()
            .map(|(raw, (state, _health, pos))| {
                let attacker = crate::Entity::from(raw);
                (attacker, state.clone(), pos.room)
            })
            .collect()
    };

    let mut outcomes = Vec::new();

    for (attacker, state, room) in attackers {
        // Verify attacker still exists and is conscious
        let attacker_conscious = world
            .query_one::<&Health>(attacker)
            .ok()
            .and_then(|mut q| q.get().map(|h| h.is_conscious()))
            .unwrap_or(false);

        if !attacker_conscious {
            continue;
        }

        // Verify attacker is active and stand them up if sitting/resting
        let mut attacker_active = true;
        let mut attacker_should_stand = false;
        if let Ok(mut q_state) = world.query_one::<&PlayerState>(attacker) {
            if let Some(state) = q_state.get() {
                match state {
                    PlayerState::Dead
                    | PlayerState::Stunned { .. }
                    | PlayerState::Resting(crate::RestState::Sleeping)
                    | PlayerState::Resting(crate::RestState::Unconscious)
                    | PlayerState::Resting(crate::RestState::Dead) => {
                        attacker_active = false;
                    }
                    PlayerState::Resting(crate::RestState::Sitting)
                    | PlayerState::Resting(crate::RestState::Resting) => {
                        attacker_should_stand = true;
                    }
                    _ => {}
                }
            }
        }
        if !attacker_active {
            continue;
        }
        if attacker_should_stand {
            let _ = crate::systems::player_state::try_transition_player_state(
                world,
                attacker,
                crate::systems::player_state::PlayerStateTrigger::Stand,
            );
        }

        match state {
            CombatState::NotInCombat => continue,
            CombatState::Engaged {
                target,
                round_started: _,
                stance: _,
            } => {
                // Verify target still exists
                if world.query_one::<&Health>(target).is_err() {
                    transition_combat_state(world, attacker, CombatState::NotInCombat);
                    continue;
                }

                // Verify same room
                let target_room = match world
                    .query_one::<&Position>(target)
                    .ok()
                    .and_then(|mut q| q.get().map(|p| p.room))
                {
                    Some(r) => r,
                    None => {
                        transition_combat_state(world, attacker, CombatState::NotInCombat);
                        continue;
                    }
                };

                if target_room != room {
                    // Target has left the room; update combat state to NotInCombat
                    transition_combat_state(world, attacker, CombatState::NotInCombat);
                    continue;
                }

                // Check if target is conscious
                let target_conscious = world
                    .query_one::<&Health>(target)
                    .ok()
                    .and_then(|mut q| q.get().map(|h| h.is_conscious()))
                    .unwrap_or(false);

                if !target_conscious {
                    transition_combat_state(world, attacker, CombatState::NotInCombat);
                    continue;
                }

                // Auto stand up the target if they are sitting/resting
                let mut target_should_stand = false;
                if let Ok(mut q_state) = world.query_one::<&PlayerState>(target) {
                    if let Some(PlayerState::Resting(
                        crate::RestState::Sitting
                        | crate::RestState::Resting
                        | crate::RestState::Sleeping,
                    )) = q_state.get()
                    {
                        target_should_stand = true;
                    }
                }
                if target_should_stand {
                    let _ = crate::systems::player_state::try_transition_player_state(
                        world,
                        target,
                        crate::systems::player_state::PlayerStateTrigger::Stand,
                    );
                }

                // Pre-fetch names before any potential despawn
                let attacker_name = world
                    .query_one::<&Name>(attacker)
                    .ok()
                    .and_then(|mut q| q.get().map(|n| n.to_string()))
                    .unwrap_or_else(|| "Someone".to_owned());
                let target_name = world
                    .query_one::<&Name>(target)
                    .ok()
                    .and_then(|mut q| q.get().map(|n| n.to_string()))
                    .unwrap_or_else(|| "Something".to_owned());
                let attacker_is_player = world
                    .query_one::<&Player>(attacker)
                    .is_ok_and(|mut q| q.get().is_some());
                let target_is_player = world
                    .query_one::<&Player>(target)
                    .is_ok_and(|mut q| q.get().is_some());

                // Auto-engage non-friendly NPCs that aren't already in combat
                let is_npc = world
                    .query_one::<&Npc>(target)
                    .is_ok_and(|mut q| q.get().is_some());
                let is_friendly = world
                    .query_one::<&Friendly>(target)
                    .is_ok_and(|mut q| q.get().is_some());
                if is_npc && !is_friendly {
                    let current_state = world
                        .query_one::<&CombatState>(target)
                        .ok()
                        .and_then(|mut q| q.get().cloned())
                        .unwrap_or(CombatState::NotInCombat);
                    if current_state == CombatState::NotInCombat {
                        let stance = crate::systems::stance::get_active_stance(world, target);
                        transition_combat_state(
                            world,
                            target,
                            CombatState::Engaged {
                                target: attacker,
                                round_started: std::time::Instant::now(),
                                stance,
                            },
                        );
                    }
                }

                // Main hand attack
                let hit_result = calculate_hit(world, attacker, target, false);
                match hit_result {
                    HitResult::Hit => {
                        let (damage, damage_type) =
                            calculate_damage(world, attacker, target, false);

                        // Read mob info before apply_damage (which despawns the target)
                        let mob_template_id = world
                            .query_one::<&crate::Npc>(target)
                            .ok()
                            .and_then(|mut q| q.get().map(|n| n.template_id.clone()));
                        let mob_level = world
                            .query_one::<&crate::Level>(target)
                            .ok()
                            .and_then(|mut q| q.get().copied())
                            .unwrap_or(crate::Level(1))
                            .0;

                        let (final_damage, killed, unconscious, xp_gained, corpse) =
                            apply_damage(world, attacker, target, damage, damage_type);
                        if final_damage > 0 {
                            let kind = if killed {
                                CombatOutcomeKind::Killed {
                                    damage: final_damage,
                                    damage_type,
                                    xp_gained,
                                    corpse: corpse.unwrap(),
                                    mob_template_id,
                                    mob_level,
                                }
                            } else {
                                CombatOutcomeKind::Hit {
                                    damage: final_damage,
                                    damage_type,
                                    unconscious,
                                }
                            };
                            outcomes.push(CombatOutcome {
                                attacker,
                                target,
                                room,
                                attacker_name: attacker_name.clone(),
                                target_name: target_name.clone(),
                                attacker_is_player,
                                target_is_player,
                                kind,
                            });
                        }
                        if killed {
                            continue;
                        }
                    }
                    HitResult::Miss => {
                        outcomes.push(CombatOutcome {
                            attacker,
                            target,
                            room,
                            attacker_name: attacker_name.clone(),
                            target_name: target_name.clone(),
                            attacker_is_player,
                            target_is_player,
                            kind: CombatOutcomeKind::Miss,
                        });
                    }
                    HitResult::Aborted => {
                        // Hook has run and aborts default attack messages.
                        // But if this attack killed/despawned/aborted, we might still check if we are dual-wielding,
                        // or we might want to continue to offhand. Let's check if target is still valid/conscious.
                        let target_exists = world.query_one::<&Health>(target).is_ok();
                        if !target_exists {
                            continue;
                        }
                    }
                }

                // Offhand attack if dual-wielding
                if is_dual_wielding(world, attacker) {
                    let oh_hit = calculate_hit(world, attacker, target, true);
                    match oh_hit {
                        HitResult::Hit => {
                            let (oh_dmg, oh_type) = calculate_damage(world, attacker, target, true);

                            // Read mob info before apply_damage (which despawns the target)
                            let mob_template_id = world
                                .query_one::<&crate::Npc>(target)
                                .ok()
                                .and_then(|mut q| q.get().map(|n| n.template_id.clone()));
                            let mob_level = world
                                .query_one::<&crate::Level>(target)
                                .ok()
                                .and_then(|mut q| q.get().copied())
                                .unwrap_or(crate::Level(1))
                                .0;

                            let (final_damage, killed, unconscious, xp_gained, corpse) =
                                apply_damage(world, attacker, target, oh_dmg, oh_type);
                            if final_damage > 0 {
                                let kind = if killed {
                                    CombatOutcomeKind::Killed {
                                        damage: final_damage,
                                        damage_type: oh_type,
                                        xp_gained,
                                        corpse: corpse.unwrap(),
                                        mob_template_id,
                                        mob_level,
                                    }
                                } else {
                                    CombatOutcomeKind::Hit {
                                        damage: final_damage,
                                        damage_type: oh_type,
                                        unconscious,
                                    }
                                };
                                outcomes.push(CombatOutcome {
                                    attacker,
                                    target,
                                    room,
                                    attacker_name,
                                    target_name,
                                    attacker_is_player,
                                    target_is_player,
                                    kind,
                                });
                            }
                        }
                        HitResult::Miss => {
                            outcomes.push(CombatOutcome {
                                attacker,
                                target,
                                room,
                                attacker_name: attacker_name.clone(),
                                target_name: target_name.clone(),
                                attacker_is_player,
                                target_is_player,
                                kind: CombatOutcomeKind::Miss,
                            });
                        }
                        HitResult::Aborted => {}
                    }
                }
            }
            CombatState::Fleeing { target, attempts } => {
                // Verify target still exists and is in the same room
                let is_valid_target = world.query_one::<&Health>(target).is_ok() && {
                    let target_room = world
                        .query_one::<&Position>(target)
                        .ok()
                        .and_then(|mut q| q.get().map(|p| p.room));
                    target_room == Some(room)
                };

                // Pre-fetch names
                let attacker_name = world
                    .query_one::<&Name>(attacker)
                    .ok()
                    .and_then(|mut q| q.get().map(|n| n.to_string()))
                    .unwrap_or_else(|| "Someone".to_owned());
                let target_name = world
                    .query_one::<&Name>(target)
                    .ok()
                    .and_then(|mut q| q.get().map(|n| n.to_string()))
                    .unwrap_or_else(|| "Something".to_owned());
                let attacker_is_player = world
                    .query_one::<&Player>(attacker)
                    .is_ok_and(|mut q| q.get().is_some());
                let target_is_player = world
                    .query_one::<&Player>(target)
                    .is_ok_and(|mut q| q.get().is_some());

                if !is_valid_target {
                    // Target has fled/died/left, so attacker succeeds in ending combat
                    transition_combat_state(world, attacker, CombatState::NotInCombat);
                    outcomes.push(CombatOutcome {
                        attacker,
                        target,
                        room,
                        attacker_name,
                        target_name,
                        attacker_is_player,
                        target_is_player,
                        kind: CombatOutcomeKind::FleeSuccess {
                            dest: room,
                            moved: false,
                        },
                    });
                    continue;
                }

                // Flee check: d20 + dex_mod >= 10
                let dex = crate::get_modified_attributes(world, attacker).dexterity;
                let dex_mod = (dex as i32 - 10) / 2;

                let roll = fastrand::i32(1..=20);
                let success = roll + dex_mod >= 10;

                if success {
                    // Move to random room exit if possible
                    let exits = world
                        .query_one::<&RoomExits>(room)
                        .ok()
                        .and_then(|mut q| q.get().map(|e| e.0.clone()));

                    let mut moved = false;
                    let mut dest_room = room;
                    if let Some(exits) = exits {
                        let visible: Vec<_> = exits.iter().filter(|e| !e.is_hidden()).collect();
                        if !visible.is_empty() {
                            let idx = fastrand::usize(..visible.len());
                            let dest = visible[idx].dest;
                            let _ = world.insert(attacker, (Position::new(dest),));
                            dest_room = dest;
                            moved = true;
                        }
                    }

                    transition_combat_state(world, attacker, CombatState::NotInCombat);
                    outcomes.push(CombatOutcome {
                        attacker,
                        target,
                        room,
                        attacker_name,
                        target_name,
                        attacker_is_player,
                        target_is_player,
                        kind: CombatOutcomeKind::FleeSuccess {
                            dest: dest_room,
                            moved,
                        },
                    });
                } else {
                    // Flee failed: transition back to Engaged
                    let active_stance = crate::systems::stance::get_active_stance(world, attacker);
                    transition_combat_state(
                        world,
                        attacker,
                        CombatState::Engaged {
                            target,
                            round_started: std::time::Instant::now(),
                            stance: active_stance,
                        },
                    );

                    outcomes.push(CombatOutcome {
                        attacker,
                        target,
                        room,
                        attacker_name,
                        target_name,
                        attacker_is_player,
                        target_is_player,
                        kind: CombatOutcomeKind::FleeFail {
                            attempts: attempts + 1,
                        },
                    });
                }
            }
        }
    }

    outcomes
}

/// Apply damage to target, handling resistance, death, corpse, and XP.
/// Returns `(final_damage, killed, unconscious, xp_gained, corpse_entity)`.
/// On kill: spawns a corpse, grants XP to attacker, despawens the target.
/// `corpse_entity` is `Some(entity)` on kill, `None` otherwise.
pub fn apply_damage(
    world: &mut World,
    attacker: Entity,
    target: Entity,
    damage: i32,
    damage_type: DamageType,
) -> (i32, bool, bool, u64, Option<Entity>) {
    // Check if target is Immortal
    if world
        .query_one::<&crate::components::Immortal>(target)
        .is_ok_and(|mut q| q.get().is_some())
    {
        return (0, false, false, 0, None);
    }

    // Apply resistance
    let final_damage = if let Ok(mut res) = world.query_one::<&Resistance>(target) {
        if let Some(r) = res.get() {
            r.apply(damage, &damage_type)
        } else {
            damage
        }
    } else {
        damage
    };

    if final_damage <= 0 {
        return (0, false, false, 0, None);
    }

    // Process on_hit triggers on attacker and defender
    process_triggers(world, attacker, "on_hit");
    process_triggers(world, target, "on_hit");

    // Apply damage to target
    let (killed, unconscious) = {
        let mut q = match world.query_one::<&mut Health>(target) {
            Ok(q) => q,
            Err(_) => return (0, false, false, 0, None),
        };
        match q.get() {
            Some(hp) => {
                hp.damage(final_damage);
                let is_player = world
                    .query_one::<&crate::Player>(target)
                    .is_ok_and(|mut q| q.get().is_some());
                let killed = if is_player {
                    hp.is_truly_dead()
                } else {
                    hp.is_dead()
                };
                (killed, hp.is_unconscious())
            }
            None => return (0, false, false, 0, None),
        }
    };

    if world
        .query_one::<&crate::Player>(target)
        .is_ok_and(|mut q| q.get().is_some())
    {
        let _ = world.insert(target, (crate::Dirty,));
    }

    if killed {
        // Compute base XP before death
        let base_xp = world
            .query_one::<&Level>(target)
            .ok()
            .and_then(|mut q| q.get().copied())
            .map(|l| (l.0 as u64).saturating_pow(2) * 50)
            .unwrap_or(0);

        // Check if attacker is in a group and has members in the same room
        let mut group_members_in_room = Vec::new();
        let attacker_room = world
            .query_one::<&Position>(attacker)
            .ok()
            .and_then(|mut q| q.get().map(|p| p.room));

        if let Some(room) = attacker_room {
            if let Ok(mut q_gm) = world.query_one::<&crate::GroupMember>(attacker) {
                if let Some(gm) = q_gm.get() {
                    let group_entity = gm.group_id;
                    if let Ok(mut q_group) = world.query_one::<&crate::Group>(group_entity) {
                        if let Some(group) = q_group.get() {
                            for member_info in &group.members {
                                if let Some(m_ent) = member_info.entity {
                                    let m_room = world
                                        .query_one::<&Position>(m_ent)
                                        .ok()
                                        .and_then(|mut q| q.get().map(|p| p.room));
                                    if m_room == Some(room) {
                                        group_members_in_room.push(m_ent);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        let mut xp_gained = 0;

        if group_members_in_room.len() > 1 {
            let n = group_members_in_room.len();
            let group_bonus = 1.0 + 0.1 * (n as f32).min(5.0);
            let total_xp = ((base_xp as f32) * group_bonus) as u64;
            let xp_share = total_xp / (n as u64);

            for member in group_members_in_room {
                let penalty_mult = world
                    .query_one::<&crate::components::MultiClassInfo>(member)
                    .ok()
                    .and_then(|mut q| q.get().map(|mc| mc.xp_penalty_multiplier()))
                    .unwrap_or(1.0);
                let member_xp = ((xp_share as f32) * penalty_mult).round() as u64;

                let mut xp_updated = false;
                if let Ok(mut q) = world.query_one::<&mut crate::Experience>(member) {
                    if let Some(xp) = q.get() {
                        xp.0 = xp.0.saturating_add(member_xp);
                        xp_updated = true;
                    }
                }
                if xp_updated {
                    let _ = world.insert(member, (crate::Dirty,));
                }

                if member == attacker {
                    xp_gained = member_xp;
                }
            }
        } else {
            // Solo XP
            let penalty_mult = world
                .query_one::<&crate::components::MultiClassInfo>(attacker)
                .ok()
                .and_then(|mut q| q.get().map(|mc| mc.xp_penalty_multiplier()))
                .unwrap_or(1.0);
            xp_gained = ((base_xp as f32) * penalty_mult).round() as u64;

            let mut xp_updated = false;
            if let Ok(mut q) = world.query_one::<&mut crate::Experience>(attacker) {
                if let Some(xp) = q.get() {
                    xp.0 = xp.0.saturating_add(xp_gained);
                    xp_updated = true;
                }
            }
            if xp_updated {
                let _ = world.insert(attacker, (crate::Dirty,));
            }
        }

        let corpse = handle_death(world, target);

        (final_damage, true, false, xp_gained, Some(corpse))
    } else if unconscious {
        handle_combatant_down(world, target);
        // Set player state to unconscious if they are a player
        if world
            .query_one::<&crate::Player>(target)
            .is_ok_and(|mut q| q.get().is_some())
        {
            let _ = crate::systems::player_state::try_transition_player_state(
                world,
                target,
                crate::systems::player_state::PlayerStateTrigger::Knockout,
            );
        }
        (final_damage, false, true, 0, None)
    } else {
        // Auto-engage non-friendly NPCs that aren't already in combat
        let is_npc = world
            .query_one::<&Npc>(target)
            .is_ok_and(|mut q| q.get().is_some());
        let is_friendly = world
            .query_one::<&Friendly>(target)
            .is_ok_and(|mut q| q.get().is_some());
        if is_npc && !is_friendly {
            let current_state = world
                .query_one::<&CombatState>(target)
                .ok()
                .and_then(|mut q| q.get().cloned())
                .unwrap_or(CombatState::NotInCombat);
            if current_state == CombatState::NotInCombat {
                let stance = crate::systems::stance::get_active_stance(world, target);
                transition_combat_state(
                    world,
                    target,
                    CombatState::Engaged {
                        target: attacker,
                        round_started: std::time::Instant::now(),
                        stance,
                    },
                );
            }
        }

        (final_damage, false, false, 0, None)
    }
}

pub fn handle_combatant_down(world: &mut World, victim: Entity) {
    let room = match world
        .query_one::<&Position>(victim)
        .ok()
        .and_then(|mut q| q.get().map(|p| p.room))
    {
        Some(r) => r,
        None => return,
    };

    let mut attackers = Vec::new();
    {
        let mut q = world.query::<(&CombatState, &Position)>();
        for (e, (state, pos)) in q.iter() {
            let entity = Entity::from(e);
            if pos.room == room && state.target() == Some(victim) {
                attackers.push(entity);
            }
        }
    }

    for attacker in attackers {
        let mut next_target = None;
        {
            let mut q = world.query::<(&CombatState, &Position, &Health)>();
            for (e, (state, pos, health)) in q.iter() {
                let potential_target = Entity::from(e);
                if pos.room == room
                    && potential_target != victim
                    && health.is_conscious()
                    && state.target() == Some(attacker)
                {
                    next_target = Some(potential_target);
                    break;
                }
            }
        }

        if let Some(target) = next_target {
            let stance = crate::systems::stance::get_active_stance(world, attacker);
            transition_combat_state(
                world,
                attacker,
                CombatState::Engaged {
                    target,
                    round_started: std::time::Instant::now(),
                    stance,
                },
            );
        } else {
            transition_combat_state(world, attacker, CombatState::NotInCombat);
        }
    }

    transition_combat_state(world, victim, CombatState::NotInCombat);
}

pub fn handle_death(world: &mut World, victim: Entity) -> Entity {
    let corpse = spawn_corpse(world, victim, None);

    handle_combatant_down(world, victim);

    let is_player = world
        .query_one::<&crate::Player>(victim)
        .is_ok_and(|mut q| q.get().is_some());

    if is_player {
        if let Ok(mut q) = world.query_one::<&mut Inventory>(victim) {
            if let Some(inv) = q.get() {
                inv.0.clear();
            }
        }

        if let Ok(mut q) = world.query_one::<&mut Equipment>(victim) {
            if let Some(eq) = q.get() {
                eq.slots.clear();
            }
        }

        let level = world
            .query_one::<&Level>(victim)
            .ok()
            .and_then(|mut q| q.get().copied())
            .unwrap_or(Level(1))
            .0;

        if let Ok(mut q) = world.query_one::<&mut crate::Experience>(victim) {
            if let Some(xp) = q.get() {
                let current_xp = xp.0;
                let min_level = if level > 5 { level - 5 } else { 1 };
                let base_xp_current = (level as u64).pow(3) * 100;
                let base_xp_min = (min_level as u64).pow(3) * 100;
                let max_loss = base_xp_current.saturating_sub(base_xp_min);

                let loss = ((current_xp as f64) * 0.10).round() as u64;
                let final_loss = loss.min(max_loss);

                xp.0 = current_xp.saturating_sub(final_loss);
            }
        }

        if let Ok(mut q) = world.query_one::<&mut Health>(victim) {
            if let Some(hp) = q.get() {
                hp.current = 1;
            }
        }

        let _ = crate::systems::player_state::try_transition_player_state(
            world,
            victim,
            crate::systems::player_state::PlayerStateTrigger::Die,
        );
        let _ = world.remove_one::<LastMessenger>(victim);

        let dest = world
            .query_one::<&RecallRoom>(victim)
            .ok()
            .and_then(|mut q| q.get().map(|r| r.0))
            .unwrap_or_else(|| {
                world
                    .query_one::<&Position>(victim)
                    .ok()
                    .and_then(|mut q| q.get().map(|p| p.room))
                    .unwrap()
            });

        let _ = world.insert(victim, (Position::new(dest), crate::Dirty));
    } else {
        // Despawn equipped item entities — they don't go on the corpse
        let equipped_items: Vec<Entity> = world
            .query_one::<&Equipment>(victim)
            .ok()
            .and_then(|mut q| q.get().map(|eq| eq.slots.iter().map(|(_, e)| *e).collect()))
            .unwrap_or_default();
        for entity in equipped_items {
            let _ = world.despawn(entity);
        }
        let _ = world.despawn(victim);
    }

    corpse
}

pub fn spawn_corpse(world: &mut World, victim: Entity, _killer: Option<Entity>) -> Entity {
    let is_player = world
        .query_one::<&crate::Player>(victim)
        .is_ok_and(|mut q| q.get().is_some());

    let inv = world
        .query_one::<&Inventory>(victim)
        .ok()
        .and_then(|mut q| q.get().cloned())
        .unwrap_or_default();

    let eq = if is_player {
        world
            .query_one::<&Equipment>(victim)
            .ok()
            .and_then(|mut q| q.get().cloned())
            .unwrap_or_default()
    } else {
        Equipment::new()
    };

    let pos = match world
        .query_one::<&Position>(victim)
        .ok()
        .and_then(|mut q| q.get().map(|p| p.room))
    {
        Some(r) => r,
        None => panic!("spawn_corpse called on victim with no Position"),
    };

    let victim_name = world
        .query_one::<&Name>(victim)
        .ok()
        .and_then(|mut q| q.get().map(|n| n.to_string()))
        .unwrap_or_else(|| "Someone".to_owned());

    let corpse_name = Name::new(format!("Corpse of {victim_name}"));

    let (owner, owner_db_id, decay_secs, lootable_by) = if is_player {
        let db_id = world
            .query_one::<&crate::components::DbId>(victim)
            .ok()
            .and_then(|mut q| q.get().map(|d| d.0));
        (Some(victim), db_id, 1800, LootRule::OwnerOnly)
    } else {
        (None, None, 300, LootRule::Public)
    };

    world.spawn((
        Corpse {
            owner,
            owner_db_id,
            created_at: std::time::Instant::now(),
            decay_secs,
            lootable_by,
        },
        corpse_name,
        inv,
        eq,
        Position::new(pos),
    ))
}

/// Grant XP to the attacker based on victim level.
pub fn grant_xp(world: &mut World, attacker: Entity, victim: Entity) {
    let victim_level = world
        .query_one::<&Level>(victim)
        .ok()
        .and_then(|mut q| q.get().copied())
        .unwrap_or(Level(1))
        .0 as u64;

    let mut xp_gain = victim_level.saturating_pow(2) * 50;

    let penalty_mult = world
        .query_one::<&crate::components::MultiClassInfo>(attacker)
        .ok()
        .and_then(|mut q| q.get().map(|mc| mc.xp_penalty_multiplier()))
        .unwrap_or(1.0);
    xp_gain = ((xp_gain as f32) * penalty_mult).round() as u64;

    if let Ok(mut q) = world.query_one::<&mut crate::Experience>(attacker) {
        if let Some(xp) = q.get() {
            xp.0 = xp.0.saturating_add(xp_gain);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Attributes, Experience, Name, PlayerState};

    fn setup_world() -> (World, Entity, Entity) {
        let mut world = World::new();
        let room = world.spawn(());
        let attacker = world.spawn((
            Position::new(room),
            Health::new(100),
            Attributes::default(),
            Level(5),
            CombatState::Engaged {
                target: Entity::from(hecs::Entity::from_bits(0x0000_0001_0000_0002).unwrap()),
                round_started: std::time::Instant::now(),
                stance: None,
            },
            Experience::default(),
            Name::new("Attacker"),
        ));
        let target = world.spawn((
            Position::new(room),
            Health::new(50),
            Attributes::default(),
            Level(3),
            Name::new("Target"),
        ));
        // Fix CombatState to point to actual target
        world
            .insert(
                attacker,
                (CombatState::Engaged {
                    target,
                    round_started: std::time::Instant::now(),
                    stance: None,
                },),
            )
            .unwrap();
        (world, attacker, target)
    }

    #[test]
    fn test_calculate_ac() {
        let mut world = World::new();
        let room = world.spawn(());
        let entity = world.spawn((
            Position::new(room),
            Health::new(100),
            Attributes::default(),
            Level(1),
        ));
        let ac = calculate_ac(&world, entity);
        // 10 + 1 (level) + 0 (dex 10) + 0 (armor) = 11
        assert_eq!(ac, 11);
    }

    #[test]
    fn test_calculate_ac_with_armor() {
        let mut world = World::new();
        let room = world.spawn(());
        let entity = world.spawn((
            Position::new(room),
            Health::new(100),
            Attributes::new(10, 14, 10, 10, 10, 10),
            Level(5),
            Armor { base: 10, bonus: 2 },
        ));
        let ac = calculate_ac(&world, entity);
        // 10 + 5 (level) + 2 (dex 14) + 12 (armor) = 29
        assert_eq!(ac, 29);
    }

    #[test]
    fn test_ability_mod() {
        assert_eq!(ability_mod(10), 0);
        assert_eq!(ability_mod(14), 2);
        assert_eq!(ability_mod(18), 4);
        assert_eq!(ability_mod(8), -1);
        assert_eq!(ability_mod(3), -3);
    }

    #[test]
    fn test_combat_pulse_no_crash() {
        let (mut world, _attacker, _target) = setup_world();
        // Just run the pulse — should not crash
        run_combat_pulse(&mut world);
    }

    #[test]
    fn test_spawn_corpse_transfers_inventory() {
        let mut world = World::new();
        let room = world.spawn(());
        let item = world.spawn(());
        let victim = world.spawn((
            Position::new(room),
            Health::new(1),
            Inventory(vec![item]),
            Equipment::new(),
            crate::Player::new(1),
            crate::components::DbId(42),
        ));
        spawn_corpse(&mut world, victim, None);
        // Corpse should exist in the same room
        let mut q = world.query::<(&Corpse, &Position)>();
        let mut iter = q.iter();
        let (_, (corpse, pos)) = iter.next().unwrap();
        assert_eq!(pos.room, room);
        assert_eq!(corpse.owner_db_id, Some(42));
        assert_eq!(corpse.owner, Some(victim));
    }

    #[test]
    fn test_flee_attempt() {
        let mut world = World::new();
        let room = world.spawn(());
        let attacker = world.spawn((
            Position::new(room),
            Health::new(100),
            Attributes::new(10, 18, 10, 10, 10, 10),
            Level(5),
            CombatState::Fleeing {
                target: Entity::from(hecs::Entity::from_bits(0x0000_0001_0000_0002).unwrap()),
                attempts: 0,
            },
            Name::new("Attacker"),
        ));
        let target = world.spawn((
            Position::new(room),
            Health::new(50),
            Attributes::default(),
            Level(3),
            Name::new("Target"),
        ));
        // Update Fleeing state to point to actual target
        world
            .insert(
                attacker,
                (CombatState::Fleeing {
                    target,
                    attempts: 0,
                },),
            )
            .unwrap();

        // Run combat pulse
        let outcomes = run_combat_pulse(&mut world);

        let state = world
            .query_one::<&CombatState>(attacker)
            .unwrap()
            .get()
            .cloned()
            .unwrap();

        assert!(matches!(
            state,
            CombatState::NotInCombat | CombatState::Engaged { .. }
        ));
        assert!(!outcomes.is_empty());
    }

    #[test]
    fn test_mob_fight_back() {
        let mut world = World::new();
        let room = world.spawn(());

        let player = world.spawn((
            Position::new(room),
            Health::new(100),
            Attributes::default(),
            Level(5),
            CombatState::NotInCombat,
            crate::Player::new(1),
            Name::new("Player"),
        ));

        let mob = world.spawn((
            Position::new(room),
            Health::new(100),
            Attributes::default(),
            Level(5),
            CombatState::NotInCombat,
            crate::Npc::new("goblin"),
            Name::new("Goblin"),
        ));

        let player_stance = crate::systems::stance::get_active_stance(&world, player);
        transition_combat_state(
            &mut world,
            player,
            CombatState::Engaged {
                target: mob,
                round_started: std::time::Instant::now(),
                stance: player_stance,
            },
        );

        // First pulse: player attacks, mob auto-engages via apply_damage
        let _outcomes = run_combat_pulse(&mut world);

        // The mob should have been auto-engaged by apply_damage
        let mob_state = world
            .query_one::<&CombatState>(mob)
            .unwrap()
            .get()
            .cloned()
            .unwrap();
        assert!(matches!(mob_state, CombatState::Engaged { target, .. } if target == player));

        // Second pulse: mob attacks back
        let outcomes = run_combat_pulse(&mut world);
        let mob_attacked = outcomes
            .iter()
            .any(|o| o.attacker == mob && o.target == player);
        assert!(mob_attacked, "Mob did not attack the player!");
    }

    #[test]
    fn test_combat_target_switching() {
        let mut world = World::new();
        let room = world.spawn(());

        let player_a = world.spawn((
            Position::new(room),
            Health::new(100),
            Attributes::default(),
            Level(5),
            CombatState::NotInCombat,
            crate::Player::new(1),
            Name::new("PlayerA"),
        ));

        let player_b = world.spawn((
            Position::new(room),
            Health::new(100),
            Attributes::default(),
            Level(5),
            CombatState::NotInCombat,
            crate::Player::new(2),
            Name::new("PlayerB"),
        ));

        let mob = world.spawn((
            Position::new(room),
            Health::new(100),
            Attributes::default(),
            Level(5),
            CombatState::NotInCombat,
            crate::Npc::new("goblin"),
            Name::new("Goblin"),
        ));

        // Mob targets Player A. Player A targets Mob. Player B targets Mob.
        transition_combat_state(
            &mut world,
            mob,
            CombatState::Engaged {
                target: player_a,
                round_started: std::time::Instant::now(),
                stance: None,
            },
        );
        transition_combat_state(
            &mut world,
            player_a,
            CombatState::Engaged {
                target: mob,
                round_started: std::time::Instant::now(),
                stance: None,
            },
        );
        transition_combat_state(
            &mut world,
            player_b,
            CombatState::Engaged {
                target: mob,
                round_started: std::time::Instant::now(),
                stance: None,
            },
        );

        // Player A goes down
        handle_combatant_down(&mut world, player_a);

        // Player A should be NotInCombat
        let state_a = world
            .query_one::<&CombatState>(player_a)
            .unwrap()
            .get()
            .cloned()
            .unwrap();
        assert_eq!(state_a, CombatState::NotInCombat);

        // Mob should now target Player B
        let mob_state = world
            .query_one::<&CombatState>(mob)
            .unwrap()
            .get()
            .cloned()
            .unwrap();
        assert!(matches!(mob_state, CombatState::Engaged { target, .. } if target == player_b));
    }

    #[test]
    fn test_player_death_loop() {
        let mut world = World::new();
        let spawn_room = world.spawn(());
        let recall_room = world.spawn(());

        let item = world.spawn(());
        let messenger = world.spawn(());
        let player = world.spawn((
            Position::new(spawn_room),
            Health::new(100),
            Attributes::default(),
            Level(5),
            CombatState::NotInCombat,
            crate::Player::new(1),
            Name::new("Player"),
            Inventory(vec![item]),
            Equipment::new(),
            RecallRoom(recall_room),
            LastMessenger(messenger),
        ));

        // Player dies
        let corpse = handle_death(&mut world, player);

        // Corpse checks
        let corpse_comp = world
            .query_one::<&Corpse>(corpse)
            .unwrap()
            .get()
            .cloned()
            .unwrap();
        assert_eq!(corpse_comp.owner, Some(player));
        assert_eq!(corpse_comp.decay_secs, 1800);
        assert_eq!(corpse_comp.lootable_by, LootRule::OwnerOnly);

        // Player checks
        let hp = world
            .query_one::<&Health>(player)
            .unwrap()
            .get()
            .cloned()
            .unwrap();
        assert_eq!(hp.current, 1);

        let pos = world
            .query_one::<&Position>(player)
            .unwrap()
            .get()
            .cloned()
            .unwrap();
        assert_eq!(pos.room, recall_room);

        let state = world
            .query_one::<&PlayerState>(player)
            .unwrap()
            .get()
            .cloned()
            .unwrap();
        assert!(matches!(state, PlayerState::Dead));

        assert!(world
            .query_one::<&LastMessenger>(player)
            .unwrap()
            .get()
            .is_none());
    }

    #[test]
    fn test_npc_death_at_zero_hp() {
        let mut world = World::new();
        let room = world.spawn(());

        let player = world.spawn((
            Position::new(room),
            Health::new(100),
            Attributes::default(),
            Level(1),
            CombatState::NotInCombat,
            crate::Player::new(1),
            Name::new("Player"),
            crate::Experience(0),
        ));

        let npc = world.spawn((
            Position::new(room),
            Health::new(10),
            Attributes::default(),
            Level(1),
            CombatState::NotInCombat,
            crate::Npc::new("river_rat"),
            Name::new("River Rat"),
        ));

        // Apply 10 damage to NPC, reducing health to 0
        let (final_damage, killed, unconscious, xp_gained, corpse) =
            apply_damage(&mut world, player, npc, 10, DamageType::Bludgeon);

        assert!(!unconscious);

        assert_eq!(final_damage, 10);
        assert!(killed);
        assert!(xp_gained > 0);
        assert!(corpse.is_some());

        // Verify NPC is despawned
        assert!(world.query_one::<&Health>(npc).is_err());

        // Verify player gained XP
        let xp = world
            .query_one::<&crate::Experience>(player)
            .unwrap()
            .get()
            .unwrap()
            .0;
        assert_eq!(xp, xp_gained);
    }

    #[test]
    fn test_effective_speed() {
        use crate::components::{Weapon, WeaponHands};
        use crate::dice::DiceRoll;
        let one_handed_wep = Weapon {
            damage_dice: DiceRoll::new(1, 6, 0),
            damage_type: DamageType::Slash,
            speed: 2.0,
            range: crate::components::WeaponRange::Melee,
            hands: WeaponHands::OneHand,
        };
        assert_eq!(one_handed_wep.effective_speed(), 2.0);

        let two_handed_wep = Weapon {
            damage_dice: DiceRoll::new(2, 6, 0),
            damage_type: DamageType::Slash,
            speed: 2.0,
            range: crate::components::WeaponRange::Melee,
            hands: WeaponHands::TwoHand,
        };
        assert_eq!(two_handed_wep.effective_speed(), 2.4); // 2.0 * 1.2
    }

    #[test]
    fn test_ambidexterity_mitigation() {
        // Check dual wield penalty with vs without Ambidexterity
        let penalty_with_main = {
            let has_ambidexterity = true;
            let base_penalty = dual_wield_penalty(false); // main hand
            if has_ambidexterity {
                base_penalty / 2
            } else {
                base_penalty
            }
        };
        let penalty_without_main = {
            let has_ambidexterity = false;
            let base_penalty = dual_wield_penalty(false); // main hand
            if has_ambidexterity {
                base_penalty / 2
            } else {
                base_penalty
            }
        };
        assert_eq!(penalty_with_main, -1);
        assert_eq!(penalty_without_main, -2);

        let penalty_with_off = {
            let has_ambidexterity = true;
            let base_penalty = dual_wield_penalty(true); // off hand
            if has_ambidexterity {
                base_penalty / 2
            } else {
                base_penalty
            }
        };
        let penalty_without_off = {
            let has_ambidexterity = false;
            let base_penalty = dual_wield_penalty(true); // off hand
            if has_ambidexterity {
                base_penalty / 2
            } else {
                base_penalty
            }
        };
        assert_eq!(penalty_with_off, -2);
        assert_eq!(penalty_without_off, -4);
    }

    #[test]
    fn test_combat_gated_by_player_state() {
        let (mut world, attacker, target) = setup_world();

        // Spawn both with default standing states
        world
            .insert(
                attacker,
                (PlayerState::Resting(crate::RestState::Standing),),
            )
            .unwrap();
        world
            .insert(target, (PlayerState::Resting(crate::RestState::Standing),))
            .unwrap();

        // 1. Stun the attacker
        world
            .insert(attacker, (PlayerState::Stunned { remaining_ms: 1000 },))
            .unwrap();

        // Run combat pulse — should skip attack because attacker is stunned
        let outcomes = run_combat_pulse(&mut world);
        assert!(outcomes.is_empty());

        // 2. Remove stun and put target to sleep
        world
            .insert(
                attacker,
                (PlayerState::Resting(crate::RestState::Standing),),
            )
            .unwrap();
        world
            .insert(target, (PlayerState::Resting(crate::RestState::Sleeping),))
            .unwrap();

        // Run combat pulse — attacker attacks target. Target should automatically stand up
        let outcomes = run_combat_pulse(&mut world);
        assert!(!outcomes.is_empty());

        let target_state = world
            .query_one::<&PlayerState>(target)
            .unwrap()
            .get()
            .cloned()
            .unwrap();
        assert_eq!(
            target_state,
            PlayerState::Resting(crate::RestState::Standing)
        );
    }
}
