use oxide_core::templates::RoomTemplate;
use ratatui::{
    buffer::Buffer,
    crossterm::event::{KeyCode, KeyEvent, MouseButton, MouseEvent, MouseEventKind},
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::Widget,
};

use crate::components::{Tree, TreeNode};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandAction {
    CreateEntity(String),
    DuplicateEntity,
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
    ToggleViewMode,
    SwitchScreen(usize),
    ToggleSidebar,
    ShowNotificationHistory,
    ShowAbout,
    Quit,
    MoveToRoom(String),
    DigRoom(String),
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
    /// Cached contextual data (change detection for re-render).
    prev_label: Option<(String, String)>,
    prev_cmds: Vec<(String, CommandAction)>,
    /// Keyboard selection among action items (index into action_rects).
    actions_selected: Option<usize>,
    /// Hovered action index (derived from mouse_pos during render).
    actions_hovered: Option<usize>,
    /// Stored rects for action command items (mouse hit-testing).
    action_rects: Vec<Rect>,
    pub room_details: Option<RoomTemplate>,
}

impl Default for CommandSidebar {
    fn default() -> Self {
        Self::new()
    }
}

impl CommandSidebar {
    pub fn new() -> Self {
        let mut sb = CommandSidebar {
            tree: Tree::new(Self::build_roots()),
            prev_label: None,
            prev_cmds: Vec::new(),
            actions_selected: None,
            actions_hovered: None,
            action_rects: Vec::new(),
            room_details: None,
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
        self.tree = Tree::new(Self::build_roots());
        if self.tree.selected.is_none() && !self.tree.flatten().is_empty() {
            self.tree.selected = Some(0);
        }
        self.actions_selected = None;
        self.action_rects.clear();
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

        // Fill with background
        for y in area.y..area.y + area.height {
            for x in area.x..area.x + area.width {
                if let Some(cell) = buf.cell_mut((x, y)) {
                    cell.set_bg(Color::Indexed(236));
                }
            }
        }

        let has_actions = !self.prev_cmds.is_empty();
        let entity_label = self.prev_label.as_ref().map(|(n, c)| {
            let s = Self::singularize(c);
            format!(" {n} ({s})")
        });
        let actions_height: usize = if has_actions {
            1 + // separator
            1 + // "Actions" header
            1 + // entity name line
            self.prev_cmds.len()
        } else {
            0
        };

        let tree_height = (area.height as usize).saturating_sub(actions_height);
        let tree_area = Rect::new(
            area.x,
            area.y,
            area.width.saturating_sub(1),
            tree_height as u16,
        );
        if let Some(ref room) = self.room_details {
            let bold_label = Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD);
            let val_style = Style::default().fg(Color::White);
            let desc_style = Style::default().fg(Color::Indexed(250));

            let mut y = tree_area.y + 1;
            let x = tree_area.x + 1;
            let w = tree_area.width.saturating_sub(2) as usize;

            // Title
            buf.set_string(x, y, "Room Attributes", bold_label.fg(Color::Green));
            y += 2;

            // ID
            if y < tree_area.y + tree_area.height {
                buf.set_string(x, y, "ID:", bold_label);
                let id_str = format!(" {}", room.id);
                buf.set_string(x + 6, y, &id_str, val_style);
                y += 1;
            }

            // Area
            if y < tree_area.y + tree_area.height {
                buf.set_string(x, y, "Area:", bold_label);
                let area_str = format!(" {}", room.area);
                buf.set_string(x + 6, y, &area_str, val_style);
                y += 1;
            }

            // Name
            if y < tree_area.y + tree_area.height {
                buf.set_string(x, y, "Name:", bold_label);
                let name_str = format!(" {}", room.name);
                buf.set_string(x + 6, y, &name_str, val_style);
                y += 2;
            }

            // Description
            if y < tree_area.y + tree_area.height {
                buf.set_string(x, y, "Description:", bold_label);
                y += 1;
                let wrapped_desc = wrap_text(&room.description, w);
                for line in wrapped_desc.iter().take(4) {
                    if y >= tree_area.y + tree_area.height {
                        break;
                    }
                    buf.set_string(x, y, line, desc_style);
                    y += 1;
                }
                if wrapped_desc.len() > 4 && y < tree_area.y + tree_area.height {
                    buf.set_string(
                        x,
                        y - 1,
                        "... (truncated)",
                        desc_style.fg(Color::Indexed(244)),
                    );
                }
            }
            y += 1;

            // Exits
            if y < tree_area.y + tree_area.height {
                buf.set_string(x, y, "Exits:", bold_label);
                y += 1;
                if room.exits.is_empty() {
                    buf.set_string(x, y, "  none", desc_style);
                    y += 1;
                } else {
                    let mut exits_sorted: Vec<(&String, &oxide_core::ExitTemplate)> =
                        room.exits.iter().collect();
                    exits_sorted.sort_by_key(|(dir, _)| dir.to_lowercase());
                    for (dir, dest) in exits_sorted {
                        if y >= tree_area.y + tree_area.height {
                            break;
                        }
                        let exit_line = format!("  {} -> {}", dir, dest.dest());
                        buf.set_string(x, y, &exit_line, desc_style);
                        y += 1;
                    }
                }
            }
            y += 1;

            // Portals
            if y < tree_area.y + tree_area.height && !room.portals.is_empty() {
                buf.set_string(x, y, "Portals:", bold_label);
                y += 1;
                for portal in &room.portals {
                    if y >= tree_area.y + tree_area.height {
                        break;
                    }
                    let portal_line = format!("  {} -> {}", portal.keyword, portal.dest);
                    buf.set_string(x, y, &portal_line, desc_style);
                    y += 1;
                }
            }
        } else {
            self.tree.indent = 1;
            self.tree.update_scroll(tree_height);
            self.tree.muted = !focused;

            // Tree hover (only tree region)
            self.tree.hovered = mouse_pos.and_then(|(col, row)| {
                if row >= tree_area.y
                    && row < tree_area.y + tree_area.height
                    && col >= area.x
                    && col < area.x + area.width
                {
                    let line = (row - tree_area.y) as usize;
                    let idx = line + self.tree.scroll.offset;
                    (idx < self.tree.flatten().len()).then_some(idx)
                } else {
                    None
                }
            });

            self.tree.render(tree_area, buf);

            // Scrollbar for tree area
            let scrollbar_area = Rect::new(
                area.x + area.width.saturating_sub(1),
                area.y,
                1,
                tree_height as u16,
            );
            // Only render tree scroll if tree doesn't fit
            if self.tree.scroll.total_lines > self.tree.scroll.visible_lines {
                self.tree.scroll.render(scrollbar_area, buf);
            }
        }

        // ---- Actions section (bottom) ----
        if !has_actions {
            return;
        }

        self.action_rects.clear();
        let actions_y = area.y + tree_height as u16;
        let actions_w = area.width;

        // Determined action hover index from mouse_pos
        self.actions_hovered = mouse_pos.and_then(|(col, row)| {
            if row >= actions_y
                && row < actions_y + actions_height as u16
                && col >= area.x
                && col < area.x + actions_w
            {
                // Check each stored rect after we build them; for now rely on
                // the rects we'll build below — but we need to compute before storing.
                // We'll set it after constructing action_rects.
                None
            } else {
                None
            }
        });

        // Separator line
        let sep_y = actions_y;
        for x in area.x..area.x + actions_w.saturating_sub(1) {
            if let Some(cell) = buf.cell_mut((x, sep_y)) {
                cell.set_char('\u{2500}');
                cell.set_fg(Color::Indexed(245));
                cell.set_bg(Color::Indexed(236));
            }
        }

        // "Actions" header (decorative, not selectable)
        let header_y = sep_y + 1;
        let header_fg = if focused {
            Color::White
        } else {
            Color::Indexed(245)
        };
        buf.set_string(
            area.x,
            header_y,
            " Actions",
            Style::default()
                .fg(header_fg)
                .bg(Color::Indexed(236))
                .add_modifier(Modifier::BOLD),
        );

        // Entity name line (decorative, not selectable)
        let entity_y = header_y + 1;
        if let Some(ref label) = entity_label {
            buf.set_string(
                area.x + 1,
                entity_y,
                label,
                Style::default()
                    .fg(Color::Indexed(245))
                    .bg(Color::Indexed(236)),
            );
        }

        // Action command items (selectable)
        let cmds_start_y = header_y + 2;
        let action_text_muted = !focused;
        let action_fg = if action_text_muted {
            Color::Indexed(245)
        } else {
            Color::White
        };

        for (i, (cmd_label, _)) in self.prev_cmds.iter().enumerate() {
            let cmd_y = cmds_start_y + i as u16;
            let is_hovered = mouse_pos.is_some_and(|(col, row)| {
                row == cmd_y && col >= area.x && col < area.x + actions_w
            });
            let is_selected = self.actions_selected == Some(i);

            let bg = if is_selected || is_hovered {
                Color::Indexed(240)
            } else {
                Color::Indexed(236)
            };

            if is_selected || is_hovered {
                for x in area.x..area.x + actions_w {
                    if let Some(cell) = buf.cell_mut((x, cmd_y)) {
                        cell.set_bg(Color::Indexed(240));
                    }
                }
            }

            let text = format!("  {cmd_label}");
            buf.set_string(area.x, cmd_y, &text, Style::default().fg(action_fg).bg(bg));

            // Store rect for mouse hit-testing
            self.action_rects
                .push(Rect::new(area.x, cmd_y, actions_w, 1));
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> Option<CommandAction> {
        let tree_len = self.tree.flatten().len();

        match key.code {
            KeyCode::Up => {
                if let Some(sel) = self.actions_selected {
                    if sel > 0 {
                        self.actions_selected = Some(sel - 1);
                    } else if tree_len > 0 {
                        // Move back to tree, last item
                        self.actions_selected = None;
                        self.tree.selected = Some(tree_len - 1);
                    }
                } else {
                    self.tree.select_prev();
                    // If tree has no items, nothing to do
                }
                None
            }
            KeyCode::Down => {
                if let Some(sel) = self.actions_selected {
                    if sel + 1 < self.prev_cmds.len() {
                        self.actions_selected = Some(sel + 1);
                    }
                } else if !self.prev_cmds.is_empty()
                    && self.tree.selected.is_none_or(|s| s + 1 >= tree_len)
                {
                    // Last tree item -> move to actions
                    self.tree.selected = None;
                    self.actions_selected = Some(0);
                } else {
                    self.tree.select_next();
                }
                None
            }
            KeyCode::Enter | KeyCode::Right => {
                // Try tree first
                if let Some(idx) = self.tree.selected {
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
                // Try actions
                if let Some(sel) = self.actions_selected {
                    if sel < self.prev_cmds.len() {
                        return Some(self.prev_cmds[sel].1.clone());
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
                let row_rel = (mouse.row as usize).saturating_sub(area.y as usize);

                // Count how many lines the tree section occupies
                let has_actions = !self.prev_cmds.is_empty();
                let actions_height = if has_actions {
                    4 + self.prev_cmds.len()
                } else {
                    0
                };
                let tree_height = (area.height as usize).saturating_sub(actions_height);

                if row_rel < tree_height {
                    // Tree section
                    let idx = row_rel.saturating_add(self.tree.scroll.offset);
                    self.actions_selected = None;
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
                } else {
                    // Actions section
                    let cmd_y = mouse.row;

                    for (i, rect) in self.action_rects.iter().enumerate() {
                        if cmd_y >= rect.y
                            && cmd_y < rect.y + rect.height
                            && mouse.column >= rect.x
                            && mouse.column < rect.x + rect.width
                        {
                            self.actions_selected = Some(i);
                            self.tree.selected = None;
                            return Some(self.prev_cmds[i].1.clone());
                        }
                    }
                    None
                }
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
        // Search tree first
        for (i, (_, node)) in self.tree.flatten().iter().enumerate() {
            if &node.data == action {
                self.tree.selected = Some(i);
                self.actions_selected = None;
                return true;
            }
        }
        // Search action items
        for (i, (_, a)) in self.prev_cmds.iter().enumerate() {
            if a == action {
                self.actions_selected = Some(i);
                self.tree.selected = None;
                return true;
            }
        }
        false
    }

    fn singularize(category: &str) -> &str {
        match category {
            "items" => "item",
            "mobs" => "mob",
            "races" => "race",
            "classes" => "class",
            "skills" => "skill",
            "stances" => "stance",
            "sets" => "set",
            "affixes" => "affix",
            "passives" => "passive",
            "areas" => "area",
            "rooms" => "room",
            _ => category,
        }
    }

    fn build_roots() -> Vec<TreeNode<CommandAction>> {
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

        roots
    }
}

fn wrap_text(text: &str, max_width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    for paragraph in text.split('\n') {
        if paragraph.is_empty() {
            lines.push(String::new());
            continue;
        }
        let mut current_line = String::new();
        for word in paragraph.split_whitespace() {
            if current_line.is_empty() {
                current_line.push_str(word);
            } else if current_line.len() + 1 + word.len() <= max_width {
                current_line.push(' ');
                current_line.push_str(word);
            } else {
                lines.push(current_line);
                current_line = word.to_string();
            }
        }
        if !current_line.is_empty() {
            lines.push(current_line);
        }
    }
    lines
}
