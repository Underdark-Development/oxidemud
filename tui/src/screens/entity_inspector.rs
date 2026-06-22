use std::path::PathBuf;

use mud_core::templates::TemplateRegistry;
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

mod affixes;
mod areas;
mod classes;
mod items;
mod mobs;
mod passives;
mod races;
mod rooms;
mod sets;
mod skills;
mod stances;

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
            "items" => self.load_items(&mut table),
            "mobs" => self.load_mobs(&mut table),
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

        if table.rows.is_empty() {
            table.add_row(vec!["message".into(), "template not found".into()]);
        }

        table.selected = Some(0);
        self.table = table;
    }

    pub(super) fn add_field(table: &mut Table, field: &str, value: impl std::fmt::Display) {
        table.add_row(vec![field.to_string(), value.to_string()]);
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
            "items" => self.update_items(field, value),
            "mobs" => self.update_mobs(field, value),
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

        let box_x = area.x + 2 + self.table.col_x(1, area);
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
