use super::defs::MobTemplate;
use super::registry::TemplateRegistry;
use crate::Entity;
use std::str::FromStr;

impl MobTemplate {
    pub fn spawn(
        &self,
        world: &mut crate::World,
        room_entity: Entity,
        registry: &TemplateRegistry,
    ) -> Entity {
        let ai_state = match self.ai_mode.as_str() {
            "wander" => crate::systems::ai::AiState::Wander { counter: 0 },
            "aggro" | "aggressive" => crate::systems::ai::AiState::Aggro { hunt_target: None },
            "patrol" => crate::systems::ai::AiState::Patrol {
                counter: 0,
                index: 0,
                forward: true,
            },
            _ => crate::systems::ai::AiState::Idle,
        };

        let npc = world.spawn((
            crate::components::Position::new(room_entity),
            crate::components::Name::new(&self.name),
            crate::components::Npc::new_with_aggro(
                &self.id,
                self.aggro_range,
                self.aggro_players,
                self.aggro_mobs,
                self.aggro_race.clone(),
            )
            .with_ai_mode(&self.ai_mode)
            .with_script(
                self.scripts
                    .iter()
                    .find(|s| s.event == "ai")
                    .map(|s| s.script.clone()),
            ),
            crate::components::Attributes::new(
                self.attributes.strength,
                self.attributes.dexterity,
                self.attributes.intelligence,
                self.attributes.wisdom,
                self.attributes.constitution,
                self.attributes.charisma,
            ),
            crate::components::Health {
                current: self.health.current,
                max: self.health.max,
            },
            crate::components::Level(self.level),
            crate::components::Armor {
                base: self.armor,
                bonus: 0,
            },
            crate::components::Equipment::new(),
            ai_state,
            crate::components::ScriptParams(self.params.clone()),
            crate::components::PlayerState::Resting(crate::components::RestState::Standing),
        ));

        if let Some(ref race_id) = self.race {
            let _ = world.insert(npc, (crate::components::Race(race_id.clone()),));
        }

        let short_desc = if self.short_desc.is_empty() {
            self.name.clone()
        } else {
            self.short_desc.clone()
        };
        let _ = world.insert(npc, (crate::components::ShortDesc(short_desc),));

        if self.friendly {
            let _ = world.insert(npc, (crate::components::Friendly,));
        }

        if self.banker {
            let _ = world.insert(npc, (crate::components::Banker,));
        }

        if let Some(ref shop_id) = self.shop {
            let stock = registry
                .shops
                .get(shop_id)
                .map(|shop| crate::components::ShopStock(crate::systems::shop::init_stock(shop)))
                .unwrap_or_default();
            let _ = world.insert(
                npc,
                (
                    crate::components::Shopkeeper {
                        shop_id: shop_id.clone(),
                    },
                    stock,
                    crate::components::LastRestock(std::time::Instant::now()),
                ),
            );
        }

        if !self.trainer_types.is_empty() {
            let _ = world.insert(
                npc,
                (crate::components::Trainer::new(self.trainer_types.clone()),),
            );
        }

        // Equip natural weapon and equipment from templates
        if let (Some(damage), Some(damage_type)) = (&self.damage, &self.damage_type) {
            if let Ok(weapon_dice) = damage.parse::<crate::dice::DiceRoll>() {
                if let Ok(dt) = crate::components::DamageType::from_str(damage_type) {
                    let weapon = crate::components::Weapon {
                        damage_dice: weapon_dice,
                        damage_type: dt,
                        speed: 2.5,
                        range: crate::components::WeaponRange::Melee,
                        hands: crate::components::WeaponHands::OneHand,
                    };
                    let natural_weapon = world.spawn((
                        crate::components::Name::new(format!("{} attack", self.name)),
                        crate::components::Item::new(format!("{}:natural_attack", self.id)),
                        weapon,
                    ));
                    if let Ok(mut q) = world.query_one::<&mut crate::components::Equipment>(npc) {
                        if let Some(eq) = q.get() {
                            eq.equip(crate::components::EquipmentSlot::Weapon, natural_weapon);
                        }
                    }
                }
            }
        }

        for entry in &self.equipment {
            if let Some(item_tpl) = registry.items.get(&entry.template_id) {
                let item = world.spawn((
                    crate::components::Name::new(&item_tpl.name),
                    crate::components::Item::new(&item_tpl.id),
                ));

                if let Some(weapon_def) = &item_tpl.weapon {
                    if let Ok(weapon_dice) = weapon_def.damage.0.parse::<crate::dice::DiceRoll>() {
                        if let Ok(dt) =
                            crate::components::DamageType::from_str(&weapon_def.damage_type)
                        {
                            let range = match weapon_def.range.to_lowercase().as_str() {
                                "ranged" => crate::components::WeaponRange::Ranged,
                                "reach" => crate::components::WeaponRange::Reach,
                                "thrown" => crate::components::WeaponRange::Thrown,
                                _ => crate::components::WeaponRange::Melee,
                            };
                            let hands = match weapon_def.hands.to_lowercase().as_str() {
                                "twohand" | "twohanded" | "two_hand" | "two_handed" => {
                                    crate::components::WeaponHands::TwoHand
                                }
                                _ => crate::components::WeaponHands::OneHand,
                            };
                            let weapon = crate::components::Weapon {
                                damage_dice: weapon_dice,
                                damage_type: dt,
                                speed: weapon_def.speed,
                                range,
                                hands,
                            };
                            let _ = world.insert(item, (weapon,));
                        }
                    }
                }

                if let Some(ref set) = item_tpl.set {
                    let membership = crate::components::SetMembership::from(set.clone());
                    let _ = world.insert(item, (membership,));
                }

                if !item_tpl.triggers.is_empty() {
                    let _ = world.insert(item, (crate::ItemTriggers(item_tpl.triggers.clone()),));
                }

                if let Some(ref req) = item_tpl.requires_skill {
                    let _ = world.insert(
                        item,
                        (crate::components::ItemSkillRequirement {
                            id: req.id.clone(),
                            level: req.level,
                        },),
                    );
                }

                let slot = crate::components::EquipmentSlot::from_str(&entry.slot)
                    .ok()
                    .or_else(|| {
                        item_tpl.equipment.as_ref().and_then(|equipment| {
                            crate::components::EquipmentSlot::from_str(&equipment.slot).ok()
                        })
                    });

                if let Some(slot) = slot {
                    if let Ok(mut q) = world.query_one::<&mut crate::components::Equipment>(npc) {
                        if let Some(eq) = q.get() {
                            eq.equip(slot, item);
                        }
                    }
                }
            }
        }

        if let Some(ref faction_id) = self.faction {
            let aggro_below = registry
                .factions
                .get(faction_id)
                .map(|f| f.aggro_below)
                .unwrap_or(-500);
            let _ = world.insert(
                npc,
                (crate::components::FactionMember::new(
                    faction_id.clone(),
                    aggro_below,
                ),),
            );
        }

        npc
    }
}

impl super::defs::ItemTemplate {
    pub fn spawn(&self, world: &mut crate::World) -> Entity {
        let item = world.spawn((
            crate::components::Name::new(&self.name),
            crate::components::Item::new(&self.id),
        ));

        if let Some(weapon_def) = &self.weapon {
            if let Ok(weapon_dice) = weapon_def.damage.0.parse::<crate::dice::DiceRoll>() {
                if let Ok(dt) = crate::components::DamageType::from_str(&weapon_def.damage_type) {
                    let range = match weapon_def.range.to_lowercase().as_str() {
                        "ranged" => crate::components::WeaponRange::Ranged,
                        "reach" => crate::components::WeaponRange::Reach,
                        "thrown" => crate::components::WeaponRange::Thrown,
                        _ => crate::components::WeaponRange::Melee,
                    };
                    let hands = match weapon_def.hands.to_lowercase().as_str() {
                        "twohand" | "twohanded" | "two_hand" | "two_handed" => {
                            crate::components::WeaponHands::TwoHand
                        }
                        _ => crate::components::WeaponHands::OneHand,
                    };
                    let weapon = crate::components::Weapon {
                        damage_dice: weapon_dice,
                        damage_type: dt,
                        speed: weapon_def.speed,
                        range,
                        hands,
                    };
                    let _ = world.insert(item, (weapon,));
                }
            }
        }

        if let Some(ref set) = self.set {
            let membership = crate::components::SetMembership::from(set.clone());
            let _ = world.insert(item, (membership,));
        }

        if !self.triggers.is_empty() {
            let _ = world.insert(item, (crate::ItemTriggers(self.triggers.clone()),));
        }

        if let Some(ref req) = self.requires_skill {
            let _ = world.insert(
                item,
                (crate::components::ItemSkillRequirement {
                    id: req.id.clone(),
                    level: req.level,
                },),
            );
        }

        if let Some(ref c) = self.consumable {
            let kind = match c.kind.to_lowercase().as_str() {
                "potion" => crate::components::ConsumableKind::Potion,
                "scroll" => crate::components::ConsumableKind::Scroll,
                "wand" => crate::components::ConsumableKind::Wand,
                "food" => crate::components::ConsumableKind::Food,
                "drink" => crate::components::ConsumableKind::Drink,
                other => crate::components::ConsumableKind::Other(other.to_string()),
            };
            let _ = world.insert(
                item,
                (crate::components::Consumable {
                    kind,
                    charges: c.charges,
                    max_charges: c.max_charges,
                    effect_script: c.effect_script.clone(),
                    restore_health: c.restore_health,
                    restore_mana: c.restore_mana,
                    restore_stamina: c.restore_stamina,
                    depleted_template: c.depleted_template.clone(),
                    replenishable: c.replenishable,
                    liquid_type: c.liquid_type.clone(),
                },),
            );
        }

        if let Some(ref cont) = self.container {
            let _ = world.insert(
                item,
                (crate::components::ItemContainer::new(
                    cont.capacity_weight,
                    cont.max_items,
                    cont.weight_reduction_pct,
                    cont.is_closed,
                    cont.is_locked,
                    cont.key_template_id.clone(),
                ),),
            );
            if cont.is_drink_container {
                let _ = world.insert(
                    item,
                    (crate::components::DrinkContainer {
                        liquid_type: cont
                            .liquid_type
                            .clone()
                            .unwrap_or_else(|| "water".to_string()),
                        charges: cont.liquid_charges,
                        max_charges: cont.max_liquid_charges,
                        replenishable: true,
                    },),
                );
            }
        }

        if let Some(ref d) = self.durability {
            let mut dur = crate::components::Durability::new(d.max);
            dur.decay_rate = d.decay_rate;
            let _ = world.insert(item, (dur,));
        }

        let is_fixed = self.flags.iter().any(|f| f == "fixed" || f == "!gettable");
        let _ = world.insert(
            item,
            (crate::components::ItemFlags {
                fixed: is_fixed,
                flags: self.flags.clone(),
            },),
        );

        item
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::templates::defs::{HealthBounds, LootTable, RaceAttributes};
    use std::collections::HashMap;

    fn base_mob(id: &str) -> MobTemplate {
        MobTemplate {
            id: id.to_string(),
            name: id.to_string(),
            description: String::new(),
            short_desc: String::new(),
            level: 1,
            attributes: RaceAttributes::default(),
            health: HealthBounds {
                current: 10,
                max: 10,
            },
            armor: 0,
            damage: None,
            damage_type: None,
            race: None,
            size: "medium".to_string(),
            equipment: Vec::new(),
            xp_value: 0,
            loot: LootTable::default(),
            ai_mode: "idle".to_string(),
            patrol_route: Vec::new(),
            wander_rooms: Vec::new(),
            wander_area: false,
            aggro_range: 0,
            aggro_players: false,
            aggro_mobs: false,
            aggro_race: Vec::new(),
            faction: None,
            faction_standing: 0,
            trainer_types: Vec::new(),
            languages: Vec::new(),
            skills: Vec::new(),
            shop: None,
            friendly: false,
            banker: false,
            scripts: Vec::new(),
            params: HashMap::new(),
        }
    }

    fn has_banker(world: &crate::World, entity: crate::Entity) -> bool {
        world
            .query_one::<&crate::components::Banker>(entity)
            .ok()
            .is_some_and(|mut q| q.get().is_some())
    }

    #[test]
    fn spawn_banker_inserts_banker_component() {
        let mut world = crate::World::new();
        let room = world.spawn(());
        let registry = TemplateRegistry::default();
        let mut mob = base_mob("teller");
        mob.banker = true;
        let npc = mob.spawn(&mut world, room, &registry);
        assert!(has_banker(&world, npc));
    }

    #[test]
    fn spawn_non_banker_has_no_banker_component() {
        let mut world = crate::World::new();
        let room = world.spawn(());
        let registry = TemplateRegistry::default();
        let mob = base_mob("guard");
        let npc = mob.spawn(&mut world, room, &registry);
        assert!(!has_banker(&world, npc));
    }
}
