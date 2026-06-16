use mud_core::templates::TemplateRegistry;
use mud_core::templates::{
    AffixDef, AreaTemplate, ClassAttributeMods, ClassTemplate, HealthBounds, ItemTemplate,
    LootTable, MobTemplate, PassiveDef, RaceAttributes, RaceTemplate, RoomContent, RoomTemplate,
    SetDef, StanceDef, WalletAmount,
};
use ratatui::{
    buffer::Buffer,
    crossterm::event::{KeyCode, KeyEvent, MouseButton, MouseEvent, MouseEventKind},
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Borders, Widget},
};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use super::{entity_inspector::EntityInspectorScreen, Screen};
use crate::components::{ScrollState, Tree, TreeNode};
use crate::content::{self, FileMap};

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

pub struct WorldTreeScreen {
    tree: Tree<NodeInfo>,
    content_path: PathBuf,
    registry: TemplateRegistry,
    file_map: FileMap,
    scrollbar: ScrollState,
    last_click: Option<(Instant, usize)>,
    detail: Option<EntityInspectorScreen>,
    focus: Focus,
    show_help: bool,
    search: Option<String>,
    search_focus: bool,
}

impl WorldTreeScreen {
    pub fn new(content_path: PathBuf) -> Self {
        let (registry, file_map) = content::load_templates(&content_path);
        Self::new_shared(content_path, registry, file_map)
    }

    pub fn new_shared(
        content_path: PathBuf,
        registry: TemplateRegistry,
        file_map: FileMap,
    ) -> Self {
        let mut screen = WorldTreeScreen {
            tree: Tree::new(Vec::new()),
            content_path,
            registry,
            file_map,
            scrollbar: ScrollState::new(),
            last_click: None,
            detail: None,
            focus: Focus::Tree,
            show_help: false,
            search: None,
            search_focus: false,
        };
        screen.rebuild_tree();
        screen
    }

    pub fn reload(&mut self) {
        let (registry, file_map) = content::load_templates(&self.content_path);
        self.registry = registry;
        self.file_map = file_map;
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

    fn rebuild_tree(&mut self) {
        let filter = self
            .search
            .as_deref()
            .filter(|s| !s.is_empty())
            .map(|s| s.to_lowercase());

        let matches = |name: &str| -> bool {
            filter
                .as_ref()
                .is_none_or(|f| name.to_lowercase().contains(f.as_str()))
        };

        let mut roots = Vec::new();

        if !self.registry.areas.is_empty() {
            let mut area_count = 0usize;
            let mut area_node = TreeNode::new(
                "Areas".to_string(),
                NodeInfo {
                    category: String::new(),
                    id: String::new(),
                },
            );
            let mut ids: Vec<&String> = self.registry.areas.keys().collect();
            ids.sort();
            for id in ids {
                let area = &self.registry.areas[id];
                let area_matches = matches(&area.name) || filter.is_none();
                let mut area_child = TreeNode::new(
                    area.name.clone(),
                    NodeInfo {
                        category: "areas".into(),
                        id: id.clone(),
                    },
                );
                let mut room_count = 0usize;
                let mut room_ids: Vec<&String> = area.rooms.keys().collect();
                room_ids.sort();
                for room_id in room_ids {
                    let room = &area.rooms[room_id];
                    if matches(&room.name) {
                        area_child.add_child(TreeNode::new(
                            room.name.clone(),
                            NodeInfo {
                                category: "rooms".into(),
                                id: room_id.clone(),
                            },
                        ));
                        room_count += 1;
                    }
                }
                if area_matches || room_count > 0 {
                    area_node.add_child(area_child);
                    area_count += 1;
                }
            }
            if area_count > 0 {
                area_node.label = format!("Areas ({area_count})");
                roots.push(area_node);
            }
        }

        let filter_str = filter.as_deref();
        add_group(
            &mut roots,
            &self.registry.items,
            "Items",
            "items",
            |i| i.name.clone(),
            filter_str,
        );
        add_group(
            &mut roots,
            &self.registry.mobs,
            "Mobs",
            "mobs",
            |m| m.name.clone(),
            filter_str,
        );
        add_group(
            &mut roots,
            &self.registry.races,
            "Races",
            "races",
            |r| r.name.clone(),
            filter_str,
        );
        add_group(
            &mut roots,
            &self.registry.classes,
            "Classes",
            "classes",
            |c| c.name.clone(),
            filter_str,
        );
        add_group(
            &mut roots,
            &self.registry.skills,
            "Skills",
            "skills",
            |s| s.name.clone(),
            filter_str,
        );
        add_group(
            &mut roots,
            &self.registry.stances,
            "Stances",
            "stances",
            |s| s.name.clone(),
            filter_str,
        );
        add_group(
            &mut roots,
            &self.registry.sets,
            "Sets",
            "sets",
            |s| s.name.clone(),
            filter_str,
        );
        add_group(
            &mut roots,
            &self.registry.affixes,
            "Affixes",
            "affixes",
            |a| a.name.clone(),
            filter_str,
        );
        add_group(
            &mut roots,
            &self.registry.passives,
            "Passives",
            "passives",
            |p| p.name.clone(),
            filter_str,
        );

        self.tree = Tree::new(roots);
        self.tree.selected = Some(0);
    }

    fn info_line(&self) -> String {
        let r = &self.registry;
        format!(
            "{} areas, {} items, {} mobs, {} races, {} classes, {} skills",
            r.areas.len(),
            r.items.len(),
            r.mobs.len(),
            r.races.len(),
            r.classes.len(),
            r.skills.len(),
        )
    }

    fn tree_width_pct(&self, area_width: u16) -> u16 {
        if self.detail.is_some() {
            (area_width * 35 / 100)
                .max(20)
                .min(area_width.saturating_sub(4))
        } else {
            area_width
        }
    }

    fn open_detail(&mut self, category: String, template_id: String) {
        self.detail = Some(EntityInspectorScreen::new(
            self.registry.clone(),
            category,
            template_id,
            self.file_map.clone(),
        ));
        self.focus = Focus::Detail;
    }

    fn create_new_entity(&mut self, category: &str, context_id: Option<&str>) {
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0);
        let id = match category {
            "rooms" => format!("room_{ts}"),
            "areas" => format!("area_{ts}"),
            _ => {
                let stem = category.trim_end_matches('s');
                format!("{stem}_{ts}")
            }
        };

        let result = if category == "rooms" {
            self.create_room_file(&id, context_id)
        } else if category == "areas" {
            self.create_area_file(&id)
        } else {
            self.create_template_file(category, &id)
        };

        match result {
            Ok(_) => {
                self.reload();
                self.open_detail(category.to_string(), id);
            }
            Err(e) => {
                tracing::error!("failed to create {category}/{id}: {e}");
            }
        }
    }

    fn create_template_file(
        &self,
        category: &str,
        id: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let path = self.content_path.join(category);
        std::fs::create_dir_all(&path)?;
        let path = path.join(format!("{id}.toml"));

        let content: String = match category {
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
                aggro_race: Vec::new(),
                faction: None,
                faction_standing: 0,
                trainer_types: Vec::new(),
                languages: Vec::new(),
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
            })?,
            "classes" => toml::to_string_pretty(&ClassTemplate {
                id: id.to_string(),
                name: id.to_string(),
                description: String::new(),
                hit_die: 8,
                attribute_mods: ClassAttributeMods::default(),
                allowed_races: Vec::new(),
                allowed_alignments: Vec::new(),
                auto_skills: Vec::new(),
                skill_pool: Vec::new(),
                starting_skill_slots: 3,
                starting_items: Vec::new(),
                starting_gold: WalletAmount::default(),
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
            _ => return Err(format!("unknown category: {category}").into()),
        };

        std::fs::write(&path, &content)?;
        Ok(())
    }

    fn create_area_file(&self, id: &str) -> Result<(), Box<dyn std::error::Error>> {
        let path = self.content_path.join("areas").join(format!("{id}.toml"));
        let area = AreaTemplate {
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
            rooms: HashMap::from([(
                "start".to_string(),
                RoomTemplate {
                    name: "Starting Room".to_string(),
                    description: String::new(),
                    exits: HashMap::new(),
                    portals: Vec::new(),
                    flags: Vec::new(),
                    content: RoomContent::default(),
                },
            )]),
        };
        std::fs::create_dir_all(path.parent().unwrap())?;
        let content = toml::to_string_pretty(&area)?;
        std::fs::write(&path, &content)?;
        Ok(())
    }

    fn create_room_file(
        &self,
        id: &str,
        context_id: Option<&str>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let area_id = context_id.ok_or("no area context for room creation")?;
        let path = self
            .file_map
            .get("areas")
            .and_then(|m| m.get(area_id))
            .ok_or_else(|| format!("area {area_id} file not found"))?;

        let content = std::fs::read_to_string(path)?;
        let mut doc: toml::Value = content.parse()?;

        let room = RoomTemplate {
            name: id.to_string(),
            description: String::new(),
            exits: HashMap::new(),
            portals: Vec::new(),
            flags: Vec::new(),
            content: RoomContent::default(),
        };
        let room_value = toml::Value::try_from(&room)?;

        if let Some(area_table) = doc.as_table_mut() {
            if let Some(rooms) = area_table.get_mut("rooms").and_then(|v| v.as_table_mut()) {
                rooms.insert(id.to_string(), room_value);
            } else {
                let mut rooms_map = toml::value::Table::new();
                rooms_map.insert(id.to_string(), room_value);
                area_table.insert("rooms".to_string(), toml::Value::Table(rooms_map));
            }
        }

        std::fs::write(path, doc.to_string())?;
        Ok(())
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
                self.detail = None;
                self.reload();
                self.focus = Focus::Tree;
            }
        }
    }

    fn handle_tree_mouse(&mut self, mouse: MouseEvent, area: Rect, _tree_width: u16) {
        match mouse.kind {
            MouseEventKind::ScrollUp => self.tree.select_prev(),
            MouseEventKind::ScrollDown => self.tree.select_next(),
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
                }
            }
            _ => {}
        }
    }

    fn render_help(&self, area: Rect, buf: &mut Buffer) {
        let help_text = vec![
            " Keyboard shortcuts ",
            "",
            "  ?          Toggle this help",
            "  Esc        Close detail / close help",
            "  Tab        Toggle focus: tree \u{2194} detail",
            "  \u{2191}/\u{2193}        Navigate current pane",
            "  \u{2192}          Expand collapsed tree node",
            "  \u{2190}          Collapse expanded tree node",
            "  Enter      Open entity detail",
            "  /          Search/filter tree",
            "",
            "  Ctrl+R     Reload content",
            "  Ctrl+1-9   Switch screens",
            "  Ctrl+D     Quit",
            "",
            " Mouse ",
            "",
            "  Scroll     Scroll tree or table",
            "  Click      Select tree node",
            "  Double-clk Expand/collapse tree node",
        ];

        let help_width = 40u16;
        let help_height = help_text.len() as u16 + 2;
        let help_x = area.x + (area.width.saturating_sub(help_width)) / 2;
        let help_y = area.y + (area.height.saturating_sub(help_height)) / 2;

        for y in area.y..area.y + area.height {
            for x in area.x..area.x + area.width {
                if let Some(cell) = buf.cell_mut((x, y)) {
                    cell.set_style(Style::default().fg(Color::DarkGray));
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
}

fn add_group<T, F>(
    roots: &mut Vec<TreeNode<NodeInfo>>,
    items: &HashMap<String, T>,
    label: &str,
    category: &str,
    display: F,
    filter: Option<&str>,
) where
    F: Fn(&T) -> String,
{
    if items.is_empty() {
        return;
    }
    let mut count = 0usize;
    let mut node = TreeNode::new(
        label.to_string(),
        NodeInfo {
            category: String::new(),
            id: String::new(),
        },
    );
    let mut ids: Vec<&String> = items.keys().collect();
    ids.sort();
    for id in ids {
        if let Some(item) = items.get(id) {
            let name = display(item);
            if filter.is_none_or(|f| name.to_lowercase().contains(&f.to_lowercase())) {
                node.add_child(TreeNode::new(
                    name,
                    NodeInfo {
                        category: category.to_string(),
                        id: id.clone(),
                    },
                ));
                count += 1;
            }
        }
    }
    if count > 0 {
        node.label = format!("{label} ({count})");
        roots.push(node);
    }
}

impl Screen for WorldTreeScreen {
    fn name(&self) -> &str {
        "World Tree"
    }

    fn handle_key(&mut self, key: KeyEvent) {
        if self.search_focus {
            self.handle_search_key(key);
            return;
        }

        if key.code == KeyCode::Char('?') {
            self.show_help = !self.show_help;
            return;
        }

        if key.code == KeyCode::Esc {
            if self.show_help {
                self.show_help = false;
            } else if self.detail.is_some() {
                if self.detail.as_ref().unwrap().is_editing() {
                    self.detail.as_mut().unwrap().handle_key(key);
                } else {
                    if let Some(detail) = self.detail.take() {
                        detail.apply_changes(&mut self.registry);
                    }
                    self.focus = Focus::Tree;
                }
            }
            return;
        }

        if key.code == KeyCode::Tab && self.detail.is_some() {
            self.focus = match self.focus {
                Focus::Tree => Focus::Detail,
                Focus::Detail => Focus::Tree,
            };
            return;
        }

        match self.focus {
            Focus::Tree => self.handle_tree_key(key),
            Focus::Detail => self.handle_detail_key(key),
        }
    }

    fn handle_mouse(&mut self, mouse: MouseEvent, area: Rect) {
        if self.show_help {
            return;
        }

        let tree_width = self.tree_width_pct(area.width);

        if mouse.column < area.x + tree_width {
            self.handle_tree_mouse(mouse, area, tree_width);
        } else if mouse.column > area.x + tree_width {
            if let Some(ref mut detail) = self.detail {
                let detail_x = area.x + tree_width + 1;
                let detail_area = Rect::new(
                    detail_x,
                    area.y + 1,
                    area.width.saturating_sub(tree_width).saturating_sub(1),
                    area.height.saturating_sub(1),
                );
                detail.handle_mouse(mouse, detail_area);
            }
        }
    }

    fn render(&mut self, area: Rect, buf: &mut Buffer) {
        if area.width < 2 || area.height < 2 {
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
                Style::default().fg(Color::Cyan),
            );
            if let Some(ref s) = self.search {
                let cursor_x = area.x + 2 + s.len() as u16;
                if cursor_x < area.x + area.width {
                    buf.set_string(
                        cursor_x,
                        area.y,
                        "\u{2588}",
                        Style::default().fg(Color::Cyan),
                    );
                }
            } else {
                buf.set_string(
                    area.x + 2,
                    area.y,
                    "\u{2588}",
                    Style::default().fg(Color::Cyan),
                );
            }
        } else {
            let info = self.info_line();
            buf.set_string(area.x, area.y, &info, Style::default().fg(Color::DarkGray));
        }

        let tree_width = self.tree_width_pct(area.width);

        if self.detail.is_some() {
            let sep_x = area.x + tree_width;
            let detail_x = sep_x + 1;

            let tree_area = Rect::new(
                area.x,
                area.y + 1,
                tree_width,
                area.height.saturating_sub(1),
            );
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
                area.y + 1,
                1,
                area.height.saturating_sub(1),
            );
            self.scrollbar.render(tree_scroll_area, buf);

            let sep_style = if self.focus == Focus::Tree {
                Style::default().fg(Color::White)
            } else {
                Style::default().fg(Color::DarkGray)
            };
            for y in (area.y + 1)..(area.y + area.height) {
                buf.set_string(sep_x, y, "\u{2502}", sep_style);
            }

            if let Some(ref mut detail) = self.detail {
                let detail_area = Rect::new(
                    detail_x,
                    area.y + 1,
                    area.width.saturating_sub(tree_width).saturating_sub(1),
                    area.height.saturating_sub(1),
                );
                if detail_area.width >= 4 && detail_area.height >= 2 {
                    detail.render(detail_area, buf);
                }
            }
        } else {
            let content_lines = area.height.saturating_sub(1) as usize;
            self.tree.update_scroll(content_lines);
            self.scrollbar = ScrollState {
                offset: self.tree.scroll.offset,
                visible_lines: self.tree.scroll.visible_lines,
                total_lines: self.tree.scroll.total_lines,
            };

            let tree_area = Rect::new(
                area.x,
                area.y + 1,
                area.width.saturating_sub(1),
                area.height.saturating_sub(1),
            );
            self.tree.render(tree_area, buf);

            let scrollbar_area = Rect::new(
                area.x + area.width - 1,
                area.y + 1,
                1,
                area.height.saturating_sub(1),
            );
            self.scrollbar.render(scrollbar_area, buf);
        }
    }
}
