use ratatui::{
    buffer::Buffer,
    crossterm::event::{KeyCode, KeyEvent, MouseButton, MouseEvent, MouseEventKind},
    layout::Rect,
    style::Color,
    widgets::Widget,
};

use crate::components::{Tree, TreeNode};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandAction {
    CreateEntity(String),
    SaveEntity,
    SaveAllEntities,
    EditEntity,
    DeleteEntity,
    LookRoom,
    LookMobRoom,
    LookMobDetail,
    LookItem,
    GoToParent,
    ExpandAll,
    CollapseAll,
    ToggleSearch,
    ReloadContent,
    ValidateContent,
    ToggleHelp,
    SwitchScreen(usize),
    ToggleSidebar,
    ShowAbout,
    Quit,
    Separator,
}

const CREATE_ENTITIES: &[(&str, &str)] = &[
    ("area", "Area"),
    ("room", "Room"),
    ("mob", "Mob"),
    ("item", "Item"),
    ("race", "Race"),
    ("class", "Class"),
    ("skill", "Skill"),
    ("stance", "Stance"),
    ("set", "Set"),
    ("affix", "Affix"),
    ("passive", "Passive"),
];

pub struct CommandSidebar {
    tree: Tree<CommandAction>,
    prev_label: Option<(String, String)>,
    prev_cmds: Vec<(String, CommandAction)>,
}

impl Default for CommandSidebar {
    fn default() -> Self {
        Self::new()
    }
}

impl CommandSidebar {
    pub fn new() -> Self {
        let mut sb = CommandSidebar {
            tree: Tree::new(Self::build_roots(&[], None)),
            prev_label: None,
            prev_cmds: Vec::new(),
        };
        sb.tree.selected = Some(0);
        sb
    }

    pub fn update_context(
        &mut self,
        context_label: Option<(String, String)>,
        contextual_cmds: &[(String, CommandAction)],
    ) {
        if self.prev_label == context_label && self.prev_cmds.as_slice() == contextual_cmds {
            return;
        }
        self.prev_label = context_label.clone();
        self.prev_cmds = contextual_cmds.to_vec();
        self.tree = Tree::new(Self::build_roots(contextual_cmds, context_label));
        if self.tree.selected.is_none() && !self.tree.flatten().is_empty() {
            self.tree.selected = Some(0);
        }
    }

    pub fn render(
        &mut self,
        area: Rect,
        buf: &mut Buffer,
        focused: bool,
        mouse_pos: Option<(u16, u16)>,
    ) {
        if area.width < 2 || area.height < 2 {
            return;
        }
        let content_lines = area.height as usize;
        self.tree.indent = 1;
        self.tree.update_scroll(content_lines);

        // Fill with background
        for y in area.y..area.y + area.height {
            for x in area.x..area.x + area.width {
                if let Some(cell) = buf.cell_mut((x, y)) {
                    cell.set_bg(Color::Indexed(236));
                }
            }
        }

        self.tree.muted = !focused;
        self.tree.hovered = mouse_pos.and_then(|(col, row)| {
            if row >= area.y
                && row < area.y + area.height
                && col >= area.x
                && col < area.x + area.width
            {
                let line = (row - area.y) as usize;
                let idx = line + self.tree.scroll.offset;
                (idx < self.tree.flatten().len()).then_some(idx)
            } else {
                None
            }
        });

        let tree_area = Rect::new(area.x, area.y, area.width.saturating_sub(1), area.height);
        self.tree.render(tree_area, buf);

        let scrollbar_area = Rect::new(
            area.x + area.width.saturating_sub(1),
            area.y,
            1,
            area.height,
        );
        self.tree.scroll.render(scrollbar_area, buf);
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> Option<CommandAction> {
        match key.code {
            KeyCode::Up => {
                self.tree.select_prev();
                None
            }
            KeyCode::Down => {
                self.tree.select_next();
                None
            }
            KeyCode::Enter | KeyCode::Right => {
                let idx = self.tree.selected;
                if let Some(idx) = idx {
                    let flat = self.tree.flatten();
                    if let Some((_, node)) = flat.get(idx) {
                        if !node.is_leaf() && (node.collapsed || key.code == KeyCode::Right) {
                            self.tree.toggle_selected();
                            return None;
                        }
                        let action = node.data.clone();
                        if action != CommandAction::Separator {
                            return Some(action);
                        }
                    }
                }
                None
            }
            KeyCode::Left => {
                if let Some(idx) = self.tree.selected {
                    let flat = self.tree.flatten();
                    if let Some((_, node)) = flat.get(idx) {
                        if !node.is_leaf() && !node.collapsed {
                            self.tree.toggle_selected();
                            return None;
                        }
                    }
                }
                None
            }
            _ => None,
        }
    }

    pub fn handle_mouse(&mut self, mouse: MouseEvent, area: Rect) -> Option<CommandAction> {
        match mouse.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                let row = (mouse.row as usize).saturating_sub(area.y as usize);
                let idx = row.saturating_add(self.tree.scroll.offset);
                let action = {
                    let flat = self.tree.flatten();
                    let (_, node) = flat.get(idx)?;
                    if node.is_leaf() {
                        let a = node.data.clone();
                        if a != CommandAction::Separator {
                            Some(a)
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                };
                self.tree.selected = Some(idx);
                if action.is_none() {
                    let flat = self.tree.flatten();
                    if let Some((_, node)) = flat.get(idx) {
                        if !node.is_leaf() {
                            self.tree.toggle_selected();
                        }
                    }
                }
                action
            }
            MouseEventKind::ScrollUp => {
                self.tree.scroll_up();
                None
            }
            MouseEventKind::ScrollDown => {
                self.tree.scroll_down();
                None
            }
            _ => None,
        }
    }

    pub fn navigate_to(&mut self, action: &CommandAction) -> bool {
        for (i, (_, node)) in self.tree.flatten().iter().enumerate() {
            if &node.data == action {
                self.tree.selected = Some(i);
                return true;
            }
        }
        false
    }

    fn build_roots(
        contextual: &[(String, CommandAction)],
        context_label: Option<(String, String)>,
    ) -> Vec<TreeNode<CommandAction>> {
        let mut roots = Vec::new();

        let mut create_node = TreeNode::new("Create".to_string(), CommandAction::Separator);
        for (id, label) in CREATE_ENTITIES {
            create_node.add_child(TreeNode::new(
                label.to_string(),
                CommandAction::CreateEntity(id.to_string()),
            ));
        }
        create_node.collapsed = false;
        roots.push(create_node);

        let mut nav_node = TreeNode::new("Navigate".to_string(), CommandAction::Separator);
        for action in [
            CommandAction::GoToParent,
            CommandAction::ExpandAll,
            CommandAction::CollapseAll,
        ] {
            let label = match action {
                CommandAction::GoToParent => "Go to Parent",
                CommandAction::ExpandAll => "Expand All",
                CommandAction::CollapseAll => "Collapse All",
                _ => unreachable!(),
            };
            nav_node.add_child(TreeNode::new(label.to_string(), action));
        }
        nav_node.collapsed = false;
        roots.push(nav_node);

        let mut tools_node = TreeNode::new("Tools".to_string(), CommandAction::Separator);
        for (action, label) in [
            (CommandAction::ToggleSearch, "Search [/]"),
            (CommandAction::ReloadContent, "Reload [r]"),
            (CommandAction::ValidateContent, "Validate"),
            (CommandAction::SaveAllEntities, "Save All"),
            (CommandAction::ToggleHelp, "Help [?]"),
            (CommandAction::Quit, "Quit [Ctrl+D]"),
        ] {
            tools_node.add_child(TreeNode::new(label.to_string(), action));
        }
        tools_node.collapsed = false;
        roots.push(tools_node);

        if !contextual.is_empty() {
            if let Some((ref name, ref cat)) = context_label {
                let header = format!("─ {name} ({cat}) ─");
                let header = if header.chars().count() > 20 {
                    let byte_end = header
                        .char_indices()
                        .take(19)
                        .last()
                        .map(|(i, c)| i + c.len_utf8())
                        .unwrap_or(0);
                    format!("{}…", &header[..byte_end])
                } else {
                    header
                };
                let mut ctx_node = TreeNode::new(header, CommandAction::Separator);
                for (label, action) in contextual {
                    ctx_node.add_child(TreeNode::new(label.clone(), action.clone()));
                }
                ctx_node.collapsed = false;
                roots.push(ctx_node);
            }
        }

        roots
    }
}
