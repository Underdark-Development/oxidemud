use crate::{
    regen::{rest_multiplier, PoolRegen},
    Attributes, Energy, Health, Mana, PlayerState, Psi, Stamina, World,
};

/// Run one regen pulse for all entities with Health and/or resource pools.
/// Regen rate is modified by the entity's rest state.
pub fn run_regen_pulse(world: &mut World) {
    // HP regen
    let hp_targets: Vec<(crate::Entity, i32)> = {
        let mut q = world.query::<(&Health, &Attributes)>();
        q.iter()
            .map(|(raw, (hp, attr))| {
                let entity = crate::Entity::from(raw);
                let rest_mult = world
                    .query_one::<&PlayerState>(entity)
                    .ok()
                    .and_then(|mut q| q.get().map(|ps| rest_multiplier(&ps.rest())))
                    .unwrap_or(1.0);
                let amount = hp.regen_amount(attr.constitution, rest_mult);
                (entity, amount)
            })
            .collect()
    };

    for (entity, amount) in hp_targets {
        if amount > 0 {
            let mut healed = false;
            if let Ok(mut q) = world.query_one::<&mut Health>(entity) {
                if let Some(hp) = q.get() {
                    hp.heal(amount);
                    healed = true;
                }
            }
            if healed && world.query_one::<&crate::Player>(entity).is_ok() {
                let _ = world.insert(entity, (crate::Dirty,));
            }
        }
    }

    // Resource pool regen — use a macro to avoid repeating the pattern
    regen_pool::<Stamina>(world);
    regen_pool::<Mana>(world);
    regen_pool::<Energy>(world);
    regen_pool::<Psi>(world);
}

fn regen_pool<T: PoolRegen + Send + Sync + 'static>(world: &mut World) {
    let targets: Vec<(crate::Entity, u16)> = {
        let mut q = world.query::<&T>();
        q.iter()
            .map(|(raw, pool)| {
                let entity = crate::Entity::from(raw);
                let rest_mult = world
                    .query_one::<&PlayerState>(entity)
                    .ok()
                    .and_then(|mut q| q.get().map(|ps| rest_multiplier(&ps.rest())))
                    .unwrap_or(1.0);
                let amount = pool.regen_amount(rest_mult);
                (entity, amount)
            })
            .collect()
    };

    for (entity, amount) in targets {
        if amount > 0 {
            if let Ok(mut q) = world.query_one::<&mut T>(entity) {
                if let Some(pool) = q.get() {
                    let new = pool.current().saturating_add(amount).min(pool.max());
                    pool.set_current(new);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Health, Name, Position};

    fn setup_world() -> (World, crate::Entity) {
        let mut world = World::new();
        let room = world.spawn(());
        let player = world.spawn((
            Position::new(room),
            Health {
                current: 10,
                max: 100,
            },
            Attributes::default(),
            PlayerState::default(),
            Name::new("Test"),
            Stamina::new(100),
            Mana::new(100),
        ));
        // Set stamina/mana to non-full values
        world
            .insert(
                player,
                (
                    Stamina {
                        current: 30,
                        max: 100,
                    },
                    Mana {
                        current: 20,
                        max: 100,
                    },
                ),
            )
            .unwrap();
        (world, player)
    }

    #[test]
    fn test_regen_pulse_heals_hp() {
        let (mut world, player) = setup_world();
        run_regen_pulse(&mut world);
        let hp = world
            .query_one::<&Health>(player)
            .unwrap()
            .get()
            .unwrap()
            .clone();
        assert!(hp.current > 10, "HP should regen: {}", hp.current);
    }

    #[test]
    fn test_regen_pulse_regen_pools() {
        let (mut world, player) = setup_world();
        run_regen_pulse(&mut world);
        let stam = world
            .query_one::<&Stamina>(player)
            .unwrap()
            .get()
            .copied()
            .unwrap();
        let mana = world
            .query_one::<&Mana>(player)
            .unwrap()
            .get()
            .copied()
            .unwrap();
        assert!(stam.current > 30, "Stamina should regen: {}", stam.current);
        assert!(mana.current > 20, "Mana should regen: {}", mana.current);
    }

    #[test]
    fn test_regen_no_overflow() {
        let mut world = World::new();
        let room = world.spawn(());
        let player = world.spawn((
            Position::new(room),
            Health {
                current: 99,
                max: 100,
            },
            Attributes::default(),
            PlayerState::default(),
            Name::new("Test"),
        ));
        run_regen_pulse(&mut world);
        let hp = world
            .query_one::<&Health>(player)
            .unwrap()
            .get()
            .unwrap()
            .clone();
        assert_eq!(hp.current, 100, "Should cap at max");
    }
}
