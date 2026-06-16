use crate::templates::TemplateRegistry;
use crate::{ActiveEffect, Class, Entity, Race, World};

/// Apply racial passives from the entity's race template.
/// Looks up each racial_ability in the PassiveDef registry and applies its effects.
pub fn apply_racial_passives(
    world: &mut World,
    entity: Entity,
    templates: &TemplateRegistry,
) -> Vec<String> {
    let mut effects: Vec<ActiveEffect> = Vec::new();
    let mut descriptions: Vec<String> = Vec::new();

    let race_id = world
        .query_one::<&Race>(entity)
        .ok()
        .and_then(|mut q| q.get().map(|r| r.0.clone()));

    let Some(race_id) = race_id else {
        return descriptions;
    };

    let Some(race) = templates.get_race(&race_id) else {
        return descriptions;
    };

    for ability in &race.racial_abilities {
        descriptions.push(format!("racial:{ability}"));

        let Some(passive) = templates.get_passive(ability) else {
            continue;
        };

        for pe in &passive.effects {
            effects.push(ActiveEffect {
                source: format!("passive:race:{ability}"),
                stat: Some(pe.target.clone()),
                amount: pe.amount,
                aura_id: None,
                radius: None,
            });
        }
    }

    apply_passive_effects(world, entity, &effects);
    descriptions
}

/// Apply class passives from the entity's class template.
/// Applies auto-skills and attribute mods (both are data-driven from templates).
pub fn apply_class_passives(
    world: &mut World,
    entity: Entity,
    templates: &TemplateRegistry,
) -> Vec<String> {
    let mut effects: Vec<ActiveEffect> = Vec::new();
    let mut descriptions: Vec<String> = Vec::new();

    let class_id = world
        .query_one::<&Class>(entity)
        .ok()
        .and_then(|mut q| q.get().map(|c| c.0.clone()));

    let Some(class_id) = class_id else {
        return descriptions;
    };

    let Some(class_def) = templates.get_class(&class_id) else {
        return descriptions;
    };

    // Auto-skills are passive abilities the class grants
    for skill_id in &class_def.auto_skills {
        descriptions.push(format!("class:{skill_id}"));
        effects.push(ActiveEffect {
            source: format!("passive:class:{skill_id}"),
            stat: Some(format!("skill:{skill_id}")),
            amount: Some(1),
            aura_id: None,
            radius: None,
        });
    }

    // Apply attribute mods from class — fully data-driven from ClassTemplate
    let mods = &class_def.attribute_mods;
    for (stat, value) in [
        ("strength", mods.strength),
        ("dexterity", mods.dexterity),
        ("intelligence", mods.intelligence),
        ("wisdom", mods.wisdom),
        ("constitution", mods.constitution),
        ("charisma", mods.charisma),
    ] {
        if value != 0 {
            effects.push(ActiveEffect {
                source: "passive:class:attr".into(),
                stat: Some(stat.into()),
                amount: Some(value as i32),
                aura_id: None,
                radius: None,
            });
        }
    }

    apply_passive_effects(world, entity, &effects);
    descriptions
}

/// Apply all passives for an entity (racial + class).
/// Called on login and level-up.
pub fn apply_all_passives(
    world: &mut World,
    entity: Entity,
    templates: &TemplateRegistry,
) -> Vec<String> {
    let mut descriptions = Vec::new();
    descriptions.extend(apply_racial_passives(world, entity, templates));
    descriptions.extend(apply_class_passives(world, entity, templates));
    descriptions
}

/// Remove all passive-sourced ActiveEffects and replace with given.
fn apply_passive_effects(world: &mut World, entity: Entity, new_effects: &[ActiveEffect]) {
    let existing = world
        .query_one::<&Vec<ActiveEffect>>(entity)
        .ok()
        .and_then(|mut q| q.get().cloned())
        .unwrap_or_default();

    let filtered: Vec<ActiveEffect> = existing
        .into_iter()
        .filter(|e| !e.source.starts_with("passive:"))
        .collect();

    let _ = world.remove_one::<Vec<ActiveEffect>>(entity);
    let mut merged = filtered;
    merged.extend(new_effects.iter().cloned());
    let _ = world.insert(entity, (merged,));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::templates::{ClassAttributeMods, ClassTemplate, PassiveDef, RaceTemplate};
    use crate::{Attributes, Class, Race};

    fn make_templates() -> TemplateRegistry {
        let mut t = TemplateRegistry::new();

        t.passives.insert(
            "adaptability".into(),
            PassiveDef {
                id: "adaptability".into(),
                name: "Adaptability".into(),
                description: "Versatile and adaptable.".into(),
                effects: vec![],
            },
        );
        t.passives.insert(
            "darkvision".into(),
            PassiveDef {
                id: "darkvision".into(),
                name: "Darkvision".into(),
                description: "See in the dark.".into(),
                effects: vec![],
            },
        );

        t.races.insert(
            "human".into(),
            RaceTemplate {
                id: "human".into(),
                name: "Human".into(),
                description: "Adaptable humans.".into(),
                attributes: crate::templates::RaceAttributes::default(),
                allowed_classes: vec![],
                allowed_alignments: vec![],
                racial_abilities: vec!["adaptability".into(), "darkvision".into()],
            },
        );
        t.classes.insert(
            "warrior".into(),
            ClassTemplate {
                id: "warrior".into(),
                name: "Warrior".into(),
                description: "A fighter.".into(),
                hit_die: 10,
                attribute_mods: ClassAttributeMods {
                    strength: 2,
                    constitution: 1,
                    ..Default::default()
                },
                allowed_races: vec!["human".into()],
                allowed_alignments: vec![],
                auto_skills: vec!["shield_bash".into()],
                skill_pool: vec!["shield_bash".into()],
                starting_skill_slots: 3,
                starting_items: vec![],
                starting_gold: crate::templates::WalletAmount::default(),
            },
        );
        t
    }

    #[test]
    fn test_racial_passives_applied() {
        let templates = make_templates();
        let mut world = World::new();

        let e = world.spawn((
            Attributes::default(),
            Race("human".into()),
            Class("warrior".into()),
        ));

        let descs = apply_racial_passives(&mut world, e, &templates);
        assert!(descs.contains(&"racial:adaptability".to_string()));
        assert!(descs.contains(&"racial:darkvision".to_string()));
    }

    #[test]
    fn test_class_passives_applied() {
        let templates = make_templates();
        let mut world = World::new();

        let e = world.spawn((
            Attributes::default(),
            Race("human".into()),
            Class("warrior".into()),
        ));

        let descs = apply_class_passives(&mut world, e, &templates);
        assert!(descs.contains(&"class:shield_bash".to_string()));

        let mut q = world.query_one::<&Vec<ActiveEffect>>(e).unwrap();
        let effects = q.get().unwrap();
        assert!(effects
            .iter()
            .any(|ef| ef.stat == Some("skill:shield_bash".to_string())));
    }

    #[test]
    fn test_apply_all_passives() {
        let templates = make_templates();
        let mut world = World::new();

        let e = world.spawn((
            Attributes::default(),
            Race("human".into()),
            Class("warrior".into()),
        ));

        let descs = apply_all_passives(&mut world, e, &templates);
        assert_eq!(descs.len(), 3); // 2 racial + 1 class
    }
}
