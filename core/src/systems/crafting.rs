use crate::components::Name;
use crate::systems::loot::{spawn_loot_item, ItemSpawn};
use crate::templates::TemplateRegistry;
use crate::Weapon;
use crate::{Entity, Inventory, Item, LearnedSkills, RoomTags, World};

/// Check if all requirements for crafting a recipe are met.
pub fn can_craft_recipe(
    world: &World,
    player: Entity,
    recipe_id: &str,
    templates: &TemplateRegistry,
) -> Result<(), String> {
    let recipe = templates
        .recipes
        .get(recipe_id)
        .ok_or_else(|| format!("Recipe '{}' not found.", recipe_id))?;

    let knows = world
        .query_one::<&crate::components::LearnedRecipes>(player)
        .ok()
        .and_then(|mut q| q.get().map(|lr| lr.knows(recipe_id)))
        .unwrap_or(false);
    if !knows {
        return Err(format!("You do not know the recipe for '{}'.", recipe.name));
    }

    if let Some(req_station) = &recipe.station {
        let room = world
            .query_one::<&crate::components::Position>(player)
            .ok()
            .and_then(|mut q| q.get().map(|p| p.room))
            .ok_or_else(|| "You are not anywhere.".to_string())?;

        let station_present = world
            .query_one::<&RoomTags>(room)
            .ok()
            .and_then(|mut q| q.get().map(|rt| rt.has_tag(req_station)))
            .unwrap_or(false);
        if !station_present {
            return Err(format!(
                "This recipe requires a {} to craft.",
                req_station.replace("station:", "")
            ));
        }
    }

    if let Some(ref skill_req) = recipe.skill_requirement {
        let learned_skills = world
            .query_one::<&LearnedSkills>(player)
            .ok()
            .and_then(|mut q| q.get().cloned())
            .unwrap_or_default();
        let player_rank = learned_skills.rank(&skill_req.id);
        if (player_rank as u32) < skill_req.rank {
            return Err(format!(
                "You need rank {} in '{}' to craft this (currently rank {}).",
                skill_req.rank, skill_req.id, player_rank
            ));
        }
    }

    let inventory = world
        .query_one::<&Inventory>(player)
        .ok()
        .and_then(|mut q| q.get().cloned())
        .unwrap_or_default();

    for material in &recipe.materials {
        let count = count_material(world, &inventory, &material.template_id);
        if count < material.quantity as usize {
            let item_name = templates
                .items
                .get(&material.template_id)
                .map(|i| i.name.as_str())
                .unwrap_or(&material.template_id);
            return Err(format!(
                "You do not have enough materials. Need {} x {}, but you have {}.",
                material.quantity, item_name, count
            ));
        }
    }

    Ok(())
}

fn count_material(world: &World, inventory: &Inventory, template_id: &str) -> usize {
    let mut count = 0;
    for &item_entity in &inventory.0 {
        if let Ok(mut q) = world.query_one::<&Item>(item_entity) {
            if let Some(item) = q.get() {
                if item.template_id == template_id {
                    count += 1;
                }
            }
        }
    }
    count
}

/// Perform crafting of a recipe, consuming materials and spawning results.
pub fn craft_recipe(
    world: &mut World,
    player: Entity,
    recipe_id: &str,
    templates: &TemplateRegistry,
) -> Result<String, String> {
    can_craft_recipe(world, player, recipe_id, templates)?;

    let recipe = match templates.recipes.get(recipe_id) {
        Some(r) => r.clone(),
        None => return Err(format!("Unknown recipe: {}", recipe_id)),
    };

    let mut inventory = world
        .query_one::<&Inventory>(player)
        .ok()
        .and_then(|mut q| q.get().cloned())
        .unwrap_or_default();

    let roll = fastrand::u8(1..=100);
    let critical_failure = roll <= 5;
    let success = !critical_failure && roll <= recipe.success_chance;

    if !success {
        let consume_ratio = if critical_failure { 1.0 } else { 0.5 };
        for material in &recipe.materials {
            let to_consume = ((material.quantity as f32) * consume_ratio).ceil() as usize;
            consume_material(world, &mut inventory, &material.template_id, to_consume);
        }
        let _ = world.insert(player, (inventory, crate::Dirty));

        if critical_failure {
            return Err(format!(
                "Critical Failure! You failed to craft {} and lost all materials.",
                recipe.name
            ));
        } else {
            return Err(format!(
                "You failed to craft {} but managed to salvage half of the materials.",
                recipe.name
            ));
        }
    }

    for material in &recipe.materials {
        consume_material(
            world,
            &mut inventory,
            &material.template_id,
            material.quantity as usize,
        );
    }

    let mut spawned_entities = Vec::new();
    let mut quality_tier = "common";
    let mut is_upgraded = false;

    if recipe.quality_scaling {
        let player_rank = world
            .query_one::<&LearnedSkills>(player)
            .ok()
            .and_then(|mut q| {
                q.get().map(|ls| {
                    if let Some(ref req) = recipe.skill_requirement {
                        ls.rank(&req.id)
                    } else {
                        1
                    }
                })
            })
            .unwrap_or(1) as i32;

        let required_rank = recipe
            .skill_requirement
            .as_ref()
            .map(|r| r.rank as i32)
            .unwrap_or(1);
        let margin = player_rank - required_rank;
        let quality_bonus = if margin > 0 { margin / 5 } else { 0 };
        let d20 = fastrand::i32(1..=20);
        let total_roll = d20 + quality_bonus;

        if total_roll >= 20 {
            quality_tier = "masterwork";
            is_upgraded = true;
        } else if total_roll >= 15 {
            quality_tier = "fine";
            is_upgraded = true;
        }
    }

    for _ in 0..recipe.result.quantity {
        let spawn = ItemSpawn {
            template_id: recipe.result.template_id.clone(),
            count: 1,
            quality: crate::systems::loot::QualityTier::Common,
            prefix_ids: Vec::new(),
            suffix_ids: Vec::new(),
        };
        if let Some(item_entity) = spawn_loot_item(world, &spawn, templates) {
            if is_upgraded {
                if let Ok(mut q_name) = world.query_one::<&mut Name>(item_entity) {
                    if let Some(n) = q_name.get() {
                        let prefix = match quality_tier {
                            "masterwork" => "Masterwork ",
                            "fine" => "Fine ",
                            _ => "",
                        };
                        n.0 = format!("{}{}", prefix, n.0);
                    }
                }
                if let Ok(mut q_wpn) = world.query_one::<&mut Weapon>(item_entity) {
                    if let Some(wpn) = q_wpn.get() {
                        let bonus = match quality_tier {
                            "masterwork" => 2,
                            "fine" => 1,
                            _ => 0,
                        };
                        wpn.damage_dice.bonus += bonus;
                    }
                }
            }
            spawned_entities.push(item_entity);
        }
    }

    inventory.0.extend(spawned_entities);
    let _ = world.insert(player, (inventory, crate::Dirty));

    let result_name = templates
        .items
        .get(&recipe.result.template_id)
        .map(|i| i.name.as_str())
        .unwrap_or(&recipe.result.template_id);

    let prefix = match quality_tier {
        "masterwork" => "a masterwork ",
        "fine" => "a fine ",
        _ => "",
    };

    Ok(format!(
        "You successfully craft {}{} x {}!",
        prefix, result_name, recipe.result.quantity
    ))
}

fn consume_material(
    world: &mut World,
    inventory: &mut Inventory,
    template_id: &str,
    quantity: usize,
) {
    let mut consumed = 0;
    let mut to_remove = Vec::new();

    for &item_entity in &inventory.0 {
        if consumed >= quantity {
            break;
        }
        if let Ok(mut q) = world.query_one::<&Item>(item_entity) {
            if let Some(item) = q.get() {
                if item.template_id == template_id {
                    to_remove.push(item_entity);
                    consumed += 1;
                }
            }
        }
    }

    for item_entity in to_remove {
        inventory.0.retain(|&e| e != item_entity);
        let _ = world.despawn(item_entity);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::templates::{ItemTemplate, RecipeDef, RecipeMaterial, RecipeResult, RecipeSkillReq};
    use crate::{LearnedRecipes, LearnedSkills, Position, RoomTags};

    fn make_test_registry() -> TemplateRegistry {
        let mut registry = TemplateRegistry::default();

        // Items
        registry.items.insert(
            "iron_ore".to_string(),
            ItemTemplate {
                id: "iron_ore".to_string(),
                name: "Iron Ore".to_string(),
                description: "Raw iron".to_string(),
                item_type: "material".to_string(),
                subtype: "".to_string(),
                rarity: "common".to_string(),
                quality: "standard".to_string(),
                level_requirement: 0,
                weight: 1.0,
                value: 5,
                flags: Vec::new(),
                allowed_classes: Vec::new(),
                allowed_races: Vec::new(),
                allowed_alignments: Vec::new(),
                weapon: None,
                equipment: None,
                set: None,
                consumable: None,
                container: None,
                durability: None,
                requires_skill: None,
                triggers: Vec::new(),
                params: std::collections::HashMap::new(),
            },
        );

        registry.items.insert(
            "iron_bar".to_string(),
            ItemTemplate {
                id: "iron_bar".to_string(),
                name: "Iron Bar".to_string(),
                description: "Refined iron".to_string(),
                item_type: "material".to_string(),
                subtype: "".to_string(),
                rarity: "common".to_string(),
                quality: "standard".to_string(),
                level_requirement: 0,
                weight: 2.0,
                value: 20,
                flags: Vec::new(),
                allowed_classes: Vec::new(),
                allowed_races: Vec::new(),
                allowed_alignments: Vec::new(),
                weapon: None,
                equipment: None,
                set: None,
                consumable: None,
                container: None,
                durability: None,
                requires_skill: None,
                triggers: Vec::new(),
                params: std::collections::HashMap::new(),
            },
        );

        // Recipe: Smelt Iron Bar
        registry.recipes.insert(
            "smelt_iron".to_string(),
            RecipeDef {
                id: "smelt_iron".to_string(),
                name: "Smelt Iron Bar".to_string(),
                description: "Smelt iron ore into a bar".to_string(),
                station: Some("station:forge".to_string()),
                skill_requirement: Some(RecipeSkillReq {
                    id: "smelting".to_string(),
                    rank: 1,
                }),
                difficulty: 1,
                materials: vec![RecipeMaterial {
                    template_id: "iron_ore".to_string(),
                    quantity: 2,
                }],
                result: RecipeResult {
                    template_id: "iron_bar".to_string(),
                    quantity: 1,
                },
                success_chance: 100,
                quality_scaling: true,
                script: None,
            },
        );

        registry
    }

    #[test]
    fn test_can_craft_recipe_checks() {
        let mut world = World::new();
        let registry = make_test_registry();

        let room_no_station = world.spawn((RoomTags::new(vec![]),));
        let room_with_station = world.spawn((RoomTags::new(vec!["station:forge".to_string()]),));

        // Player knows no recipes initially
        let player = world.spawn((
            Position::new(room_with_station),
            Inventory::new(),
            LearnedRecipes::new(),
            LearnedSkills::new(),
        ));

        // 1. Should fail if recipe not learned
        let res = can_craft_recipe(&world, player, "smelt_iron", &registry);
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("do not know the recipe"));

        // Learn recipe
        let mut lr = LearnedRecipes::new();
        lr.learn("smelt_iron");
        let _ = world.insert(player, (lr,));

        // 2. Should fail if wrong room / no station
        let _ = world.insert(player, (Position::new(room_no_station),));
        let res = can_craft_recipe(&world, player, "smelt_iron", &registry);
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("requires a forge"));

        // Move to correct room
        let _ = world.insert(player, (Position::new(room_with_station),));

        // 3. Should fail if skills not satisfied
        let res = can_craft_recipe(&world, player, "smelt_iron", &registry);
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("need rank 1 in 'smelting'"));

        // Learn skill
        let mut skills = LearnedSkills::new();
        skills.set_rank("smelting", 1);
        let _ = world.insert(player, (skills,));

        // 4. Should fail if materials missing
        let res = can_craft_recipe(&world, player, "smelt_iron", &registry);
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("do not have enough materials"));

        // Add materials
        let m1 = world.spawn((Item::new("iron_ore"), Name::new("Iron Ore")));
        let m2 = world.spawn((Item::new("iron_ore"), Name::new("Iron Ore")));
        let mut inv = Inventory::new();
        inv.0.push(m1);
        inv.0.push(m2);
        let _ = world.insert(player, (inv,));

        // Should succeed now
        assert!(can_craft_recipe(&world, player, "smelt_iron", &registry).is_ok());
    }

    #[test]
    fn test_craft_recipe_success_consumes_materials() {
        fastrand::seed(0);
        let mut world = World::new();
        let registry = make_test_registry();
        let room = world.spawn((RoomTags::new(vec!["station:forge".to_string()]),));

        let m1 = world.spawn((Item::new("iron_ore"), Name::new("Iron Ore")));
        let m2 = world.spawn((Item::new("iron_ore"), Name::new("Iron Ore")));

        let mut lr = LearnedRecipes::new();
        lr.learn("smelt_iron");

        let mut skills = LearnedSkills::new();
        skills.set_rank("smelting", 1);

        let mut inv = Inventory::new();
        inv.0.push(m1);
        inv.0.push(m2);

        let player = world.spawn((Position::new(room), inv, lr, skills));

        let res = craft_recipe(&mut world, player, "smelt_iron", &registry);
        assert!(res.is_ok());
        assert!(res.unwrap().contains("successfully craft"));

        // Verify materials consumed, and bar added
        let inv_after = world
            .query_one::<&Inventory>(player)
            .unwrap()
            .get()
            .cloned()
            .unwrap();
        assert_eq!(inv_after.0.len(), 1); // 2 ores removed, 1 bar added

        let result_item = inv_after.0[0];
        let item_comp = world
            .query_one::<&Item>(result_item)
            .unwrap()
            .get()
            .cloned()
            .unwrap();
        assert_eq!(item_comp.template_id, "iron_bar");
    }

    #[test]
    fn test_craft_recipe_failure_consumes_half() {
        let mut world = World::new();
        let mut registry = make_test_registry();

        // Force 0% success chance
        registry
            .recipes
            .get_mut("smelt_iron")
            .unwrap()
            .success_chance = 0;

        let room = world.spawn((RoomTags::new(vec!["station:forge".to_string()]),));

        let m1 = world.spawn((Item::new("iron_ore"), Name::new("Iron Ore")));
        let m2 = world.spawn((Item::new("iron_ore"), Name::new("Iron Ore")));

        let mut lr = LearnedRecipes::new();
        lr.learn("smelt_iron");

        let mut skills = LearnedSkills::new();
        skills.set_rank("smelting", 1);

        let mut inv = Inventory::new();
        inv.0.push(m1);
        inv.0.push(m2);

        let player = world.spawn((Position::new(room), inv, lr, skills));

        let res = craft_recipe(&mut world, player, "smelt_iron", &registry);
        assert!(res.is_err());
        let err_msg = res.unwrap_err();
        assert!(err_msg.contains("failed to craft") || err_msg.contains("Failure!"));

        // Verify materials consumed (1 of 2 ore consumed on normal, all on critical failure)
        let inv_after = world
            .query_one::<&Inventory>(player)
            .unwrap()
            .get()
            .cloned()
            .unwrap();
        if err_msg.contains("Critical Failure") {
            assert_eq!(inv_after.0.len(), 0);
        } else {
            assert_eq!(inv_after.0.len(), 1);
        }
    }
}
