use mud_core::templates::TemplateRegistry;
use ratatui::{
    buffer::Buffer,
    crossterm::event::{KeyCode, KeyEvent},
    layout::Rect,
    style::{Color, Style},
    widgets::Widget,
};
use std::collections::HashMap;
use std::path::PathBuf;

use super::Screen;
use crate::components::{ScrollState, Tree, TreeNode};
use crate::content;

pub struct WorldTreeScreen {
    tree: Tree<String>,
    content_path: PathBuf,
    registry: TemplateRegistry,
    scrollbar: ScrollState,
}

impl WorldTreeScreen {
    pub fn new(content_path: PathBuf) -> Self {
        let registry = content::load_templates(&content_path);
        let mut screen = WorldTreeScreen {
            tree: Tree::new(Vec::new()),
            content_path,
            registry,
            scrollbar: ScrollState::new(),
        };
        screen.rebuild_tree();
        screen
    }

    pub fn reload(&mut self) {
        self.registry = content::load_templates(&self.content_path);
        self.rebuild_tree();
    }

    fn rebuild_tree(&mut self) {
        let mut roots = Vec::new();

        if !self.registry.areas.is_empty() {
            let mut node = TreeNode::new(
                format!("Areas ({})", self.registry.areas.len()),
                String::new(),
            );
            let mut ids: Vec<&String> = self.registry.areas.keys().collect();
            ids.sort();
            for id in ids {
                let area = &self.registry.areas[id];
                let mut area_child = TreeNode::new(area.name.clone(), id.clone());
                let mut room_ids: Vec<&String> = area.rooms.keys().collect();
                room_ids.sort();
                for room_id in room_ids {
                    let room = &area.rooms[room_id];
                    area_child.add_child(TreeNode::new(room.name.clone(), room_id.clone()));
                }
                node.add_child(area_child);
            }
            roots.push(node);
        }

        add_group(&mut roots, &self.registry.items, "Items", |i| {
            i.name.clone()
        });
        add_group(&mut roots, &self.registry.mobs, "Mobs", |m| m.name.clone());
        add_group(&mut roots, &self.registry.races, "Races", |r| {
            r.name.clone()
        });
        add_group(&mut roots, &self.registry.classes, "Classes", |c| {
            c.name.clone()
        });
        add_group(&mut roots, &self.registry.skills, "Skills", |s| {
            s.name.clone()
        });
        add_group(&mut roots, &self.registry.stances, "Stances", |s| {
            s.name.clone()
        });
        add_group(&mut roots, &self.registry.sets, "Sets", |s| s.name.clone());
        add_group(&mut roots, &self.registry.affixes, "Affixes", |a| {
            a.name.clone()
        });
        add_group(&mut roots, &self.registry.passives, "Passives", |p| {
            p.name.clone()
        });

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
}

fn add_group<T, F>(
    roots: &mut Vec<TreeNode<String>>,
    items: &HashMap<String, T>,
    label: &str,
    display: F,
) where
    F: Fn(&T) -> String,
{
    if items.is_empty() {
        return;
    }
    let mut node = TreeNode::new(format!("{} ({})", label, items.len()), String::new());
    let mut ids: Vec<&String> = items.keys().collect();
    ids.sort();
    for id in ids {
        if let Some(item) = items.get(id) {
            node.add_child(TreeNode::new(display(item), id.clone()));
        }
    }
    roots.push(node);
}

impl Screen for WorldTreeScreen {
    fn name(&self) -> &str {
        "World Tree"
    }

    fn handle_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Up => self.tree.select_prev(),
            KeyCode::Down => self.tree.select_next(),
            KeyCode::Enter | KeyCode::Right => self.tree.toggle_selected(),
            KeyCode::Char('r') => self.reload(),
            _ => {}
        }
    }

    fn render(&mut self, area: Rect, buf: &mut Buffer) {
        if area.width < 2 || area.height < 2 {
            return;
        }

        let content_lines = area.height.saturating_sub(1) as usize;
        self.tree.update_scroll(content_lines);
        self.scrollbar = ScrollState {
            offset: self.tree.scroll.offset,
            visible_lines: self.tree.scroll.visible_lines,
            total_lines: self.tree.scroll.total_lines,
        };

        let info = self.info_line();
        buf.set_string(area.x, area.y, &info, Style::default().fg(Color::DarkGray));

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
