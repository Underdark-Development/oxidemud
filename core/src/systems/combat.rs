use crate::dice::DiceRoll;
use crate::{
    Armor, Attributes, CombatTarget, Corpse, DamageType, Entity, Equipment, EquipmentSlot, Health,
    Inventory, Level, LootRule, Position, Resistance, Weapon, WeaponHands, World,
};

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

/// Run one combat pulse for all entities with CombatTarget.
pub fn run_combat_pulse(world: &mut World) {
    let targets: Vec<(Entity, Entity, Entity)> = {
        let mut q = world.query::<(&CombatTarget, &Health, &Position)>();
        q.iter()
            .map(|(raw, (target, _health, pos))| {
                let attacker = crate::Entity::from(raw);
                (attacker, target.0, pos.room)
            })
            .collect()
    };

    for (attacker, target, room) in targets {
        // Verify target still exists
        if world.query_one::<&Health>(target).is_err() {
            // Target despawned — clear combat target
            let _ = world.remove_one::<CombatTarget>(attacker);
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
                let _ = world.remove_one::<CombatTarget>(attacker);
                continue;
            }
        };

        if target_room != room {
            continue;
        }

        // Check if target is alive
        let target_dead = world
            .query_one::<&Health>(target)
            .ok()
            .and_then(|mut q| q.get().map(|h| h.is_dead()))
            .unwrap_or(true);

        if target_dead {
            let _ = world.remove_one::<CombatTarget>(attacker);
            continue;
        }

        // Main hand attack
        let is_hit = calculate_hit(world, attacker, target, false);
        if is_hit {
            let (damage, damage_type) = calculate_damage(world, attacker, target, false);
            if apply_damage(world, attacker, target, damage, damage_type) {
                continue;
            }
        }

        // Offhand attack if dual-wielding
        if is_dual_wielding(world, attacker) {
            let oh_hit = calculate_hit(world, attacker, target, true);
            if oh_hit {
                let (oh_dmg, oh_type) = calculate_damage(world, attacker, target, true);
                apply_damage(world, attacker, target, oh_dmg, oh_type);
            }
        }
    }
}

/// Apply damage to target, handling resistance, death, corpse, and XP.
/// Returns true if the target died (caller should not continue attacking).
fn apply_damage(
    world: &mut World,
    attacker: Entity,
    target: Entity,
    damage: i32,
    damage_type: DamageType,
) -> bool {
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
        return false;
    }

    // Apply damage to target
    let killed = {
        let mut q = match world.query_one::<&mut Health>(target) {
            Ok(q) => q,
            Err(_) => return false,
        };
        match q.get() {
            Some(hp) => {
                let killed = hp.current <= final_damage;
                hp.damage(final_damage);
                killed
            }
            None => return false,
        }
    };

    if killed {
        spawn_corpse(world, target, None);
        grant_xp(world, attacker, target);
        let _ = world.remove_one::<CombatTarget>(attacker);
        let _ = world.remove_one::<CombatTarget>(target);
    }

    killed
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

    let _corpse = world.spawn((
        Corpse {
            owner: None,
            created_at: std::time::Instant::now(),
            decay_secs: 300, // 5 min default
            lootable_by: LootRule::Public,
        },
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
            CombatTarget(Entity::from(
                hecs::Entity::from_bits(0x0000_0001_0000_0002).unwrap(),
            )),
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
        // Fix CombatTarget to point to actual target
        world.insert(attacker, (CombatTarget(target),)).unwrap();
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
}
