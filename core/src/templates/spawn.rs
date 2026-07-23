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
