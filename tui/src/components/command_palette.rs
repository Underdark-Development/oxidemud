use ratatui::{
    buffer::Buffer,
    crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind},
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Clear, Widget},
};

use crate::components::CommandAction;
use crate::screens::ScreenId;

#[derive(Debug, Clone)]
pub struct PaletteItem {
    pub label: String,
    pub shortcut: Option<String>,
    pub action: CommandAction,
}

impl PaletteItem {
    pub fn new(label: &str, shortcut: Option<&str>, action: CommandAction) -> Self {
        Self {
            label: label.to_string(),
            shortcut: shortcut.map(|s| s.to_string()),
            action,
        }
    }
}

pub struct CommandPalette {
    pub input: String,
    pub selected_index: usize,
    pub items: Vec<PaletteItem>,
    pub item_rects: Vec<Rect>,
    pub hovered_index: Option<usize>,
}

impl CommandPalette {
    pub fn new() -> Self {
        let items = vec![
            PaletteItem::new(
                "Switch to Entities Editor",
                Some("F1"),
                CommandAction::SwitchScreen(ScreenId::Entities.as_index()),
            ),
            PaletteItem::new(
                "Switch to Room Grid",
                Some("F2"),
                CommandAction::SwitchScreen(ScreenId::RoomGrid.as_index()),
            ),
            PaletteItem::new(
                "Switch to Validation Panel",
                Some("F3"),
                CommandAction::SwitchScreen(ScreenId::Validation.as_index()),
            ),
            PaletteItem::new(
                "Switch to File Browser",
                Some("F4"),
                CommandAction::SwitchScreen(ScreenId::FileBrowser.as_index()),
            ),
            PaletteItem::new(
                "Switch to Script Console",
                Some("F5"),
                CommandAction::SwitchScreen(ScreenId::ScriptConsole.as_index()),
            ),
            PaletteItem::new(
                "Switch to Live Dashboard",
                Some("F6"),
                CommandAction::SwitchScreen(ScreenId::LiveDashboard.as_index()),
            ),
            PaletteItem::new(
                "Save Active Entity",
                Some("Ctrl+S"),
                CommandAction::SaveEntity,
            ),
            PaletteItem::new(
                "Duplicate Active Entity",
                Some("Ctrl+D"),
                CommandAction::DuplicateEntity,
            ),
            PaletteItem::new("Save All Entities", None, CommandAction::SaveAllEntities),
            PaletteItem::new(
                "Reload Content Templates",
                Some("Ctrl+R"),
                CommandAction::ReloadContent,
            ),
            PaletteItem::new("Validate Content", None, CommandAction::ValidateContent),
            PaletteItem::new(
                "Toggle Sidebar",
                Some("Ctrl+B"),
                CommandAction::ToggleSidebar,
            ),
            PaletteItem::new(
                "Show Notification History Log",
                None,
                CommandAction::ShowNotificationHistory,
            ),
            PaletteItem::new(
                "Toggle Raw TOML / Form View",
                Some("Ctrl+E"),
                CommandAction::ToggleViewMode,
            ),
            PaletteItem::new("Toggle Help Modal", Some("?"), CommandAction::ToggleHelp),
            PaletteItem::new(
                "Create Area",
                None,
                CommandAction::CreateEntity("area".to_string()),
            ),
            PaletteItem::new(
                "Create Room",
                None,
                CommandAction::CreateEntity("room".to_string()),
            ),
            PaletteItem::new(
                "Create Mob",
                None,
                CommandAction::CreateEntity("mob".to_string()),
            ),
            PaletteItem::new(
                "Create Item",
                None,
                CommandAction::CreateEntity("item".to_string()),
            ),
            PaletteItem::new(
                "Create Race",
                None,
                CommandAction::CreateEntity("race".to_string()),
            ),
            PaletteItem::new(
                "Create Class",
                None,
                CommandAction::CreateEntity("class".to_string()),
            ),
            PaletteItem::new(
                "Create Skill",
                None,
                CommandAction::CreateEntity("skill".to_string()),
            ),
            PaletteItem::new(
                "Create Stance",
                None,
                CommandAction::CreateEntity("stance".to_string()),
            ),
            PaletteItem::new(
                "Create Set",
                None,
                CommandAction::CreateEntity("set".to_string()),
            ),
            PaletteItem::new(
                "Create Affix",
                None,
                CommandAction::CreateEntity("affix".to_string()),
            ),
            PaletteItem::new(
                "Create Passive",
                None,
                CommandAction::CreateEntity("passive".to_string()),
            ),
            PaletteItem::new("Expand All Nodes", None, CommandAction::ExpandAll),
            PaletteItem::new("Collapse All Nodes", None, CommandAction::CollapseAll),
            PaletteItem::new("Search Entities", Some("/"), CommandAction::ToggleSearch),
            PaletteItem::new(
                "Switch Mode to Online (WSS)",
                None,
                CommandAction::SwitchMode(crate::app::Mode::Online),
            ),
            PaletteItem::new(
                "Switch Mode to Offline",
                None,
                CommandAction::SwitchMode(crate::app::Mode::Offline),
            ),
            PaletteItem::new(
                "Switch Mode to Split",
                None,
                CommandAction::SwitchMode(crate::app::Mode::Split),
            ),
            PaletteItem::new("Show About Dialog", None, CommandAction::ShowAbout),
            PaletteItem::new("Quit Spade", Some("Ctrl+D"), CommandAction::Quit),
        ];

        Self {
            input: String::new(),
            selected_index: 0,
            items,
            item_rects: Vec::new(),
            hovered_index: None,
        }
    }

    pub fn reset(&mut self) {
        self.input.clear();
        self.selected_index = 0;
        self.item_rects.clear();
        self.hovered_index = None;
    }

    pub fn filtered_items(&self) -> Vec<PaletteItem> {
        if self.input.is_empty() {
            return self.items.clone();
        }

        let mut scored: Vec<(usize, PaletteItem)> = self
            .items
            .iter()
            .filter_map(|item| {
                match_score(&item.label, &self.input).map(|score| (score, item.clone()))
            })
            .collect();

        scored.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.label.cmp(&b.1.label)));

        scored.into_iter().map(|(_, item)| item).collect()
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> Option<CommandAction> {
        let filtered = self.filtered_items();
        if self.selected_index >= filtered.len() {
            self.selected_index = 0;
        }

        match key.code {
            KeyCode::Up => {
                if !filtered.is_empty() {
                    if self.selected_index == 0 {
                        self.selected_index = filtered.len() - 1;
                    } else {
                        self.selected_index -= 1;
                    }
                }
                None
            }
            KeyCode::Down => {
                if !filtered.is_empty() {
                    if self.selected_index + 1 >= filtered.len() {
                        self.selected_index = 0;
                    } else {
                        self.selected_index += 1;
                    }
                }
                None
            }
            KeyCode::Enter => {
                if !filtered.is_empty() && self.selected_index < filtered.len() {
                    Some(filtered[self.selected_index].action.clone())
                } else {
                    None
                }
            }
            KeyCode::Char(c) => {
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    return None;
                }
                self.input.push(c);
                self.selected_index = 0;
                None
            }
            KeyCode::Backspace => {
                self.input.pop();
                self.selected_index = 0;
                None
            }
            _ => None,
        }
    }

    pub fn handle_mouse(&mut self, mouse: MouseEvent, _area: Rect) -> Option<CommandAction> {
        let filtered = self.filtered_items();

        if mouse.kind == MouseEventKind::Down(MouseButton::Left) {
            for (i, rect) in self.item_rects.iter().enumerate() {
                if mouse.column >= rect.x
                    && mouse.column < rect.x + rect.width
                    && mouse.row == rect.y
                    && i < filtered.len()
                {
                    return Some(filtered[i].action.clone());
                }
            }
        }
        None
    }

    pub fn render(&mut self, area: Rect, buf: &mut Buffer, mouse_pos: Option<(u16, u16)>) {
        let width = 60.min(area.width.saturating_sub(4));
        let height = 15.min(area.height.saturating_sub(4));

        let x = area.x + (area.width.saturating_sub(width)) / 2;
        let y = area.y + (area.height.saturating_sub(height)) / 2;
        let palette_area = Rect::new(x, y, width, height);

        Clear.render(palette_area, buf);

        // Draw border
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan))
            .title(ratatui::text::Span::styled(
                " Command Palette ",
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ))
            .style(Style::default().bg(Color::Black));
        let inner = block.inner(palette_area);
        block.render(palette_area, buf);

        // Fill background black
        for iy in inner.y..inner.y + inner.height {
            for ix in inner.x..inner.x + inner.width {
                if let Some(cell) = buf.cell_mut((ix, iy)) {
                    cell.set_char(' ');
                    cell.set_bg(Color::Black);
                }
            }
        }

        if inner.height < 3 {
            return;
        }

        // Draw Input Prompt
        let prompt = "> ";
        buf.set_string(inner.x, inner.y, prompt, Style::default().fg(Color::Cyan));

        if self.input.is_empty() {
            buf.set_string(
                inner.x + 2,
                inner.y,
                "Search commands...",
                Style::default().fg(Color::Indexed(240)),
            );
        } else {
            buf.set_string(
                inner.x + 2,
                inner.y,
                &self.input,
                Style::default().fg(Color::White),
            );
        }

        // Draw active cursor
        let cursor_x = inner.x + 2 + self.input.chars().count() as u16;
        if cursor_x < inner.x + inner.width {
            if let Some(cell) = buf.cell_mut((cursor_x, inner.y)) {
                cell.set_bg(Color::Indexed(248));
                cell.set_fg(Color::Black);
            }
        }

        // Separator line
        let sep_y = inner.y + 1;
        for ix in inner.x..inner.x + inner.width {
            if let Some(cell) = buf.cell_mut((ix, sep_y)) {
                cell.set_char('─');
                cell.set_fg(Color::Indexed(240));
            }
        }

        // Matching items list
        let filtered = self.filtered_items();
        let list_y = sep_y + 1;
        let list_height = inner.height.saturating_sub(2) as usize;

        if self.selected_index >= filtered.len() {
            self.selected_index = 0;
        }

        self.item_rects.clear();
        self.hovered_index = None;

        for i in 0..list_height {
            if i >= filtered.len() {
                break;
            }
            let item = &filtered[i];
            let item_y = list_y + i as u16;
            let is_selected = i == self.selected_index;

            let row_rect = Rect::new(inner.x, item_y, inner.width, 1);
            self.item_rects.push(row_rect);

            if let Some((mx, my)) = mouse_pos {
                if mx >= row_rect.x && mx < row_rect.x + row_rect.width && my == row_rect.y {
                    self.hovered_index = Some(i);
                }
            }

            let is_hovered = self.hovered_index == Some(i);

            let style = if is_selected {
                Style::default().fg(Color::Black).bg(Color::Cyan)
            } else if is_hovered {
                Style::default().fg(Color::White).bg(Color::Indexed(238))
            } else {
                Style::default().fg(Color::Indexed(250)).bg(Color::Black)
            };

            for ix in row_rect.x..row_rect.x + row_rect.width {
                if let Some(cell) = buf.cell_mut((ix, item_y)) {
                    cell.set_bg(style.bg.unwrap_or(Color::Black));
                }
            }

            let display_label = format!(" {}", item.label);
            buf.set_string(row_rect.x, item_y, &display_label, style);

            if let Some(ref sc) = item.shortcut {
                let sc_str = format!("{} ", sc);
                let sc_x = (row_rect.x + row_rect.width).saturating_sub(sc_str.len() as u16);
                if sc_x > row_rect.x + display_label.len() as u16 {
                    buf.set_string(sc_x, item_y, &sc_str, style);
                }
            }
        }
    }
}

impl Default for CommandPalette {
    fn default() -> Self {
        Self::new()
    }
}

fn match_score(label: &str, query: &str) -> Option<usize> {
    if query.is_empty() {
        return Some(1);
    }
    let label_lower = label.to_lowercase();
    let query_lower = query.to_lowercase();

    if label_lower == query_lower {
        return Some(1000);
    }
    if let Some(idx) = label_lower.find(&query_lower) {
        return Some(500 - idx);
    }
    let mut score = 0;
    let mut label_chars = label_lower.chars().enumerate();
    let mut last_idx = 0;
    let mut matched_chars = 0;
    for q_char in query_lower.chars() {
        let mut found = false;
        for (idx, l_char) in label_chars.by_ref() {
            if l_char == q_char {
                let gap = idx - last_idx;
                score += 100_usize.saturating_sub(gap * 5);
                last_idx = idx;
                matched_chars += 1;
                found = true;
                break;
            }
        }
        if !found {
            return None;
        }
    }
    if matched_chars == query_lower.len() {
        Some(score.max(1))
    } else {
        None
    }
}
