use oxide_core::templates::{AreaTemplate, TemplateRegistry};
use oxide_core::{
    AiState, Armor, Attributes, Direction, Entity, Equipment, EquipmentSlot, Exit, Friendly,
    Health, Item, Level, Name, Npc, Position, Race, Room, RoomExits, ShortDesc, Weapon,
    WeaponHands, WeaponRange, World,
};
use std::str::FromStr;

pub fn init_world() -> (World, Entity) {
    let mut world = World::new();

    let void_room = world.spawn((
        Room::new("The Void", "You are floating in a void"),
        oxide_core::VoidRoom,
    ));

    world
        .insert(void_room, (Position::new(void_room),))
        .expect("void room should exist");

    (world, void_room)
}

/// Spawn all rooms and their mobs from the given area template into the ECS world.
///
/// Returns the Entity of the room designated as `spawn_room` in the template.
/// Each room gets [`Room`], [`Position`], and [`RoomExits`] components.
/// Exits are resolved from room IDs to entity references in a second pass.
/// Mob spawns defined in `RoomTemplate.content.mobs` are instantiated after
/// room entities are created.
pub fn spawn_area(world: &mut World, area: &AreaTemplate, registry: &TemplateRegistry) -> Entity {
    use std::collections::HashMap;

    let mut room_map: HashMap<&str, Entity> = HashMap::new();

    // First pass: spawn all room entities
    for (room_id, room_tpl) in &area.rooms {
        let key = format!("{}:{room_id}", area.id);
        let entity = world.spawn((
            Room::new(&room_tpl.name, &room_tpl.description),
            oxide_core::RoomFlags::default(),
            oxide_core::SpawnKey(key),
        ));
        world.insert(entity, (Position::new(entity),)).unwrap();
        room_map.insert(room_id.as_str(), entity);
    }

    // Second pass: resolve exits
    for (room_id, room_tpl) in &area.rooms {
        let room_entity = room_map[room_id.as_str()];
        let mut exits = Vec::new();

        for (dir_str, dest_id) in &room_tpl.exits {
            if let Some(direction) = Direction::try_from(dir_str) {
                if let Some(&dest_entity) = room_map.get(dest_id.as_str()) {
                    exits.push(Exit::new(direction, dest_entity));
                }
            }
        }

        if !exits.is_empty() {
            exits.sort_by_key(|e| e.direction as u8);
            world.insert(room_entity, (RoomExits(exits),)).unwrap();
        }
    }

    // Third pass: spawn mobs from room content
    for (room_id, room_tpl) in &area.rooms {
        let room_entity = room_map[room_id.as_str()];

        for spawn in &room_tpl.content.mobs {
            let Some(mob_tpl) = registry.mobs.get(&spawn.template_id) else {
                tracing::warn!(
                    "Mob template '{}' not found, skipping spawn in {}/{}",
                    spawn.template_id,
                    area.id,
                    room_id
                );
                continue;
            };

            for _ in 0..spawn.count {
                let npc = world.spawn((
                    Position::new(room_entity),
                    Name::new(&mob_tpl.name),
                    Npc::new(&mob_tpl.id),
                    Attributes::new(
                        mob_tpl.attributes.strength,
                        mob_tpl.attributes.dexterity,
                        mob_tpl.attributes.intelligence,
                        mob_tpl.attributes.wisdom,
                        mob_tpl.attributes.constitution,
                        mob_tpl.attributes.charisma,
                    ),
                    Health {
                        current: mob_tpl.health.current,
                        max: mob_tpl.health.max,
                    },
                    Level(mob_tpl.level),
                    Armor {
                        base: mob_tpl.armor,
                        bonus: 0,
                    },
                    Equipment::new(),
                    AiState {
                        ai_mode: mob_tpl.ai_mode.clone(),
                        threat_table: HashMap::new(),
                        wander_counter: 0,
                        patrol_index: 0,
                        aggro_range: mob_tpl.aggro_range,
                        aggro_players: mob_tpl.aggro_players,
                        aggro_race: mob_tpl.aggro_race.clone(),
                        aggro_mobs: mob_tpl.aggro_mobs,
                    },
                ));

                // Add Race component if the mob template has one
                if let Some(ref race_id) = mob_tpl.race {
                    world.insert(npc, (Race(race_id.clone()),)).unwrap();
                }

                // Add ShortDesc component (falls back to name if not set)
                let short_desc = if mob_tpl.short_desc.is_empty() {
                    mob_tpl.name.clone()
                } else {
                    mob_tpl.short_desc.clone()
                };
                world.insert(npc, (ShortDesc(short_desc),)).unwrap();

                // Add Friendly marker if the mob template says so
                if mob_tpl.friendly {
                    world.insert(npc, (Friendly,)).unwrap();
                }

                // Add Trainer component if trainer_types is non-empty
                if !mob_tpl.trainer_types.is_empty() {
                    world
                        .insert(
                            npc,
                            (oxide_core::Trainer::new(mob_tpl.trainer_types.clone()),),
                        )
                        .unwrap();
                }

                equip_mob_template_items(world, npc, mob_tpl, registry);
            }
        }
    }

    room_map[area.spawn_room.as_str()]
}

fn equip_mob_template_items(
    world: &mut World,
    npc: Entity,
    mob_tpl: &oxide_core::templates::MobTemplate,
    registry: &TemplateRegistry,
) {
    if let (Some(damage), Some(damage_type)) = (&mob_tpl.damage, &mob_tpl.damage_type) {
        if let Some(weapon) = make_weapon(damage, damage_type, 2.5, "melee") {
            let natural_weapon = world.spawn((
                Name::new(format!("{} attack", mob_tpl.name)),
                Item::new(format!("{}:natural_attack", mob_tpl.id)),
                weapon,
            ));
            equip_item(world, npc, EquipmentSlot::Weapon, natural_weapon);
        }
    }

    for entry in &mob_tpl.equipment {
        let Some(item_tpl) = registry.get_item(&entry.template_id) else {
            tracing::warn!(
                "Mob template '{}' references unknown equipment '{}'",
                mob_tpl.id,
                entry.template_id
            );
            continue;
        };

        let item = world.spawn((Name::new(&item_tpl.name), Item::new(&item_tpl.id)));

        if let Some(weapon_def) = &item_tpl.weapon {
            if let Some(weapon) = make_weapon(
                &weapon_def.damage.0,
                &weapon_def.damage_type,
                weapon_def.speed,
                &weapon_def.range,
            ) {
                world.insert(item, (weapon,)).unwrap();
            }
        }

        let slot = EquipmentSlot::from_str(&entry.slot).ok().or_else(|| {
            item_tpl
                .equipment
                .as_ref()
                .and_then(|equipment| EquipmentSlot::from_str(&equipment.slot).ok())
        });

        if let Some(slot) = slot {
            equip_item(world, npc, slot, item);
        } else {
            tracing::warn!(
                "Mob template '{}' has invalid equipment slot '{}' for '{}'",
                mob_tpl.id,
                entry.slot,
                entry.template_id
            );
        }
    }
}

fn make_weapon(damage: &str, damage_type: &str, speed: f32, range: &str) -> Option<Weapon> {
    let damage_dice = damage.parse().ok()?;
    let damage_type = oxide_core::DamageType::from_str(damage_type).ok()?;
    let range = match range.to_lowercase().as_str() {
        "ranged" => WeaponRange::Ranged,
        "reach" => WeaponRange::Reach,
        "thrown" => WeaponRange::Thrown,
        _ => WeaponRange::Melee,
    };

    Some(Weapon {
        damage_dice,
        damage_type,
        speed,
        range,
        hands: WeaponHands::OneHand,
    })
}

fn equip_item(world: &mut World, npc: Entity, slot: EquipmentSlot, item: Entity) {
    if let Ok(mut q) = world.query_one::<&mut Equipment>(npc) {
        if let Some(equipment) = q.get() {
            equipment.equip(slot, item);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::{Path, PathBuf};

    fn content_path() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../content")
    }

    #[test]
    fn spawn_area_applies_mob_combat_template_fields() {
        let (mut world, _) = init_world();
        let (registry, _) = oxide_core::content::load_registry(&content_path());
        let area = registry
            .get_area("starting_vale")
            .expect("starting_vale should load");

        spawn_area(&mut world, area, &registry);

        let goblin = {
            let mut q = world.query::<(&Npc, &Attributes, &Armor, &Equipment)>();
            q.iter()
                .find(|(_, (npc, _, _, _))| npc.template_id == "goblin_scavenger")
                .map(|(entity, _)| Entity::from(entity))
                .expect("goblin should spawn")
        };

        let mut attrs = world
            .query_one::<&Attributes>(goblin)
            .expect("goblin should exist");
        assert_eq!(
            attrs.get().expect("goblin should have attributes").strength,
            10
        );

        let mut armor = world
            .query_one::<&Armor>(goblin)
            .expect("goblin should exist");
        assert_eq!(armor.get().expect("goblin should have armor").base, 2);

        let weapon = {
            let mut equipment = world
                .query_one::<&Equipment>(goblin)
                .expect("goblin should exist");
            *equipment
                .get()
                .expect("goblin should have equipment")
                .equipped(&EquipmentSlot::Weapon)
                .expect("goblin should have a weapon")
        };

        assert!(world.query_one::<&Weapon>(weapon).is_ok());
    }

    #[test]
    fn actual_content_loads_all_mobs() {
        let (registry, _) = oxide_core::content::load_registry(&content_path());

        assert!(registry.get_mob("temple_acolyte").is_some());
        assert!(registry.get_mob("trainer").is_some());
        assert_eq!(registry.mobs.len(), 6);
        let errs = registry.validate();
        if !errs.is_empty() {
            println!("VALIDATION ERRORS: {:#?}", errs);
        }
        assert!(errs.is_empty());
    }

    #[test]
    fn spawn_trainer_npc_attaches_trainer_component() {
        let (mut world, _) = init_world();
        let (registry, _) = oxide_core::content::load_registry(&content_path());
        let area = registry
            .get_area("starting_vale")
            .expect("starting_vale should load");

        spawn_area(&mut world, area, &registry);

        let trainer = {
            let mut q = world.query::<(&Npc, &oxide_core::Trainer)>();
            q.iter()
                .find(|(_, (npc, _))| npc.template_id == "trainer")
                .map(|(entity, _)| Entity::from(entity))
                .expect("trainer should spawn and have Trainer component")
        };

        let mut q_trainer = world
            .query_one::<&oxide_core::Trainer>(trainer)
            .expect("trainer should exist in world");
        let t = q_trainer.get().expect("should have Trainer component");
        assert_eq!(t.trainer_types, vec!["attributes".to_string()]);
    }
}
