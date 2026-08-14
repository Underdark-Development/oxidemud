use std::path::PathBuf;

use oxide_core::templates::TemplateRegistry;
use ratatui::{
    buffer::Buffer,
    crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind},
    layout::{Constraint, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Widget},
};

use super::entities::remove_from_registry;
use super::Screen;
use crate::components::dropdown::{
    dropdown_item_style, highlight_dropdown_row, render_dropdown_box,
};
use crate::components::{Dialog, ScrollState, Table};
use crate::content::FileMap;

mod affixes;
mod areas;
mod classes;
mod factions;
mod items;
mod mobs;
mod passives;
mod quests;
mod races;
mod recipes;
mod rooms;
mod sets;
mod skills;
mod stances;

pub mod raw_editor;
use raw_editor::{RawTomlEditor, ViewMode};

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
    pub view_mode: ViewMode,
    pub raw_editor: RawTomlEditor,
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
            view_mode: ViewMode::Structured,
            raw_editor: RawTomlEditor::new(),
        };
        screen.load_table();
        screen.original_path = screen.resolve_save_path().ok();
        screen
    }

    pub fn validate_entity(&self) -> Vec<String> {
        let mut errors = Vec::new();

        let room_exists = |room_id: &str| -> bool {
            if room_id.is_empty() {
                return true;
            }
            self.registry
                .areas
                .values()
                .any(|a| a.rooms.contains_key(room_id))
        };

        match self.category.as_str() {
            "rooms" => {
                if let Some(room) = self
                    .registry
                    .areas
                    .values()
                    .find_map(|a| a.rooms.get(&self.template_id))
                {
                    // Validate exits
                    for (dir, exit) in &room.exits {
                        let dest = exit.dest();
                        if !dest.is_empty() && !room_exists(dest) {
                            errors.push(format!(
                                "exit.{dir}: destination room '{dest}' does not exist"
                            ));
                        }
                    }
                    // Validate portals
                    for (i, portal) in room.portals.iter().enumerate() {
                        if !portal.dest.is_empty() && !room_exists(&portal.dest) {
                            errors.push(format!(
                                "portal[{i}]: destination room '{}' does not exist",
                                portal.dest
                            ));
                        }
                    }
                    // Validate mob spawns
                    for (i, spawn) in room.content.mobs.iter().enumerate() {
                        if !spawn.template_id.is_empty()
                            && !self.registry.mobs.contains_key(&spawn.template_id)
                        {
                            errors.push(format!(
                                "content.mobs[{i}]: mob template '{}' does not exist",
                                spawn.template_id
                            ));
                        }
                    }
                    // Validate item spawns
                    for (i, spawn) in room.content.items.iter().enumerate() {
                        if !spawn.template_id.is_empty()
                            && !self.registry.items.contains_key(&spawn.template_id)
                        {
                            errors.push(format!(
                                "content.items[{i}]: item template '{}' does not exist",
                                spawn.template_id
                            ));
                        }
                    }
                }
            }
            "items" => {
                if let Some(item) = self.registry.items.get(&self.template_id) {
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
                    if !item.item_type.is_empty() && !valid_types.contains(&item.item_type.as_str())
                    {
                        errors.push(format!("item_type: invalid item type '{}'", item.item_type));
                    }
                    let valid_qualities = [
                        "common",
                        "uncommon",
                        "rare",
                        "epic",
                        "legendary",
                        "artifact",
                    ];
                    if !item.quality.is_empty() && !valid_qualities.contains(&item.quality.as_str())
                    {
                        errors.push(format!("quality: invalid quality '{}'", item.quality));
                    }
                    if let Some(ref w) = item.weapon {
                        let valid_hands = [
                            "one_hand",
                            "two_hand",
                            "one_or_two_hand",
                            "onehand",
                            "twohand",
                            "one_hand_or_two_hand",
                        ];
                        if !w.hands.is_empty()
                            && !valid_hands.contains(&w.hands.to_lowercase().as_str())
                        {
                            errors.push(format!("weapon.hands: invalid hands mode '{}'", w.hands));
                        }
                        let valid_dmg = [
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
                        ];
                        if !w.damage_type.is_empty() && !valid_dmg.contains(&w.damage_type.as_str())
                        {
                            errors.push(format!(
                                "weapon.damage_type: invalid damage type '{}'",
                                w.damage_type
                            ));
                        }
                    }
                    if let Some(ref eq) = item.equipment {
                        let valid_slots = [
                            "head", "neck", "torso", "back", "arms", "hands", "finger", "legs",
                            "feet", "shield", "weapon", "ranged", "ammo", "wrist", "waist",
                        ];
                        if !eq.slot.is_empty() && !valid_slots.contains(&eq.slot.as_str()) {
                            errors.push(format!("equipment.slot: invalid slot '{}'", eq.slot));
                        }
                    }
                }
            }
            "mobs" => {
                if let Some(mob) = self.registry.mobs.get(&self.template_id) {
                    let valid_modes = ["idle", "wander", "patrol", "aggressive", "guard", "flee"];
                    if !mob.ai_mode.is_empty() && !valid_modes.contains(&mob.ai_mode.as_str()) {
                        errors.push(format!("ai_mode: invalid AI mode '{}'", mob.ai_mode));
                    }
                    let valid_sizes = ["tiny", "small", "medium", "large", "huge", "giant"];
                    if !mob.size.is_empty() && !valid_sizes.contains(&mob.size.as_str()) {
                        errors.push(format!("size: invalid size '{}'", mob.size));
                    }
                    for entry in &mob.equipment {
                        if !entry.template_id.is_empty()
                            && !self.registry.items.contains_key(&entry.template_id)
                        {
                            errors.push(format!(
                                "equipment.{}: item template '{}' does not exist",
                                entry.slot, entry.template_id
                            ));
                        }
                    }
                    for (i, entry) in mob.loot.entries.iter().enumerate() {
                        if !entry.item.is_empty() && !self.registry.items.contains_key(&entry.item)
                        {
                            errors.push(format!(
                                "loot.entries[{i}]: item template '{}' does not exist",
                                entry.item
                            ));
                        }
                    }
                    for room_id in &mob.patrol_route {
                        if !room_exists(room_id) {
                            errors.push(format!("patrol_route: room '{room_id}' does not exist"));
                        }
                    }
                }
            }
            _ => {}
        }

        errors
    }

    pub fn validate_raw_editor(&mut self) {
        let category = self.category.clone();
        self.raw_editor
            .validate_with_schema(|content| match category.as_str() {
                "items" => {
                    toml::from_str::<oxide_core::templates::ItemTemplate>(content).map(|_| ())
                }
                "mobs" => toml::from_str::<oxide_core::templates::MobTemplate>(content).map(|_| ()),
                "quests" => toml::from_str::<oxide_core::templates::QuestDef>(content).map(|_| ()),
                "recipes" => {
                    toml::from_str::<oxide_core::templates::RecipeDef>(content).map(|_| ())
                }
                "factions" => {
                    toml::from_str::<oxide_core::templates::FactionDef>(content).map(|_| ())
                }
                "areas" => {
                    toml::from_str::<oxide_core::templates::AreaTemplate>(content).map(|_| ())
                }
                "rooms" => {
                    toml::from_str::<oxide_core::templates::RoomTemplate>(content).map(|_| ())
                }
                "races" => {
                    toml::from_str::<oxide_core::templates::RaceTemplate>(content).map(|_| ())
                }
                "classes" => {
                    toml::from_str::<oxide_core::templates::ClassTemplate>(content).map(|_| ())
                }
                _ => Ok(()),
            });
        if self.raw_editor.error.is_none() {
            let sem_errors = self.validate_entity();
            if let Some(err) = sem_errors.first() {
                self.raw_editor.error = Some(raw_editor::RawEditorError {
                    line: 0,
                    col: 0,
                    message: err.clone(),
                    is_syntax: false,
                });
            }
        }
    }

    pub fn toggle_view_mode(&mut self) {
        match self.view_mode {
            ViewMode::Structured => {
                if let Ok(toml_str) = self.serialize_registry_data() {
                    self.raw_editor.set_content(&toml_str);
                    self.validate_raw_editor();
                }
                self.view_mode = ViewMode::RawToml;
            }
            ViewMode::RawToml => {
                let toml_str = self.raw_editor.to_string_content();
                match self.validate_toml(&toml_str) {
                    Ok(_) => {
                        super::entities::insert_draft_into_registry(
                            &mut self.registry,
                            &self.category,
                            &self.template_id,
                            &toml_str,
                        );
                        self.load_table();
                        self.view_mode = ViewMode::Structured;
                    }
                    Err(e) => {
                        self.raw_editor.error = Some(raw_editor::RawEditorError {
                            line: 0,
                            col: 0,
                            message: format!("Cannot switch to Form mode: {e}"),
                            is_syntax: false,
                        });
                    }
                }
            }
        }
    }

    pub fn duplicate_entity(&mut self) {
        if let Ok(toml_str) = self.serialize_registry_data() {
            let base_id = self.template_id.clone();
            let mut suffix_idx = 1;
            let mut new_id = format!("{}_copy", base_id);
            while template_exists_in_registry(&self.registry, &self.category, &new_id) {
                new_id = format!("{}_copy_{}", base_id, suffix_idx);
                suffix_idx += 1;
            }
            super::entities::insert_draft_into_registry(
                &mut self.registry,
                &self.category,
                &new_id,
                &toml_str,
            );
            self.template_id = new_id;
            self.dirty = true;
            self.is_draft = true;
            self.load_table();
        }
    }

    fn load_table(&mut self) {
        let mut table = Table::new(vec!["Field".into(), "Value".into()]);

        match self.category.as_str() {
            "items" => self.load_items(&mut table),
            "mobs" => self.load_mobs(&mut table),
            "quests" => self.load_quests(&mut table),
            "factions" => self.load_factions(&mut table),
            "recipes" => self.load_recipes(&mut table),
            "races" => self.load_races(&mut table),
            "classes" => self.load_classes(&mut table),
            "skills" => self.load_skills(&mut table),
            "stances" => self.load_stances(&mut table),
            "sets" => self.load_sets(&mut table),
            "affixes" => self.load_affixes(&mut table),
            "passives" => self.load_passives(&mut table),
            "areas" => self.load_areas(&mut table),
            "rooms" => self.load_rooms(&mut table),
            _ => {
                table.add_row(vec![
                    "error".into(),
                    format!("unknown category: {}", self.category),
                ]);
            }
        }

        let max_field_len = table.rows.iter().map(|r| r[0].len()).max().unwrap_or(20);
        let first_col_width = (max_field_len as u16).max(20);
        table.column_widths = vec![Constraint::Length(first_col_width), Constraint::Fill(1)];

        if table.rows.is_empty() {
            table.add_row(vec!["message".into(), "template not found".into()]);
        }

        table.selected = Some(0);
        self.table = table;
    }

    pub(super) fn add_field(table: &mut Table, field: &str, value: impl std::fmt::Display) {
        table.add_row(vec![field.to_string(), value.to_string()]);
    }

    pub(super) fn add_array_header(table: &mut Table, name: &str, count: usize) {
        let val_str = if count == 0 {
            "(array, 0 items)   [ + Add Entry ]".to_string()
        } else {
            let item_str = if count == 1 { "item" } else { "items" };
            format!("(array, {count} {item_str})   [ + Add Entry ]   [ 🗑 Clear ]")
        };
        table.add_row(vec![name.to_string(), val_str]);
    }

    pub(super) fn add_array_item(table: &mut Table, field: &str, value: impl std::fmt::Display) {
        table.add_row(vec![format!("  {field}"), format!("{value}   [ ✕ ]")]);
    }

    fn detect_field_kind(&self, row: usize) -> EditMode {
        let value = &self.table.rows[row][1];
        let clean_field = self.table.rows[row][0].trim();

        if value.starts_with("(array")
            || value.contains("[ + Add Entry ]")
            || value.contains("press + to add")
        {
            return EditMode::Idle;
        }

        if clean_field == "description"
            || clean_field.ends_with(".description")
            || clean_field.ends_with(".script")
            || value.len() > 50
        {
            return EditMode::Multiline {
                row,
                cursor: value.len(),
                scroll: 0,
                original: value.clone(),
            };
        }

        if let Some(options) = dropdown_options(clean_field) {
            let sel = options.iter().position(|o| o == value).unwrap_or(0);
            return EditMode::Dropdown {
                row,
                selection: sel,
                options: options.iter().map(|&s| s.to_string()).collect(),
                original: value.clone(),
            };
        }

        let is_number = matches!(
            clean_field
                .rsplit('.')
                .next()
                .unwrap_or(clean_field)
                .trim_end_matches(']'),
            "level"
                | "weight"
                | "value"
                | "speed"
                | "chance"
                | "min"
                | "max"
                | "max_items"
                | "capacity_weight"
                | "charges"
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
        if row < self.table.rows.len() {
            let val = self.table.rows[row][1].clone();
            if let Some(pos) = val.rfind("   [ ✕ ]") {
                self.table.rows[row][1] = val[..pos].to_string();
            }
        }
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

        let field = self.table.rows[row][0].trim().to_string();
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
    pub fn save_to_disk(&mut self) -> Result<(), String> {
        let toml_str = self.serialize_registry_data().map_err(|e| e.to_string())?;

        // Round-trip validation: try to parse the serialized TOML back through the concrete struct.
        self.validate_toml(&toml_str)?;

        let old_id = self.template_id.clone();
        let new_id = self.get_internal_id().unwrap_or_else(|| old_id.clone());

        let old_path = self.resolve_save_path_for_id(&old_id)?;
        let new_path = self.resolve_save_path_for_id(&new_id)?;

        if !self.is_draft && old_path != new_path {
            if self.category == "areas" {
                if let (Some(old_parent), Some(new_parent)) = (old_path.parent(), new_path.parent())
                {
                    if old_parent.exists() {
                        if let Some(grandparent) = new_parent.parent() {
                            std::fs::create_dir_all(grandparent)
                                .map_err(|e| format!("cannot create dir: {e}"))?;
                        }
                        std::fs::rename(old_parent, new_parent)
                            .map_err(|e| format!("failed to rename area dir: {e}"))?;
                    }
                }
            } else {
                if old_path.exists() {
                    let _ = std::fs::remove_file(&old_path);
                }
            }
        }

        if let Some(parent) = new_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("cannot create dirs: {e}"))?;
        }
        std::fs::write(&new_path, &toml_str).map_err(|e| format!("write failed: {e}"))?;

        // Update the inspector's local registry mapping if the ID changed
        if old_id != new_id {
            match self.category.as_str() {
                "items" => {
                    if let Some(t) = self.registry.items.remove(&old_id) {
                        self.registry.items.insert(new_id.clone(), t);
                    }
                }
                "mobs" => {
                    if let Some(t) = self.registry.mobs.remove(&old_id) {
                        self.registry.mobs.insert(new_id.clone(), t);
                    }
                }
                "races" => {
                    if let Some(t) = self.registry.races.remove(&old_id) {
                        self.registry.races.insert(new_id.clone(), t);
                    }
                }
                "classes" => {
                    if let Some(t) = self.registry.classes.remove(&old_id) {
                        self.registry.classes.insert(new_id.clone(), t);
                    }
                }
                "skills" => {
                    if let Some(t) = self.registry.skills.remove(&old_id) {
                        self.registry.skills.insert(new_id.clone(), t);
                    }
                }
                "stances" => {
                    if let Some(t) = self.registry.stances.remove(&old_id) {
                        self.registry.stances.insert(new_id.clone(), t);
                    }
                }
                "sets" => {
                    if let Some(t) = self.registry.sets.remove(&old_id) {
                        self.registry.sets.insert(new_id.clone(), t);
                    }
                }
                "affixes" => {
                    if let Some(t) = self.registry.affixes.remove(&old_id) {
                        self.registry.affixes.insert(new_id.clone(), t);
                    }
                }
                "passives" => {
                    if let Some(t) = self.registry.passives.remove(&old_id) {
                        self.registry.passives.insert(new_id.clone(), t);
                    }
                }
                "areas" => {
                    if let Some(t) = self.registry.areas.remove(&old_id) {
                        self.registry.areas.insert(new_id.clone(), t);
                    }
                }
                "rooms" => {
                    for area in self.registry.areas.values_mut() {
                        if let Some(room) = area.rooms.remove(&old_id) {
                            area.rooms.insert(new_id.clone(), room);
                            break;
                        }
                    }
                }
                _ => {}
            }
            self.template_id = new_id.clone();
            self.original_path = Some(new_path);
        }

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

    pub(crate) fn resolve_save_path(&self) -> Result<std::path::PathBuf, String> {
        self.resolve_save_path_for_id(&self.template_id)
    }

    fn resolve_save_path_for_id(&self, target_id: &str) -> Result<std::path::PathBuf, String> {
        let lookup_key = if self.category == "rooms" {
            let area_id = self
                .registry
                .areas
                .values()
                .find_map(|a| a.rooms.get(&self.template_id))
                .map(|r| r.area.as_str())
                .ok_or_else(|| format!("cannot find area for room {}", self.template_id))?;
            format!("{}:{}", area_id, self.template_id)
        } else {
            self.template_id.clone()
        };

        if let Some(path) = self
            .file_map
            .get(&self.category)
            .and_then(|m| m.get(&lookup_key))
        {
            let mut new_path = path.clone();
            if self.category == "areas" {
                if let Some(parent) = path.parent() {
                    if let Some(grandparent) = parent.parent() {
                        return Ok(grandparent.join(target_id).join("area.toml"));
                    }
                }
            } else {
                new_path.set_file_name(format!("{target_id}.toml"));
                return Ok(new_path);
            }
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
                .join(format!("{target_id}.toml")))
        } else if self.category == "areas" {
            Ok(cp.join("areas").join(target_id).join("area.toml"))
        } else {
            let dir = cp.join(&self.category);
            Ok(dir.join(format!("{target_id}.toml")))
        }
    }

    fn get_internal_id(&self) -> Option<String> {
        match self.category.as_str() {
            "items" => self
                .registry
                .items
                .get(&self.template_id)
                .map(|i| i.id.clone()),
            "mobs" => self
                .registry
                .mobs
                .get(&self.template_id)
                .map(|m| m.id.clone()),
            "races" => self
                .registry
                .races
                .get(&self.template_id)
                .map(|r| r.id.clone()),
            "classes" => self
                .registry
                .classes
                .get(&self.template_id)
                .map(|c| c.id.clone()),
            "skills" => self
                .registry
                .skills
                .get(&self.template_id)
                .map(|s| s.id.clone()),
            "stances" => self
                .registry
                .stances
                .get(&self.template_id)
                .map(|s| s.id.clone()),
            "sets" => self
                .registry
                .sets
                .get(&self.template_id)
                .map(|s| s.id.clone()),
            "affixes" => self
                .registry
                .affixes
                .get(&self.template_id)
                .map(|a| a.id.clone()),
            "passives" => self
                .registry
                .passives
                .get(&self.template_id)
                .map(|p| p.id.clone()),
            "areas" => self
                .registry
                .areas
                .get(&self.template_id)
                .map(|a| a.id.clone()),
            "rooms" => self
                .registry
                .areas
                .values()
                .find_map(|a| a.rooms.get(&self.template_id))
                .map(|r| r.id.clone()),
            _ => None,
        }
    }

    fn validate_toml(&self, toml_str: &str) -> Result<(), String> {
        match self.category.as_str() {
            "items" => {
                toml::from_str::<oxide_core::templates::ItemTemplate>(toml_str)
                    .map_err(|e| format!("invalid ItemTemplate: {e}"))?;
            }
            "mobs" => {
                toml::from_str::<oxide_core::templates::MobTemplate>(toml_str)
                    .map_err(|e| format!("invalid MobTemplate: {e}"))?;
            }
            "races" => {
                toml::from_str::<oxide_core::templates::RaceTemplate>(toml_str)
                    .map_err(|e| format!("invalid RaceTemplate: {e}"))?;
            }
            "classes" => {
                toml::from_str::<oxide_core::templates::ClassTemplate>(toml_str)
                    .map_err(|e| format!("invalid ClassTemplate: {e}"))?;
            }
            "skills" => {
                toml::from_str::<oxide_core::SkillDef>(toml_str)
                    .map_err(|e| format!("invalid SkillDef: {e}"))?;
            }
            "stances" => {
                toml::from_str::<oxide_core::templates::StanceDef>(toml_str)
                    .map_err(|e| format!("invalid StanceDef: {e}"))?;
            }
            "sets" => {
                toml::from_str::<oxide_core::templates::SetDef>(toml_str)
                    .map_err(|e| format!("invalid SetDef: {e}"))?;
            }
            "affixes" => {
                toml::from_str::<oxide_core::templates::AffixDef>(toml_str)
                    .map_err(|e| format!("invalid AffixDef: {e}"))?;
            }
            "passives" => {
                toml::from_str::<oxide_core::templates::PassiveDef>(toml_str)
                    .map_err(|e| format!("invalid PassiveDef: {e}"))?;
            }
            "areas" => {
                toml::from_str::<oxide_core::templates::AreaTemplate>(toml_str)
                    .map_err(|e| format!("invalid AreaTemplate: {e}"))?;
            }
            "rooms" => {
                toml::from_str::<oxide_core::templates::RoomTemplate>(toml_str)
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
    pub fn apply_changes(&mut self, registry: &mut TemplateRegistry) {
        let old_id = self.template_id.clone();
        let new_id = self.get_internal_id().unwrap_or_else(|| old_id.clone());

        if old_id != new_id {
            remove_from_registry(registry, &self.category, &old_id);
            self.template_id = new_id.clone();
        }

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
                if let Some(room) = self
                    .registry
                    .areas
                    .values()
                    .find_map(|a| a.rooms.get(template_id))
                    .cloned()
                {
                    let target_id = room.area.clone();
                    for area in registry.areas.values_mut() {
                        area.rooms.remove(&old_id);
                        area.rooms.remove(template_id);
                    }
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
            "items" => self.update_items(field, value),
            "mobs" => self.update_mobs(field, value),
            "quests" => self.update_quests(field, value),
            "factions" => self.update_factions(field, value),
            "recipes" => self.update_recipes(field, value),
            "races" => self.update_races(field, value),
            "classes" => self.update_classes(field, value),
            "skills" => self.update_skills(field, value),
            "stances" => self.update_stances(field, value),
            "sets" => self.update_sets(field, value),
            "affixes" => self.update_affixes(field, value),
            "passives" => self.update_passives(field, value),
            "areas" => self.update_areas(field, value),
            "rooms" => self.update_rooms(field, value),
            _ => Err(format!("unknown category: {}", self.category)),
        }
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

    fn modal_overlay_active(&self) -> bool {
        self.delete_dialog.is_some()
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

        if (key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('e'))
            || key.code == KeyCode::F(12)
        {
            self.toggle_view_mode();
            return true;
        }

        if self.view_mode == ViewMode::RawToml {
            let handled = self.raw_editor.handle_key(key);
            self.validate_raw_editor();
            if self.raw_editor.dirty {
                self.dirty = true;
            }
            return handled;
        }

        match self.edit_mode {
            EditMode::Idle => self.handle_key_idle(key),
            EditMode::Text { .. } | EditMode::Number { .. } => self.handle_key_inline(key),
            EditMode::Multiline { .. } => self.handle_key_multiline(key),
            EditMode::Dropdown { .. } => self.handle_key_dropdown(key),
        }
        true
    }

    fn unsaved_count(&self) -> usize {
        if self.dirty {
            1
        } else {
            0
        }
    }

    fn syntax_error_count(&self) -> usize {
        if self.view_mode == ViewMode::RawToml {
            if let Some(ref err) = self.raw_editor.error {
                if err.is_syntax {
                    return 1;
                }
            }
        }
        0
    }

    fn validation_error_count(&self) -> usize {
        if self.view_mode == ViewMode::RawToml {
            if let Some(ref err) = self.raw_editor.error {
                if !err.is_syntax {
                    return 1;
                }
            }
            0
        } else {
            self.validate_entity().len()
        }
    }

    fn handle_command_action(
        &mut self,
        action: &crate::components::CommandAction,
    ) -> Result<bool, String> {
        if action == &crate::components::CommandAction::ToggleViewMode {
            self.toggle_view_mode();
            return Ok(true);
        }
        Ok(false)
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
        if self.view_mode == ViewMode::RawToml {
            let inner_area = Rect::new(
                area.x,
                area.y + 1,
                area.width,
                area.height.saturating_sub(1),
            );
            self.raw_editor.handle_mouse(mouse, inner_area);
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

        let mode_tab = match self.view_mode {
            ViewMode::Structured => "[Form]  Raw TOML (Ctrl+E)",
            ViewMode::RawToml => "Form  [Raw TOML (Ctrl+E)]",
        };
        let val_errors = self.validate_entity();
        let err_suffix = if let Some(err) = val_errors.first() {
            format!("  │  ⚠ {}", err)
        } else {
            String::new()
        };
        let dirty_suffix = if self.dirty || self.raw_editor.dirty {
            " *"
        } else {
            ""
        };
        let info = format!(
            " {} > {}{}  │  {}{}",
            self.category, self.template_id, dirty_suffix, mode_tab, err_suffix
        );
        let header_style = if val_errors.is_empty() {
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(ratatui::style::Modifier::BOLD)
        } else {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(ratatui::style::Modifier::BOLD)
        };
        buf.set_string(area.x, area.y, &info, header_style);

        let inner_area = Rect::new(
            area.x,
            area.y + 1,
            area.width,
            area.height.saturating_sub(1),
        );

        if self.view_mode == ViewMode::RawToml {
            self.raw_editor.render(inner_area, buf, !self.muted);
            return;
        }

        self.table.row_errors.clear();

        // 1. TOML Syntax / Schema Errors
        if let Some(ref toml_err) = self.raw_editor.error {
            let msg = toml_err.message.clone();
            for (row_idx, row) in self.table.rows.iter().enumerate() {
                let f_name = row[0].trim();
                if f_name.len() > 1 && msg.contains(f_name) {
                    self.table.row_errors.insert(
                        row_idx,
                        crate::components::table::RowErrorInfo {
                            message: msg.clone(),
                            is_toml: true,
                        },
                    );
                }
            }
        }

        // 2. Semantic Validation Errors (TOML errors take precedence)
        for err in &val_errors {
            if let Some(colon_pos) = err.find(':') {
                let err_field = &err[..colon_pos];
                for (row_idx, row) in self.table.rows.iter().enumerate() {
                    let field_name = row[0].trim();
                    let is_match = field_name == err_field
                        || field_name.ends_with(err_field)
                        || err_field.ends_with(field_name)
                        || err_field.split('.').next_back() == Some(field_name);
                    if is_match {
                        self.table.row_errors.entry(row_idx).or_insert_with(|| {
                            crate::components::table::RowErrorInfo {
                                message: err.clone(),
                                is_toml: false,
                            }
                        });
                    }
                }
            }
        }

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
        self.table.render_table(table_area, buf);

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
        use ratatui::crossterm::event::KeyModifiers;

        if key.modifiers.contains(KeyModifiers::CONTROL)
            || key.modifiers.contains(KeyModifiers::SUPER)
        {
            match key.code {
                KeyCode::Char('d') | KeyCode::Char('D') => {
                    self.duplicate_entity();
                    return;
                }
                _ => {}
            }
        }

        match key.code {
            KeyCode::Up | KeyCode::Char('k') => self.table.select_prev(),
            KeyCode::Down | KeyCode::Char('j') => self.table.select_next(),
            KeyCode::Home => self.table.select_first(),
            KeyCode::End => self.table.select_last(),
            KeyCode::PageUp => self.table.page_up(),
            KeyCode::PageDown => self.table.page_down(),
            KeyCode::Enter | KeyCode::Tab => {
                if let Some(row) = self.table.selected {
                    self.start_edit(row);
                }
            }
            KeyCode::Char('+') | KeyCode::Char('=') => {
                if let Some(row) = self.table.selected {
                    let field = &self.table.rows[row][0];
                    if let Some((prefix, idx)) = parse_array_field(field) {
                        match self.add_array_entry(&prefix, idx) {
                            Ok(_) => {
                                self.dirty = true;
                                self.load_table();
                                self.table.selected = Some(row);
                            }
                            Err(e) => {
                                tracing::error!("failed to add array entry: {e}");
                            }
                        }
                    }
                }
            }
            KeyCode::Char('-') => {
                if let Some(row) = self.table.selected {
                    let field = &self.table.rows[row][0];
                    if let Some((prefix, idx)) = parse_array_field(field) {
                        match self.remove_array_entry(&prefix, idx) {
                            Ok(_) => {
                                self.dirty = true;
                                self.load_table();
                                if let Some(ref mut sel) = self.table.selected {
                                    if *sel >= self.table.rows.len() {
                                        self.table.selected =
                                            self.table.rows.len().checked_sub(1).or(Some(0));
                                    }
                                }
                            }
                            Err(e) => {
                                tracing::error!("failed to remove array entry: {e}");
                            }
                        }
                    }
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

    fn add_array_entry(&mut self, prefix: &str, index: usize) -> Result<(), String> {
        match self.category.as_str() {
            "items" => self.add_item_array(prefix, index),
            "mobs" => self.add_mob_array(prefix, index),
            "quests" => self.add_quest_array(prefix, index),
            "factions" => self.add_faction_array(prefix, index),
            "recipes" => self.add_recipe_array(prefix, index),
            "areas" => self.add_area_array(prefix, index),
            "rooms" => self.add_room_array(prefix, index),
            "races" => self.add_race_array(prefix, index),
            "classes" => self.add_class_array(prefix, index),
            _ => Err(format!(
                "adding array entries for category {} not supported",
                self.category
            )),
        }
    }

    fn remove_array_entry(&mut self, prefix: &str, index: usize) -> Result<(), String> {
        match self.category.as_str() {
            "items" => self.remove_item_array(prefix, index),
            "mobs" => self.remove_mob_array(prefix, index),
            "quests" => self.remove_quest_array(prefix, index),
            "factions" => self.remove_faction_array(prefix, index),
            "recipes" => self.remove_recipe_array(prefix, index),
            "areas" => self.remove_area_array(prefix, index),
            "rooms" => self.remove_room_array(prefix, index),
            "races" => self.remove_race_array(prefix, index),
            "classes" => self.remove_class_array(prefix, index),
            _ => Err(format!(
                "removing array entries for category {} not supported",
                self.category
            )),
        }
    }

    fn clear_array(&mut self, prefix: &str) -> Result<(), String> {
        match self.category.as_str() {
            "items" => self.clear_item_array(prefix),
            "mobs" => self.clear_mob_array(prefix),
            "quests" => self.clear_quest_array(prefix),
            "factions" => self.clear_faction_array(prefix),
            "recipes" => self.clear_recipe_array(prefix),
            "areas" => self.clear_area_array(prefix),
            "rooms" => self.clear_room_array(prefix),
            "races" => self.clear_race_array(prefix),
            "classes" => self.clear_class_array(prefix),
            _ => Err(format!(
                "clearing array for category {} not supported",
                self.category
            )),
        }
    }

    fn swap_array_entries(&mut self, prefix: &str, i1: usize, i2: usize) -> Result<(), String> {
        match self.category.as_str() {
            "items" => self.swap_item_array(prefix, i1, i2),
            _ => Ok(()),
        }
    }

    fn handle_mouse_idle(&mut self, mouse: MouseEvent, area: Rect) {
        match mouse.kind {
            MouseEventKind::ScrollUp => self.table.scroll_up(),
            MouseEventKind::ScrollDown => self.table.scroll_down(),
            MouseEventKind::Down(MouseButton::Left) => {
                let row = (mouse.row as usize)
                    .saturating_sub(area.y as usize)
                    .saturating_sub(2)
                    .saturating_add(self.table.scroll.offset);
                if row < self.table.rows.len() {
                    let field = self.table.rows[row][0].clone();
                    let val = self.table.rows[row][1].clone();
                    let col1_x = area.x + 2 + self.table.col_x(1, area);
                    let relative_x = (mouse.column as usize).saturating_sub(col1_x as usize);

                    // Check if click was on array header buttons: [ + Add Entry ] or [ 🗑 Clear ]
                    if val.contains("[ + Add Entry ]") || val.contains("[ 🗑 Clear ]") {
                        if let Some(start_add) = val.find("[ + Add Entry ]") {
                            let end_add = start_add + "[ + Add Entry ]".len();
                            if relative_x >= start_add && relative_x < end_add {
                                let clean_field = field.trim();
                                let prefix = clean_field.trim_end_matches("[]").to_string();
                                let idx = self
                                    .table
                                    .rows
                                    .iter()
                                    .filter(|r| {
                                        parse_array_field(r[0].trim())
                                            .map(|(p, _)| p == prefix)
                                            .unwrap_or(false)
                                    })
                                    .count()
                                    .saturating_sub(1);
                                if let Err(e) = self.add_array_entry(&prefix, idx) {
                                    tracing::error!("add array entry error: {e}");
                                } else {
                                    self.dirty = true;
                                    self.load_table();
                                }
                                self.table.selected = Some(row);
                                return;
                            }
                        }
                        if let Some(start_clear) = val.find("[ 🗑 Clear ]") {
                            let end_clear = start_clear + "[ 🗑 Clear ]".len();
                            if relative_x >= start_clear && relative_x < end_clear {
                                let clean_field = field.trim();
                                let prefix = clean_field.trim_end_matches("[]").to_string();
                                if let Err(e) = self.clear_array(&prefix) {
                                    tracing::error!("clear array error: {e}");
                                } else {
                                    self.dirty = true;
                                    self.load_table();
                                }
                                self.table.selected = Some(row);
                                return;
                            }
                        }
                    }

                    if val.contains("[ ▲ ]") {
                        if let Some(start_up) = val.find("[ ▲ ]") {
                            let end_up = start_up + "[ ▲ ]".len();
                            if relative_x >= start_up && relative_x < end_up {
                                let clean_field = field.trim();
                                if let Some((prefix, idx)) = parse_array_field(clean_field) {
                                    if idx > 0
                                        && self.swap_array_entries(&prefix, idx, idx - 1).is_ok()
                                    {
                                        self.dirty = true;
                                        self.load_table();
                                        self.table.selected = Some(row.saturating_sub(1));
                                    }
                                    return;
                                }
                            }
                        }
                    }
                    if val.contains("[ ▼ ]") {
                        if let Some(start_dn) = val.find("[ ▼ ]") {
                            let end_dn = start_dn + "[ ▼ ]".len();
                            if relative_x >= start_dn && relative_x < end_dn {
                                let clean_field = field.trim();
                                if let Some((prefix, idx)) = parse_array_field(clean_field) {
                                    if self.swap_array_entries(&prefix, idx, idx + 1).is_ok() {
                                        self.dirty = true;
                                        self.load_table();
                                        self.table.selected = Some(row + 1);
                                    }
                                    return;
                                }
                            }
                        }
                    }

                    // Check if click was on array item delete button: [ ✕ ]
                    if val.contains("[ ✕ ]") {
                        if let Some(start_del) = val.rfind("[ ✕ ]") {
                            let end_del = start_del + "[ ✕ ]".len();
                            if relative_x >= start_del && relative_x < end_del {
                                let clean_field = field.trim();
                                if let Some((prefix, idx)) = parse_array_field(clean_field) {
                                    if let Err(e) = self.remove_array_entry(&prefix, idx) {
                                        tracing::error!("remove array entry error: {e}");
                                    } else {
                                        self.dirty = true;
                                        self.load_table();
                                    }
                                    self.table.selected = Some(row.saturating_sub(1));
                                    return;
                                }
                            }
                        }
                    }

                    let was_already_selected = self.table.selected == Some(row);
                    self.table.selected = Some(row);
                    if was_already_selected {
                        self.start_edit(row);
                    }
                }
            }
            _ => {}
        }
    }

    // ---------------------------------------------------------------------------
    // Inline (Text / Number) edit handlers
    // ---------------------------------------------------------------------------
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
            KeyCode::Left if m.contains(KeyModifiers::CONTROL) => self.cursor_word_left(),
            KeyCode::Right if m.contains(KeyModifiers::CONTROL) => self.cursor_word_right(),
            KeyCode::Char('b') if m.contains(KeyModifiers::ALT) => self.cursor_word_left(),
            KeyCode::Char('f') if m.contains(KeyModifiers::ALT) => self.cursor_word_right(),

            // Plain arrows
            KeyCode::Left => self.cursor_left(),
            KeyCode::Right => self.cursor_right(),

            // Line deletion
            KeyCode::Backspace if m.contains(KeyModifiers::SUPER) => self.delete_to_home(),
            KeyCode::Delete if m.contains(KeyModifiers::SUPER) => self.delete_to_end(),
            KeyCode::Char('u') if m.contains(KeyModifiers::CONTROL) => self.delete_to_home(),
            KeyCode::Char('k') if m.contains(KeyModifiers::CONTROL) => self.delete_to_end(),

            // Word deletion
            KeyCode::Char('w') if m.contains(KeyModifiers::CONTROL) => self.delete_word_backward(),
            KeyCode::Backspace if m.contains(KeyModifiers::ALT) => self.delete_word_backward(),
            KeyCode::Char('d') if m.contains(KeyModifiers::ALT) => self.delete_word_forward(),

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

        let value_col_x = area.x + 2 + self.table.col_x(1, area) + 1;
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

        let max_option_len = options.iter().map(|o| o.len()).max().unwrap_or(10);
        let col_width = self.table.col_x(1, area);
        let max_width = ((max_option_len + 7) as u16)
            .max(col_width)
            .max(25)
            .min(area.width.saturating_sub(4));

        let box_height = (options.len() as u16).min(10).saturating_add(2);
        let mut box_y = row_y + 1;
        if box_y + box_height > area.y + area.height {
            box_y = row_y.saturating_sub(box_height);
        }

        let mut box_x = area.x + 2 + self.table.col_x(1, area);
        if box_x + max_width > area.x + area.width {
            box_x = (area.x + area.width).saturating_sub(max_width);
        }
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
        "weapon.hands" | "hands" => Some(vec!["one_hand", "two_hand", "one_or_two_hand"]),
        "type" => Some(vec!["prefix", "suffix"]),
        "quality_min" => Some(vec!["common", "uncommon", "rare", "epic", "legendary"]),
        "friendly" | "banker" | "wander_area" | "aggro_players" | "is_locked" | "is_opaque"
        | "allow_revive" => Some(vec!["true", "false"]),
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

fn parse_array_field(field: &str) -> Option<(String, usize)> {
    let open_bracket = field.find('[')?;
    let close_bracket = field.find(']')?;
    if close_bracket > open_bracket {
        let prefix = field[..open_bracket].to_string();
        let idx_str = &field[open_bracket + 1..close_bracket];
        if idx_str.is_empty() {
            return Some((prefix, 0));
        } else if let Ok(idx) = idx_str.parse::<usize>() {
            return Some((prefix, idx));
        }
    }
    None
}

fn template_exists_in_registry(registry: &TemplateRegistry, category: &str, id: &str) -> bool {
    match category {
        "rooms" => registry.areas.values().any(|a| a.rooms.contains_key(id)),
        "mobs" => registry.mobs.contains_key(id),
        "items" => registry.items.contains_key(id),
        "quests" => registry.quests.contains_key(id),
        "recipes" => registry.recipes.contains_key(id),
        "factions" => registry.factions.contains_key(id),
        "races" => registry.races.contains_key(id),
        "classes" => registry.classes.contains_key(id),
        "areas" => registry.areas.contains_key(id),
        "skills" => registry.skills.contains_key(id),
        "stances" => registry.stances.contains_key(id),
        "sets" => registry.sets.contains_key(id),
        "affixes" => registry.affixes.contains_key(id),
        "passives" => registry.passives.contains_key(id),
        _ => false,
    }
}
