use mud_core::templates::TemplateRegistry;
use ratatui::{
    buffer::Buffer,
    crossterm::event::{KeyCode, KeyEvent, MouseEvent, MouseEventKind},
    layout::Rect,
    style::{Color, Style},
    widgets::Widget,
};

use super::Screen;
use crate::components::{ScrollState, Table};

pub struct EntityInspectorScreen {
    registry: TemplateRegistry,
    category: String,
    template_id: String,
    table: Table,
    scrollbar: ScrollState,
}

impl EntityInspectorScreen {
    pub fn new(registry: TemplateRegistry, category: String, template_id: String) -> Self {
        let mut screen = EntityInspectorScreen {
            registry,
            category,
            template_id,
            table: Table::new(vec!["Field".into(), "Value".into()]),
            scrollbar: ScrollState::new(),
        };
        screen.load_table();
        screen
    }

    fn load_table(&mut self) {
        let mut table = Table::new(vec!["Field".into(), "Value".into()]);

        match self.category.as_str() {
            "items" => self.load_item(&mut table),
            "mobs" => self.load_mob(&mut table),
            "races" => self.load_race(&mut table),
            "classes" => self.load_class(&mut table),
            "skills" => self.load_skill(&mut table),
            "stances" => self.load_stance(&mut table),
            "sets" => self.load_set(&mut table),
            "affixes" => self.load_affix(&mut table),
            "passives" => self.load_passive(&mut table),
            "areas" => self.load_area(&mut table),
            "rooms" => self.load_room(&mut table),
            _ => {
                table.add_row(vec![
                    "error".into(),
                    format!("unknown category: {}", self.category),
                ]);
            }
        }

        if table.rows.is_empty() {
            table.add_row(vec!["message".into(), "template not found".into()]);
        }

        self.table = table;
    }

    fn add_field(table: &mut Table, field: &str, value: impl std::fmt::Display) {
        table.add_row(vec![field.to_string(), value.to_string()]);
    }

    fn load_item(&self, table: &mut Table) {
        let item = match self.registry.items.get(&self.template_id) {
            Some(i) => i,
            None => return,
        };
        Self::add_field(table, "id", &item.id);
        Self::add_field(table, "name", &item.name);
        Self::add_field(table, "description", &item.description);
        Self::add_field(table, "item_type", &item.item_type);
        Self::add_field(table, "subtype", &item.subtype);
        Self::add_field(table, "quality", &item.quality);
        Self::add_field(table, "level_requirement", item.level_requirement);
        Self::add_field(table, "weight", item.weight);
        Self::add_field(table, "value", item.value);
        Self::add_field(table, "flags", item.flags.join(", "));
        for (i, cls) in item.allowed_classes.iter().enumerate() {
            Self::add_field(table, &format!("allowed_classes[{i}]"), cls);
        }
        for (i, race) in item.allowed_races.iter().enumerate() {
            Self::add_field(table, &format!("allowed_races[{i}]"), race);
        }
        for (i, align) in item.allowed_alignments.iter().enumerate() {
            Self::add_field(table, &format!("allowed_alignments[{i}]"), align);
        }
        if let Some(ref req) = item.requires_skill {
            Self::add_field(table, "requires_skill.id", &req.id);
            Self::add_field(table, "requires_skill.level", req.level);
        }
        if let Some(ref w) = item.weapon {
            Self::add_field(table, "weapon.damage", w.damage.as_str());
            Self::add_field(table, "weapon.damage_type", &w.damage_type);
            Self::add_field(table, "weapon.speed", w.speed);
            Self::add_field(table, "weapon.range", &w.range);
        }
        if let Some(ref eq) = item.equipment {
            Self::add_field(table, "equipment.slot", &eq.slot);
        }
        if let Some(ref set) = item.set {
            Self::add_field(table, "set.id", &set.id);
            Self::add_field(table, "set.piece_type", &set.piece_type);
        }
        for (i, trigger) in item.triggers.iter().enumerate() {
            Self::add_field(table, &format!("triggers[{i}].event"), &trigger.event);
            Self::add_field(table, &format!("triggers[{i}].chance"), trigger.chance);
            Self::add_field(table, &format!("triggers[{i}].cast"), &trigger.cast);
            Self::add_field(table, &format!("triggers[{i}].target"), &trigger.target);
        }
    }

    fn load_mob(&self, table: &mut Table) {
        let mob = match self.registry.mobs.get(&self.template_id) {
            Some(m) => m,
            None => return,
        };
        Self::add_field(table, "id", &mob.id);
        Self::add_field(table, "name", &mob.name);
        Self::add_field(table, "description", &mob.description);
        Self::add_field(table, "level", mob.level);
        Self::add_field(table, "armor", mob.armor);
        Self::add_field(table, "size", &mob.size);
        Self::add_field(table, "xp_value", mob.xp_value);
        Self::add_field(table, "ai_mode", &mob.ai_mode);
        Self::add_field(table, "aggro_range", mob.aggro_range);
        Self::add_field(table, "aggro_players", mob.aggro_players);
        Self::add_field(table, "faction_standing", mob.faction_standing);
        if let Some(ref d) = mob.damage {
            Self::add_field(table, "damage", d);
        }
        if let Some(ref dt) = mob.damage_type {
            Self::add_field(table, "damage_type", dt);
        }
        if let Some(ref race) = mob.race {
            Self::add_field(table, "race", race);
        }
        if let Some(ref faction) = mob.faction {
            Self::add_field(table, "faction", faction);
        }
        Self::add_field(table, "health.current", mob.health.current);
        Self::add_field(table, "health.max", mob.health.max);
        Self::add_field(table, "attributes.str", mob.attributes.strength);
        Self::add_field(table, "attributes.dex", mob.attributes.dexterity);
        Self::add_field(table, "attributes.con", mob.attributes.constitution);
        Self::add_field(table, "attributes.int", mob.attributes.intelligence);
        Self::add_field(table, "attributes.wis", mob.attributes.wisdom);
        Self::add_field(table, "attributes.cha", mob.attributes.charisma);
        for entry in &mob.equipment {
            Self::add_field(
                table,
                &format!("equipment.{}", entry.slot),
                &entry.template_id,
            );
        }
        for entry in &mob.loot.entries {
            Self::add_field(table, "loot.item", &entry.item);
            Self::add_field(table, "loot.chance", entry.chance);
            if let Some(ref tc) = entry.treasure_class {
                Self::add_field(table, "loot.treasure_class", tc);
            }
            if let Some(ref count) = entry.count {
                Self::add_field(table, "loot.count.min", count.min);
                Self::add_field(table, "loot.count.max", count.max);
            }
        }
        for (i, race_id) in mob.aggro_race.iter().enumerate() {
            Self::add_field(table, &format!("aggro_race[{i}]"), race_id);
        }
        for (i, lang) in mob.languages.iter().enumerate() {
            Self::add_field(table, &format!("languages[{i}]"), lang);
        }
        for (i, skill) in mob.skills.iter().enumerate() {
            Self::add_field(table, &format!("skills[{i}].id"), &skill.id);
            Self::add_field(table, &format!("skills[{i}].level"), skill.level);
        }
        for (i, trainer_type) in mob.trainer_types.iter().enumerate() {
            Self::add_field(table, &format!("trainer_types[{i}]"), trainer_type);
        }
        for (i, script) in mob.scripts.iter().enumerate() {
            Self::add_field(table, &format!("scripts[{i}].event"), &script.event);
            Self::add_field(table, &format!("scripts[{i}].script"), &script.script);
        }
    }

    fn load_race(&self, table: &mut Table) {
        let race = match self.registry.races.get(&self.template_id) {
            Some(r) => r,
            None => return,
        };
        Self::add_field(table, "id", &race.id);
        Self::add_field(table, "name", &race.name);
        Self::add_field(table, "description", &race.description);
        Self::add_field(table, "attributes.str", race.attributes.strength);
        Self::add_field(table, "attributes.dex", race.attributes.dexterity);
        Self::add_field(table, "attributes.con", race.attributes.constitution);
        Self::add_field(table, "attributes.int", race.attributes.intelligence);
        Self::add_field(table, "attributes.wis", race.attributes.wisdom);
        Self::add_field(table, "attributes.cha", race.attributes.charisma);
        for (i, cls) in race.allowed_classes.iter().enumerate() {
            Self::add_field(table, &format!("allowed_classes[{i}]"), cls);
        }
        for (i, align) in race.allowed_alignments.iter().enumerate() {
            Self::add_field(table, &format!("allowed_alignments[{i}]"), align);
        }
        for (i, ability) in race.racial_abilities.iter().enumerate() {
            Self::add_field(table, &format!("racial_abilities[{i}]"), ability);
        }
    }

    fn load_class(&self, table: &mut Table) {
        let class = match self.registry.classes.get(&self.template_id) {
            Some(c) => c,
            None => return,
        };
        Self::add_field(table, "id", &class.id);
        Self::add_field(table, "name", &class.name);
        Self::add_field(table, "description", &class.description);
        Self::add_field(table, "hit_die", class.hit_die);
        Self::add_field(table, "starting_skill_slots", class.starting_skill_slots);
        Self::add_field(table, "attribute_mods.str", class.attribute_mods.strength);
        Self::add_field(table, "attribute_mods.dex", class.attribute_mods.dexterity);
        Self::add_field(
            table,
            "attribute_mods.con",
            class.attribute_mods.constitution,
        );
        Self::add_field(
            table,
            "attribute_mods.int",
            class.attribute_mods.intelligence,
        );
        Self::add_field(table, "attribute_mods.wis", class.attribute_mods.wisdom);
        Self::add_field(table, "attribute_mods.cha", class.attribute_mods.charisma);
        for (i, race) in class.allowed_races.iter().enumerate() {
            Self::add_field(table, &format!("allowed_races[{i}]"), race);
        }
        for (i, align) in class.allowed_alignments.iter().enumerate() {
            Self::add_field(table, &format!("allowed_alignments[{i}]"), align);
        }
        for (i, skill) in class.auto_skills.iter().enumerate() {
            Self::add_field(table, &format!("auto_skills[{i}]"), skill);
        }
        for (i, skill) in class.skill_pool.iter().enumerate() {
            Self::add_field(table, &format!("skill_pool[{i}]"), skill);
        }
        for (i, item) in class.starting_items.iter().enumerate() {
            Self::add_field(table, &format!("starting_items[{i}]"), item);
        }
        Self::add_field(table, "starting_gold.copper", class.starting_gold.copper);
        Self::add_field(table, "starting_gold.silver", class.starting_gold.silver);
        Self::add_field(table, "starting_gold.gold", class.starting_gold.gold);
        Self::add_field(
            table,
            "starting_gold.platinum",
            class.starting_gold.platinum,
        );
    }

    fn load_skill(&self, table: &mut Table) {
        let skill = match self.registry.skills.get(&self.template_id) {
            Some(s) => s,
            None => return,
        };
        Self::add_field(table, "id", &skill.id);
        Self::add_field(table, "name", &skill.name);
        Self::add_field(table, "description", &skill.description);
        Self::add_field(table, "skill_type", format!("{:?}", skill.skill_type));
        Self::add_field(table, "max_rank", skill.max_rank);
    }

    fn load_stance(&self, table: &mut Table) {
        let stance = match self.registry.stances.get(&self.template_id) {
            Some(s) => s,
            None => return,
        };
        Self::add_field(table, "id", &stance.id);
        Self::add_field(table, "name", &stance.name);
        Self::add_field(table, "ac_bonus", stance.ac_bonus);
        Self::add_field(table, "attack_penalty", stance.attack_penalty);
        Self::add_field(table, "damage_bonus", stance.damage_bonus);
        Self::add_field(table, "ac_penalty", stance.ac_penalty);
        Self::add_field(table, "min_level", stance.min_level);
    }

    fn load_set(&self, table: &mut Table) {
        let set = match self.registry.sets.get(&self.template_id) {
            Some(s) => s,
            None => return,
        };
        Self::add_field(table, "id", &set.id);
        Self::add_field(table, "name", &set.name);
        for (i, bonus) in set.bonuses.iter().enumerate() {
            Self::add_field(table, &format!("bonuses[{i}].min_pieces"), bonus.min_pieces);
            for (j, cond) in bonus.conditions.iter().enumerate() {
                Self::add_field(
                    table,
                    &format!("bonuses[{i}].conditions[{j}].piece_type"),
                    &cond.piece_type,
                );
                Self::add_field(
                    table,
                    &format!("bonuses[{i}].conditions[{j}].min"),
                    cond.min,
                );
            }
            for (j, effect) in bonus.effects.iter().enumerate() {
                Self::add_field(
                    table,
                    &format!("bonuses[{i}].effects[{j}].effect_type"),
                    &effect.effect_type,
                );
                if let Some(ref stat) = effect.stat {
                    Self::add_field(table, &format!("bonuses[{i}].effects[{j}].stat"), stat);
                }
                if let Some(ref amt) = effect.amount {
                    Self::add_field(table, &format!("bonuses[{i}].effects[{j}].amount"), amt);
                }
                if let Some(ref aura) = effect.aura_id {
                    Self::add_field(table, &format!("bonuses[{i}].effects[{j}].aura_id"), aura);
                }
                if let Some(ref radius) = effect.radius {
                    Self::add_field(table, &format!("bonuses[{i}].effects[{j}].radius"), radius);
                }
            }
        }
    }

    fn load_affix(&self, table: &mut Table) {
        let affix = match self.registry.affixes.get(&self.template_id) {
            Some(a) => a,
            None => return,
        };
        Self::add_field(table, "id", &affix.id);
        Self::add_field(table, "name", &affix.name);
        Self::add_field(table, "description", &affix.description);
        Self::add_field(table, "type", &affix.affix_type);
        Self::add_field(table, "quality_min", &affix.quality_min);
        Self::add_field(table, "weight", affix.weight);
        Self::add_field(table, "slot", affix.slot.join(", "));
        if let Some(ref el) = affix.element {
            Self::add_field(table, "element", el);
        }
        if let Some(ref amt) = affix.amount {
            Self::add_field(table, "amount", amt);
        }
        if let Some(ref stat) = affix.stat {
            Self::add_field(table, "stat", stat);
        }
    }

    fn load_passive(&self, table: &mut Table) {
        let passive = match self.registry.passives.get(&self.template_id) {
            Some(p) => p,
            None => return,
        };
        Self::add_field(table, "id", &passive.id);
        Self::add_field(table, "name", &passive.name);
        Self::add_field(table, "description", &passive.description);
        for (i, effect) in passive.effects.iter().enumerate() {
            Self::add_field(
                table,
                &format!("effects[{i}].effect_type"),
                &effect.effect_type,
            );
            Self::add_field(table, &format!("effects[{i}].target"), &effect.target);
            if let Some(ref amt) = effect.amount {
                Self::add_field(table, &format!("effects[{i}].amount"), amt);
            }
        }
    }

    fn load_area(&self, table: &mut Table) {
        let area = match self.registry.areas.get(&self.template_id) {
            Some(a) => a,
            None => return,
        };
        Self::add_field(table, "id", &area.id);
        Self::add_field(table, "name", &area.name);
        Self::add_field(table, "description", &area.description);
        Self::add_field(table, "spawn_room", &area.spawn_room);
        Self::add_field(table, "rooms", area.rooms.len());
        Self::add_field(table, "flags", area.flags.join(", "));
        if let Some(ref range) = area.level_range {
            Self::add_field(table, "level_range", format!("{}–{}", range[0], range[1]));
        }
        if let Some(ref zone) = area.weather_zone {
            Self::add_field(table, "weather_zone", zone);
        }
        if let Some(ref interval) = area.reset_interval {
            Self::add_field(table, "reset_interval_secs", interval.secs);
        }
        if let Some(ref credits) = area.credits {
            Self::add_field(table, "credits", credits);
        }
    }

    fn load_room(&self, table: &mut Table) {
        for area in self.registry.areas.values() {
            if let Some(room) = area.rooms.get(&self.template_id) {
                Self::add_field(table, "name", &room.name);
                Self::add_field(table, "description", &room.description);
                Self::add_field(table, "flags", room.flags.join(", "));
                for (dir, dest) in &room.exits {
                    Self::add_field(table, &format!("exit.{dir}"), dest);
                }
                for (i, portal) in room.portals.iter().enumerate() {
                    Self::add_field(table, &format!("portal[{i}].keyword"), &portal.keyword);
                    Self::add_field(table, &format!("portal[{i}].destination"), &portal.dest);
                    Self::add_field(
                        table,
                        &format!("portal[{i}].description"),
                        &portal.description,
                    );
                    Self::add_field(
                        table,
                        &format!("portal[{i}].flags"),
                        portal.flags.join(", "),
                    );
                }
                for (i, spawn) in room.content.mobs.iter().enumerate() {
                    Self::add_field(
                        table,
                        &format!("content.mobs[{i}].template_id"),
                        &spawn.template_id,
                    );
                    Self::add_field(table, &format!("content.mobs[{i}].count"), spawn.count);
                    if let Some(ref secs) = spawn.respawn_secs {
                        Self::add_field(table, &format!("content.mobs[{i}].respawn_secs"), secs);
                    }
                }
                for (i, spawn) in room.content.items.iter().enumerate() {
                    Self::add_field(
                        table,
                        &format!("content.items[{i}].template_id"),
                        &spawn.template_id,
                    );
                    Self::add_field(table, &format!("content.items[{i}].count"), spawn.count);
                }
                return;
            }
        }
    }
}

impl EntityInspectorScreen {
    pub(super) fn select_prev(&mut self) {
        self.table.select_prev();
    }

    pub(super) fn select_next(&mut self) {
        self.table.select_next();
    }
}

impl Screen for EntityInspectorScreen {
    fn name(&self) -> &str {
        "Entity Inspector"
    }

    fn handle_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Up => self.table.select_prev(),
            KeyCode::Down => self.table.select_next(),
            _ => {}
        }
    }

    fn handle_mouse(&mut self, mouse: MouseEvent, _area: Rect) {
        match mouse.kind {
            MouseEventKind::ScrollUp => self.table.select_prev(),
            MouseEventKind::ScrollDown => self.table.select_next(),
            _ => {}
        }
    }

    fn render(&mut self, area: Rect, buf: &mut Buffer) {
        if area.width < 3 || area.height < 2 {
            return;
        }

        let info = format!(" {} — {} ", self.category, self.template_id);
        buf.set_string(area.x, area.y, &info, Style::default().fg(Color::DarkGray));

        let content_lines = area.height.saturating_sub(2) as usize;
        self.table.update_scroll(content_lines);
        self.scrollbar = ScrollState {
            offset: self.table.scroll.offset,
            visible_lines: self.table.scroll.visible_lines,
            total_lines: self.table.scroll.total_lines,
        };

        let table_area = Rect::new(
            area.x,
            area.y + 1,
            area.width.saturating_sub(1),
            area.height.saturating_sub(2),
        );
        self.table.render(table_area, buf);

        let scrollbar_area = Rect::new(
            area.x + area.width - 1,
            area.y + 1,
            1,
            area.height.saturating_sub(2),
        );
        self.scrollbar.render(scrollbar_area, buf);
    }
}
