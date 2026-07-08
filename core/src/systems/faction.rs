use crate::templates::TemplateRegistry;
use crate::{Entity, FactionStanding, World};

/// Adjust player's standing with a faction, handling relationships recursively (1 level deep).
pub fn adjust_faction_standing(
    world: &mut World,
    player: Entity,
    faction_id: &str,
    delta: i32,
    templates: &TemplateRegistry,
) -> Vec<String> {
    adjust_standing_recursive(world, player, faction_id, delta, templates, true)
}

fn adjust_standing_recursive(
    world: &mut World,
    player: Entity,
    faction_id: &str,
    delta: i32,
    templates: &TemplateRegistry,
    propagate: bool,
) -> Vec<String> {
    let mut msgs = Vec::new();
    if delta == 0 {
        return msgs;
    }

    let faction_def = match templates.factions.get(faction_id) {
        Some(def) => def,
        None => return msgs,
    };

    let mut fs_comp = world
        .query_one::<&FactionStanding>(player)
        .ok()
        .and_then(|mut q| q.get().cloned())
        .unwrap_or_default();

    let current_standing = if let Some(&val) = fs_comp.standings.get(faction_id) {
        val
    } else {
        faction_def.starting_standing
    };

    let new_standing =
        (current_standing + delta).clamp(faction_def.min_standing, faction_def.max_standing);
    let actual_change = new_standing - current_standing;

    if actual_change != 0 {
        fs_comp
            .standings
            .insert(faction_id.to_string(), new_standing);
        let _ = world.insert(player, (fs_comp, crate::Dirty));

        let change_str = if actual_change > 0 {
            format!("increased by {actual_change}")
        } else {
            format!("decreased by {}", actual_change.abs())
        };
        msgs.push(format!(
            "Your standing with {} has {} (currently {} - {}).",
            faction_def.name,
            change_str,
            new_standing,
            faction_def.get_rank(new_standing)
        ));

        if propagate {
            for (rel_id, multiplier) in &faction_def.relationships {
                let rel_delta = (delta as f32 * multiplier) as i32;
                msgs.extend(adjust_standing_recursive(
                    world, player, rel_id, rel_delta, templates, false,
                ));
            }
        }
    }

    msgs
}

/// Handle a player killing a mob, adjusting faction standing accordingly.
pub fn handle_faction_kill(
    world: &mut World,
    player: Entity,
    mob_id: &str,
    templates: &TemplateRegistry,
) -> Vec<String> {
    let mob_tmpl = match templates.mobs.get(mob_id) {
        Some(m) => m,
        None => return Vec::new(),
    };

    let faction_id = match &mob_tmpl.faction {
        Some(f) => f,
        None => return Vec::new(),
    };

    let faction_def = match templates.factions.get(faction_id) {
        Some(f) => f,
        None => return Vec::new(),
    };

    let fs_comp = world
        .query_one::<&FactionStanding>(player)
        .ok()
        .and_then(|mut q| q.get().cloned())
        .unwrap_or_default();

    let current_standing = if let Some(&val) = fs_comp.standings.get(faction_id) {
        val
    } else {
        faction_def.starting_standing
    };

    let delta = if current_standing < faction_def.aggro_below {
        5
    } else {
        -10
    };

    adjust_faction_standing(world, player, faction_id, delta, templates)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::templates::{FactionDef, FactionRank, MobTemplate};
    use crate::FactionStanding;
    use std::collections::HashMap;

    fn create_test_registry() -> TemplateRegistry {
        let mut registry = TemplateRegistry::default();

        // Faction A: Town Guard
        let ranks_a = vec![
            FactionRank {
                name: "Hostile".to_string(),
                threshold: -1000,
            },
            FactionRank {
                name: "Neutral".to_string(),
                threshold: 0,
            },
            FactionRank {
                name: "Friendly".to_string(),
                threshold: 500,
            },
        ];

        let mut relationships_a = HashMap::new();
        relationships_a.insert("outlaws".to_string(), -1.0); // rival
        relationships_a.insert("merchants".to_string(), 0.5); // ally

        registry.factions.insert(
            "town_guard".to_string(),
            FactionDef {
                id: "town_guard".to_string(),
                name: "Town Guard".to_string(),
                description: "Keepers of peace".to_string(),
                starting_standing: 0,
                min_standing: -2000,
                max_standing: 2000,
                ranks: ranks_a,
                relationships: relationships_a,
                aggro_below: -500,
            },
        );

        // Faction B: Outlaws
        registry.factions.insert(
            "outlaws".to_string(),
            FactionDef {
                id: "outlaws".to_string(),
                name: "Outlaws".to_string(),
                description: "Bandits".to_string(),
                starting_standing: -1000,
                min_standing: -5000,
                max_standing: 5000,
                ranks: vec![
                    FactionRank {
                        name: "Hated".to_string(),
                        threshold: -5000,
                    },
                    FactionRank {
                        name: "Outlaw".to_string(),
                        threshold: -1000,
                    },
                ],
                relationships: HashMap::new(),
                aggro_below: 0,
            },
        );

        // Faction C: Merchants
        registry.factions.insert(
            "merchants".to_string(),
            FactionDef {
                id: "merchants".to_string(),
                name: "Merchants".to_string(),
                description: "Traders guild".to_string(),
                starting_standing: 100,
                min_standing: -1000,
                max_standing: 1000,
                ranks: vec![FactionRank {
                    name: "Neutral".to_string(),
                    threshold: 0,
                }],
                relationships: HashMap::new(),
                aggro_below: -200,
            },
        );

        // Mob template
        let mob: MobTemplate = toml::from_str(
            r#"
            id = "guard"
            name = "A Town Guard"
            description = "A simple guard."
            health = { current = 100, max = 100 }
            faction = "town_guard"
        "#,
        )
        .unwrap();
        registry.mobs.insert("guard".to_string(), mob);

        registry
    }

    #[test]
    fn test_adjust_faction_standing_basic() {
        let mut world = World::new();
        let player = world.spawn((FactionStanding::new(),));
        let registry = create_test_registry();

        // Default standing starts at starting_standing (0 for town_guard)
        let msgs = adjust_faction_standing(&mut world, player, "town_guard", 100, &registry);
        assert_eq!(msgs.len(), 3); // Town Guard increased + Outlaws changed (rival) + Merchants changed (ally)

        let fs = world
            .query_one::<&FactionStanding>(player)
            .unwrap()
            .get()
            .cloned()
            .unwrap();
        assert_eq!(fs.standing("town_guard"), 100);
        // Clamping check
        adjust_faction_standing(&mut world, player, "town_guard", 3000, &registry);
        let fs2 = world
            .query_one::<&FactionStanding>(player)
            .unwrap()
            .get()
            .cloned()
            .unwrap();
        assert_eq!(fs2.standing("town_guard"), 2000); // Clamped to max_standing
    }

    #[test]
    fn test_adjust_faction_standing_relationships() {
        let mut world = World::new();
        let player = world.spawn((FactionStanding::new(),));
        let registry = create_test_registry();

        // Increase town_guard by 100
        adjust_faction_standing(&mut world, player, "town_guard", 100, &registry);

        let fs = world
            .query_one::<&FactionStanding>(player)
            .unwrap()
            .get()
            .cloned()
            .unwrap();
        // Town Guard 0 -> 100 (+100)
        assert_eq!(fs.standing("town_guard"), 100);
        // Outlaws (rival: -1.0 multiplier): -1000 + (-100) = -1100
        assert_eq!(fs.standing("outlaws"), -1100);
        // Merchants (ally: 0.5 multiplier): 100 + (50) = 150
        assert_eq!(fs.standing("merchants"), 150);
    }

    #[test]
    fn test_handle_faction_kill() {
        let mut world = World::new();
        let player = world.spawn((FactionStanding::new(),));
        let registry = create_test_registry();

        // 1. Player is friendly/neutral (standing = 0 > aggro_below = -500). Kill should decrease standing (-10)
        let msgs = handle_faction_kill(&mut world, player, "guard", &registry);
        assert!(!msgs.is_empty());
        let fs = world
            .query_one::<&FactionStanding>(player)
            .unwrap()
            .get()
            .cloned()
            .unwrap();
        assert_eq!(fs.standing("town_guard"), -10);

        // 2. Set player standing to -600 (hostile, standing < aggro_below). Kill should increase standing (+5)
        let mut fs_comp = FactionStanding::new();
        fs_comp.set_standing("town_guard", -600);
        let _ = world.insert(player, (fs_comp,));

        handle_faction_kill(&mut world, player, "guard", &registry);
        let fs2 = world
            .query_one::<&FactionStanding>(player)
            .unwrap()
            .get()
            .cloned()
            .unwrap();
        assert_eq!(fs2.standing("town_guard"), -595);
    }
}
