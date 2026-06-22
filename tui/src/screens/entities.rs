use mud_core::templates::TemplateRegistry;
use mud_core::templates::{
    AffixDef, AppearanceBounds, AreaTemplate, ClassAttributeMods, ClassTemplate, DeityPolicy,
    HealthBounds, ItemTemplate, LootTable, MobTemplate, PassiveDef, RaceAttributes, RaceTemplate,
    RoomContent, RoomTemplate, SetDef, StanceDef, WalletAmount,
};
use ratatui::{
    buffer::Buffer,
    crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind},
    layout::Rect,
    style::{Color, Modifier as RatModifier, Style},
    widgets::{Block, Borders, Widget},
};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use super::{entity_inspector::EntityInspectorScreen, EntityContext, Screen};
use crate::components::{CommandAction, ScrollState, Tree};
use crate::content::{self, FileMap};

mod tree_builder;
use mud_core::format::preview;
use mud_core::format::RichText;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Tree,
    Detail,
}

#[derive(Debug, Clone)]
pub struct NodeInfo {
    pub category: String,
    pub id: String,
}

struct PreviewState {
    title: String,
    content: RichText,
}

pub struct EntitiesScreen {
    tree: Tree<NodeInfo>,
    content_path: PathBuf,
    registry: TemplateRegistry,
    file_map: FileMap,
    scrollbar: ScrollState,
    last_click: Option<(Instant, usize)>,
    detail: Option<EntityInspectorScreen>,
    focus: Focus,
    previous_focus: Focus,
    sidebar_focused: bool,
    show_help: bool,
    search: Option<String>,
    search_focus: bool,
    preview: Option<PreviewState>,
    /// (category, id) -> raw TOML content for entities not yet written to disk.
    draft_data: HashMap<(String, String), String>,
    /// Entities with in-memory edits not yet saved to disk.
    unsaved: HashSet<(String, String)>,
}

impl EntitiesScreen {
    pub fn new(content_path: PathBuf) -> Self {
        let (registry, file_map) = content::load_templates(&content_path);
        Self::new_shared(content_path, registry, file_map)
    }

    pub fn new_shared(
        content_path: PathBuf,
        registry: TemplateRegistry,
        file_map: FileMap,
    ) -> Self {
        let mut screen = EntitiesScreen {
            tree: Tree::new(Vec::new()),
            content_path,
            registry,
            file_map,
            scrollbar: ScrollState::new(),
            last_click: None,
            detail: None,
            focus: Focus::Tree,
            previous_focus: Focus::Tree,
            sidebar_focused: false,
            show_help: false,
            search: None,
            search_focus: false,
            preview: None,
            draft_data: HashMap::new(),
            unsaved: HashSet::new(),
        };
        screen.rebuild_tree();
        screen
    }

    pub fn reload(&mut self) {
        let saved_drafts = std::mem::take(&mut self.draft_data);
        let (mut registry, file_map) = content::load_templates(&self.content_path);
        // Re-insert draft entities into the new registry
        for ((cat, id), toml_str) in &saved_drafts {
            insert_draft_into_registry(&mut registry, cat, id, toml_str);
        }
        self.registry = registry;
        self.file_map = file_map;
        self.draft_data = saved_drafts;
        self.unsaved.clear();
        self.detail = None;
        self.focus = Focus::Tree;
        self.show_help = false;
        self.search = None;
        self.search_focus = false;
        self.rebuild_tree();
    }

    pub fn registry(&self) -> &TemplateRegistry {
        &self.registry
    }

    fn tree_width_pct(&self, area_width: u16) -> u16 {
        (area_width * 25 / 100).clamp(20, 35)
    }

    fn open_detail(&mut self, category: String, template_id: String) {
        let is_draft = self
            .draft_data
            .contains_key(&(category.clone(), template_id.clone()));
        let detail = EntityInspectorScreen::new(
            self.registry.clone(),
            category,
            template_id,
            self.file_map.clone(),
            if is_draft {
                Some(self.content_path.clone())
            } else {
                None
            },
            is_draft,
        );
        self.detail = Some(detail);
        self.focus = Focus::Detail;
    }

    fn create_new_entity(&mut self, category: &str, context_id: Option<&str>) {
        let category = normalize_category(category);
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0);
        let stem = category.strip_suffix('s').unwrap_or(category);
        let id = format!("{stem}_{ts}");

        let toml_str = match generate_default_content(category, &id) {
            Ok(s) => s,
            Err(e) => {
                tracing::error!("failed to generate default content for {category}/{id}: {e}");
                return;
            }
        };

        // For rooms, directly insert into the parent area instead of
        // insert_draft_into_registry (which can't handle area-less rooms).
        if category == "rooms" {
            if let Some(area_id) = context_id {
                if let Ok(mut room) = toml::from_str::<mud_core::templates::RoomTemplate>(&toml_str)
                {
                    room.area = area_id.to_string();
                    if let Ok(new_toml) = toml::to_string_pretty(&room) {
                        if let Some(area) = self.registry.areas.get_mut(area_id) {
                            area.rooms.insert(id.clone(), room);
                            self.draft_data
                                .insert((category.to_string(), id.clone()), new_toml);
                        }
                    }
                }
            }
        } else {
            insert_draft_into_registry(&mut self.registry, category, &id, &toml_str);
            self.draft_data
                .insert((category.to_string(), id.clone()), toml_str);
        }

        self.rebuild_tree();
        self.open_detail(category.to_string(), id);
    }

    fn handle_search_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.search = None;
                self.search_focus = false;
                self.rebuild_tree();
            }
            KeyCode::Enter => {
                self.search_focus = false;
            }
            KeyCode::Backspace => {
                if let Some(ref mut s) = self.search {
                    s.pop();
                    if s.is_empty() {
                        self.search = None;
                        self.search_focus = false;
                    }
                }
                self.rebuild_tree();
            }
            KeyCode::Char(c) if !c.is_control() => {
                self.search.get_or_insert_with(String::new).push(c);
                self.rebuild_tree();
            }
            _ => {}
        }
    }

    fn handle_tree_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Up => self.tree.select_prev(),
            KeyCode::Down => self.tree.select_next(),
            KeyCode::Enter => {
                if let Some(data) = self.tree.selected_data() {
                    if !data.id.is_empty() {
                        self.open_detail(data.category.clone(), data.id.clone());
                    } else if let Some(idx) = self.tree.selected {
                        if let Some((_, node)) = self.tree.flatten().get(idx) {
                            if !node.is_leaf() {
                                self.tree.toggle_selected();
                            }
                        }
                    }
                }
            }
            KeyCode::Right => {
                if let Some(idx) = self.tree.selected {
                    if let Some((_, node)) = self.tree.flatten().get(idx) {
                        if !node.is_leaf() && node.collapsed {
                            self.tree.toggle_selected();
                        }
                    }
                }
            }
            KeyCode::Left => {
                if let Some(idx) = self.tree.selected {
                    if let Some((_, node)) = self.tree.flatten().get(idx) {
                        if !node.is_leaf() && !node.collapsed {
                            self.tree.toggle_selected();
                        }
                    }
                }
            }
            KeyCode::Char('r') => self.reload(),
            KeyCode::Char('n') => {
                let node_info = self.tree.selected_data().cloned();
                if let Some(info) = node_info {
                    if info.id.is_empty() {
                        self.create_new_entity("items", None);
                    } else if info.category == "areas" || info.category == "rooms" {
                        let area_id = if info.category == "areas" {
                            Some(info.id.clone())
                        } else {
                            self.registry
                                .areas
                                .iter()
                                .find(|(_, a)| a.rooms.contains_key(&info.id))
                                .map(|(id, _)| id.clone())
                        };
                        if let Some(aid) = area_id {
                            self.create_new_entity("rooms", Some(&aid));
                        }
                    } else {
                        self.create_new_entity(&info.category, None);
                    }
                }
            }
            KeyCode::Char('/') => {
                self.search_focus = true;
                self.search.get_or_insert_with(String::new);
            }
            _ => {}
        }
    }

    fn handle_detail_key(&mut self, key: KeyEvent) {
        if let Some(ref mut detail) = self.detail {
            detail.handle_key(key);
            if detail.deleted {
                let cat = detail.category.clone();
                let id = detail.template_id.clone();
                let is_draft = detail.is_draft;
                self.detail = None;
                self.unsaved.remove(&(cat.clone(), id.clone()));
                if is_draft {
                    // Remove from registry and drafts without reloading disk
                    self.draft_data.remove(&(cat.clone(), id.clone()));
                    remove_from_registry(&mut self.registry, &cat, &id);
                    self.rebuild_tree();
                } else {
                    self.reload();
                }
                self.focus = Focus::Tree;
            }
        }
    }

    fn handle_tree_mouse(&mut self, mouse: MouseEvent, area: Rect, _tree_width: u16) {
        match mouse.kind {
            MouseEventKind::ScrollUp => self.tree.scroll_up(),
            MouseEventKind::ScrollDown => self.tree.scroll_down(),
            MouseEventKind::Down(MouseButton::Left) => {
                let tree_area_top = area.y + 1;
                let row_in_tree = mouse.row.saturating_sub(tree_area_top) as usize;
                let idx = row_in_tree.saturating_add(self.tree.scroll.offset);
                if idx < self.tree.flatten().len() {
                    if let Some((t, last_idx)) = self.last_click {
                        if last_idx == idx && t.elapsed() < Duration::from_millis(400) {
                            self.last_click = None;
                            self.tree.selected = Some(idx);
                            self.tree.toggle_selected();
                            return;
                        }
                    }
                    self.last_click = Some((Instant::now(), idx));
                    self.tree.selected = Some(idx);
                    if let Some(data) = self.tree.selected_data() {
                        if !data.id.is_empty() {
                            self.open_detail(data.category.clone(), data.id.clone());
                        }
                    }
                }
            }
            _ => {}
        }
    }

    fn render_help(&self, area: Rect, buf: &mut Buffer) {
        let help_text = vec![
            " Keyboard shortcuts ",
            "",
            "  ?          Toggle help",
            "  Esc        Close detail / help",
            "  Tab        Focus: tree \u{2194} detail",
            "  Shift+Tab  Reverse focus",
            "  \u{2191}/\u{2193}        Navigate current pane",
            "  \u{2192}/\u{2190}        Expand / collapse node",
            "  Enter      Open entity detail",
            "  /          Search/filter tree",
            "",
            "  Ctrl+S     Save entity",
            "  Ctrl+R     Reload content",
            "  Ctrl+B     Toggle sidebar",
            "  Ctrl+1-9   Switch screens",
            "  Ctrl+D     Quit",
            "",
            " Mouse ",
            "",
            "  Scroll     Scroll tree or table",
            "  Click      Select tree node",
            "  Double-clk Open detail / toggle",
        ];

        let help_width = 40u16;
        let help_height = help_text.len() as u16 + 2;
        let help_x = area.x + (area.width.saturating_sub(help_width)) / 2;
        let help_y = area.y + (area.height.saturating_sub(help_height)) / 2;

        for y in area.y..area.y + area.height {
            for x in area.x..area.x + area.width {
                if let Some(cell) = buf.cell_mut((x, y)) {
                    cell.set_style(Style::default().fg(Color::Indexed(245)));
                }
            }
        }

        let help_area = Rect::new(help_x, help_y, help_width, help_height);
        let help_block = Block::default()
            .title(" Help ")
            .borders(Borders::ALL)
            .style(Style::default().bg(Color::Black).fg(Color::White));
        help_block.render(help_area, buf);

        for (i, line) in help_text.iter().enumerate() {
            let ly = help_y + 1 + i as u16;
            if ly < help_y + help_height - 1 {
                buf.set_string(help_x + 2, ly, line, Style::default().fg(Color::White));
            }
        }
    }

    fn rich_text_to_ratatui_style(seg: &mud_core::format::Segment) -> Style {
        use mud_core::format::Color as CoreColor;
        let fg = match seg.fg {
            CoreColor::Default => ratatui::style::Color::Reset,
            CoreColor::Black => ratatui::style::Color::Black,
            CoreColor::Red => ratatui::style::Color::Red,
            CoreColor::Green => ratatui::style::Color::Green,
            CoreColor::Yellow => ratatui::style::Color::Yellow,
            CoreColor::Blue => ratatui::style::Color::Blue,
            CoreColor::Magenta => ratatui::style::Color::Magenta,
            CoreColor::Cyan => ratatui::style::Color::Cyan,
            CoreColor::White => ratatui::style::Color::White,
            CoreColor::BrightBlack => ratatui::style::Color::Indexed(245),
            CoreColor::BrightRed => ratatui::style::Color::LightRed,
            CoreColor::BrightGreen => ratatui::style::Color::LightGreen,
            CoreColor::BrightYellow => ratatui::style::Color::LightYellow,
            CoreColor::BrightBlue => ratatui::style::Color::LightBlue,
            CoreColor::BrightMagenta => ratatui::style::Color::LightMagenta,
            CoreColor::BrightCyan => ratatui::style::Color::LightCyan,
            CoreColor::BrightWhite => ratatui::style::Color::White,
            CoreColor::Indexed(i) => ratatui::style::Color::Indexed(i),
        };
        let mut style = Style::default().fg(fg);
        let mut mods = RatModifier::default();
        if seg.modifiers.has(mud_core::format::Modifier::BOLD) {
            mods |= RatModifier::BOLD;
        }
        if seg.modifiers.has(mud_core::format::Modifier::ITALIC) {
            mods |= RatModifier::ITALIC;
        }
        if seg.modifiers.has(mud_core::format::Modifier::DIM) {
            mods |= RatModifier::DIM;
        }
        if seg.modifiers.has(mud_core::format::Modifier::UNDERLINE) {
            mods |= RatModifier::UNDERLINED;
        }
        if seg.modifiers.has(mud_core::format::Modifier::BLINK) {
            mods |= RatModifier::SLOW_BLINK;
        }
        style = style.add_modifier(mods);
        style
    }

    fn rich_text_to_buf(rt: &RichText, buf: &mut Buffer, x: u16, y: u16, max_w: u16, max_h: u16) {
        let mut cx = x;
        let mut cy = y;
        for seg in rt.segments() {
            let style = Self::rich_text_to_ratatui_style(seg);
            for ch in seg.text.chars() {
                if ch == '\n' {
                    cy += 1;
                    cx = x;
                    if cy >= y + max_h {
                        return;
                    }
                    continue;
                }
                if cx >= x + max_w {
                    cy += 1;
                    cx = x;
                    if cy >= y + max_h {
                        return;
                    }
                }
                if let Some(cell) = buf.cell_mut((cx, cy)) {
                    cell.set_char(ch);
                    cell.set_style(style);
                }
                cx += 1;
            }
        }
    }

    fn show_preview(&mut self, title: String, content: RichText) {
        self.preview = Some(PreviewState { title, content });
    }

    fn render_preview(&self, area: Rect, buf: &mut Buffer) {
        let preview = match self.preview {
            Some(ref p) => p,
            None => return,
        };

        let plain_text = preview.content.as_plain();
        let line_count = plain_text.lines().count().max(1);
        let max_content_w = plain_text
            .lines()
            .map(|l| l.len())
            .max()
            .unwrap_or(40)
            .min(60) as u16;
        let content_h = (line_count as u16).min(area.height.saturating_sub(6));
        let modal_w = (max_content_w + 4)
            .min(area.width.saturating_sub(4))
            .max(20);
        let modal_h = (content_h + 4).min(area.height.saturating_sub(4)).max(5);
        let modal_x = area.x + (area.width.saturating_sub(modal_w)) / 2;
        let modal_y = area.y + (area.height.saturating_sub(modal_h)) / 2;

        // Dim background
        for y in area.y..area.y + area.height {
            for x in area.x..area.x + area.width {
                if let Some(cell) = buf.cell_mut((x, y)) {
                    cell.set_style(Style::default().fg(Color::Indexed(245)));
                }
            }
        }

        let modal_area = Rect::new(modal_x, modal_y, modal_w, modal_h);
        let title_str = format!(" {} ", preview.title);
        let block = ratatui::widgets::Block::default()
            .title(title_str)
            .borders(ratatui::widgets::Borders::ALL)
            .style(Style::default().bg(Color::Black).fg(Color::White));
        block.render(modal_area, buf);

        let inner_x = modal_x + 2;
        let inner_y = modal_y + 1;
        let inner_w = modal_w.saturating_sub(4).max(1);
        let inner_h = modal_h.saturating_sub(2).max(1);

        Self::rich_text_to_buf(&preview.content, buf, inner_x, inner_y, inner_w, inner_h);

        // Dismiss hint
        if inner_y + content_h + 1 < modal_y + modal_h - 1 {
            let hint_y = modal_y + modal_h - 2;
            buf.set_string(
                modal_x + 2,
                hint_y,
                "Esc/Enter to dismiss",
                Style::default().fg(Color::Indexed(245)),
            );
        }
    }

    fn selected_entity_name(&self) -> Option<String> {
        let data = self.tree.selected_data()?;
        if data.id.is_empty() {
            return None;
        }
        if data.category == "areas" {
            return self.registry.areas.get(&data.id).map(|a| a.name.clone());
        }
        if data.category == "rooms" {
            for area in self.registry.areas.values() {
                if let Some(room) = area.rooms.get(&data.id) {
                    return Some(room.name.clone());
                }
            }
            return None;
        }
        match data.category.as_str() {
            "mobs" => self.registry.mobs.get(&data.id).map(|m| m.name.clone()),
            "items" => self.registry.items.get(&data.id).map(|i| i.name.clone()),
            "races" => self.registry.races.get(&data.id).map(|r| r.name.clone()),
            "classes" => self.registry.classes.get(&data.id).map(|c| c.name.clone()),
            "skills" => self.registry.skills.get(&data.id).map(|s| s.name.clone()),
            "stances" => self.registry.stances.get(&data.id).map(|s| s.name.clone()),
            "sets" => self.registry.sets.get(&data.id).map(|s| s.name.clone()),
            "affixes" => self.registry.affixes.get(&data.id).map(|a| a.name.clone()),
            "passives" => self.registry.passives.get(&data.id).map(|p| p.name.clone()),
            _ => None,
        }
    }
}

fn normalize_category(category: &str) -> &str {
    const MAP: &[(&str, &str)] = &[
        ("class", "classes"),
        ("race", "races"),
        ("skill", "skills"),
        ("stance", "stances"),
        ("set", "sets"),
        ("affix", "affixes"),
        ("passive", "passives"),
        ("item", "items"),
        ("mob", "mobs"),
        ("room", "rooms"),
        ("area", "areas"),
    ];
    for (s, p) in MAP {
        if *s == category {
            return p;
        }
    }
    category
}

fn generate_default_content(
    category: &str,
    id: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let content = match category {
        "items" => toml::to_string_pretty(&ItemTemplate {
            id: id.to_string(),
            name: id.to_string(),
            description: String::new(),
            item_type: "misc".to_string(),
            subtype: String::new(),
            quality: "common".to_string(),
            level_requirement: 1,
            weight: 1.0,
            value: 0,
            flags: Vec::new(),
            allowed_classes: Vec::new(),
            allowed_races: Vec::new(),
            allowed_alignments: Vec::new(),
            requires_skill: None,
            weapon: None,
            equipment: None,
            set: None,
            triggers: Vec::new(),
        })?,
        "mobs" => toml::to_string_pretty(&MobTemplate {
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
            aggro_range: 0,
            aggro_players: false,
            aggro_mobs: false,
            aggro_race: Vec::new(),
            faction: None,
            faction_standing: 0,
            trainer_types: Vec::new(),
            languages: Vec::new(),
            shop: None,
            friendly: false,
            skills: Vec::new(),
            scripts: Vec::new(),
        })?,
        "races" => toml::to_string_pretty(&RaceTemplate {
            id: id.to_string(),
            name: id.to_string(),
            description: String::new(),
            attributes: RaceAttributes::default(),
            allowed_classes: Vec::new(),
            allowed_alignments: Vec::new(),
            racial_abilities: Vec::new(),
            allowed_genders: HashMap::new(),
            appearance_bounds: AppearanceBounds::default(),
            age_default: 20,
            age_max: 100,
        })?,
        "classes" => toml::to_string_pretty(&ClassTemplate {
            id: id.to_string(),
            name: id.to_string(),
            description: String::new(),
            hit_die: 8,
            attribute_mods: ClassAttributeMods::default(),
            bab: "poor".to_string(),
            fort_save: "poor".to_string(),
            ref_save: "poor".to_string(),
            will_save: "poor".to_string(),
            allowed_races: Vec::new(),
            allowed_alignments: Vec::new(),
            auto_skills: Vec::new(),
            skill_pool: Vec::new(),
            starting_skill_slots: 3,
            starting_items: Vec::new(),
            starting_gold: WalletAmount::default(),
            deity_policy: DeityPolicy::Any,
        })?,
        "skills" => toml::to_string_pretty(&mud_core::SkillDef {
            id: id.to_string(),
            name: id.to_string(),
            description: String::new(),
            skill_type: mud_core::SkillType::Combat,
            max_rank: 100,
        })?,
        "stances" => toml::to_string_pretty(&StanceDef {
            id: id.to_string(),
            name: id.to_string(),
            ac_bonus: 0,
            attack_penalty: 0,
            damage_bonus: 0,
            ac_penalty: 0,
            min_level: 1,
        })?,
        "sets" => toml::to_string_pretty(&SetDef {
            id: id.to_string(),
            name: id.to_string(),
            bonuses: Vec::new(),
        })?,
        "affixes" => toml::to_string_pretty(&AffixDef {
            id: id.to_string(),
            name: id.to_string(),
            description: String::new(),
            affix_type: "prefix".to_string(),
            element: None,
            amount: None,
            stat: None,
            quality_min: "common".to_string(),
            slot: Vec::new(),
            weight: 1,
        })?,
        "passives" => toml::to_string_pretty(&PassiveDef {
            id: id.to_string(),
            name: id.to_string(),
            description: String::new(),
            effects: Vec::new(),
        })?,
        "areas" => toml::to_string_pretty(&AreaTemplate {
            id: id.to_string(),
            name: id.to_string(),
            description: String::new(),
            spawn_room: "start".to_string(),
            level_range: None,
            flags: Vec::new(),
            weather_zone: None,
            reset_interval: None,
            credits: None,
            spawns: Vec::new(),
            rooms: HashMap::new(),
        })?,
        "rooms" => toml::to_string_pretty(&RoomTemplate {
            id: id.to_string(),
            area: String::new(),
            name: id.to_string(),
            description: String::new(),
            exits: HashMap::new(),
            portals: Vec::new(),
            flags: Vec::new(),
            content: RoomContent::default(),
        })?,
        _ => return Err(format!("unknown category: {category}").into()),
    };
    Ok(content)
}

/// Parse a raw TOML string into the proper struct and insert into the registry.
fn insert_draft_into_registry(
    registry: &mut TemplateRegistry,
    category: &str,
    id: &str,
    toml_str: &str,
) {
    match category {
        "items" => {
            if let Ok(t) = toml::from_str::<ItemTemplate>(toml_str) {
                registry.items.insert(id.to_string(), t);
            }
        }
        "mobs" => {
            if let Ok(t) = toml::from_str::<MobTemplate>(toml_str) {
                registry.mobs.insert(id.to_string(), t);
            }
        }
        "races" => {
            if let Ok(t) = toml::from_str::<RaceTemplate>(toml_str) {
                registry.races.insert(id.to_string(), t);
            }
        }
        "classes" => {
            if let Ok(t) = toml::from_str::<ClassTemplate>(toml_str) {
                registry.classes.insert(id.to_string(), t);
            }
        }
        "skills" => {
            if let Ok(t) = toml::from_str::<mud_core::SkillDef>(toml_str) {
                registry.skills.insert(id.to_string(), t);
            }
        }
        "stances" => {
            if let Ok(t) = toml::from_str::<StanceDef>(toml_str) {
                registry.stances.insert(id.to_string(), t);
            }
        }
        "sets" => {
            if let Ok(t) = toml::from_str::<SetDef>(toml_str) {
                registry.sets.insert(id.to_string(), t);
            }
        }
        "affixes" => {
            if let Ok(t) = toml::from_str::<AffixDef>(toml_str) {
                registry.affixes.insert(id.to_string(), t);
            }
        }
        "passives" => {
            if let Ok(t) = toml::from_str::<PassiveDef>(toml_str) {
                registry.passives.insert(id.to_string(), t);
            }
        }
        "areas" => {
            if let Ok(t) = toml::from_str::<AreaTemplate>(toml_str) {
                registry.areas.insert(id.to_string(), t);
            }
        }
        "rooms" => {
            if let Ok(room) = toml::from_str::<RoomTemplate>(toml_str) {
                let area_id = room.area.clone();
                if !area_id.is_empty() {
                    if let Some(area) = registry.areas.get_mut(&area_id) {
                        area.rooms.insert(id.to_string(), room);
                    }
                }
            }
        }
        _ => {}
    }
}

/// Remove an entity from the in-memory registry (used for draft deletion).
fn remove_from_registry(registry: &mut TemplateRegistry, category: &str, id: &str) {
    match category {
        "items" => {
            registry.items.remove(id);
        }
        "mobs" => {
            registry.mobs.remove(id);
        }
        "races" => {
            registry.races.remove(id);
        }
        "classes" => {
            registry.classes.remove(id);
        }
        "skills" => {
            registry.skills.remove(id);
        }
        "stances" => {
            registry.stances.remove(id);
        }
        "sets" => {
            registry.sets.remove(id);
        }
        "affixes" => {
            registry.affixes.remove(id);
        }
        "passives" => {
            registry.passives.remove(id);
        }
        "areas" => {
            registry.areas.remove(id);
        }
        "rooms" => {
            for area in registry.areas.values_mut() {
                area.rooms.remove(id);
            }
        }
        _ => {}
    }
}

impl Screen for EntitiesScreen {
    fn name(&self) -> &str {
        "Entities Editor"
    }

    fn handle_key(&mut self, key: KeyEvent) -> bool {
        if self.search_focus {
            self.handle_search_key(key);
            return true;
        }

        if key.code == KeyCode::Char('?') {
            self.show_help = !self.show_help;
            return true;
        }

        if key.code == KeyCode::Esc {
            if self.preview.is_some() {
                self.preview = None;
            } else if self.show_help {
                self.show_help = false;
            } else if self.detail.is_some() {
                if self.detail.as_ref().unwrap().is_editing() {
                    self.detail.as_mut().unwrap().handle_key(key);
                } else {
                    if let Some(detail) = self.detail.take() {
                        if detail.dirty {
                            detail.apply_changes(&mut self.registry);
                            self.unsaved
                                .insert((detail.category.clone(), detail.template_id.clone()));
                        }
                    }
                    self.focus = Focus::Tree;
                    self.rebuild_tree();
                }
            }
            return true;
        }

        match (key.code, key.modifiers) {
            (KeyCode::BackTab, _) | (KeyCode::Tab, KeyModifiers::SHIFT) => match self.focus {
                Focus::Detail => {
                    self.previous_focus = self.focus;
                    self.focus = Focus::Tree;
                    true
                }
                Focus::Tree => {
                    self.previous_focus = self.focus;
                    false
                }
            },
            (KeyCode::Tab, _) => match self.focus {
                Focus::Tree => {
                    self.previous_focus = self.focus;
                    self.focus = Focus::Detail;
                    true
                }
                Focus::Detail => {
                    self.previous_focus = self.focus;
                    false
                }
            },
            _ => {
                match self.focus {
                    Focus::Tree => self.handle_tree_key(key),
                    Focus::Detail => self.handle_detail_key(key),
                }
                true
            }
        }
    }

    fn unsaved_count(&self) -> usize {
        self.unsaved.len()
    }

    fn set_sidebar_focused(&mut self, focused: bool) {
        self.sidebar_focused = focused;
    }

    fn sidebar_focus_lost(&mut self, backward: bool) {
        self.focus = if backward { Focus::Detail } else { Focus::Tree };
    }

    fn handle_mouse(&mut self, mouse: MouseEvent, area: Rect) {
        if self.show_help || area.width < 2 {
            return;
        }

        let tree_width = self.tree_width_pct(area.width);

        if mouse.column < area.x + tree_width {
            self.focus = Focus::Tree;
            self.handle_tree_mouse(mouse, area, tree_width);
        } else if mouse.column > area.x + tree_width {
            if let Some(ref mut detail) = self.detail {
                self.focus = Focus::Detail;
                let detail_x = area.x + tree_width + 1;
                let detail_area = Rect::new(
                    detail_x,
                    area.y + 1,
                    area.width.saturating_sub(tree_width).saturating_sub(1),
                    area.height.saturating_sub(1),
                );
                detail.handle_mouse(mouse, detail_area);
                if detail.deleted {
                    self.unsaved
                        .remove(&(detail.category.clone(), detail.template_id.clone()));
                    self.draft_data
                        .remove(&(detail.category.clone(), detail.template_id.clone()));
                    remove_from_registry(&mut self.registry, &detail.category, &detail.template_id);
                    self.detail = None;
                    self.focus = Focus::Tree;
                    self.rebuild_tree();
                }
            }
        }
    }

    fn selection_context(&self) -> Option<EntityContext> {
        let data = self.tree.selected_data()?;
        if data.id.is_empty() {
            return None;
        }
        let name = self.selected_entity_name()?;
        Some(EntityContext {
            category: data.category.clone(),
            id: data.id.clone(),
            name,
        })
    }

    fn contextual_commands(&self) -> Vec<(String, CommandAction)> {
        let data = match self.tree.selected_data() {
            Some(d) => d,
            None => return Vec::new(),
        };
        if data.id.is_empty() {
            return Vec::new();
        }
        let mut cmds = match data.category.as_str() {
            "rooms" => vec![
                ("Look".to_string(), CommandAction::LookRoom),
                ("Edit".to_string(), CommandAction::EditEntity),
                ("Delete".to_string(), CommandAction::DeleteEntity),
            ],
            "mobs" => vec![
                ("Room look".to_string(), CommandAction::LookMobRoom),
                ("Look at".to_string(), CommandAction::LookMobDetail),
                ("Edit".to_string(), CommandAction::EditEntity),
                ("Delete".to_string(), CommandAction::DeleteEntity),
            ],
            "items" => vec![
                ("Look at".to_string(), CommandAction::LookItem),
                ("Edit".to_string(), CommandAction::EditEntity),
                ("Delete".to_string(), CommandAction::DeleteEntity),
            ],
            _ => vec![
                ("Edit".to_string(), CommandAction::EditEntity),
                ("Delete".to_string(), CommandAction::DeleteEntity),
            ],
        };
        if self.detail.is_some()
            || self
                .unsaved
                .contains(&(data.category.clone(), data.id.clone()))
        {
            cmds.push(("Save".to_string(), CommandAction::SaveEntity));
        }
        cmds
    }

    fn handle_command_action(&mut self, action: &CommandAction) -> Result<bool, String> {
        match action {
            CommandAction::CreateEntity(cat) => {
                let ctx = self.selection_context();
                let normalized = normalize_category(cat);
                let context_id = match normalized {
                    "rooms" => {
                        match ctx {
                            Some(ref c) if c.category == "areas" => Some(c.id.clone()),
                            Some(ref c) if c.category == "rooms" => {
                                // Find parent area
                                self.registry
                                    .areas
                                    .iter()
                                    .find(|(_, a)| a.rooms.contains_key(&c.id))
                                    .map(|(id, _)| id.clone())
                            }
                            _ => None,
                        }
                    }
                    "areas" => None,
                    _ => None,
                };
                self.create_new_entity(cat, context_id.as_deref());
                Ok(true)
            }
            CommandAction::SaveEntity => {
                let detail_open = self.detail.is_some();
                if !detail_open {
                    let data = self
                        .tree
                        .selected_data()
                        .ok_or_else(|| "No entity selected".to_string())?;
                    let cat = data.category.clone();
                    let id = data.id.clone();
                    let is_draft = self.draft_data.contains_key(&(cat.clone(), id.clone()));
                    let inspector = EntityInspectorScreen::new(
                        self.registry.clone(),
                        cat.clone(),
                        id.clone(),
                        self.file_map.clone(),
                        Some(self.content_path.clone()),
                        is_draft,
                    );
                    inspector.save_to_disk()?;
                    self.draft_data.remove(&(cat.clone(), id.clone()));
                    if !self.file_map.contains_key(&cat) || !self.file_map[&cat].contains_key(&id) {
                        let path = self.content_path.join(&cat).join(format!("{id}.toml"));
                        self.file_map
                            .entry(cat.clone())
                            .or_default()
                            .insert(id.clone(), path);
                    }
                    self.unsaved.remove(&(cat, id));
                    self.rebuild_tree();
                    return Ok(true);
                }
                // Take save data before mutating
                let (is_draft, cat, id) = {
                    let d = self.detail.as_ref().unwrap();
                    (d.is_draft, d.category.clone(), d.template_id.clone())
                };
                // Save to disk
                self.detail.as_ref().unwrap().save_to_disk()?;
                // Mark as no longer a draft so future saves use file_map
                let detail = self.detail.as_mut().unwrap();
                if is_draft {
                    detail.is_draft = false;
                    detail.content_path = None;
                    self.draft_data.remove(&(cat.clone(), id.clone()));
                    if !self.file_map.contains_key(&cat) || !self.file_map[&cat].contains_key(&id) {
                        let path = self.content_path.join(&cat).join(format!("{id}.toml"));
                        let cat_key = cat.clone();
                        self.file_map
                            .entry(cat_key)
                            .or_default()
                            .insert(id.clone(), path);
                    }
                }
                self.unsaved.remove(&(cat, id));
                self.rebuild_tree();
                Ok(true)
            }
            CommandAction::EditEntity => {
                if let Some(data) = self.tree.selected_data().cloned() {
                    if !data.id.is_empty() {
                        self.open_detail(data.category.clone(), data.id.clone());
                    }
                }
                Ok(true)
            }
            CommandAction::DeleteEntity => {
                let data = match self.tree.selected_data().cloned() {
                    Some(d) if !d.id.is_empty() => d,
                    _ => return Ok(false),
                };
                if self.detail.is_none() {
                    self.open_detail(data.category.clone(), data.id.clone());
                }
                if let Some(ref mut detail) = self.detail {
                    let singular = crate::screens::entity_inspector::singularize(&data.category);
                    detail.delete_dialog = Some(crate::components::Dialog::new(
                        ratatui::style::Color::Red,
                        "Confirm Delete",
                        &format!("Delete {} \"{}\"?", singular, data.id),
                        &["Cancel".to_string(), "Delete".to_string()],
                    ));
                }
                Ok(true)
            }
            CommandAction::LookRoom => {
                let data = match self.tree.selected_data().cloned() {
                    Some(d) if d.category == "rooms" && !d.id.is_empty() => d,
                    _ => return Ok(false),
                };
                let room = match self
                    .registry
                    .areas
                    .iter()
                    .find_map(|(_, a)| a.rooms.get(&data.id))
                {
                    Some(r) => r,
                    None => return Ok(false),
                };
                let exit_dirs: Vec<String> = room
                    .exits
                    .keys()
                    .filter_map(|dir| {
                        mud_core::Direction::try_from(dir.as_str())
                            .map(|d| d.short_name().to_string())
                    })
                    .collect();
                let mob_names: Vec<String> = room
                    .content
                    .mobs
                    .iter()
                    .filter_map(|m| {
                        self.registry
                            .mobs
                            .get(&m.template_id)
                            .map(|mt| mt.name.clone())
                    })
                    .collect();
                let item_names: Vec<String> = room
                    .content
                    .items
                    .iter()
                    .filter_map(|i| {
                        self.registry
                            .items
                            .get(&i.template_id)
                            .map(|it| it.name.clone())
                    })
                    .collect();
                let rt = preview::room_look(
                    &room.name,
                    &room.description,
                    &exit_dirs,
                    &mob_names,
                    &item_names,
                );
                self.show_preview(format!("Room Look: {}", room.name), rt);
                Ok(true)
            }
            CommandAction::LookMobRoom => {
                let data = match self.tree.selected_data().cloned() {
                    Some(d) if d.category == "mobs" && !d.id.is_empty() => d,
                    _ => return Ok(false),
                };
                let mob = match self.registry.mobs.get(&data.id) {
                    Some(m) => m,
                    None => return Ok(false),
                };
                let rt = preview::mob_room_appearance(&mob.name);
                self.show_preview(format!("Room Look: {}", mob.name), rt);
                Ok(true)
            }
            CommandAction::LookMobDetail => {
                let data = match self.tree.selected_data().cloned() {
                    Some(d) if d.category == "mobs" && !d.id.is_empty() => d,
                    _ => return Ok(false),
                };
                let mob = match self.registry.mobs.get(&data.id) {
                    Some(m) => m,
                    None => return Ok(false),
                };
                let rt = preview::mob_look_template(mob);
                self.show_preview(format!("Look at: {}", mob.name), rt);
                Ok(true)
            }
            CommandAction::LookItem => {
                let data = match self.tree.selected_data().cloned() {
                    Some(d) if d.category == "items" && !d.id.is_empty() => d,
                    _ => return Ok(false),
                };
                let item = match self.registry.items.get(&data.id) {
                    Some(i) => i,
                    None => return Ok(false),
                };
                let rt = preview::item_look_template(item);
                self.show_preview(format!("Look at: {}", item.name), rt);
                Ok(true)
            }
            CommandAction::GoToParent => {
                if let Some(parent_idx) = self.tree.selected_parent_index() {
                    self.tree.selected = Some(parent_idx);
                }
                Ok(true)
            }
            CommandAction::ExpandAll => {
                self.tree.expand_all();
                Ok(true)
            }
            CommandAction::CollapseAll => {
                self.tree.collapse_all();
                Ok(true)
            }
            CommandAction::ToggleSearch => {
                self.search_focus = true;
                self.search.get_or_insert_with(String::new);
                Ok(true)
            }
            CommandAction::SaveAllEntities => {
                let draft_ids: Vec<(String, String)> = if self.draft_data.is_empty() {
                    // Save all unsaved entities that exist in registry
                    self.unsaved.iter().cloned().collect()
                } else {
                    self.draft_data.keys().cloned().collect()
                };
                if draft_ids.is_empty() {
                    return Err("No unsaved entities".to_string());
                }
                let mut errors = Vec::new();
                for (cat, id) in &draft_ids {
                    let inspector = EntityInspectorScreen::new(
                        self.registry.clone(),
                        cat.clone(),
                        id.clone(),
                        self.file_map.clone(),
                        Some(self.content_path.clone()),
                        true,
                    );
                    let result = inspector.save_to_disk();
                    match result {
                        Ok(()) => {
                            self.draft_data.remove(&(cat.clone(), id.clone()));
                            self.unsaved.remove(&(cat.clone(), id.clone()));
                            let path = self.content_path.join(cat).join(format!("{id}.toml"));
                            self.file_map
                                .entry(cat.clone())
                                .or_default()
                                .insert(id.clone(), path);
                        }
                        Err(e) => {
                            errors.push(format!("{cat}/{id}: {e}"));
                        }
                    }
                }
                self.rebuild_tree();
                if errors.is_empty() {
                    Ok(true)
                } else {
                    Err(format!(
                        "Saved {} entities, {} errors: {}",
                        draft_ids.len() - errors.len(),
                        errors.len(),
                        errors.join("; ")
                    ))
                }
            }
            CommandAction::ReloadContent => {
                self.reload();
                Ok(true)
            }
            CommandAction::ToggleHelp => {
                self.show_help = !self.show_help;
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    fn render(&mut self, area: Rect, buf: &mut Buffer, mouse_pos: Option<(u16, u16)>) {
        if area.width < 2 || area.height < 2 {
            return;
        }

        let tree_muted = self.focus == Focus::Detail || self.sidebar_focused;
        let detail_muted = self.focus == Focus::Tree || self.sidebar_focused;

        // Fill full area with DarkGray (Entities panel background)
        for y in area.y..area.y + area.height {
            for x in area.x..area.x + area.width {
                if let Some(cell) = buf.cell_mut((x, y)) {
                    cell.set_bg(Color::Indexed(236));
                }
            }
        }

        let content_y = area.y + 1;
        let content_h = area.height.saturating_sub(1);

        let tree_width = self.tree_width_pct(area.width);

        self.tree.hovered = mouse_pos.and_then(|(col, row)| {
            if row >= content_y
                && row < content_y + content_h
                && col >= area.x
                && col < area.x + tree_width
            {
                let line = (row - content_y) as usize;
                let idx = line + self.tree.scroll.offset;
                (idx < self.tree.flatten().len()).then_some(idx)
            } else {
                None
            }
        });

        if self.preview.is_some() {
            self.render_preview(area, buf);
            return;
        }

        if self.show_help {
            self.render_help(area, buf);
            return;
        }

        if self.search_focus {
            let search_text = format!(" / {}", self.search.as_deref().unwrap_or(""));
            buf.set_string(
                area.x,
                area.y,
                &search_text,
                Style::default().fg(Color::Cyan).bg(Color::Indexed(236)),
            );
            if let Some(ref s) = self.search {
                let cursor_x = area.x + 2 + s.len() as u16;
                if cursor_x < area.x + area.width {
                    buf.set_string(
                        cursor_x,
                        area.y,
                        "\u{2588}",
                        Style::default().fg(Color::Cyan).bg(Color::Indexed(236)),
                    );
                }
            } else {
                buf.set_string(
                    area.x + 2,
                    area.y,
                    "\u{2588}",
                    Style::default().fg(Color::Cyan).bg(Color::Indexed(236)),
                );
            }
        } else {
            let title_fg = if tree_muted {
                Color::Indexed(245)
            } else {
                Color::White
            };
            buf.set_string(
                area.x,
                area.y,
                " Entities ",
                Style::default().fg(title_fg).bg(Color::Indexed(236)),
            );
        }

        let tree_width = self.tree_width_pct(area.width);

        let detail_x = area.x + tree_width + 1;

        self.tree.muted = tree_muted;
        let tree_area = Rect::new(area.x, content_y, tree_width, content_h);
        let content_lines = tree_area.height as usize;
        self.tree.update_scroll(content_lines);
        self.scrollbar = ScrollState {
            offset: self.tree.scroll.offset,
            visible_lines: self.tree.scroll.visible_lines,
            total_lines: self.tree.scroll.total_lines,
        };
        self.tree.render(tree_area, buf);

        let tree_scroll_area = Rect::new(
            tree_area.x + tree_area.width.saturating_sub(1),
            content_y,
            1,
            content_h,
        );
        self.scrollbar.render(tree_scroll_area, buf);

        // Detail side: fill with Black, render " Detail " title
        let detail_area = Rect::new(
            detail_x,
            content_y,
            area.width.saturating_sub(tree_width).saturating_sub(1),
            content_h,
        );
        for y in detail_area.y..detail_area.y + detail_area.height {
            for x in detail_area.x..detail_area.x + detail_area.width {
                if let Some(cell) = buf.cell_mut((x, y)) {
                    cell.set_bg(Color::Black);
                }
            }
        }
        let detail_title_fg = if detail_muted {
            Color::Indexed(245)
        } else {
            Color::White
        };
        buf.set_string(
            detail_area.x,
            detail_area.y - 1,
            " Detail ",
            Style::default().fg(detail_title_fg).bg(Color::Black),
        );

        if let Some(ref mut detail) = self.detail {
            detail.muted = detail_muted;
            if detail_area.width >= 4 && detail_area.height >= 2 {
                detail.render(detail_area, buf, mouse_pos);
            }
        }
    }
}
