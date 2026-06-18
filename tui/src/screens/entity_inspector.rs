use std::path::PathBuf;

use mud_core::templates::{DiceString, ResetInterval, TemplateRegistry};
use ratatui::{
    buffer::Buffer,
    crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind},
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Borders, Widget},
};

use super::Screen;
use crate::components::dropdown::{
    dropdown_item_style, highlight_dropdown_row, render_dropdown_box,
};
use crate::components::{Dialog, ScrollState, Table};
use crate::content::FileMap;

enum EditMode {
    Idle,
    Text {
        row: usize,
        cursor: usize,
        original: String,
    },
    Number {
        row: usize,
        cursor: usize,
        original: String,
    },
    Multiline {
        row: usize,
        cursor: usize,
        scroll: usize,
        original: String,
    },
    Dropdown {
        row: usize,
        selection: usize,
        options: Vec<String>,
        original: String,
    },
}

pub struct EntityInspectorScreen {
    registry: TemplateRegistry,
    pub category: String,
    pub template_id: String,
    table: Table,
    scrollbar: ScrollState,
    edit_mode: EditMode,
    pub dirty: bool,
    cursor_char: u8,
    file_map: FileMap,
    pub deleted: bool,
    pub delete_dialog: Option<Dialog>,
    pub content_path: Option<PathBuf>,
    pub is_draft: bool,
    pub muted: bool,
    dropdown_rect: Option<Rect>,
    dropdown_hovered: Option<usize>,
    dropdown_scroll: usize,
    /// Original file path at construction time, used to detect file relocation.
    original_path: Option<PathBuf>,
}

impl EntityInspectorScreen {
    pub fn new(
        registry: TemplateRegistry,
        category: String,
        template_id: String,
        file_map: FileMap,
        content_path: Option<PathBuf>,
        is_draft: bool,
    ) -> Self {
        let mut screen = EntityInspectorScreen {
            registry,
            category,
            template_id,
            table: Table::new(vec!["Field".into(), "Value".into()]),
            scrollbar: ScrollState::new(),
            edit_mode: EditMode::Idle,
            dirty: false,
            cursor_char: 0,
            file_map,
            deleted: false,
            delete_dialog: None,
            content_path,
            is_draft,
            muted: false,
            dropdown_rect: None,
            dropdown_hovered: None,
            dropdown_scroll: 0,
            original_path: None,
        };
        screen.load_table();
        screen.original_path = screen.resolve_save_path().ok();
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

        table.selected = Some(0);
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
                Self::add_field(table, "area", &area.id);
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

    fn detect_field_kind(&self, row: usize) -> EditMode {
        let value = &self.table.rows[row][1];
        let field = &self.table.rows[row][0];

        if field == "description"
            || field.ends_with(".description")
            || field.ends_with(".script")
            || value.len() > 50
        {
            return EditMode::Multiline {
                row,
                cursor: value.len(),
                scroll: 0,
                original: value.clone(),
            };
        }

        if let Some(options) = dropdown_options(field) {
            let sel = options.iter().position(|o| o == value).unwrap_or(0);
            return EditMode::Dropdown {
                row,
                selection: sel,
                options: options.iter().map(|&s| s.to_string()).collect(),
                original: value.clone(),
            };
        }

        let is_number = matches!(
            field
                .rsplit('.')
                .next()
                .unwrap_or(field)
                .trim_end_matches(']'),
            "level"
                | "weight"
                | "value"
                | "speed"
                | "chance"
                | "min"
                | "max"
                | "count"
                | "armor"
                | "xp_value"
                | "aggro_range"
                | "faction_standing"
                | "current"
                | "hp"
                | "mp"
                | "min_level"
                | "quality_min"
                | "hit_die"
                | "max_rank"
                | "min_pieces"
                | "secs"
                | "radius"
                | "amount"
                | "ac_bonus"
                | "attack_penalty"
                | "damage_bonus"
                | "ac_penalty"
                | "strength"
                | "dexterity"
                | "constitution"
                | "intelligence"
                | "wisdom"
                | "charisma"
                | "copper"
                | "silver"
                | "gold"
                | "platinum"
                | "level_requirement"
                | "starting_skill_slots"
                | "respawn_secs"
        );

        if is_number || value.parse::<f64>().is_ok() {
            return EditMode::Number {
                row,
                cursor: value.len(),
                original: value.clone(),
            };
        }

        EditMode::Text {
            row,
            cursor: value.len(),
            original: value.clone(),
        }
    }

    fn start_edit(&mut self, row: usize) {
        self.edit_mode = self.detect_field_kind(row);
    }

    fn commit_edit(&mut self) {
        let (row, value_opt) = match &self.edit_mode {
            EditMode::Idle => return,
            EditMode::Text {
                row,
                cursor: _,
                original: _,
            }
            | EditMode::Number {
                row,
                cursor: _,
                original: _,
            } => (*row, Some(self.table.rows[*row][1].clone())),
            EditMode::Multiline {
                row,
                cursor: _,
                scroll: _,
                original: _,
            } => (*row, Some(self.table.rows[*row][1].clone())),
            EditMode::Dropdown { row, selection, .. } => {
                let options = dropdown_options(&self.table.rows[*row][0]);
                let value = options.map_or_else(
                    || self.table.rows[*row][1].clone(),
                    |o| o[*selection].to_string(),
                );
                self.table.rows[*row][1] = value.clone();
                (*row, Some(value))
            }
        };

        let field = self.table.rows[row][0].clone();
        let old_value = match &self.edit_mode {
            EditMode::Idle => return,
            m => match m {
                EditMode::Text { original, .. }
                | EditMode::Number { original, .. }
                | EditMode::Multiline { original, .. }
                | EditMode::Dropdown { original, .. } => original.clone(),
                _ => return,
            },
        };
        self.edit_mode = EditMode::Idle;

        if let Some(value) = value_opt {
            if value != old_value {
                if self.update_registry(&field, &value).is_ok() {
                    self.dirty = true;
                } else {
                    self.table.rows[row][1] = old_value;
                }
            }
        }
    }

    /// Persist the entity to disk with round-trip validation.
    /// Returns Ok(()) on success, Err(msg) on failure.
    pub fn save_to_disk(&self) -> Result<(), String> {
        let toml_str = self.serialize_registry_data().map_err(|e| e.to_string())?;

        // Round-trip validation: try to parse the serialized TOML back through the concrete struct.
        self.validate_toml(&toml_str)?;

        let path = self.resolve_save_path()?;

        // Detect room relocation: if original path differs from new path, delete old file.
        if self.category == "rooms" && !self.is_draft {
            if let Some(ref orig) = self.original_path {
                if *orig != path {
                    let _ = std::fs::remove_file(orig);
                }
            }
        }

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("cannot create dirs: {e}"))?;
        }
        std::fs::write(&path, &toml_str).map_err(|e| format!("write failed: {e}"))?;
        Ok(())
    }

    fn serialize_registry_data(&self) -> Result<String, Box<dyn std::error::Error>> {
        let s = match self.category.as_str() {
            "items" => toml::to_string_pretty(
                self.registry
                    .items
                    .get(&self.template_id)
                    .ok_or("item not found")?,
            )?,
            "mobs" => toml::to_string_pretty(
                self.registry
                    .mobs
                    .get(&self.template_id)
                    .ok_or("mob not found")?,
            )?,
            "races" => toml::to_string_pretty(
                self.registry
                    .races
                    .get(&self.template_id)
                    .ok_or("race not found")?,
            )?,
            "classes" => toml::to_string_pretty(
                self.registry
                    .classes
                    .get(&self.template_id)
                    .ok_or("class not found")?,
            )?,
            "skills" => toml::to_string_pretty(
                self.registry
                    .skills
                    .get(&self.template_id)
                    .ok_or("skill not found")?,
            )?,
            "stances" => toml::to_string_pretty(
                self.registry
                    .stances
                    .get(&self.template_id)
                    .ok_or("stance not found")?,
            )?,
            "sets" => toml::to_string_pretty(
                self.registry
                    .sets
                    .get(&self.template_id)
                    .ok_or("set not found")?,
            )?,
            "affixes" => toml::to_string_pretty(
                self.registry
                    .affixes
                    .get(&self.template_id)
                    .ok_or("affix not found")?,
            )?,
            "passives" => toml::to_string_pretty(
                self.registry
                    .passives
                    .get(&self.template_id)
                    .ok_or("passive not found")?,
            )?,
            "areas" => {
                let area = self
                    .registry
                    .areas
                    .get(&self.template_id)
                    .ok_or("area not found")?;
                // Write area metadata without rooms (rooms are separate files)
                let mut area_meta = area.clone();
                area_meta.rooms = std::collections::HashMap::new();
                toml::to_string_pretty(&area_meta)?
            }
            "rooms" => toml::to_string_pretty(
                &self
                    .registry
                    .areas
                    .values()
                    .find_map(|a| a.rooms.get(&self.template_id))
                    .ok_or("room not found")?,
            )?,
            cat => return Err(format!("unknown category: {cat}").into()),
        };
        Ok(s)
    }

    fn resolve_save_path(&self) -> Result<std::path::PathBuf, String> {
        if let Some(path) = self
            .file_map
            .get(&self.category)
            .and_then(|m| m.get(&self.template_id))
        {
            return Ok(path.clone());
        }

        // Draft entity — construct path from content_path
        let cp = self
            .content_path
            .as_ref()
            .ok_or_else(|| "no content_path for draft".to_string())?;

        if self.category == "rooms" {
            let area_id = self
                .registry
                .areas
                .values()
                .find_map(|a| a.rooms.get(&self.template_id))
                .map(|r| &r.area)
                .ok_or_else(|| "cannot resolve area for draft room".to_string())?;
            Ok(cp
                .join("areas")
                .join(area_id)
                .join("rooms")
                .join(format!("{}.toml", self.template_id)))
        } else if self.category == "areas" {
            Ok(cp.join("areas").join(&self.template_id).join("area.toml"))
        } else {
            let dir = cp.join(&self.category);
            Ok(dir.join(format!("{}.toml", self.template_id)))
        }
    }

    fn validate_toml(&self, toml_str: &str) -> Result<(), String> {
        match self.category.as_str() {
            "items" => {
                toml::from_str::<mud_core::templates::ItemTemplate>(toml_str)
                    .map_err(|e| format!("invalid ItemTemplate: {e}"))?;
            }
            "mobs" => {
                toml::from_str::<mud_core::templates::MobTemplate>(toml_str)
                    .map_err(|e| format!("invalid MobTemplate: {e}"))?;
            }
            "races" => {
                toml::from_str::<mud_core::templates::RaceTemplate>(toml_str)
                    .map_err(|e| format!("invalid RaceTemplate: {e}"))?;
            }
            "classes" => {
                toml::from_str::<mud_core::templates::ClassTemplate>(toml_str)
                    .map_err(|e| format!("invalid ClassTemplate: {e}"))?;
            }
            "skills" => {
                toml::from_str::<mud_core::SkillDef>(toml_str)
                    .map_err(|e| format!("invalid SkillDef: {e}"))?;
            }
            "stances" => {
                toml::from_str::<mud_core::templates::StanceDef>(toml_str)
                    .map_err(|e| format!("invalid StanceDef: {e}"))?;
            }
            "sets" => {
                toml::from_str::<mud_core::templates::SetDef>(toml_str)
                    .map_err(|e| format!("invalid SetDef: {e}"))?;
            }
            "affixes" => {
                toml::from_str::<mud_core::templates::AffixDef>(toml_str)
                    .map_err(|e| format!("invalid AffixDef: {e}"))?;
            }
            "passives" => {
                toml::from_str::<mud_core::templates::PassiveDef>(toml_str)
                    .map_err(|e| format!("invalid PassiveDef: {e}"))?;
            }
            "areas" => {
                toml::from_str::<mud_core::templates::AreaTemplate>(toml_str)
                    .map_err(|e| format!("invalid AreaTemplate: {e}"))?;
            }
            "rooms" => {
                toml::from_str::<mud_core::templates::RoomTemplate>(toml_str)
                    .map_err(|e| format!("invalid RoomTemplate: {e}"))?;
            }
            cat => return Err(format!("unknown category: {cat}")),
        }
        Ok(())
    }

    /// Delete the entity file from disk (no-op for drafts).
    fn delete_from_disk(&self) -> Result<(), Box<dyn std::error::Error>> {
        if self.is_draft {
            return Ok(());
        }
        if self.category == "rooms" {
            let room = self
                .registry
                .areas
                .values()
                .find_map(|a| a.rooms.get(&self.template_id))
                .ok_or_else(|| format!("room '{}' not found in registry", self.template_id))?;
            let room_key = format!("{}:{}", room.area, self.template_id);
            let path = self
                .file_map
                .get("rooms")
                .and_then(|m| m.get(&room_key))
                .ok_or_else(|| format!("room file for '{}' not found", self.template_id))?;
            std::fs::remove_file(path)?;
            return Ok(());
        }

        let path = self
            .file_map
            .get(&self.category)
            .and_then(|m| m.get(&self.template_id))
            .ok_or_else(|| format!("no file mapping for {}/{}", self.category, self.template_id))?;

        std::fs::remove_file(path)?;
        Ok(())
    }

    pub fn apply_changes(&self, registry: &mut TemplateRegistry) {
        let template_id = &self.template_id;
        match self.category.as_str() {
            "items" => {
                if let Some(t) = self.registry.items.get(template_id) {
                    registry.items.insert(template_id.clone(), t.clone());
                }
            }
            "mobs" => {
                if let Some(t) = self.registry.mobs.get(template_id) {
                    registry.mobs.insert(template_id.clone(), t.clone());
                }
            }
            "races" => {
                if let Some(t) = self.registry.races.get(template_id) {
                    registry.races.insert(template_id.clone(), t.clone());
                }
            }
            "classes" => {
                if let Some(t) = self.registry.classes.get(template_id) {
                    registry.classes.insert(template_id.clone(), t.clone());
                }
            }
            "skills" => {
                if let Some(t) = self.registry.skills.get(template_id) {
                    registry.skills.insert(template_id.clone(), t.clone());
                }
            }
            "stances" => {
                if let Some(t) = self.registry.stances.get(template_id) {
                    registry.stances.insert(template_id.clone(), t.clone());
                }
            }
            "sets" => {
                if let Some(t) = self.registry.sets.get(template_id) {
                    registry.sets.insert(template_id.clone(), t.clone());
                }
            }
            "affixes" => {
                if let Some(t) = self.registry.affixes.get(template_id) {
                    registry.affixes.insert(template_id.clone(), t.clone());
                }
            }
            "passives" => {
                if let Some(t) = self.registry.passives.get(template_id) {
                    registry.passives.insert(template_id.clone(), t.clone());
                }
            }
            "areas" => {
                if let Some(t) = self.registry.areas.get(template_id) {
                    registry.areas.insert(template_id.clone(), t.clone());
                }
            }
            "rooms" => {
                // Rooms are nested within areas. If the room's area field was changed,
                // relocate it from the old area to the new area in the main registry.
                if let Some(room) = self
                    .registry
                    .areas
                    .values()
                    .find_map(|a| a.rooms.get(template_id))
                    .cloned()
                {
                    let target_id = room.area.clone();
                    // Remove from any area in main registry that still holds it
                    if let Some(old_area) = registry
                        .areas
                        .values_mut()
                        .find(|a| a.rooms.contains_key(template_id))
                    {
                        if old_area.id != target_id {
                            old_area.rooms.remove(template_id);
                        } else {
                            // Same area — just update the room in-place
                            old_area.rooms.insert(template_id.clone(), room);
                            return;
                        }
                    }
                    // Insert into target area
                    if let Some(target) = registry.areas.get_mut(&target_id) {
                        target.rooms.insert(template_id.clone(), room);
                    }
                }
            }
            _ => {}
        }
    }

    fn cancel_edit(&mut self) {
        let (row, original) = match &self.edit_mode {
            EditMode::Idle => return,
            EditMode::Text { row, original, .. }
            | EditMode::Number { row, original, .. }
            | EditMode::Multiline { row, original, .. }
            | EditMode::Dropdown { row, original, .. } => (*row, original.clone()),
        };
        self.table.rows[row][1] = original;
        self.edit_mode = EditMode::Idle;
    }

    fn insert_char(&mut self, c: char) {
        if matches!(self.edit_mode, EditMode::Number { .. })
            && !c.is_ascii_digit()
            && c != '.'
            && c != '-'
        {
            return;
        }
        let row;
        let cursor;
        match &self.edit_mode {
            EditMode::Text {
                row: r, cursor: c2, ..
            }
            | EditMode::Number {
                row: r, cursor: c2, ..
            }
            | EditMode::Multiline {
                row: r, cursor: c2, ..
            } => {
                row = *r;
                cursor = *c2;
            }
            _ => return,
        }
        self.table.rows[row][1].insert(cursor, c);
        match &mut self.edit_mode {
            EditMode::Text { cursor: c2, .. }
            | EditMode::Number { cursor: c2, .. }
            | EditMode::Multiline { cursor: c2, .. } => *c2 += 1,
            _ => {}
        }
    }

    fn delete_char(&mut self) {
        let row;
        let cursor;
        match &self.edit_mode {
            EditMode::Text {
                row: r, cursor: c, ..
            }
            | EditMode::Number {
                row: r, cursor: c, ..
            }
            | EditMode::Multiline {
                row: r, cursor: c, ..
            } => {
                row = *r;
                cursor = *c;
            }
            _ => return,
        }
        let s = &mut self.table.rows[row][1];
        if cursor < s.len() {
            s.remove(cursor);
        }
    }

    fn backspace_char(&mut self) {
        let row;
        let cursor;
        match &self.edit_mode {
            EditMode::Text {
                row: r, cursor: c, ..
            }
            | EditMode::Number {
                row: r, cursor: c, ..
            }
            | EditMode::Multiline {
                row: r, cursor: c, ..
            } => {
                row = *r;
                cursor = *c;
            }
            _ => return,
        }
        if cursor == 0 {
            return;
        }
        self.table.rows[row][1].remove(cursor - 1);
        match &mut self.edit_mode {
            EditMode::Text { cursor: c2, .. }
            | EditMode::Number { cursor: c2, .. }
            | EditMode::Multiline { cursor: c2, .. } => *c2 -= 1,
            _ => {}
        }
    }

    fn cursor_left(&mut self) {
        match &mut self.edit_mode {
            EditMode::Text { cursor, .. }
            | EditMode::Number { cursor, .. }
            | EditMode::Multiline { cursor, .. } => {
                *cursor = cursor.saturating_sub(1);
            }
            _ => {}
        }
    }

    fn cursor_right(&mut self) {
        let row;
        let cursor;
        match &self.edit_mode {
            EditMode::Text {
                row: r, cursor: c, ..
            }
            | EditMode::Number {
                row: r, cursor: c, ..
            }
            | EditMode::Multiline {
                row: r, cursor: c, ..
            } => {
                row = *r;
                cursor = *c;
            }
            _ => return,
        }
        let len = self.table.rows[row][1].len();
        if cursor < len {
            match &mut self.edit_mode {
                EditMode::Text { cursor: c2, .. }
                | EditMode::Number { cursor: c2, .. }
                | EditMode::Multiline { cursor: c2, .. } => *c2 += 1,
                _ => {}
            }
        }
    }

    fn cursor_home(&mut self) {
        match &mut self.edit_mode {
            EditMode::Text { cursor, .. }
            | EditMode::Number { cursor, .. }
            | EditMode::Multiline { cursor, .. } => {
                *cursor = 0;
            }
            _ => {}
        }
    }

    fn cursor_end(&mut self) {
        let row = match &self.edit_mode {
            EditMode::Text { row: r, .. }
            | EditMode::Number { row: r, .. }
            | EditMode::Multiline { row: r, .. } => *r,
            _ => return,
        };
        let len = self.table.rows[row][1].len();
        match &mut self.edit_mode {
            EditMode::Text { cursor, .. }
            | EditMode::Number { cursor, .. }
            | EditMode::Multiline { cursor, .. } => *cursor = len,
            _ => {}
        }
    }

    fn cursor_word_left(&mut self) {
        let (row, cursor) = match &self.edit_mode {
            EditMode::Text { row, cursor, .. }
            | EditMode::Number { row, cursor, .. }
            | EditMode::Multiline { row, cursor, .. } => (*row, *cursor),
            _ => return,
        };
        let s = &self.table.rows[row][1];
        if cursor == 0 {
            return;
        }
        let mut pos = cursor;
        let bytes = s.as_bytes();
        // Skip trailing whitespace
        while pos > 0 && bytes[pos - 1] == b' ' {
            pos -= 1;
        }
        // Skip word characters
        while pos > 0 && bytes[pos - 1] != b' ' {
            pos -= 1;
        }
        match &mut self.edit_mode {
            EditMode::Text { cursor, .. }
            | EditMode::Number { cursor, .. }
            | EditMode::Multiline { cursor, .. } => *cursor = pos,
            _ => {}
        }
    }

    fn cursor_word_right(&mut self) {
        let (row, cursor) = match &self.edit_mode {
            EditMode::Text { row, cursor, .. }
            | EditMode::Number { row, cursor, .. }
            | EditMode::Multiline { row, cursor, .. } => (*row, *cursor),
            _ => return,
        };
        let s = &self.table.rows[row][1];
        let len = s.len();
        if cursor >= len {
            return;
        }
        let bytes = s.as_bytes();
        let mut pos = cursor;
        // Skip current word
        while pos < len && bytes[pos] != b' ' {
            pos += 1;
        }
        // Skip whitespace
        while pos < len && bytes[pos] == b' ' {
            pos += 1;
        }
        match &mut self.edit_mode {
            EditMode::Text { cursor, .. }
            | EditMode::Number { cursor, .. }
            | EditMode::Multiline { cursor, .. } => *cursor = pos,
            _ => {}
        }
    }

    fn delete_to_home(&mut self) {
        let (row, cursor) = match &self.edit_mode {
            EditMode::Text { row, cursor, .. }
            | EditMode::Number { row, cursor, .. }
            | EditMode::Multiline { row, cursor, .. } => (*row, *cursor),
            _ => return,
        };
        if cursor == 0 {
            return;
        }
        self.table.rows[row][1].drain(..cursor);
        match &mut self.edit_mode {
            EditMode::Text { cursor, .. }
            | EditMode::Number { cursor, .. }
            | EditMode::Multiline { cursor, .. } => *cursor = 0,
            _ => {}
        }
    }

    fn delete_to_end(&mut self) {
        let (row, cursor) = match &self.edit_mode {
            EditMode::Text { row, cursor, .. }
            | EditMode::Number { row, cursor, .. }
            | EditMode::Multiline { row, cursor, .. } => (*row, *cursor),
            _ => return,
        };
        let s = &mut self.table.rows[row][1];
        if cursor >= s.len() {
            return;
        }
        s.truncate(cursor);
    }

    fn delete_word_backward(&mut self) {
        let (row, cursor) = match &self.edit_mode {
            EditMode::Text { row, cursor, .. }
            | EditMode::Number { row, cursor, .. }
            | EditMode::Multiline { row, cursor, .. } => (*row, *cursor),
            _ => return,
        };
        if cursor == 0 {
            return;
        }
        let s = &self.table.rows[row][1].clone();
        let bytes = s.as_bytes();
        let mut pos = cursor;
        // Skip trailing whitespace
        while pos > 0 && bytes[pos - 1] == b' ' {
            pos -= 1;
        }
        // Skip word characters
        while pos > 0 && bytes[pos - 1] != b' ' {
            pos -= 1;
        }
        let s = &mut self.table.rows[row][1];
        s.drain(pos..cursor);
        match &mut self.edit_mode {
            EditMode::Text { cursor, .. }
            | EditMode::Number { cursor, .. }
            | EditMode::Multiline { cursor, .. } => *cursor = pos,
            _ => {}
        }
    }

    fn delete_word_forward(&mut self) {
        let (row, cursor) = match &self.edit_mode {
            EditMode::Text { row, cursor, .. }
            | EditMode::Number { row, cursor, .. }
            | EditMode::Multiline { row, cursor, .. } => (*row, *cursor),
            _ => return,
        };
        let s = &self.table.rows[row][1].clone();
        let len = s.len();
        if cursor >= len {
            return;
        }
        let bytes = s.as_bytes();
        let mut pos = cursor;
        // Skip word characters
        while pos < len && bytes[pos] != b' ' {
            pos += 1;
        }
        // Skip whitespace
        while pos < len && bytes[pos] == b' ' {
            pos += 1;
        }
        let s = &mut self.table.rows[row][1];
        s.drain(cursor..pos);
    }

    fn multiline_newline(&mut self) {
        let row;
        let cursor;
        match &self.edit_mode {
            EditMode::Multiline {
                row: r, cursor: c, ..
            } => {
                row = *r;
                cursor = *c;
            }
            _ => return,
        }
        self.table.rows[row][1].insert(cursor, '\n');
        if let EditMode::Multiline { cursor: c2, .. } = &mut self.edit_mode {
            *c2 += 1
        }
    }

    fn update_registry(&mut self, field: &str, value: &str) -> Result<(), String> {
        match self.category.as_str() {
            "items" => self.update_item(field, value),
            "mobs" => self.update_mob(field, value),
            "races" => self.update_race(field, value),
            "classes" => self.update_class(field, value),
            "skills" => self.update_skill(field, value),
            "stances" => self.update_stance(field, value),
            "sets" => self.update_set(field, value),
            "affixes" => self.update_affix(field, value),
            "passives" => self.update_passive(field, value),
            "areas" => self.update_area(field, value),
            "rooms" => self.update_room(field, value),
            _ => Err(format!("unknown category: {}", self.category)),
        }
    }

    fn update_item(&mut self, field: &str, value: &str) -> Result<(), String> {
        let item = self
            .registry
            .items
            .get_mut(&self.template_id)
            .ok_or_else(|| "item not found".to_string())?;
        match field {
            "id" => item.id = value.to_string(),
            "name" => item.name = value.to_string(),
            "description" => item.description = value.to_string(),
            "item_type" => item.item_type = value.to_string(),
            "subtype" => item.subtype = value.to_string(),
            "quality" => item.quality = value.to_string(),
            "level_requirement" => {
                item.level_requirement = value.parse().map_err(|_| "invalid number")?
            }
            "weight" => item.weight = value.parse().map_err(|_| "invalid number")?,
            "value" => item.value = value.parse().map_err(|_| "invalid number")?,
            "flags" => item.flags = value.split(',').map(|s| s.trim().to_string()).collect(),
            _ if field.starts_with("allowed_classes[") => {
                let idx: usize = field
                    .trim_start_matches("allowed_classes[")
                    .trim_end_matches(']')
                    .parse()
                    .map_err(|_| "invalid index")?;
                if idx < item.allowed_classes.len() {
                    item.allowed_classes[idx] = value.to_string();
                }
            }
            _ if field.starts_with("allowed_races[") => {
                let idx: usize = field
                    .trim_start_matches("allowed_races[")
                    .trim_end_matches(']')
                    .parse()
                    .map_err(|_| "invalid index")?;
                if idx < item.allowed_races.len() {
                    item.allowed_races[idx] = value.to_string();
                }
            }
            _ if field.starts_with("allowed_alignments[") => {
                let idx: usize = field
                    .trim_start_matches("allowed_alignments[")
                    .trim_end_matches(']')
                    .parse()
                    .map_err(|_| "invalid index")?;
                if idx < item.allowed_alignments.len() {
                    item.allowed_alignments[idx] = value.to_string();
                }
            }
            "requires_skill.id" => {
                if let Some(ref mut r) = item.requires_skill {
                    r.id = value.to_string();
                }
            }
            "requires_skill.level" => {
                if let Some(ref mut r) = item.requires_skill {
                    r.level = value.parse().map_err(|_| "invalid number")?;
                }
            }
            "weapon.damage" => {
                if let Some(ref mut w) = item.weapon {
                    w.damage = DiceString(value.to_string());
                }
            }
            "weapon.damage_type" => {
                if let Some(ref mut w) = item.weapon {
                    w.damage_type = value.to_string();
                }
            }
            "weapon.speed" => {
                if let Some(ref mut w) = item.weapon {
                    w.speed = value.parse().map_err(|_| "invalid number")?;
                }
            }
            "weapon.range" => {
                if let Some(ref mut w) = item.weapon {
                    w.range = value.to_string();
                }
            }
            "equipment.slot" => {
                if let Some(ref mut eq) = item.equipment {
                    eq.slot = value.to_string();
                }
            }
            "set.id" => {
                if let Some(ref mut s) = item.set {
                    s.id = value.to_string();
                }
            }
            "set.piece_type" => {
                if let Some(ref mut s) = item.set {
                    s.piece_type = value.to_string();
                }
            }
            _ if field.starts_with("triggers[") => {
                let rest = field.trim_start_matches("triggers[");
                let (idx_str, _rest) = rest.split_once(']').ok_or("invalid trigger path")?;
                let idx: usize = idx_str.parse().map_err(|_| "invalid index")?;
                if idx < item.triggers.len() {
                    let t = &mut item.triggers[idx];
                    if rest.contains(".event") {
                        t.event = value.to_string();
                    } else if rest.contains(".chance") {
                        t.chance = value.parse().map_err(|_| "invalid number")?;
                    } else if rest.contains(".cast") {
                        t.cast = value.to_string();
                    } else if rest.contains(".target") {
                        t.target = value.to_string();
                    }
                }
            }
            _ => return Err(format!("unknown field: {field}")),
        }
        Ok(())
    }

    fn update_mob(&mut self, field: &str, value: &str) -> Result<(), String> {
        let mob = self
            .registry
            .mobs
            .get_mut(&self.template_id)
            .ok_or_else(|| "mob not found".to_string())?;
        match field {
            "id" => mob.id = value.to_string(),
            "name" => mob.name = value.to_string(),
            "description" => mob.description = value.to_string(),
            "level" => mob.level = value.parse().map_err(|_| "invalid number")?,
            "armor" => mob.armor = value.parse().map_err(|_| "invalid number")?,
            "size" => mob.size = value.to_string(),
            "xp_value" => mob.xp_value = value.parse().map_err(|_| "invalid number")?,
            "ai_mode" => mob.ai_mode = value.to_string(),
            "aggro_range" => mob.aggro_range = value.parse().map_err(|_| "invalid number")?,
            "aggro_players" => mob.aggro_players = value.parse().unwrap_or(false),
            "faction_standing" => {
                mob.faction_standing = value.parse().map_err(|_| "invalid number")?
            }
            "damage" => mob.damage = Some(value.to_string()),
            "damage_type" => mob.damage_type = Some(value.to_string()),
            "race" => mob.race = Some(value.to_string()),
            "faction" => mob.faction = Some(value.to_string()),
            "health.current" => mob.health.current = value.parse().map_err(|_| "invalid number")?,
            "health.max" => mob.health.max = value.parse().map_err(|_| "invalid number")?,
            "attributes.str" => {
                mob.attributes.strength = value.parse().map_err(|_| "invalid number")?
            }
            "attributes.dex" => {
                mob.attributes.dexterity = value.parse().map_err(|_| "invalid number")?
            }
            "attributes.con" => {
                mob.attributes.constitution = value.parse().map_err(|_| "invalid number")?
            }
            "attributes.int" => {
                mob.attributes.intelligence = value.parse().map_err(|_| "invalid number")?
            }
            "attributes.wis" => {
                mob.attributes.wisdom = value.parse().map_err(|_| "invalid number")?
            }
            "attributes.cha" => {
                mob.attributes.charisma = value.parse().map_err(|_| "invalid number")?
            }
            _ if field.starts_with("equipment.") => {
                let slot = field.trim_start_matches("equipment.");
                for entry in mob.equipment.iter_mut() {
                    if entry.slot == slot {
                        entry.template_id = value.to_string();
                        break;
                    }
                }
            }
            _ if field.starts_with("loot.") => {
                // Simple: update first loot entry
                if let Some(entry) = mob.loot.entries.first_mut() {
                    match field.trim_start_matches("loot.") {
                        "item" => entry.item = value.to_string(),
                        "chance" => entry.chance = value.parse().map_err(|_| "invalid number")?,
                        "treasure_class" => entry.treasure_class = Some(value.to_string()),
                        "count.min" => {
                            if let Some(ref mut c) = entry.count {
                                c.min = value.parse().map_err(|_| "invalid number")?;
                            }
                        }
                        "count.max" => {
                            if let Some(ref mut c) = entry.count {
                                c.max = value.parse().map_err(|_| "invalid number")?;
                            }
                        }
                        _ => return Err(format!("unknown loot field: {field}")),
                    }
                }
            }
            _ if field.starts_with("aggro_race[")
                || field.starts_with("languages[")
                || field.starts_with("trainer_types[") =>
            {
                // Skip array element edits for now
            }
            _ if field.starts_with("skills[") => {
                let rest = field.trim_start_matches("skills[");
                let (idx_str, _rest) = rest.split_once(']').ok_or("invalid skill path")?;
                let idx: usize = idx_str.parse().map_err(|_| "invalid index")?;
                if idx < mob.skills.len() {
                    let s = &mut mob.skills[idx];
                    if rest.contains(".id") {
                        s.id = value.to_string();
                    } else if rest.contains(".level") {
                        s.level = value.parse().map_err(|_| "invalid number")?;
                    }
                }
            }
            _ if field.starts_with("scripts[") => {
                let rest = field.trim_start_matches("scripts[");
                let (idx_str, _rest) = rest.split_once(']').ok_or("invalid script path")?;
                let idx: usize = idx_str.parse().map_err(|_| "invalid index")?;
                if idx < mob.scripts.len() {
                    let s = &mut mob.scripts[idx];
                    if rest.contains(".event") {
                        s.event = value.to_string();
                    } else if rest.contains(".script") {
                        s.script = value.to_string();
                    }
                }
            }
            _ => return Err(format!("unknown field: {field}")),
        }
        Ok(())
    }

    fn update_race(&mut self, field: &str, value: &str) -> Result<(), String> {
        let race = self
            .registry
            .races
            .get_mut(&self.template_id)
            .ok_or_else(|| "race not found".to_string())?;
        match field {
            "id" => race.id = value.to_string(),
            "name" => race.name = value.to_string(),
            "description" => race.description = value.to_string(),
            "attributes.str" => {
                race.attributes.strength = value.parse().map_err(|_| "invalid number")?
            }
            "attributes.dex" => {
                race.attributes.dexterity = value.parse().map_err(|_| "invalid number")?
            }
            "attributes.con" => {
                race.attributes.constitution = value.parse().map_err(|_| "invalid number")?
            }
            "attributes.int" => {
                race.attributes.intelligence = value.parse().map_err(|_| "invalid number")?
            }
            "attributes.wis" => {
                race.attributes.wisdom = value.parse().map_err(|_| "invalid number")?
            }
            "attributes.cha" => {
                race.attributes.charisma = value.parse().map_err(|_| "invalid number")?
            }
            _ if field.starts_with("allowed_classes[")
                || field.starts_with("allowed_alignments[")
                || field.starts_with("racial_abilities[") => {}
            _ => return Err(format!("unknown field: {field}")),
        }
        Ok(())
    }

    fn update_class(&mut self, field: &str, value: &str) -> Result<(), String> {
        let cls = self
            .registry
            .classes
            .get_mut(&self.template_id)
            .ok_or_else(|| "class not found".to_string())?;
        match field {
            "id" => cls.id = value.to_string(),
            "name" => cls.name = value.to_string(),
            "description" => cls.description = value.to_string(),
            "hit_die" => cls.hit_die = value.parse().map_err(|_| "invalid number")?,
            "starting_skill_slots" => {
                cls.starting_skill_slots = value.parse().map_err(|_| "invalid number")?
            }
            "attribute_mods.str" => {
                cls.attribute_mods.strength = value.parse().map_err(|_| "invalid number")?
            }
            "attribute_mods.dex" => {
                cls.attribute_mods.dexterity = value.parse().map_err(|_| "invalid number")?
            }
            "attribute_mods.con" => {
                cls.attribute_mods.constitution = value.parse().map_err(|_| "invalid number")?
            }
            "attribute_mods.int" => {
                cls.attribute_mods.intelligence = value.parse().map_err(|_| "invalid number")?
            }
            "attribute_mods.wis" => {
                cls.attribute_mods.wisdom = value.parse().map_err(|_| "invalid number")?
            }
            "attribute_mods.cha" => {
                cls.attribute_mods.charisma = value.parse().map_err(|_| "invalid number")?
            }
            "starting_gold.copper" => {
                cls.starting_gold.copper = value.parse().map_err(|_| "invalid number")?
            }
            "starting_gold.silver" => {
                cls.starting_gold.silver = value.parse().map_err(|_| "invalid number")?
            }
            "starting_gold.gold" => {
                cls.starting_gold.gold = value.parse().map_err(|_| "invalid number")?
            }
            "starting_gold.platinum" => {
                cls.starting_gold.platinum = value.parse().map_err(|_| "invalid number")?
            }
            _ if field.starts_with("allowed_races[")
                || field.starts_with("allowed_alignments[")
                || field.starts_with("auto_skills[")
                || field.starts_with("skill_pool[")
                || field.starts_with("starting_items[") => {}
            _ => return Err(format!("unknown field: {field}")),
        }
        Ok(())
    }

    fn update_skill(&mut self, field: &str, value: &str) -> Result<(), String> {
        let skill = self
            .registry
            .skills
            .get_mut(&self.template_id)
            .ok_or_else(|| "skill not found".to_string())?;
        match field {
            "id" => skill.id = value.to_string(),
            "name" => skill.name = value.to_string(),
            "description" => skill.description = value.to_string(),
            "max_rank" => skill.max_rank = value.parse().map_err(|_| "invalid number")?,
            _ => return Err(format!("unknown field: {field}")),
        }
        Ok(())
    }

    fn update_stance(&mut self, field: &str, value: &str) -> Result<(), String> {
        let stance = self
            .registry
            .stances
            .get_mut(&self.template_id)
            .ok_or_else(|| "stance not found".to_string())?;
        match field {
            "id" => stance.id = value.to_string(),
            "name" => stance.name = value.to_string(),
            "ac_bonus" => stance.ac_bonus = value.parse().map_err(|_| "invalid number")?,
            "attack_penalty" => {
                stance.attack_penalty = value.parse().map_err(|_| "invalid number")?
            }
            "damage_bonus" => stance.damage_bonus = value.parse().map_err(|_| "invalid number")?,
            "ac_penalty" => stance.ac_penalty = value.parse().map_err(|_| "invalid number")?,
            "min_level" => stance.min_level = value.parse().map_err(|_| "invalid number")?,
            _ => return Err(format!("unknown field: {field}")),
        }
        Ok(())
    }

    fn update_set(&mut self, field: &str, value: &str) -> Result<(), String> {
        let set = self
            .registry
            .sets
            .get_mut(&self.template_id)
            .ok_or_else(|| "set not found".to_string())?;
        match field {
            "id" => set.id = value.to_string(),
            "name" => set.name = value.to_string(),
            _ if field.starts_with("bonuses[") => {
                let rest = field.trim_start_matches("bonuses[");
                let (idx_str, path_rest) = rest.split_once(']').ok_or("invalid bonus path")?;
                let idx: usize = idx_str.parse().map_err(|_| "invalid index")?;
                if idx < set.bonuses.len() {
                    let bonus = &mut set.bonuses[idx];
                    if path_rest == ".min_pieces" || path_rest.starts_with(".min_pieces") {
                        bonus.min_pieces = value.parse().map_err(|_| "invalid number")?;
                    } else if path_rest.starts_with(".conditions[") {
                        let cond_rest = path_rest.trim_start_matches(".conditions[");
                        let (cidx_str, cpath_rest) =
                            cond_rest.split_once(']').ok_or("invalid cond path")?;
                        let cidx: usize = cidx_str.parse().map_err(|_| "invalid index")?;
                        if cidx < bonus.conditions.len() {
                            let cond = &mut bonus.conditions[cidx];
                            if cpath_rest == ".piece_type" {
                                cond.piece_type = value.to_string();
                            } else if cpath_rest == ".min" || cpath_rest.starts_with(".min") {
                                cond.min = value.parse().map_err(|_| "invalid number")?;
                            }
                        }
                    } else if path_rest.starts_with(".effects[") {
                        let eff_rest = path_rest.trim_start_matches(".effects[");
                        let (eidx_str, epath_rest) =
                            eff_rest.split_once(']').ok_or("invalid effect path")?;
                        let eidx: usize = eidx_str.parse().map_err(|_| "invalid index")?;
                        if eidx < bonus.effects.len() {
                            let eff = &mut bonus.effects[eidx];
                            if epath_rest == ".effect_type" {
                                eff.effect_type = value.to_string();
                            } else if epath_rest == ".stat" {
                                eff.stat = Some(value.to_string());
                            } else if epath_rest == ".amount" {
                                eff.amount = Some(value.parse().map_err(|_| "invalid number")?);
                            } else if epath_rest == ".aura_id" {
                                eff.aura_id = Some(value.to_string());
                            } else if epath_rest == ".radius" {
                                eff.radius = Some(value.parse().map_err(|_| "invalid number")?);
                            }
                        }
                    }
                }
            }
            _ => return Err(format!("unknown field: {field}")),
        }
        Ok(())
    }

    fn update_affix(&mut self, field: &str, value: &str) -> Result<(), String> {
        let affix = self
            .registry
            .affixes
            .get_mut(&self.template_id)
            .ok_or_else(|| "affix not found".to_string())?;
        match field {
            "id" => affix.id = value.to_string(),
            "name" => affix.name = value.to_string(),
            "description" => affix.description = value.to_string(),
            "type" => affix.affix_type = value.to_string(),
            "quality_min" => affix.quality_min = value.to_string(),
            "weight" => affix.weight = value.parse().map_err(|_| "invalid number")?,
            "slot" => affix.slot = value.split(',').map(|s| s.trim().to_string()).collect(),
            "element" => affix.element = Some(value.to_string()),
            "amount" => affix.amount = Some(value.to_string()),
            "stat" => affix.stat = Some(value.to_string()),
            _ => return Err(format!("unknown field: {field}")),
        }
        Ok(())
    }

    fn update_passive(&mut self, field: &str, value: &str) -> Result<(), String> {
        let passive = self
            .registry
            .passives
            .get_mut(&self.template_id)
            .ok_or_else(|| "passive not found".to_string())?;
        match field {
            "id" => passive.id = value.to_string(),
            "name" => passive.name = value.to_string(),
            "description" => passive.description = value.to_string(),
            _ if field.starts_with("effects[") => {
                let rest = field.trim_start_matches("effects[");
                let (idx_str, path_rest) = rest.split_once(']').ok_or("invalid effect path")?;
                let idx: usize = idx_str.parse().map_err(|_| "invalid index")?;
                if idx < passive.effects.len() {
                    let e = &mut passive.effects[idx];
                    if path_rest == ".effect_type" {
                        e.effect_type = value.to_string();
                    } else if path_rest == ".target" {
                        e.target = value.to_string();
                    } else if path_rest == ".amount" {
                        e.amount = Some(value.parse().map_err(|_| "invalid number")?);
                    }
                }
            }
            _ => return Err(format!("unknown field: {field}")),
        }
        Ok(())
    }

    fn update_area(&mut self, field: &str, value: &str) -> Result<(), String> {
        let area = self
            .registry
            .areas
            .get_mut(&self.template_id)
            .ok_or_else(|| "area not found".to_string())?;
        match field {
            "id" => area.id = value.to_string(),
            "name" => area.name = value.to_string(),
            "description" => area.description = value.to_string(),
            "spawn_room" => area.spawn_room = value.to_string(),
            "flags" => area.flags = value.split(',').map(|s| s.trim().to_string()).collect(),
            "level_range" => {
                let parts: Vec<&str> = value.split(&['–', '-'][..]).collect();
                if parts.len() == 2 {
                    let lo: u8 = parts[0].trim().parse().map_err(|_| "invalid number")?;
                    let hi: u8 = parts[1].trim().parse().map_err(|_| "invalid number")?;
                    area.level_range = Some([lo, hi]);
                }
            }
            "weather_zone" => area.weather_zone = Some(value.to_string()),
            "reset_interval_secs" => {
                let secs: u64 = value.parse().map_err(|_| "invalid number")?;
                area.reset_interval = Some(ResetInterval { secs });
            }
            "credits" => area.credits = Some(value.to_string()),
            _ => return Err(format!("unknown field: {field}")),
        }
        Ok(())
    }

    fn update_room(&mut self, field: &str, value: &str) -> Result<(), String> {
        if field == "area" && !self.registry.areas.contains_key(value) {
            let valid: Vec<String> = self.registry.areas.keys().cloned().collect();
            return Err(format!(
                "area '{value}' does not exist. Valid areas: {}",
                valid.join(", ")
            ));
        }
        let room = self
            .registry
            .areas
            .values_mut()
            .find_map(|a| a.rooms.get_mut(&self.template_id))
            .ok_or_else(|| "room not found".to_string())?;
        match field {
            "area" => {
                room.area = value.to_string();
            }
            "name" => room.name = value.to_string(),
            "description" => room.description = value.to_string(),
            "flags" => room.flags = value.split(',').map(|s| s.trim().to_string()).collect(),
            _ if field.starts_with("exit.") => {
                let dir = field.trim_start_matches("exit.").to_string();
                room.exits.insert(dir, value.to_string());
            }
            _ if field.starts_with("portal[") => {
                let rest = field.trim_start_matches("portal[");
                let (idx_str, path_rest) = rest.split_once(']').ok_or("invalid portal path")?;
                let idx: usize = idx_str.parse().map_err(|_| "invalid index")?;
                if idx < room.portals.len() {
                    let p = &mut room.portals[idx];
                    if path_rest == ".keyword" {
                        p.keyword = value.to_string();
                    } else if path_rest == ".destination" {
                        p.dest = value.to_string();
                    } else if path_rest == ".description" {
                        p.description = value.to_string();
                    } else if path_rest == ".flags" {
                        p.flags = value.split(',').map(|s| s.trim().to_string()).collect();
                    }
                }
            }
            _ if field.starts_with("content.mobs[") => {
                let rest = field.trim_start_matches("content.mobs[");
                let (idx_str, path_rest) = rest.split_once(']').ok_or("invalid mob spawn path")?;
                let idx: usize = idx_str.parse().map_err(|_| "invalid index")?;
                if idx < room.content.mobs.len() {
                    let m = &mut room.content.mobs[idx];
                    if path_rest == ".template_id" {
                        m.template_id = value.to_string();
                    } else if path_rest == ".count" {
                        m.count = value.parse().map_err(|_| "invalid number")?;
                    } else if path_rest == ".respawn_secs" {
                        m.respawn_secs = Some(value.parse().map_err(|_| "invalid number")?);
                    }
                }
            }
            _ if field.starts_with("content.items[") => {
                let rest = field.trim_start_matches("content.items[");
                let (idx_str, path_rest) = rest.split_once(']').ok_or("invalid item spawn path")?;
                let idx: usize = idx_str.parse().map_err(|_| "invalid index")?;
                if idx < room.content.items.len() {
                    let it = &mut room.content.items[idx];
                    if path_rest == ".template_id" {
                        it.template_id = value.to_string();
                    } else if path_rest == ".count" {
                        it.count = value.parse().map_err(|_| "invalid number")?;
                    }
                }
            }
            _ => return Err(format!("unknown field: {field}")),
        }
        Ok(())
    }
}

impl EntityInspectorScreen {
    pub(super) fn is_editing(&self) -> bool {
        !matches!(self.edit_mode, EditMode::Idle)
    }
}

impl Screen for EntityInspectorScreen {
    fn name(&self) -> &str {
        if self.dirty {
            "Entity Inspector *"
        } else {
            "Entity Inspector"
        }
    }

    fn handle_key(&mut self, key: KeyEvent) -> bool {
        if let Some(ref mut dialog) = self.delete_dialog {
            if let Some(btn) = dialog.handle_key(key) {
                if btn == 0 {
                    // Cancel
                    self.delete_dialog = None;
                } else if btn == 1 {
                    // Confirm delete (convention: button[1] = destructive action)
                    if let Err(e) = self.delete_from_disk() {
                        tracing::error!(
                            "failed to delete {}/{}: {e}",
                            self.category,
                            self.template_id
                        );
                    }
                    self.deleted = true;
                }
            }
            return true;
        }
        match self.edit_mode {
            EditMode::Idle => self.handle_key_idle(key),
            EditMode::Text { .. } | EditMode::Number { .. } => self.handle_key_inline(key),
            EditMode::Multiline { .. } => self.handle_key_multiline(key),
            EditMode::Dropdown { .. } => self.handle_key_dropdown(key),
        }
        true
    }

    fn handle_mouse(&mut self, mouse: MouseEvent, area: Rect) {
        if let Some(ref mut dialog) = self.delete_dialog {
            if let Some(btn) = dialog.handle_mouse(mouse) {
                if btn == 0 {
                    self.delete_dialog = None;
                } else if btn == 1 {
                    if let Err(e) = self.delete_from_disk() {
                        tracing::error!(
                            "failed to delete {}/{}: {e}",
                            self.category,
                            self.template_id
                        );
                    }
                    self.deleted = true;
                }
            }
            return;
        }
        match self.edit_mode {
            EditMode::Idle => self.handle_mouse_idle(mouse, area),
            EditMode::Dropdown { .. } => self.handle_mouse_dropdown(mouse, area),
            _ => {}
        }
    }

    fn render(&mut self, area: Rect, buf: &mut Buffer, mouse_pos: Option<(u16, u16)>) {
        if area.width < 3 || area.height < 2 {
            return;
        }

        self.dropdown_rect = None;

        let info = format!(" {} > {} ", self.category, self.template_id);
        buf.set_string(
            area.x,
            area.y,
            &info,
            Style::default().fg(Color::Indexed(245)),
        );

        let content_lines = area.height.saturating_sub(2) as usize;
        self.table.update_scroll(content_lines);

        self.table.hovered = match self.edit_mode {
            EditMode::Dropdown { .. } => None,
            _ => mouse_pos.and_then(|(col, row)| {
                let table_top = area.y + 2;
                if row >= table_top
                    && row < table_top + content_lines as u16
                    && col >= area.x
                    && col < area.x + area.width
                {
                    let line = (row - table_top) as usize;
                    let idx = line + self.table.scroll.offset;
                    (idx < self.table.rows.len()).then_some(idx)
                } else {
                    None
                }
            }),
        };
        self.scrollbar = ScrollState {
            offset: self.table.scroll.offset,
            visible_lines: self.table.scroll.visible_lines,
            total_lines: self.table.scroll.total_lines,
        };

        self.table.muted = self.muted;
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

        match &self.edit_mode {
            EditMode::Text { row, cursor, .. } | EditMode::Number { row, cursor, .. } => {
                self.render_cursor(area, buf, *row, *cursor);
            }
            EditMode::Multiline {
                row,
                cursor,
                scroll,
                ..
            } => {
                self.render_multiline(area, buf, *row, *cursor, *scroll);
            }
            EditMode::Dropdown { row, selection, .. } => {
                // Recompute hover from current mouse_pos so it's render-cycle
                // accurate, not dependent on handle_mouse_dropdown timing.
                if let Some(rect) = self.dropdown_rect {
                    let inner_top = rect.y + 1;
                    let inner_bottom = rect.y + rect.height - 1;
                    self.dropdown_hovered = mouse_pos.and_then(|(col, row_pos)| {
                        let inside_inner = col > rect.x
                            && col < rect.x + rect.width - 1
                            && row_pos >= inner_top
                            && row_pos < inner_bottom;
                        if inside_inner {
                            let idx = self.dropdown_scroll + (row_pos - inner_top) as usize;
                            if let EditMode::Dropdown { options, .. } = &self.edit_mode {
                                (idx < options.len()).then_some(idx)
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    });
                }
                self.render_dropdown(area, buf, *row, *selection);
            }
            EditMode::Idle => {
                self.dropdown_hovered = None;
            }
        }

        if let Some(ref mut dialog) = self.delete_dialog {
            dialog.render(area, buf, mouse_pos);
        }
    }
}

// ---------------------------------------------------------------------------
// Idle-mode handlers
// ---------------------------------------------------------------------------
impl EntityInspectorScreen {
    fn handle_key_idle(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => self.table.select_prev(),
            KeyCode::Down | KeyCode::Char('j') => self.table.select_next(),
            KeyCode::Enter | KeyCode::Tab => {
                if let Some(row) = self.table.selected {
                    self.start_edit(row);
                }
            }
            KeyCode::Char('D') => {
                self.delete_dialog = Some(Dialog::new(
                    Color::Red,
                    "Confirm Delete",
                    &format!(
                        "Delete {} \"{}\"?",
                        singularize(&self.category),
                        self.template_id
                    ),
                    &["Cancel".to_string(), "Delete".to_string()],
                ));
            }
            _ => {}
        }
    }

    fn handle_mouse_idle(&mut self, mouse: MouseEvent, area: Rect) {
        match mouse.kind {
            MouseEventKind::ScrollUp => self.table.scroll_up(),
            MouseEventKind::ScrollDown => self.table.scroll_down(),
            MouseEventKind::Down(_) => {
                let row = (mouse.row as usize)
                    .saturating_sub(area.y as usize)
                    .saturating_sub(2)
                    .saturating_add(self.table.scroll.offset);
                if row < self.table.rows.len() {
                    self.table.selected = Some(row);
                    self.start_edit(row);
                }
            }
            _ => {}
        }
    }
}

// ---------------------------------------------------------------------------
// Inline (Text / Number) edit handlers
// ---------------------------------------------------------------------------
impl EntityInspectorScreen {
    fn handle_key_inline(&mut self, key: KeyEvent) {
        let m = key.modifiers;
        match key.code {
            KeyCode::Esc => self.cancel_edit(),
            KeyCode::Enter => self.commit_edit(),

            // Line navigation
            KeyCode::Home => self.cursor_home(),
            KeyCode::End => self.cursor_end(),
            KeyCode::Char('a') if m == KeyModifiers::CONTROL || m == KeyModifiers::SUPER => {
                self.cursor_home()
            }
            KeyCode::Char('e') if m == KeyModifiers::CONTROL || m == KeyModifiers::SUPER => {
                self.cursor_end()
            }

            // Word navigation
            KeyCode::Left if m == KeyModifiers::CONTROL => self.cursor_word_left(),
            KeyCode::Right if m == KeyModifiers::CONTROL => self.cursor_word_right(),
            KeyCode::Char('b') if m == KeyModifiers::ALT => self.cursor_word_left(),
            KeyCode::Char('f') if m == KeyModifiers::ALT => self.cursor_word_right(),

            // Plain arrows
            KeyCode::Left => self.cursor_left(),
            KeyCode::Right => self.cursor_right(),

            // Line deletion
            KeyCode::Backspace if m == KeyModifiers::SUPER => self.delete_to_home(),
            KeyCode::Delete if m == KeyModifiers::SUPER => self.delete_to_end(),
            KeyCode::Char('u') if m == KeyModifiers::CONTROL => self.delete_to_home(),
            KeyCode::Char('k') if m == KeyModifiers::CONTROL => self.delete_to_end(),

            // Word deletion
            KeyCode::Char('w') if m == KeyModifiers::CONTROL => self.delete_word_backward(),
            KeyCode::Backspace if m == KeyModifiers::ALT => self.delete_word_backward(),
            KeyCode::Char('d') if m == KeyModifiers::ALT => self.delete_word_forward(),

            // Single char deletion
            KeyCode::Backspace => self.backspace_char(),
            KeyCode::Delete => self.delete_char(),

            KeyCode::Char(c) => self.insert_char(c),
            _ => {}
        }
    }

    fn render_cursor(&mut self, area: Rect, buf: &mut Buffer, row: usize, cursor: usize) {
        let visible_row = row.saturating_sub(self.table.scroll.offset);
        if visible_row >= self.table.scroll.visible_lines {
            return;
        }

        let value_col_x = area.x + 2 + self.table.col_x(1) + 1;
        let y = area.y + 1 + 1 + visible_row as u16;

        if y >= area.y + area.height {
            return;
        }

        let value = &self.table.rows[row][1];
        let c_cursor = cursor.min(value.len());
        let x = value_col_x + c_cursor as u16;

        if x >= area.x + area.width {
            return;
        }

        let show = (self.cursor_char / 8) % 2 == 0;
        if show {
            buf.set_string(x, y, "▎", Style::default().fg(Color::Cyan));
        }
        self.cursor_char = self.cursor_char.wrapping_add(1);
    }
}

// ---------------------------------------------------------------------------
// Multiline edit handlers
// ---------------------------------------------------------------------------
impl EntityInspectorScreen {
    fn handle_key_multiline(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => self.cancel_edit(),
            KeyCode::F(2) => self.commit_edit(),
            KeyCode::Enter => self.multiline_newline(),
            KeyCode::Backspace => self.backspace_char(),
            KeyCode::Delete => self.delete_char(),
            KeyCode::Left => self.cursor_left(),
            KeyCode::Right => self.cursor_right(),
            KeyCode::Home => self.cursor_home(),
            KeyCode::End => self.cursor_end(),
            KeyCode::Up => self.multiline_scroll_up(),
            KeyCode::Down => self.multiline_scroll_down(),
            KeyCode::Char(c) => self.insert_char(c),
            _ => {}
        }
    }

    fn multiline_scroll_up(&mut self) {
        if let EditMode::Multiline { scroll, .. } = &mut self.edit_mode {
            *scroll = scroll.saturating_sub(1);
        }
    }

    fn multiline_scroll_down(&mut self) {
        if let EditMode::Multiline { scroll, .. } = &mut self.edit_mode {
            *scroll += 1;
        }
    }

    fn render_multiline(
        &mut self,
        area: Rect,
        buf: &mut Buffer,
        _row: usize,
        cursor: usize,
        scroll: usize,
    ) {
        let value = match &self.edit_mode {
            EditMode::Multiline { row, .. } => &self.table.rows[*row][1],
            _ => return,
        };

        let box_width = area.width.clamp(20, 60);
        let box_height = area.height.clamp(6, 18);
        let x = area.x + (area.width.saturating_sub(box_width)) / 2;
        let y = area.y + (area.height.saturating_sub(box_height)) / 2;

        let overlay = Rect::new(x, y, box_width, box_height);
        let block = Block::default()
            .title(" Edit (Esc=cancel, F2=save) ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow));
        let inner = block.inner(overlay);
        block.render(overlay, buf);

        let inner_width = inner.width.saturating_sub(1) as usize;
        if inner_width == 0 || inner.height < 2 {
            return;
        }

        let lines = wrap_text(value, inner_width);
        let total_lines = lines.len();
        let max_scroll = total_lines.saturating_sub(inner.height as usize);
        let effective_scroll = scroll.min(max_scroll);

        let visible_lines = lines
            .iter()
            .skip(effective_scroll)
            .take(inner.height as usize);

        let cursor_vpos = cursor_visual_pos(value, cursor, inner_width);
        let cursor_visual_row = cursor_vpos.0;
        let cursor_visual_col = cursor_vpos.1;

        let cursor_visible = cursor_visual_row >= effective_scroll
            && cursor_visual_row < effective_scroll + inner.height as usize;

        for (i, line) in visible_lines.enumerate() {
            let y = inner.y + i as u16;
            if y >= inner.y + inner.height {
                break;
            }
            buf.set_string(inner.x, y, line, Style::default().fg(Color::White));
        }

        if cursor_visible {
            let render_row = cursor_visual_row.saturating_sub(effective_scroll);
            let y = inner.y + render_row as u16;
            let x = inner.x + cursor_visual_col as u16;
            if x < inner.x + inner.width && y < inner.y + inner.height {
                let show = (self.cursor_char / 8) % 2 == 0;
                if show {
                    buf.set_string(x, y, "▎", Style::default().fg(Color::Cyan));
                }
                self.cursor_char = self.cursor_char.wrapping_add(1);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Dropdown edit handlers
// ---------------------------------------------------------------------------
impl EntityInspectorScreen {
    fn handle_key_dropdown(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => self.cancel_edit(),
            KeyCode::Enter => self.commit_edit(),
            KeyCode::Up | KeyCode::Char('k') => {
                if let EditMode::Dropdown { selection, .. } = &mut self.edit_mode {
                    *selection = selection.saturating_sub(1);
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if let EditMode::Dropdown {
                    selection, options, ..
                } = &mut self.edit_mode
                {
                    let max = options.len().saturating_sub(1);
                    *selection = (*selection + 1).min(max);
                }
            }
            KeyCode::Char(c) if c.is_ascii_alphanumeric() => {
                if let EditMode::Dropdown {
                    selection, options, ..
                } = &mut self.edit_mode
                {
                    if let Some(pos) = options
                        .iter()
                        .position(|o| o.starts_with(c))
                        .filter(|&p| p != *selection)
                    {
                        *selection = pos;
                    }
                }
            }
            _ => {}
        }
    }

    fn handle_mouse_dropdown(&mut self, mouse: MouseEvent, _area: Rect) {
        let rect = match self.dropdown_rect {
            Some(r) => r,
            None => return,
        };

        let col = mouse.column;
        let row = mouse.row;
        let inside_inner = col > rect.x
            && col < rect.x + rect.width - 1
            && row > rect.y
            && row < rect.y + rect.height - 1;

        match mouse.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                if inside_inner {
                    if let EditMode::Dropdown {
                        selection, options, ..
                    } = &mut self.edit_mode
                    {
                        let clicked_idx = self.dropdown_scroll + (row - (rect.y + 1)) as usize;
                        if clicked_idx < options.len() {
                            *selection = clicked_idx;
                            self.commit_edit();
                        }
                    }
                } else {
                    self.cancel_edit();
                }
            }
            MouseEventKind::ScrollUp => {
                if let EditMode::Dropdown { selection, .. } = &mut self.edit_mode {
                    *selection = selection.saturating_sub(1);
                }
            }
            MouseEventKind::ScrollDown => {
                if let EditMode::Dropdown {
                    selection, options, ..
                } = &mut self.edit_mode
                {
                    let max = options.len().saturating_sub(1);
                    *selection = (*selection + 1).min(max);
                }
            }
            _ => {}
        }
    }

    fn render_dropdown(&mut self, area: Rect, buf: &mut Buffer, row: usize, selection: usize) {
        let options = match &self.edit_mode {
            EditMode::Dropdown { options, .. } => options.clone(),
            _ => return,
        };

        if options.is_empty() {
            return;
        }

        let visible_row = row.saturating_sub(self.table.scroll.offset);
        let row_y = area.y + 1 + 1 + visible_row as u16;

        let max_width = options
            .iter()
            .map(|o| o.len() + 4)
            .max()
            .unwrap_or(20)
            .max(10) as u16;

        let box_height = (options.len() as u16).min(10).saturating_add(2);
        let mut box_y = row_y + 1;
        if box_y + box_height > area.y + area.height {
            box_y = row_y.saturating_sub(box_height);
        }

        let box_x = area.x + 2 + self.table.col_x(1);
        let overlay = Rect::new(box_x, box_y, max_width, box_height);
        self.dropdown_rect = Some(overlay);

        render_dropdown_box(buf, overlay, Style::default().fg(Color::Cyan));

        let visible_count = (overlay.height.saturating_sub(2)) as usize;
        let scroll = selection.saturating_sub(visible_count.saturating_sub(1));
        self.dropdown_scroll = scroll;

        for i in 0..visible_count {
            let idx = scroll + i;
            if idx >= options.len() {
                break;
            }
            let y = overlay.y + 1 + i as u16;
            let is_selected = idx == selection;
            let is_hovered = self.dropdown_hovered == Some(idx);
            let highlighted = is_selected || is_hovered;

            if highlighted {
                highlight_dropdown_row(buf, overlay, y);
            }

            let item_style = dropdown_item_style(highlighted);
            let prefix = if is_selected { "▸ " } else { "  " };
            let text = format!("{prefix}{}", options[idx]);
            buf.set_string(overlay.x + 2, y, &text, item_style);
        }
    }

    // end of EntityInspectorScreen
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------
fn dropdown_options(field: &str) -> Option<Vec<&'static str>> {
    match field {
        "item_type" => Some(vec![
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
        ]),
        "quality" => Some(vec![
            "common",
            "uncommon",
            "rare",
            "epic",
            "legendary",
            "artifact",
        ]),
        "ai_mode" => Some(vec![
            "idle",
            "wander",
            "patrol",
            "aggressive",
            "guard",
            "flee",
        ]),
        "size" => Some(vec!["tiny", "small", "medium", "large", "huge", "giant"]),
        "equipment.slot" | "slot" => Some(vec![
            "head", "neck", "torso", "back", "arms", "hands", "finger", "legs", "feet", "shield",
            "weapon", "ranged", "ammo", "wrist", "waist",
        ]),
        "weapon.damage_type" | "damage_type" => Some(vec![
            "slashing",
            "piercing",
            "bludgeoning",
            "fire",
            "cold",
            "lightning",
            "acid",
            "poison",
            "holy",
            "shadow",
            "arcane",
        ]),
        "weapon.range" | "range" => Some(vec!["melee", "short", "medium", "long", "very_long"]),
        "type" => Some(vec!["prefix", "suffix"]),
        "quality_min" => Some(vec!["common", "uncommon", "rare", "epic", "legendary"]),
        _ => None,
    }
}

pub fn singularize(s: &str) -> String {
    if let Some(stripped) = s.strip_suffix("ies") {
        format!("{}y", stripped)
    } else if let Some(stripped) = s
        .strip_suffix("ses")
        .or_else(|| s.strip_suffix("xes"))
        .or_else(|| s.strip_suffix("ches"))
        .or_else(|| s.strip_suffix("shes"))
    {
        stripped.to_string()
    } else if s.ends_with('s') && !s.ends_with("ss") {
        s[..s.len() - 1].to_string()
    } else {
        s.to_string()
    }
}

fn wrap_text(text: &str, max_width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    for line in text.lines() {
        if line.is_empty() {
            lines.push(String::new());
            continue;
        }
        let mut remaining = line;
        while !remaining.is_empty() {
            let chunk_width = max_width.min(remaining.len());
            let (chunk, rest) = remaining.split_at(chunk_width);
            lines.push(chunk.to_string());
            remaining = rest;
        }
    }
    lines
}

fn cursor_visual_pos(text: &str, cursor: usize, line_width: usize) -> (usize, usize) {
    let prefix = &text[..cursor.min(text.len())];
    let lines_before = prefix.matches('\n').count();
    let last_newline = prefix.rfind('\n');
    let col = match last_newline {
        Some(pos) => prefix.len() - pos - 1,
        None => prefix.len(),
    };
    let visual_row = lines_before + col / line_width;
    let visual_col = col % line_width;
    (visual_row, visual_col)
}
