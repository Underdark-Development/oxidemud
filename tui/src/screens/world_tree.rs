use mud_core::templates::TemplateRegistry;
use ratatui::{
    buffer::Buffer,
    crossterm::event::{KeyCode, KeyEvent, MouseButton, MouseEvent, MouseEventKind},
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Borders, Widget},
};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use super::{entity_inspector::EntityInspectorScreen, Screen};
use crate::components::{ScrollState, Tree, TreeNode};
use crate::content;

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
    scrollbar: ScrollState,
    last_click: Option<(Instant, usize)>,
    detail: Option<EntityInspectorScreen>,
    focus: Focus,
    show_help: bool,
}

impl WorldTreeScreen {
    pub fn new(content_path: PathBuf) -> Self {
        let registry = content::load_templates(&content_path);
        Self::new_shared(content_path, registry)
    }

    pub fn new_shared(content_path: PathBuf, registry: TemplateRegistry) -> Self {
        let mut screen = WorldTreeScreen {
            tree: Tree::new(Vec::new()),
            content_path,
            registry,
            scrollbar: ScrollState::new(),
            last_click: None,
            detail: None,
            focus: Focus::Tree,
            show_help: false,
        };
        screen.rebuild_tree();
        screen
    }

    pub fn reload(&mut self) {
        self.registry = content::load_templates(&self.content_path);
        self.detail = None;
        self.focus = Focus::Tree;
        self.show_help = false;
        self.rebuild_tree();
    }

    pub fn registry(&self) -> &TemplateRegistry {
        &self.registry
    }

    fn rebuild_tree(&mut self) {
        let mut roots = Vec::new();

        if !self.registry.areas.is_empty() {
            let mut node = TreeNode::new(
                format!("Areas ({})", self.registry.areas.len()),
                NodeInfo {
                    category: String::new(),
                    id: String::new(),
                },
            );
            let mut ids: Vec<&String> = self.registry.areas.keys().collect();
            ids.sort();
            for id in ids {
                let area = &self.registry.areas[id];
                let mut area_child = TreeNode::new(
                    area.name.clone(),
                    NodeInfo {
                        category: "areas".into(),
                        id: id.clone(),
                    },
                );
                let mut room_ids: Vec<&String> = area.rooms.keys().collect();
                room_ids.sort();
                for room_id in room_ids {
                    let room = &area.rooms[room_id];
                    area_child.add_child(TreeNode::new(
                        room.name.clone(),
                        NodeInfo {
                            category: "rooms".into(),
                            id: room_id.clone(),
                        },
                    ));
                }
                node.add_child(area_child);
            }
            roots.push(node);
        }

        add_group(&mut roots, &self.registry.items, "Items", "items", |i| {
            i.name.clone()
        });
        add_group(&mut roots, &self.registry.mobs, "Mobs", "mobs", |m| {
            m.name.clone()
        });
        add_group(&mut roots, &self.registry.races, "Races", "races", |r| {
            r.name.clone()
        });
        add_group(
            &mut roots,
            &self.registry.classes,
            "Classes",
            "classes",
            |c| c.name.clone(),
        );
        add_group(&mut roots, &self.registry.skills, "Skills", "skills", |s| {
            s.name.clone()
        });
        add_group(
            &mut roots,
            &self.registry.stances,
            "Stances",
            "stances",
            |s| s.name.clone(),
        );
        add_group(&mut roots, &self.registry.sets, "Sets", "sets", |s| {
            s.name.clone()
        });
        add_group(
            &mut roots,
            &self.registry.affixes,
            "Affixes",
            "affixes",
            |a| a.name.clone(),
        );
        add_group(
            &mut roots,
            &self.registry.passives,
            "Passives",
            "passives",
            |p| p.name.clone(),
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
        ));
        self.focus = Focus::Detail;
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
            _ => {}
        }
    }

    fn handle_detail_key(&mut self, key: KeyEvent) {
        if let Some(ref mut detail) = self.detail {
            detail.handle_key(key);
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
) where
    F: Fn(&T) -> String,
{
    if items.is_empty() {
        return;
    }
    let mut node = TreeNode::new(
        format!("{} ({})", label, items.len()),
        NodeInfo {
            category: String::new(),
            id: String::new(),
        },
    );
    let mut ids: Vec<&String> = items.keys().collect();
    ids.sort();
    for id in ids {
        if let Some(item) = items.get(id) {
            node.add_child(TreeNode::new(
                display(item),
                NodeInfo {
                    category: category.to_string(),
                    id: id.clone(),
                },
            ));
        }
    }
    roots.push(node);
}

impl Screen for WorldTreeScreen {
    fn name(&self) -> &str {
        "World Tree"
    }

    fn handle_key(&mut self, key: KeyEvent) {
        if key.code == KeyCode::Char('?') {
            self.show_help = !self.show_help;
            return;
        }

        if key.code == KeyCode::Esc {
            if self.show_help {
                self.show_help = false;
            } else if let Some(ref mut detail) = self.detail {
                if detail.is_editing() {
                    detail.handle_key(key);
                } else {
                    self.detail = None;
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

        let info = self.info_line();
        buf.set_string(area.x, area.y, &info, Style::default().fg(Color::DarkGray));

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
