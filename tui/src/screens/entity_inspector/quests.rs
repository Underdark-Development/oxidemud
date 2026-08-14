use crate::components::Table;
use crate::screens::entity_inspector::EntityInspectorScreen;
use oxide_core::templates::QuestObjective;

impl EntityInspectorScreen {
    pub(super) fn load_quests(&self, table: &mut Table) {
        let quest = match self.registry.quests.get(&self.template_id) {
            Some(q) => q,
            None => return,
        };

        Self::add_field(table, "id", &quest.id);
        Self::add_field(table, "name", &quest.name);
        Self::add_field(table, "description", &quest.description);
        Self::add_field(table, "level_requirement", quest.level_requirement);
        Self::add_field(table, "repeatable", quest.repeatable);
        Self::add_field(table, "auto_complete", quest.auto_complete);
        Self::add_field(table, "giver_npc", quest.giver_npc.as_deref().unwrap_or(""));
        Self::add_field(
            table,
            "turn_in_npc",
            quest.turn_in_npc.as_deref().unwrap_or(""),
        );

        Self::add_array_header(table, "prerequisites", quest.prerequisites.len());
        for (i, pre) in quest.prerequisites.iter().enumerate() {
            Self::add_array_item(table, &format!("prerequisites[{i}]"), pre);
        }

        Self::add_array_header(table, "objectives", quest.objectives.len());
        for (i, obj) in quest.objectives.iter().enumerate() {
            let formatted = match obj {
                QuestObjective::Kill { mob, count } => format!("Kill: {mob} x{count}"),
                QuestObjective::Gather { item, count } => format!("Gather: {item} x{count}"),
                QuestObjective::Deliver { item, npc } => format!("Deliver: {item} to {npc}"),
                QuestObjective::Explore { room } => format!("Explore: {room}"),
                QuestObjective::Talk { npc } => format!("Talk: {npc}"),
            };
            Self::add_array_item(table, &format!("objectives[{i}]"), formatted);
        }

        Self::add_field(table, "rewards.xp", quest.rewards.xp);
        Self::add_field(table, "rewards.gold", quest.rewards.gold);

        Self::add_array_header(table, "rewards.items", quest.rewards.items.len());
        for (i, item) in quest.rewards.items.iter().enumerate() {
            Self::add_array_item(
                table,
                &format!("rewards.items[{i}]"),
                format!("{} x{}", item.item_template_id, item.count),
            );
        }

        Self::add_array_header(table, "rewards.faction", quest.rewards.faction.len());
        for (i, fac) in quest.rewards.faction.iter().enumerate() {
            Self::add_array_item(
                table,
                &format!("rewards.faction[{i}]"),
                format!("{}: {}", fac.faction_id, fac.amount),
            );
        }

        if let Some(ref scripts) = quest.scripts {
            Self::add_field(
                table,
                "scripts.on_accept",
                scripts.on_accept.as_deref().unwrap_or(""),
            );
            Self::add_field(
                table,
                "scripts.on_update",
                scripts.on_update.as_deref().unwrap_or(""),
            );
            Self::add_field(
                table,
                "scripts.on_complete",
                scripts.on_complete.as_deref().unwrap_or(""),
            );
        } else {
            Self::add_field(table, "scripts.on_accept", "");
            Self::add_field(table, "scripts.on_update", "");
            Self::add_field(table, "scripts.on_complete", "");
        }
    }

    pub(super) fn update_quests(&mut self, field: &str, value: &str) -> Result<(), String> {
        let quest = match self.registry.quests.get_mut(&self.template_id) {
            Some(q) => q,
            None => return Err(format!("Quest template not found: {}", self.template_id)),
        };

        match field {
            "id" => {
                quest.id = value.to_string();
            }
            "name" => {
                quest.name = value.to_string();
            }
            "description" => {
                quest.description = value.to_string();
            }
            "level_requirement" => {
                quest.level_requirement = value
                    .parse()
                    .map_err(|_| "level_requirement must be a valid u8".to_string())?;
            }
            "repeatable" => {
                quest.repeatable = value.parse().unwrap_or(false);
            }
            "auto_complete" => {
                quest.auto_complete = value.parse().unwrap_or(false);
            }
            "giver_npc" => {
                quest.giver_npc = if value.trim().is_empty() {
                    None
                } else {
                    Some(value.to_string())
                };
            }
            "turn_in_npc" => {
                quest.turn_in_npc = if value.trim().is_empty() {
                    None
                } else {
                    Some(value.to_string())
                };
            }
            "rewards.xp" => {
                quest.rewards.xp = value
                    .parse()
                    .map_err(|_| "rewards.xp must be a u64".to_string())?;
            }
            "rewards.gold" => {
                quest.rewards.gold = value
                    .parse()
                    .map_err(|_| "rewards.gold must be a u64".to_string())?;
            }
            "scripts.on_accept" => {
                let scripts = quest.scripts.get_or_insert_with(Default::default);
                scripts.on_accept = if value.trim().is_empty() {
                    None
                } else {
                    Some(value.to_string())
                };
            }
            "scripts.on_update" => {
                let scripts = quest.scripts.get_or_insert_with(Default::default);
                scripts.on_update = if value.trim().is_empty() {
                    None
                } else {
                    Some(value.to_string())
                };
            }
            "scripts.on_complete" => {
                let scripts = quest.scripts.get_or_insert_with(Default::default);
                scripts.on_complete = if value.trim().is_empty() {
                    None
                } else {
                    Some(value.to_string())
                };
            }
            _ => {
                if let Some(idx_str) = field
                    .strip_prefix("prerequisites[")
                    .and_then(|s| s.strip_suffix(']'))
                {
                    if let Ok(idx) = idx_str.parse::<usize>() {
                        if idx < quest.prerequisites.len() {
                            quest.prerequisites[idx] = value.to_string();
                        } else if !value.trim().is_empty() {
                            quest.prerequisites.push(value.to_string());
                        }
                    }
                }
            }
        }
        Ok(())
    }

    pub(super) fn add_quest_array(&mut self, prefix: &str, index: usize) -> Result<(), String> {
        let quest = self
            .registry
            .quests
            .get_mut(&self.template_id)
            .ok_or("quest not found")?;
        match prefix {
            "prerequisites" => {
                quest.prerequisites.insert(
                    (index + 1).min(quest.prerequisites.len()),
                    "quest_id".to_string(),
                );
            }
            "objectives" => {
                quest.objectives.insert(
                    (index + 1).min(quest.objectives.len()),
                    oxide_core::templates::QuestObjective::Kill {
                        mob: "goblin".to_string(),
                        count: 1,
                    },
                );
            }
            "rewards.items" => {
                quest.rewards.items.insert(
                    (index + 1).min(quest.rewards.items.len()),
                    oxide_core::templates::QuestRewardItem {
                        item_template_id: "item_id".to_string(),
                        count: 1,
                    },
                );
            }
            "rewards.faction" => {
                quest.rewards.faction.insert(
                    (index + 1).min(quest.rewards.faction.len()),
                    oxide_core::templates::QuestRewardFaction {
                        faction_id: "faction_id".to_string(),
                        amount: 10,
                    },
                );
            }
            _ => return Err(format!("unknown quest array: {prefix}")),
        }
        Ok(())
    }

    pub(super) fn remove_quest_array(&mut self, prefix: &str, index: usize) -> Result<(), String> {
        let quest = self
            .registry
            .quests
            .get_mut(&self.template_id)
            .ok_or("quest not found")?;
        match prefix {
            "prerequisites" => {
                if index < quest.prerequisites.len() {
                    quest.prerequisites.remove(index);
                }
            }
            "objectives" => {
                if index < quest.objectives.len() {
                    quest.objectives.remove(index);
                }
            }
            "rewards.items" => {
                if index < quest.rewards.items.len() {
                    quest.rewards.items.remove(index);
                }
            }
            "rewards.faction" => {
                if index < quest.rewards.faction.len() {
                    quest.rewards.faction.remove(index);
                }
            }
            _ => return Err(format!("unknown quest array: {prefix}")),
        }
        Ok(())
    }

    pub(super) fn clear_quest_array(&mut self, prefix: &str) -> Result<(), String> {
        let quest = self
            .registry
            .quests
            .get_mut(&self.template_id)
            .ok_or("quest not found")?;
        match prefix {
            "prerequisites" => quest.prerequisites.clear(),
            "objectives" => quest.objectives.clear(),
            "rewards.items" => quest.rewards.items.clear(),
            "rewards.faction" => quest.rewards.faction.clear(),
            _ => return Err(format!("unknown quest array: {prefix}")),
        }
        Ok(())
    }
}
