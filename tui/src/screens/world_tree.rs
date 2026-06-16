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

use super::{Screen, ScreenAction};
use crate::components::{ScrollState, Tree, TreeNode};
use crate::content;

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
    inspect_request: Option<(String, String)>,
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
            inspect_request: None,
        };
        screen.rebuild_tree();
        screen
    }

    pub fn reload(&mut self) {
        self.registry = content::load_templates(&self.content_path);
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
        match key.code {
            KeyCode::Up => self.tree.select_prev(),
            KeyCode::Down => self.tree.select_next(),
            KeyCode::Enter | KeyCode::Right => {
                if let Some(data) = self.tree.selected_data() {
                    if !data.id.is_empty() {
                        if let Some(idx) = self.tree.selected {
                            if let Some((_, node)) = self.tree.flatten().get(idx) {
                                if node.is_leaf() {
                                    self.inspect_request =
                                        Some((data.category.clone(), data.id.clone()));
                                }
                            }
                        }
                    }
                }
                self.tree.toggle_selected();
            }
            KeyCode::Char('r') => self.reload(),
            _ => {}
        }
    }

    fn take_action(&mut self) -> ScreenAction {
        self.inspect_request
            .take()
            .map(|(cat, id)| ScreenAction::Inspect(cat, id))
            .unwrap_or(ScreenAction::None)
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
