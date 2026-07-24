use crate::{Entity, Equipment, EquipmentSlot, ItemSkillRequirement, LearnedSkills, World};

/// Run one skill gate pulse for all entities with Equipment + LearnedSkills.
/// Checks each equipped item for skill requirements; auto-unequips on violation.
pub fn run_skill_gate_pulse(world: &mut World) {
    let entities: Vec<Entity> = {
        let mut q = world.query::<(&Equipment, &LearnedSkills)>();
        q.iter().map(|(raw, _)| raw).collect()
    };

    for entity in entities {
        let skills = match world
            .query_one::<&LearnedSkills>(entity)
            .ok()
            .and_then(|mut q| q.get().cloned())
        {
            Some(s) => s,
            None => continue,
        };

        let mut to_unequip: Vec<(EquipmentSlot, Entity)> = Vec::new();

        if let Ok(mut q) = world.query_one::<&Equipment>(entity) {
            if let Some(eq) = q.get() {
                for (slot, item_entity) in &eq.slots {
                    if let Some(req) = world
                        .query_one::<&ItemSkillRequirement>(*item_entity)
                        .ok()
                        .and_then(|mut q| q.get().cloned())
                    {
                        if skills.rank(&req.id) < req.level {
                            to_unequip.push((*slot, *item_entity));
                        }
                    }
                }
            }
        }

        for (slot, item_entity) in to_unequip {
            if let Ok(mut eq) = world.query_one::<&mut Equipment>(entity) {
                if let Some(eq) = eq.get() {
                    if let Some(removed) = eq.unequip(&slot) {
                        if let Ok(mut inv) = world.query_one::<&mut crate::Inventory>(entity) {
                            if let Some(inv) = inv.get() {
                                inv.0.push(removed);
                            }
                        }
                        tracing::info!(
                            "skill_gate entity={entity:?}: auto-unequipped {item_entity:?} from {slot:?} (requires {} level >= {})",
                            req_id(&item_entity, world),
                            req_level(&item_entity, world),
                        );
                    }
                }
            }
        }
    }
}

fn req_id(item: &Entity, world: &World) -> String {
    world
        .query_one::<&ItemSkillRequirement>(*item)
        .ok()
        .and_then(|mut q| q.get().map(|r| r.id.clone()))
        .unwrap_or_default()
}

fn req_level(item: &Entity, world: &World) -> u16 {
    world
        .query_one::<&ItemSkillRequirement>(*item)
        .ok()
        .and_then(|mut q| q.get().map(|r| r.level))
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Inventory, Item, Name};

    fn test_entity_with_skills(world: &mut World, skill_id: &str, skill_rank: u16) -> Entity {
        let entity = world.spawn((
            Equipment::new(),
            LearnedSkills::new(),
            Inventory::new(),
            Name::new("Test Entity"),
        ));
        if skill_rank > 0 {
            let mut skills = LearnedSkills::new();
            skills.set_rank(skill_id, skill_rank);
            let _ = world.insert(entity, (skills,));
        }
        entity
    }

    fn spawn_skill_required_item(world: &mut World, req_id: &str, req_level: u16) -> Entity {
        world.spawn((
            Item::new("test_item"),
            Name::new("Test Item"),
            ItemSkillRequirement {
                id: req_id.to_string(),
                level: req_level,
            },
        ))
    }

    #[test]
    fn test_skip_when_no_skill_requirement() {
        let mut world = World::new();
        let entity = test_entity_with_skills(&mut world, "test_skill", 1);
        let item = world.spawn((Item::new("test_item"), Name::new("No Req Item")));

        let _ = world.insert(
            entity,
            (Equipment {
                slots: vec![(EquipmentSlot::Weapon, item)],
            },),
        );

        run_skill_gate_pulse(&mut world);

        let mut q = world.query_one::<&Equipment>(entity).unwrap();
        let eq = q.get().unwrap();
        assert!(eq.equipped(&EquipmentSlot::Weapon).is_some());
    }

    #[test]
    fn test_unequip_on_insufficient_skill() {
        let mut world = World::new();
        let entity = test_entity_with_skills(&mut world, "weapon_mastery", 1);
        let item = spawn_skill_required_item(&mut world, "weapon_mastery", 3);

        let _ = world.insert(
            entity,
            (Equipment {
                slots: vec![(EquipmentSlot::Weapon, item)],
            },),
        );

        run_skill_gate_pulse(&mut world);

        let mut q = world.query_one::<&Equipment>(entity).unwrap();
        let eq = q.get().unwrap();
        assert!(eq.equipped(&EquipmentSlot::Weapon).is_none());

        let mut iq = world.query_one::<&Inventory>(entity).unwrap();
        let inv = iq.get().unwrap();
        assert!(inv.0.contains(&item));
    }

    #[test]
    fn test_allow_on_sufficient_skill() {
        let mut world = World::new();
        let entity = test_entity_with_skills(&mut world, "weapon_mastery", 5);
        let item = spawn_skill_required_item(&mut world, "weapon_mastery", 3);

        let _ = world.insert(
            entity,
            (Equipment {
                slots: vec![(EquipmentSlot::Weapon, item)],
            },),
        );

        run_skill_gate_pulse(&mut world);

        let mut q = world.query_one::<&Equipment>(entity).unwrap();
        let eq = q.get().unwrap();
        assert!(eq.equipped(&EquipmentSlot::Weapon).is_some());
    }

    #[test]
    fn test_only_removes_violating_items() {
        let mut world = World::new();
        let entity = test_entity_with_skills(&mut world, "weapon_mastery", 2);
        let good_item = spawn_skill_required_item(&mut world, "weapon_mastery", 1);
        let bad_item = spawn_skill_required_item(&mut world, "weapon_mastery", 5);

        let _ = world.insert(
            entity,
            (Equipment {
                slots: vec![
                    (EquipmentSlot::Weapon, good_item),
                    (EquipmentSlot::Shield, bad_item),
                ],
            },),
        );

        run_skill_gate_pulse(&mut world);

        let mut q = world.query_one::<&Equipment>(entity).unwrap();
        let eq = q.get().unwrap();
        assert!(eq.equipped(&EquipmentSlot::Weapon).is_some());
        assert!(eq.equipped(&EquipmentSlot::Shield).is_none());
    }
}
