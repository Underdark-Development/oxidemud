use super::defs::*;
use super::weather::WeatherConfig;
use crate::components::SkillDef;
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Derived indices — pre-computed lookup tables
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default)]
pub struct DerivedIndices {
    /// Set ID → item template IDs that belong to it
    pub items_by_set: HashMap<String, Vec<String>>,
    /// Equipment slot name → item template IDs for that slot
    pub items_by_slot: HashMap<String, Vec<String>>,
}

// ---------------------------------------------------------------------------
// Registry — holds all template types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default)]
pub struct TemplateRegistry {
    pub races: HashMap<String, RaceTemplate>,
    pub classes: HashMap<String, ClassTemplate>,
    pub items: HashMap<String, ItemTemplate>,
    pub mobs: HashMap<String, MobTemplate>,
    pub stances: HashMap<String, StanceDef>,
    pub sets: HashMap<String, SetDef>,
    pub affixes: HashMap<String, AffixDef>,
    pub passives: HashMap<String, PassiveDef>,
    pub areas: HashMap<String, AreaTemplate>,
    pub skills: HashMap<String, SkillDef>,
    pub shops: HashMap<String, ShopTemplate>,
    pub deities: HashMap<String, DeityTemplate>,
    pub quests: HashMap<String, QuestDef>,
    pub factions: HashMap<String, FactionDef>,
    pub recipes: HashMap<String, RecipeDef>,
    pub socials: HashMap<String, SocialDef>,
    pub weather: Option<WeatherConfig>,
    pub indices: DerivedIndices,
}

impl TemplateRegistry {
    pub fn new() -> Self {
        TemplateRegistry::default()
    }

    /// Validate all templates and return any errors found.
    pub fn validate(&self) -> Vec<ValidationError> {
        let mut errors = Vec::new();

        // Validate race ↔ class cross-references
        for (id, race) in &self.races {
            for class_id in &race.allowed_classes {
                if !self.classes.contains_key(class_id) {
                    errors.push(ValidationError {
                        template_type: "race",
                        template_id: id.clone(),
                        field: "allowed_classes".into(),
                        message: format!("references unknown class template: {class_id}"),
                    });
                }
            }
        }

        for (id, class) in &self.classes {
            for race_id in &class.allowed_races {
                if !self.races.contains_key(race_id) {
                    errors.push(ValidationError {
                        template_type: "class",
                        template_id: id.clone(),
                        field: "allowed_races".into(),
                        message: format!("references unknown race template: {race_id}"),
                    });
                }
            }
            if let DeityPolicy::Subset(list) = &class.deity_policy {
                for deity_id in list {
                    if !self.deities.contains_key(deity_id) {
                        errors.push(ValidationError {
                            template_type: "class",
                            template_id: id.clone(),
                            field: "deity_policy".into(),
                            message: format!("subset references unknown deity: {deity_id}"),
                        });
                    }
                }
            }
        }

        // Validate items
        for (id, item) in &self.items {
            // Set membership
            if let Some(set) = &item.set {
                if !self.sets.contains_key(&set.id) {
                    errors.push(ValidationError {
                        template_type: "item",
                        template_id: id.clone(),
                        field: "set.id".into(),
                        message: format!("references unknown set: {}", set.id),
                    });
                }
            }

            // Skill requirement
            if let Some(req) = &item.requires_skill {
                if !self.skills.contains_key(&req.id) {
                    errors.push(ValidationError {
                        template_type: "item",
                        template_id: id.clone(),
                        field: "requires_skill.id".into(),
                        message: format!("references unknown skill: {}", req.id),
                    });
                }
            }

            // Allowed classes
            for class_id in &item.allowed_classes {
                if !self.classes.contains_key(class_id) {
                    errors.push(ValidationError {
                        template_type: "item",
                        template_id: id.clone(),
                        field: "allowed_classes".into(),
                        message: format!("references unknown class: {class_id}"),
                    });
                }
            }

            // Item type validation
            let valid_types = [
                "weapon",
                "armor",
                "potion",
                "scroll",
                "food",
                "drink",
                "container",
                "key",
                "quest",
                "misc",
            ];
            if !item.item_type.is_empty() && !valid_types.contains(&item.item_type.as_str()) {
                errors.push(ValidationError {
                    template_type: "item",
                    template_id: id.clone(),
                    field: "item_type".into(),
                    message: format!("invalid item type '{}'", item.item_type),
                });
            }

            // Quality validation
            let valid_qualities = [
                "common",
                "uncommon",
                "rare",
                "epic",
                "legendary",
                "artifact",
            ];
            if !item.quality.is_empty() && !valid_qualities.contains(&item.quality.as_str()) {
                errors.push(ValidationError {
                    template_type: "item",
                    template_id: id.clone(),
                    field: "quality".into(),
                    message: format!("invalid quality '{}'", item.quality),
                });
            }

            // Weapon validation
            if let Some(ref w) = item.weapon {
                let valid_hands = [
                    "one_hand",
                    "two_hand",
                    "one_or_two_hand",
                    "onehand",
                    "twohand",
                    "one_hand_or_two_hand",
                ];
                if !w.hands.is_empty() && !valid_hands.contains(&w.hands.to_lowercase().as_str()) {
                    errors.push(ValidationError {
                        template_type: "item",
                        template_id: id.clone(),
                        field: "weapon.hands".into(),
                        message: format!("invalid hands mode '{}'", w.hands),
                    });
                }
            }

            // Allowed races
            for race_id in &item.allowed_races {
                if !self.races.contains_key(race_id) {
                    errors.push(ValidationError {
                        template_type: "item",
                        template_id: id.clone(),
                        field: "allowed_races".into(),
                        message: format!("references unknown race: {race_id}"),
                    });
                }
            }
        }

        // Validate mobs
        for (id, mob) in &self.mobs {
            for eq_entry in &mob.equipment {
                if !self.items.contains_key(&eq_entry.template_id) {
                    errors.push(ValidationError {
                        template_type: "mob",
                        template_id: id.clone(),
                        field: "equipment".into(),
                        message: format!(
                            "references unknown item template: {}",
                            eq_entry.template_id
                        ),
                    });
                }
            }

            for entry in &mob.loot.entries {
                if !entry.item.is_empty() && !self.items.contains_key(&entry.item) {
                    errors.push(ValidationError {
                        template_type: "mob",
                        template_id: id.clone(),
                        field: "loot".into(),
                        message: format!("references unknown item template: {}", entry.item),
                    });
                }
            }

            if let Some(ref race_id) = mob.race {
                if !self.races.contains_key(race_id) {
                    errors.push(ValidationError {
                        template_type: "mob",
                        template_id: id.clone(),
                        field: "race".into(),
                        message: format!("references unknown race: {race_id}"),
                    });
                }
            }

            if let Some(ref shop_id) = mob.shop {
                if !self.shops.contains_key(shop_id) {
                    errors.push(ValidationError {
                        template_type: "mob",
                        template_id: id.clone(),
                        field: "shop".into(),
                        message: format!("references unknown shop: {shop_id}"),
                    });
                }
            }
        }

        // Validate shops
        for (id, shop) in &self.shops {
            for (idx, entry) in shop.inventory.iter().enumerate() {
                if !self.items.contains_key(&entry.item) {
                    errors.push(ValidationError {
                        template_type: "shop",
                        template_id: id.clone(),
                        field: format!("inventory[{idx}].item"),
                        message: format!("references unknown item template: {}", entry.item),
                    });
                }
            }
        }

        // Validate passives referenced by race/class/items
        for race in self.races.values() {
            for ability in &race.racial_abilities {
                if !self.passives.contains_key(ability) {
                    errors.push(ValidationError {
                        template_type: "race",
                        template_id: race.id.clone(),
                        field: "racial_abilities".into(),
                        message: format!("references unknown passive: {ability}"),
                    });
                }
            }
        }

        // Validate areas and their rooms
        for (area_id, area) in &self.areas {
            for (room_id, room) in &area.rooms {
                for exit_tpl in room.exits.values() {
                    let dest = exit_tpl.dest();
                    if let Some((target_area, target_room)) = dest.split_once(':') {
                        if !self.areas.contains_key(target_area) {
                            errors.push(ValidationError {
                                template_type: "area",
                                template_id: area_id.clone(),
                                field: format!("rooms.{room_id}.exits"),
                                message: format!("references unknown area '{target_area}'"),
                            });
                        } else if !self.room_exists(target_area, target_room) {
                            errors.push(ValidationError {
                                template_type: "area",
                                template_id: area_id.clone(),
                                field: format!("rooms.{room_id}.exits"),
                                message: format!(
                                    "references unknown room '{target_room}' in area '{target_area}'"
                                ),
                            });
                        }
                    } else if !area.rooms.contains_key(dest) {
                        errors.push(ValidationError {
                            template_type: "area",
                            template_id: area_id.clone(),
                            field: format!("rooms.{room_id}.exits"),
                            message: format!(
                                "references unknown room '{dest}' in area '{area_id}'"
                            ),
                        });
                    }
                }
                for portal in &room.portals {
                    if let Some((target_area, target_room)) = portal.dest.split_once(':') {
                        if !self.areas.contains_key(target_area) {
                            errors.push(ValidationError {
                                template_type: "area",
                                template_id: area_id.clone(),
                                field: format!("rooms.{room_id}.portals"),
                                message: format!("references unknown area '{target_area}'"),
                            });
                        } else if !self.room_exists(target_area, target_room) {
                            errors.push(ValidationError {
                                template_type: "area",
                                template_id: area_id.clone(),
                                field: format!("rooms.{room_id}.portals"),
                                message: format!(
                                    "references unknown room '{target_room}' in area '{target_area}'"
                                ),
                            });
                        }
                    } else if !area.rooms.contains_key(&portal.dest) {
                        errors.push(ValidationError {
                            template_type: "area",
                            template_id: area_id.clone(),
                            field: format!("rooms.{room_id}.portals"),
                            message: format!(
                                "references unknown room '{}' in area '{}'",
                                portal.dest, area_id
                            ),
                        });
                    }
                }
                for entry in &room.content.mobs {
                    if !self.mobs.contains_key(&entry.template_id) {
                        errors.push(ValidationError {
                            template_type: "area",
                            template_id: area_id.clone(),
                            field: format!("rooms.{room_id}.content.mobs"),
                            message: format!(
                                "references unknown mob template '{}'",
                                entry.template_id
                            ),
                        });
                    }
                }
                for entry in &room.content.items {
                    if !self.items.contains_key(&entry.template_id) {
                        errors.push(ValidationError {
                            template_type: "area",
                            template_id: area_id.clone(),
                            field: format!("rooms.{room_id}.content.items"),
                            message: format!(
                                "references unknown item template '{}'",
                                entry.template_id
                            ),
                        });
                    }
                }
            }
            // Validate spawn entries
            for (i, spawn) in area.spawns.iter().enumerate() {
                if !area.rooms.contains_key(&spawn.room) {
                    errors.push(ValidationError {
                        template_type: "area",
                        template_id: area_id.clone(),
                        field: format!("spawns[{i}].room"),
                        message: format!(
                            "references unknown room '{}' in area '{}'",
                            spawn.room, area_id
                        ),
                    });
                }
            }
        }

        // Validate at least one spawn exists globally across all areas
        let total_spawns: usize = self.areas.values().map(|a| a.spawns.len()).sum();
        if total_spawns == 0 {
            errors.push(ValidationError {
                template_type: "world",
                template_id: "*".into(),
                field: "spawns".into(),
                message: "World has zero spawn points — at least one [[spawns]] entry is required across all areas".into(),
            });
        }

        // Validate deities
        for (id, deity) in &self.deities {
            if let Some(align) = &deity.alignment {
                if !crate::components::Alignment::is_valid(align) {
                    errors.push(ValidationError {
                        template_type: "deity",
                        template_id: id.clone(),
                        field: "alignment".into(),
                        message: format!("invalid alignment: {align}"),
                    });
                }
            }
            for race_id in &deity.allowed_races {
                if !self.races.contains_key(race_id) {
                    errors.push(ValidationError {
                        template_type: "deity",
                        template_id: id.clone(),
                        field: "allowed_races".into(),
                        message: format!("references unknown race: {race_id}"),
                    });
                }
            }
            for class_id in &deity.allowed_classes {
                if !self.classes.contains_key(class_id) {
                    errors.push(ValidationError {
                        template_type: "deity",
                        template_id: id.clone(),
                        field: "allowed_classes".into(),
                        message: format!("references unknown class: {class_id}"),
                    });
                }
            }
            for align in &deity.allowed_alignments {
                if !crate::components::Alignment::is_valid(align) {
                    errors.push(ValidationError {
                        template_type: "deity",
                        template_id: id.clone(),
                        field: "allowed_alignments".into(),
                        message: format!("invalid alignment constraint: {align}"),
                    });
                }
            }
        }

        // Validate quests
        for (id, quest) in &self.quests {
            if let Some(giver) = &quest.giver_npc {
                if !self.mobs.contains_key(giver) {
                    errors.push(ValidationError {
                        template_type: "quest",
                        template_id: id.clone(),
                        field: "giver_npc".into(),
                        message: format!("references unknown mob: {giver}"),
                    });
                }
            }
            if let Some(turn_in) = &quest.turn_in_npc {
                if !self.mobs.contains_key(turn_in) {
                    errors.push(ValidationError {
                        template_type: "quest",
                        template_id: id.clone(),
                        field: "turn_in_npc".into(),
                        message: format!("references unknown mob: {turn_in}"),
                    });
                }
            }
            for prereq in &quest.prerequisites {
                if !self.quests.contains_key(prereq) {
                    errors.push(ValidationError {
                        template_type: "quest",
                        template_id: id.clone(),
                        field: "prerequisites".into(),
                        message: format!("references unknown quest prerequisite: {prereq}"),
                    });
                }
            }
            for (idx, obj) in quest.objectives.iter().enumerate() {
                match obj {
                    QuestObjective::Kill { mob, .. } => {
                        if !self.mobs.contains_key(mob) {
                            errors.push(ValidationError {
                                template_type: "quest",
                                template_id: id.clone(),
                                field: format!("objectives[{}].mob", idx),
                                message: format!("references unknown mob: {mob}"),
                            });
                        }
                    }
                    QuestObjective::Gather { item, .. } => {
                        if !self.items.contains_key(item) {
                            errors.push(ValidationError {
                                template_type: "quest",
                                template_id: id.clone(),
                                field: format!("objectives[{}].item", idx),
                                message: format!("references unknown item: {item}"),
                            });
                        }
                    }
                    QuestObjective::Deliver { item, npc } => {
                        if !self.items.contains_key(item) {
                            errors.push(ValidationError {
                                template_type: "quest",
                                template_id: id.clone(),
                                field: format!("objectives[{}].item", idx),
                                message: format!("references unknown item: {item}"),
                            });
                        }
                        if !self.mobs.contains_key(npc) {
                            errors.push(ValidationError {
                                template_type: "quest",
                                template_id: id.clone(),
                                field: format!("objectives[{}].npc", idx),
                                message: format!("references unknown mob: {npc}"),
                            });
                        }
                    }
                    QuestObjective::Explore { room } => {
                        let mut valid = false;
                        if let Some((area_id, room_id)) = room.split_once(':') {
                            if let Some(area) = self.areas.get(area_id) {
                                if area.rooms.contains_key(room_id) {
                                    valid = true;
                                }
                            }
                        }
                        if !valid {
                            errors.push(ValidationError {
                                template_type: "quest",
                                template_id: id.clone(),
                                field: format!("objectives[{}].room", idx),
                                message: format!("references unknown room: {room}"),
                            });
                        }
                    }
                    QuestObjective::Talk { npc } => {
                        if !self.mobs.contains_key(npc) {
                            errors.push(ValidationError {
                                template_type: "quest",
                                template_id: id.clone(),
                                field: format!("objectives[{}].npc", idx),
                                message: format!("references unknown mob: {npc}"),
                            });
                        }
                    }
                }
            }
            for (idx, item_reward) in quest.rewards.items.iter().enumerate() {
                if !self.items.contains_key(&item_reward.item_template_id) {
                    errors.push(ValidationError {
                        template_type: "quest",
                        template_id: id.clone(),
                        field: format!("rewards.items[{}].item_template_id", idx),
                        message: format!(
                            "references unknown item: {}",
                            item_reward.item_template_id
                        ),
                    });
                }
            }
        }

        errors
    }

    /// Build derived indices from all loaded templates.
    pub fn build_indices(&mut self) {
        let mut items_by_set: HashMap<String, Vec<String>> = HashMap::new();
        let mut items_by_slot: HashMap<String, Vec<String>> = HashMap::new();

        for (id, item) in &self.items {
            if let Some(set) = &item.set {
                items_by_set
                    .entry(set.id.clone())
                    .or_default()
                    .push(id.clone());
            }

            if let Some(eq) = &item.equipment {
                items_by_slot
                    .entry(eq.slot.clone())
                    .or_default()
                    .push(id.clone());
            }
        }

        self.indices = DerivedIndices {
            items_by_set,
            items_by_slot,
        };
    }

    // ── Race helpers ──

    pub fn get_race(&self, id: &str) -> Option<&RaceTemplate> {
        self.races.get(id)
    }

    pub fn get_class(&self, id: &str) -> Option<&ClassTemplate> {
        self.classes.get(id)
    }

    pub fn available_classes_for_race(&self, race_id: &str) -> Vec<&ClassTemplate> {
        let race = match self.races.get(race_id) {
            Some(r) => r,
            None => return Vec::new(),
        };
        let mut classes: Vec<&ClassTemplate> = self
            .classes
            .values()
            .filter(|c| {
                (race.allowed_classes.is_empty() || race.allowed_classes.contains(&c.id))
                    && (c.allowed_races.is_empty()
                        || c.allowed_races.contains(&race_id.to_string()))
            })
            .collect();
        classes.sort_by(|a, b| a.id.cmp(&b.id));
        classes
    }

    pub fn available_races_for_class(&self, class_id: &str) -> Vec<&RaceTemplate> {
        let class = match self.classes.get(class_id) {
            Some(c) => c,
            None => return Vec::new(),
        };
        let mut races: Vec<&RaceTemplate> = self
            .races
            .values()
            .filter(|r| {
                r.allowed_classes.is_empty()
                    || class.allowed_races.is_empty()
                    || (r.allowed_classes.contains(&class_id.to_string())
                        && class.allowed_races.contains(&r.id.to_string()))
            })
            .collect();
        races.sort_by(|a, b| a.id.cmp(&b.id));
        races
    }

    // ── Item helpers ──

    pub fn get_item(&self, id: &str) -> Option<&ItemTemplate> {
        self.items.get(id)
    }

    // ── Mob helpers ──

    pub fn get_mob(&self, id: &str) -> Option<&MobTemplate> {
        self.mobs.get(id)
    }

    // ── Stance helpers ──

    pub fn get_stance(&self, id: &str) -> Option<&StanceDef> {
        self.stances.get(id)
    }

    // ── Set helpers ──

    pub fn get_set(&self, id: &str) -> Option<&SetDef> {
        self.sets.get(id)
    }

    // ── Passive helpers ──

    pub fn get_passive(&self, id: &str) -> Option<&PassiveDef> {
        self.passives.get(id)
    }

    pub fn passives_for_race(&self, race_id: &str) -> Vec<&PassiveDef> {
        self.races
            .get(race_id)
            .map(|race| {
                race.racial_abilities
                    .iter()
                    .filter_map(|id| self.passives.get(id))
                    .collect()
            })
            .unwrap_or_default()
    }

    // ── Skill helpers ──

    pub fn get_skill(&self, id: &str) -> Option<&SkillDef> {
        self.skills.get(id)
    }

    pub fn resolve_skill(
        &self,
        input: &str,
        pool: Option<&[String]>,
    ) -> Result<String, SkillResolveError> {
        let input_lower = input.to_lowercase();

        let candidates: Vec<&SkillDef> = if let Some(pool) = pool {
            pool.iter()
                .filter_map(|id| self.skills.get(id.as_str()))
                .collect()
        } else {
            self.skills.values().collect()
        };

        for skill in &candidates {
            if skill.id.to_lowercase() == input_lower || skill.name.to_lowercase() == input_lower {
                return Ok(skill.id.clone());
            }
        }

        let mut matches: Vec<&SkillDef> = Vec::new();
        let mut seen_ids = std::collections::HashSet::new();
        for skill in candidates {
            if seen_ids.insert(skill.id.as_str())
                && (skill.id.to_lowercase().starts_with(&input_lower)
                    || skill.name.to_lowercase().starts_with(&input_lower))
            {
                matches.push(skill);
            }
        }

        match matches.len() {
            0 => Err(SkillResolveError::NotFound),
            1 => Ok(matches[0].id.clone()),
            _ => Err(SkillResolveError::Multiple(
                matches
                    .into_iter()
                    .map(|s| (s.id.clone(), s.name.clone()))
                    .collect(),
            )),
        }
    }

    // ── Area helpers ──

    pub fn get_area(&self, id: &str) -> Option<&AreaTemplate> {
        self.areas.get(id)
    }

    // ── Room helpers ──

    pub fn get_room(&self, area_id: &str, room_id: &str) -> Option<&RoomTemplate> {
        self.areas.get(area_id)?.rooms.get(room_id)
    }

    pub fn get_room_mut(&mut self, area_id: &str, room_id: &str) -> Option<&mut RoomTemplate> {
        self.areas.get_mut(area_id)?.rooms.get_mut(room_id)
    }

    pub fn room_exists(&self, area_id: &str, room_id: &str) -> bool {
        self.areas
            .get(area_id)
            .is_some_and(|a| a.rooms.contains_key(room_id))
    }

    pub fn available_spawns(
        &self,
        race: &str,
        class: &str,
        alignment: &str,
    ) -> Vec<(&str, &SpawnEntry)> {
        let mut result = Vec::new();
        for (area_id, area) in &self.areas {
            for spawn in &area.spawns {
                let race_ok =
                    spawn.allowed_races.is_empty() || spawn.allowed_races.iter().any(|r| r == race);
                let class_ok = spawn.allowed_classes.is_empty()
                    || spawn.allowed_classes.iter().any(|c| c == class);
                let align_ok = spawn.allowed_alignments.is_empty()
                    || spawn.allowed_alignments.iter().any(|a| a == alignment);
                if race_ok && class_ok && align_ok {
                    result.push((area_id.as_str(), spawn));
                }
            }
        }
        result.sort_by(|a, b| a.0.cmp(b.0).then_with(|| a.1.room.cmp(&b.1.room)));
        result
    }

    pub fn find_room_by_key(&self, world: &crate::World, key: &str) -> Option<crate::Entity> {
        use crate::RoomKey;
        let mut query = world.query::<(&RoomKey,)>();
        for (e, (sk,)) in query.iter() {
            if sk.0 == key {
                return Some(e);
            }
        }
        None
    }

    // ── Shop helpers ──

    pub fn get_shop(&self, id: &str) -> Option<&ShopTemplate> {
        self.shops.get(id)
    }

    // ── Affix helpers ──

    pub fn get_affix(&self, id: &str) -> Option<&AffixDef> {
        self.affixes.get(id)
    }

    // ── Social helpers ──

    pub fn get_social(&self, id: &str) -> Option<&SocialDef> {
        self.socials.get(id)
    }

    pub fn resolve_social(&self, input: &str) -> Option<&SocialDef> {
        let lower = input.to_lowercase();
        if let Some(s) = self.socials.get(&lower) {
            return Some(s);
        }
        if let Some(s) = self.socials.get(input) {
            return Some(s);
        }
        for (key, social) in &self.socials {
            if key.starts_with(&lower) || social.name.to_lowercase().starts_with(&lower) {
                return Some(social);
            }
        }
        None
    }

    // ── Index helpers ──

    pub fn items_for_set(&self, set_id: &str) -> Option<&[String]> {
        self.indices.items_by_set.get(set_id).map(|v| v.as_slice())
    }

    pub fn items_for_slot(&self, slot: &str) -> Option<&[String]> {
        self.indices.items_by_slot.get(slot).map(|v| v.as_slice())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    fn test_race() -> RaceTemplate {
        RaceTemplate {
            id: "human".into(),
            name: "Human".into(),
            description: "A versatile race.".into(),
            attributes: RaceAttributes::default(),
            allowed_classes: vec!["warrior".into(), "mage".into()],
            allowed_alignments: Vec::new(),
            racial_abilities: vec!["adaptability".into()],
            allowed_genders: HashMap::new(),
            appearance_bounds: AppearanceBounds::default(),
            age_default: 20,
            age_max: 100,
            params: HashMap::new(),
        }
    }

    fn test_class() -> ClassTemplate {
        ClassTemplate {
            id: "warrior".into(),
            name: "Warrior".into(),
            description: "A master of arms.".into(),
            hit_die: 10,
            attribute_mods: ClassAttributeMods {
                strength: 2,
                constitution: 1,
                ..Default::default()
            },
            bab: "full".to_string(),
            fort_save: "good".to_string(),
            ref_save: "poor".to_string(),
            will_save: "poor".to_string(),
            allowed_races: vec!["human".into()],
            allowed_alignments: Vec::new(),
            auto_skills: vec!["power_attack".into(), "shield_bash".into()],
            skill_pool: vec![
                "power_attack".into(),
                "shield_bash".into(),
                "tactics".into(),
            ],
            starting_skill_slots: 3,
            starting_items: Vec::new(),
            starting_gold: WalletAmount::default(),
            deity_policy: DeityPolicy::Any,
            params: HashMap::new(),
            prestige: false,
            prestige_gate: None,
        }
    }

    #[test]
    fn test_get_race() {
        let reg = TemplateRegistry::new();
        let reg = TemplateRegistry {
            races: vec![("human".into(), test_race())].into_iter().collect(),
            ..reg
        };
        assert!(reg.get_race("human").is_some());
        assert!(reg.get_race("elf").is_none());
    }

    #[test]
    fn test_get_class() {
        let mut reg = TemplateRegistry::new();
        reg.classes.insert("warrior".into(), test_class());
        assert!(reg.get_class("warrior").is_some());
        assert!(reg.get_class("mage").is_none());
    }

    #[test]
    fn test_available_classes_for_race() {
        let mut reg = TemplateRegistry::new();
        reg.classes.insert("warrior".into(), test_class());
        let mut mage = test_class();
        mage.id = "mage".into();
        mage.allowed_races = vec!["human".into()];
        reg.classes.insert("mage".into(), mage);
        let mut elf_class = test_class();
        elf_class.id = "elf_only".into();
        elf_class.allowed_races = vec!["elf".into()];
        reg.classes.insert("elf_only".into(), elf_class);

        reg.races.insert("human".into(), test_race());
        let available = reg.available_classes_for_race("human");
        assert_eq!(available.len(), 2);
        assert!(available.iter().any(|c| c.id == "warrior"));
        assert!(available.iter().any(|c| c.id == "mage"));
    }

    #[test]
    fn test_dice_string_deserialize() {
        let toml_str = r#"dice = "2d6+3""#;
        #[derive(Deserialize)]
        struct Wrapper {
            dice: DiceString,
        }
        let w: Wrapper = toml::from_str(toml_str).unwrap();
        assert_eq!(w.dice.as_str(), "2d6+3");
    }

    #[test]
    fn test_dice_string_invalid() {
        let toml_str = r#"dice = "not_dice""#;
        #[derive(Deserialize)]
        #[allow(dead_code)]
        struct Wrapper {
            dice: DiceString,
        }
        assert!(toml::from_str::<Wrapper>(toml_str).is_err());
    }

    #[test]
    fn test_item_template_defaults() {
        let toml_str = r#"
id = "test_sword"
name = "Test Sword"
description = "A test blade."
item_type = "weapon"
"#;
        let item: ItemTemplate = toml::from_str(toml_str).unwrap();
        assert_eq!(item.quality, "common");
        assert_eq!(item.level_requirement, 0);
        assert!(item.weapon.is_none());
        assert!(item.equipment.is_none());
    }

    #[test]
    fn test_mob_template_parse() {
        let toml_str = r#"
id = "goblin"
name = "goblin"
description = "A goblin."
health = { current = 20, max = 20 }
"#;
        let mob: MobTemplate = toml::from_str(toml_str).unwrap();
        assert_eq!(mob.id, "goblin");
        assert_eq!(mob.health.max, 20);
        assert_eq!(mob.ai_mode, "idle");
        assert_eq!(mob.size, "medium");
    }

    #[test]
    fn test_stance_def_parse() {
        let toml_str = r#"
id = "defensive"
name = "Defensive Stance"
ac_bonus = 2
attack_penalty = -2
"#;
        let stance: StanceDef = toml::from_str(toml_str).unwrap();
        assert_eq!(stance.ac_bonus, 2);
        assert_eq!(stance.attack_penalty, -2);
        assert_eq!(stance.min_level, 1);
    }

    #[test]
    fn test_set_def_parse() {
        let toml_str = r#"
id = "templar_armor"
name = "Templar Armor Set"
[[bonuses]]
min_pieces = 2
effects = [{ effect_type = "stat", stat = "constitution", amount = 2 }]
"#;
        let set: SetDef = toml::from_str(toml_str).unwrap();
        assert_eq!(set.id, "templar_armor");
        assert_eq!(set.bonuses.len(), 1);
        assert_eq!(set.bonuses[0].min_pieces, 2);
    }

    #[test]
    fn test_class_attribute_mods_default() {
        let m = ClassAttributeMods::default();
        assert_eq!(m.strength, 0);
    }

    #[test]
    fn test_race_defaults() {
        let r = RaceTemplate {
            id: "test".into(),
            name: "Test".into(),
            description: "Desc.".into(),
            attributes: RaceAttributes::default(),
            allowed_classes: vec![],
            allowed_alignments: vec![],
            racial_abilities: vec![],
            allowed_genders: HashMap::new(),
            appearance_bounds: AppearanceBounds::default(),
            age_default: 20,
            age_max: 100,
            params: HashMap::new(),
        };
        assert_eq!(r.attributes.strength, 10);
        assert!(r.allowed_classes.is_empty());
    }

    #[test]
    fn test_available_races_for_class() {
        let mut reg = TemplateRegistry::new();
        let mut elf_race = test_race();
        elf_race.id = "elf".into();
        elf_race.allowed_classes = vec!["mage".into()];
        reg.races.insert("elf".into(), elf_race);
        reg.races.insert("human".into(), test_race());

        reg.classes.insert("warrior".into(), test_class());
        let available = reg.available_races_for_class("warrior");
        assert_eq!(available.len(), 1);
        assert_eq!(available[0].id, "human");
    }
}
