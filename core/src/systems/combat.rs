use crate::dice::DiceRoll;
use crate::{
    Armor, Attributes, CombatState, Corpse, DamageType, Entity, Equipment, EquipmentSlot, Health,
    Inventory, Level, LootRule, Name, Player, Position, Resistance, RoomExits, Weapon, WeaponHands,
    World,
};

// ---------------------------------------------------------------------------
// Combat outcome types — consumed by the server layer to send messages
// ---------------------------------------------------------------------------

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
    },
    Miss,
    Killed {
        xp_gained: u64,
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

    let dex = world
        .query_one::<&Attributes>(entity)
        .ok()
        .and_then(|mut q| q.get().cloned())
        .unwrap_or_default()
        .dexterity;

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

    10 + level.0 as i32 + ability_mod(dex) + armor.total() + shield_bonus
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
    world: &World,
    attacker: Entity,
    _target: Entity,
    is_offhand: bool,
) -> (i32, DamageType) {
    let str = world
        .query_one::<&Attributes>(attacker)
        .ok()
        .and_then(|mut q| q.get().cloned())
        .unwrap_or_default()
        .strength;

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

    if let Some((_, wep)) = weapon_damage {
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
    }
}

/// Calculate to-hit value for attacker vs target. Returns true if hit.
/// `is_offhand` should be true for the offhand weapon when dual-wielding.
pub fn calculate_hit(world: &World, attacker: Entity, target: Entity, is_offhand: bool) -> bool {
    let atk_level = world
        .query_one::<&Level>(attacker)
        .ok()
        .and_then(|mut q| q.get().copied())
        .unwrap_or(Level(1))
        .0 as i32;

    let str = world
        .query_one::<&Attributes>(attacker)
        .ok()
        .and_then(|mut q| q.get().cloned())
        .unwrap_or_default()
        .strength;

    let ac = calculate_ac(world, target);

    let dw_penalty = if is_dual_wielding(world, attacker) {
        dual_wield_penalty(is_offhand)
    } else {
        0
    };

    // d20 + level + str_mod + dw_penalty >= AC
    let roll = fastrand::i32(1..=20);
    if roll == 1 {
        return false; // auto-miss
    }
    if roll == 20 {
        return true; // auto-crit
    }

    roll + atk_level + ability_mod(str) + dw_penalty >= ac
}

pub fn transition_combat_state(world: &mut World, entity: Entity, new_state: CombatState) {
    let old_state = world
        .query_one::<&CombatState>(entity)
        .ok()
        .and_then(|mut q| q.get().cloned())
        .unwrap_or(CombatState::NotInCombat);

    if old_state != new_state {
        let _ = world.insert(entity, (new_state.clone(),));

        let _event = crate::GameEvent::CombatStateChanged {
            entity,
            from: old_state,
            to: new_state,
        };
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
        // Verify attacker still exists and is alive
        let attacker_alive = world
            .query_one::<&Health>(attacker)
            .ok()
            .and_then(|mut q| q.get().map(|h| h.is_alive()))
            .unwrap_or(false);

        if !attacker_alive {
            continue;
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

                // Check if target is alive
                let target_dead = world
                    .query_one::<&Health>(target)
                    .ok()
                    .and_then(|mut q| q.get().map(|h| h.is_dead()))
                    .unwrap_or(true);

                if target_dead {
                    transition_combat_state(world, attacker, CombatState::NotInCombat);
                    continue;
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

                // Main hand attack
                let is_hit = calculate_hit(world, attacker, target, false);
                if is_hit {
                    let (damage, damage_type) = calculate_damage(world, attacker, target, false);
                    let (final_damage, killed, xp_gained) =
                        apply_damage(world, attacker, target, damage, damage_type);
                    if final_damage > 0 {
                        let kind = if killed {
                            CombatOutcomeKind::Killed { xp_gained }
                        } else {
                            CombatOutcomeKind::Hit {
                                damage: final_damage,
                                damage_type,
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
                } else {
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

                // Offhand attack if dual-wielding
                if is_dual_wielding(world, attacker) {
                    let oh_hit = calculate_hit(world, attacker, target, true);
                    if oh_hit {
                        let (oh_dmg, oh_type) = calculate_damage(world, attacker, target, true);
                        let (final_damage, killed, xp_gained) =
                            apply_damage(world, attacker, target, oh_dmg, oh_type);
                        if final_damage > 0 {
                            let kind = if killed {
                                CombatOutcomeKind::Killed { xp_gained }
                            } else {
                                CombatOutcomeKind::Hit {
                                    damage: final_damage,
                                    damage_type: oh_type,
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
                    } else {
                        outcomes.push(CombatOutcome {
                            attacker,
                            target,
                            room,
                            attacker_name,
                            target_name,
                            attacker_is_player,
                            target_is_player,
                            kind: CombatOutcomeKind::Miss,
                        });
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
                let dex = world
                    .query_one::<&Attributes>(attacker)
                    .ok()
                    .and_then(|mut q| q.get().map(|a| a.dexterity))
                    .unwrap_or(10);
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
/// Returns `(final_damage, killed, xp_gained)`.
/// On kill: spawns a corpse, grants XP to attacker, despawns the target.
fn apply_damage(
    world: &mut World,
    attacker: Entity,
    target: Entity,
    damage: i32,
    damage_type: DamageType,
) -> (i32, bool, u64) {
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
        return (0, false, 0);
    }

    // Apply damage to target
    let killed = {
        let mut q = match world.query_one::<&mut Health>(target) {
            Ok(q) => q,
            Err(_) => return (0, false, 0),
        };
        match q.get() {
            Some(hp) => {
                let killed = hp.current <= final_damage;
                hp.damage(final_damage);
                killed
            }
            None => return (0, false, 0),
        }
    };

    if world.query_one::<&crate::Player>(target).is_ok() {
        let _ = world.insert(target, (crate::Dirty,));
    }

    if killed {
        // Compute XP before despawning the target
        let xp_gained = world
            .query_one::<&Level>(target)
            .ok()
            .and_then(|mut q| q.get().copied())
            .map(|l| (l.0 as u64).saturating_pow(2) * 50)
            .unwrap_or(0);

        spawn_corpse(world, target, None);

        // Grant XP to attacker
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

        transition_combat_state(world, attacker, CombatState::NotInCombat);
        // Despawn the victim — the corpse entity replaces it in the room
        let _ = world.despawn(target);

        (final_damage, true, xp_gained)
    } else {
        (final_damage, false, 0)
    }
}

/// Spawn a corpse entity with the victim's inventory + equipment.
pub fn spawn_corpse(world: &mut World, victim: Entity, _killer: Option<Entity>) {
    let inv = world
        .query_one::<&Inventory>(victim)
        .ok()
        .and_then(|mut q| q.get().cloned())
        .unwrap_or_default();

    let eq = world
        .query_one::<&Equipment>(victim)
        .ok()
        .and_then(|mut q| q.get().cloned())
        .unwrap_or_default();

    let pos = match world
        .query_one::<&Position>(victim)
        .ok()
        .and_then(|mut q| q.get().map(|p| p.room))
    {
        Some(r) => r,
        None => return,
    };

    let victim_name = world
        .query_one::<&Name>(victim)
        .ok()
        .and_then(|mut q| q.get().map(|n| n.to_string()))
        .unwrap_or_else(|| "Someone".to_owned());

    let corpse_name = Name::new(format!("Corpse of {victim_name}"));

    let _corpse = world.spawn((
        Corpse {
            owner: None,
            created_at: std::time::Instant::now(),
            decay_secs: 300,
            lootable_by: LootRule::Public,
        },
        corpse_name,
        inv,
        eq,
        Position::new(pos),
    ));
}

/// Grant XP to the attacker based on victim level.
pub fn grant_xp(world: &mut World, attacker: Entity, victim: Entity) {
    let victim_level = world
        .query_one::<&Level>(victim)
        .ok()
        .and_then(|mut q| q.get().copied())
        .unwrap_or(Level(1))
        .0 as u64;

    let xp_gain = victim_level.saturating_pow(2) * 50;

    if let Ok(mut q) = world.query_one::<&mut crate::Experience>(attacker) {
        if let Some(xp) = q.get() {
            xp.0 = xp.0.saturating_add(xp_gain);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Experience, Name};

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
        ));
        spawn_corpse(&mut world, victim, None);
        // Corpse should exist in the same room
        let corpse_count = world
            .query::<(&Corpse, &Position)>()
            .iter()
            .filter(|(_, (_, pos))| pos.room == room)
            .count();
        assert_eq!(corpse_count, 1);
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

        if world
            .query_one::<&crate::Npc>(mob)
            .is_ok_and(|mut q| q.get().is_some())
            && !world
                .query_one::<&crate::Friendly>(mob)
                .is_ok_and(|mut q| q.get().is_some())
        {
            let target_stance = crate::systems::stance::get_active_stance(&world, mob);
            transition_combat_state(
                &mut world,
                mob,
                CombatState::Engaged {
                    target: player,
                    round_started: std::time::Instant::now(),
                    stance: target_stance,
                },
            );
        }

        let player_state = world
            .query_one::<&CombatState>(player)
            .unwrap()
            .get()
            .cloned()
            .unwrap();
        let mob_state = world
            .query_one::<&CombatState>(mob)
            .unwrap()
            .get()
            .cloned()
            .unwrap();
        assert!(matches!(player_state, CombatState::Engaged { target, .. } if target == mob));
        assert!(matches!(mob_state, CombatState::Engaged { target, .. } if target == player));

        let outcomes = run_combat_pulse(&mut world);

        let mob_attacked = outcomes
            .iter()
            .any(|o| o.attacker == mob && o.target == player);
        assert!(mob_attacked, "Mob did not attack the player!");
    }
}
