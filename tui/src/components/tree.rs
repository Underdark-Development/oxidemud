use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Widget,
};

use crate::components::ScrollState;

pub struct TreeNode<T> {
    pub label: String,
    pub data: T,
    pub children: Vec<TreeNode<T>>,
    pub collapsed: bool,
}

impl<T> TreeNode<T> {
    pub fn new(label: String, data: T) -> Self {
        TreeNode {
            label,
            data,
            children: Vec::new(),
            collapsed: false,
        }
    }

    pub fn add_child(&mut self, child: TreeNode<T>) {
        self.children.push(child);
    }

    pub fn is_leaf(&self) -> bool {
        self.children.is_empty()
    }
}

pub struct Tree<T> {
    pub roots: Vec<TreeNode<T>>,
    pub selected: Option<usize>,
    pub indent: u16,
    pub scroll: ScrollState,
}

impl<T> Tree<T> {
    pub fn new(roots: Vec<TreeNode<T>>) -> Self {
        Tree {
            roots,
            selected: None,
            indent: 2,
            scroll: ScrollState::new(),
        }
    }

    pub fn flatten(&self) -> Vec<(u16, &TreeNode<T>)> {
        let mut result = Vec::new();
        for node in &self.roots {
            flatten_node(node, 0, &mut result);
        }
        result
    }

    pub fn select_next(&mut self) {
        let count = self.flatten().len();
        if count == 0 {
            return;
        }
        let idx = self.selected.unwrap_or(0);
        self.selected = Some((idx + 1) % count);
        self.ensure_selected_visible();
    }

    pub fn select_prev(&mut self) {
        let count = self.flatten().len();
        if count == 0 {
            return;
        }
        let idx = self.selected.unwrap_or(0);
        self.selected = Some(if idx == 0 { count - 1 } else { idx - 1 });
        self.ensure_selected_visible();
    }

    pub fn toggle_selected(&mut self) {
        let idx = match self.selected {
            Some(i) => i,
            None => return,
        };
        let mut counter = 0;
        toggle_at(&mut self.roots, idx, &mut counter);
    }

    pub fn selected_data(&self) -> Option<&T> {
        let idx = self.selected?;
        self.flatten().get(idx).map(|(_, node)| &node.data)
    }

    pub fn ensure_selected_visible(&mut self) {
        let idx = match self.selected {
            Some(i) => i,
            None => return,
        };
        self.scroll.total_lines = self.flatten().len();
        if idx < self.scroll.offset {
            self.scroll.offset = idx;
        } else if idx >= self.scroll.offset + self.scroll.visible_lines {
            self.scroll.offset = idx
                .saturating_add(1)
                .saturating_sub(self.scroll.visible_lines);
        }
    }

    pub fn update_scroll(&mut self, area_height: usize) {
        let total = self.flatten().len();
        self.scroll.total_lines = total;
        self.scroll.visible_lines = area_height;
        self.ensure_selected_visible();
    }
}

fn flatten_node<'a, T>(
    node: &'a TreeNode<T>,
    depth: u16,
    result: &mut Vec<(u16, &'a TreeNode<T>)>,
) {
    result.push((depth, node));
    if !node.collapsed {
        for child in &node.children {
            flatten_node(child, depth + 1, result);
        }
    }
}

fn toggle_at<T>(nodes: &mut [TreeNode<T>], target: usize, counter: &mut usize) -> bool {
    for node in nodes.iter_mut() {
        if *counter == target {
            node.collapsed = !node.collapsed;
            return true;
        }
        *counter += 1;
        if !node.collapsed && toggle_at(&mut node.children, target, counter) {
            return true;
        }
    }
    false
}

impl<T> Widget for &Tree<T> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.height < 1 {
            return;
        }
        let flat = self.flatten();
        let offset = self.scroll.offset;
        let visible = self.scroll.visible_lines.max(1);

        for i in 0..visible {
            let idx = offset + i;
            if idx >= flat.len() {
                break;
            }
            let (depth, node) = flat[idx];
            let is_selected = Some(idx) == self.selected;
            let y = area.y + i as u16;

            let indent_str = " ".repeat((depth * self.indent) as usize);
            let prefix = if node.is_leaf() {
                " "
            } else if node.collapsed {
                "▸"
            } else {
                "▾"
            };

            let label_style = if is_selected {
                Style::default()
                    .fg(Color::White)
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };

            let prefix_style = if is_selected {
                label_style
            } else {
                Style::default().fg(Color::DarkGray)
            };

            let line = Line::from(vec![
                Span::styled(indent_str, Style::default()),
                Span::styled(format!("{} ", prefix), prefix_style),
                Span::styled(&node.label, label_style),
            ]);
            buf.set_line(area.x, y, &line, area.width);
        }
    }
}
