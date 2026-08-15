use ratatui::{
    buffer::Buffer,
    crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind},
    layout::Rect,
    style::{Color, Style},
};

use super::dropdown::{dropdown_item_style, highlight_dropdown_row, render_dropdown_box};
use super::CommandAction;
use crate::screens::ScreenId;

const APP_NAME: &str = " spade ";

#[derive(Debug, Clone)]
pub struct MenuItem {
    pub label: String,
    pub shortcut: Option<String>,
    pub action: Option<CommandAction>,
    pub submenu: Vec<MenuItem>,
}

impl MenuItem {
    pub fn action(
        label: impl Into<String>,
        shortcut: Option<String>,
        action: CommandAction,
    ) -> Self {
        MenuItem {
            label: label.into(),
            shortcut,
            action: Some(action),
            submenu: Vec::new(),
        }
    }

    pub fn separator() -> Self {
        MenuItem {
            label: String::new(),
            shortcut: None,
            action: None,
            submenu: Vec::new(),
        }
    }

    pub fn submenu(label: impl Into<String>, items: Vec<MenuItem>) -> Self {
        MenuItem {
            label: label.into(),
            shortcut: None,
            action: None,
            submenu: items,
        }
    }

    pub fn is_separator(&self) -> bool {
        self.label.is_empty() && self.action.is_none() && self.submenu.is_empty()
    }

    pub fn has_submenu(&self) -> bool {
        !self.submenu.is_empty()
    }
}

#[derive(Debug, Clone)]
struct Menu {
    label: String,
    hotkey: char,
    items: Vec<MenuItem>,
    x_pos: u16,
}

impl Menu {
    fn new(label: &str, hotkey: char, items: Vec<MenuItem>) -> Self {
        Menu {
            label: label.to_string(),
            hotkey,
            items,
            x_pos: 0,
        }
    }

    fn label_end(&self) -> u16 {
        self.x_pos + self.label.len() as u16 + 2 // +2 for leading/trailing spaces
    }
}

#[derive(Debug, Clone)]
pub struct SubmenuState {
    pub selected: Option<usize>,
    pub hovered: Option<usize>,
}

pub struct MenuBar {
    menus: Vec<Menu>,
    pub open_menu: Option<usize>,
    pub selected: Option<usize>,
    pub hovered: Option<usize>,
    pub open_submenu: Option<Box<(usize, SubmenuState)>>,
    pub hovered_label: Option<usize>,
}

impl Default for MenuBar {
    fn default() -> Self {
        Self::new()
    }
}

impl MenuBar {
    pub fn new() -> Self {
        let menus = Self::build_menus();
        MenuBar {
            menus,
            open_menu: None,
            selected: None,
            hovered: None,
            open_submenu: None,
            hovered_label: None,
        }
    }

    fn build_menus() -> Vec<Menu> {
        let new_items = vec![
            MenuItem::action("Area", None, CommandAction::CreateEntity("area".into())),
            MenuItem::action("Room", None, CommandAction::CreateEntity("room".into())),
            MenuItem::action("Mob", None, CommandAction::CreateEntity("mob".into())),
            MenuItem::action("Item", None, CommandAction::CreateEntity("item".into())),
            MenuItem::action("Race", None, CommandAction::CreateEntity("race".into())),
            MenuItem::action("Class", None, CommandAction::CreateEntity("class".into())),
            MenuItem::action("Skill", None, CommandAction::CreateEntity("skill".into())),
            MenuItem::action("Stance", None, CommandAction::CreateEntity("stance".into())),
            MenuItem::action("Set", None, CommandAction::CreateEntity("set".into())),
            MenuItem::action("Affix", None, CommandAction::CreateEntity("affix".into())),
            MenuItem::action(
                "Passive",
                None,
                CommandAction::CreateEntity("passive".into()),
            ),
        ];

        let screen_items: Vec<MenuItem> = ScreenId::all()
            .iter()
            .map(|id| {
                let shortcut = id.fkey().map(|n| format!("F{n}"));
                MenuItem::action(
                    id.name(),
                    shortcut,
                    CommandAction::SwitchScreen(id.as_index()),
                )
            })
            .collect();

        vec![
            Menu::new(
                "World",
                'w',
                vec![
                    MenuItem::submenu("New", new_items),
                    MenuItem::separator(),
                    MenuItem::action("Save", Some("Ctrl+S".into()), CommandAction::SaveEntity),
                    MenuItem::action("Save All", None, CommandAction::SaveAllEntities),
                    MenuItem::action(
                        "Reload",
                        Some("Ctrl+R".into()),
                        CommandAction::ReloadContent,
                    ),
                    MenuItem::separator(),
                    MenuItem::action("Connect to Server", None, CommandAction::ConnectServer),
                    MenuItem::action("Disconnect Server", None, CommandAction::DisconnectServer),
                    MenuItem::separator(),
                    MenuItem::action("Quit", Some("Ctrl+D".into()), CommandAction::Quit),
                ],
            ),
            Menu::new(
                "View",
                'v',
                vec![
                    MenuItem::submenu("Screens", screen_items),
                    MenuItem::separator(),
                    MenuItem::action(
                        "Toggle Sidebar",
                        Some("Ctrl+B".into()),
                        CommandAction::ToggleSidebar,
                    ),
                    MenuItem::action(
                        "Toggle Raw TOML View",
                        Some("Ctrl+E".into()),
                        CommandAction::ToggleViewMode,
                    ),
                    MenuItem::action("Search", Some("/".into()), CommandAction::ToggleSearch),
                    MenuItem::separator(),
                    MenuItem::action("Expand All", None, CommandAction::ExpandAll),
                    MenuItem::action("Collapse All", None, CommandAction::CollapseAll),
                ],
            ),
            Menu::new(
                "Help",
                'h',
                vec![
                    MenuItem::action("Shortcuts", Some("?".into()), CommandAction::ToggleHelp),
                    MenuItem::separator(),
                    MenuItem::action("About", None, CommandAction::ShowAbout),
                ],
            ),
        ]
    }

    pub fn close_all(&mut self) {
        self.open_menu = None;
        self.selected = None;
        self.hovered = None;
        self.open_submenu = None;
    }

    pub fn render_top_bar(&mut self, buf: &mut Buffer, area: Rect, screen_name: &str) {
        if area.height < 1 {
            return;
        }

        for x in area.x..area.x + area.width {
            if let Some(cell) = buf.cell_mut((x, area.y)) {
                cell.set_bg(Color::Indexed(236));
            }
        }

        let mut x = area.x;

        buf.set_string(
            x,
            area.y,
            " ",
            Style::default().fg(Color::White).bg(Color::Indexed(236)),
        );
        x += 1;

        buf.set_string(
            x,
            area.y,
            APP_NAME,
            Style::default().fg(Color::White).bg(Color::Indexed(236)),
        );
        for (i, _) in APP_NAME.char_indices() {
            let cx = x + i as u16;
            if let Some(cell) = buf.cell_mut((cx, area.y)) {
                cell.set_fg(Color::Black);
                cell.set_bg(Color::White);
            }
        }
        x += APP_NAME.len() as u16;

        for (i, menu) in self.menus.iter_mut().enumerate() {
            menu.x_pos = x;
            let is_active = self.open_menu == Some(i) || self.hovered_label == Some(i);
            let label = format!(" {} ", menu.label);
            let (fg, bg) = if is_active {
                (Color::Black, Color::White)
            } else {
                (Color::White, Color::Indexed(236))
            };
            buf.set_string(x, area.y, &label, Style::default().fg(fg).bg(bg));
            x += label.len() as u16;
        }

        let used = x - area.x;
        let remaining = area.width.saturating_sub(used);
        if remaining > screen_name.len() as u16 {
            let sx = used + (remaining - screen_name.len() as u16) / 2;
            buf.set_string(
                area.x + sx,
                area.y,
                screen_name,
                Style::default()
                    .fg(Color::Indexed(245))
                    .bg(Color::Indexed(236)),
            );
        }
    }

    pub fn render_dropdowns(&mut self, buf: &mut Buffer, area: Rect) {
        if self.open_menu.is_none() {
            return;
        }
        let menu_idx = self.open_menu.unwrap();
        if menu_idx >= self.menus.len() {
            return;
        }
        let menu = &self.menus[menu_idx];

        let items: Vec<&MenuItem> = menu.items.iter().collect();
        let dropdown_rect = self.dropdown_rect(area, menu_idx, &items);
        self.render_dropdown(
            buf,
            dropdown_rect,
            &items,
            self.selected,
            self.hovered,
            false,
        );

        if let Some(ref sub) = self.open_submenu {
            let sub_idx = sub.0;
            if sub_idx < menu.items.len() {
                let item = &menu.items[sub_idx];
                if item.has_submenu() {
                    let sub_items: Vec<&MenuItem> = item.submenu.iter().collect();
                    let sub_rect = self.submenu_rect(area, dropdown_rect, sub_idx, &sub_items);
                    self.render_dropdown(
                        buf,
                        sub_rect,
                        &sub_items,
                        sub.1.selected,
                        sub.1.hovered,
                        false,
                    );
                }
            }
        }
    }

    fn dropdown_rect(&self, area: Rect, menu_idx: usize, items: &[&MenuItem]) -> Rect {
        let menu = &self.menus[menu_idx];
        let mut max_w = 0u16;
        for item in items {
            if item.is_separator() {
                continue;
            }
            let label_w = item.label.len() as u16;
            let shortcut_w = item
                .shortcut
                .as_ref()
                .map(|s| s.len() as u16 + 2)
                .unwrap_or(0);
            max_w = max_w.max(label_w + shortcut_w);
        }
        let w = (max_w + 6).clamp(18, 40);
        let mut dropdown_x = menu.x_pos;
        if dropdown_x + w > area.x + area.width {
            dropdown_x = (area.x + area.width).saturating_sub(w);
        }
        let h = items.len() as u16 + 2;
        Rect::new(
            dropdown_x,
            area.y + 1,
            w,
            h.min(area.height.saturating_sub(2)),
        )
    }

    fn submenu_rect(
        &self,
        area: Rect,
        parent_rect: Rect,
        item_idx: usize,
        items: &[&MenuItem],
    ) -> Rect {
        let mut max_w = 0u16;
        for item in items {
            if item.is_separator() {
                continue;
            }
            let label_w = item.label.len() as u16;
            let shortcut_w = item
                .shortcut
                .as_ref()
                .map(|s| s.len() as u16 + 2)
                .unwrap_or(0);
            max_w = max_w.max(label_w + shortcut_w);
        }
        let w = (max_w + 6).clamp(18, 36);
        let sub_x = parent_rect.x + parent_rect.width;
        let sub_x = sub_x.min(area.x + area.width - w);
        let item_y = parent_rect.y + 1 + item_idx as u16;
        let h = items.len() as u16 + 2;
        Rect::new(sub_x, item_y, w, h.min(area.height.saturating_sub(2)))
    }

    fn render_dropdown(
        &self,
        buf: &mut Buffer,
        rect: Rect,
        items: &[&MenuItem],
        selected: Option<usize>,
        hovered: Option<usize>,
        _is_sub: bool,
    ) {
        render_dropdown_box(buf, rect, Style::default().fg(Color::White));

        for (i, item) in items.iter().enumerate() {
            let y = rect.y + 1 + i as u16;
            if y >= rect.y + rect.height - 1 {
                break;
            }

            if item.is_separator() {
                let sep_x = rect.x + 2;
                let sep_end = rect.x + rect.width - 2;
                for sx in sep_x..sep_end {
                    if let Some(cell) = buf.cell_mut((sx, y)) {
                        cell.set_char('─');
                        cell.set_fg(Color::Indexed(245));
                        cell.set_bg(Color::Black);
                    }
                }
                continue;
            }

            let is_hovered = hovered == Some(i);
            let is_sel = selected == Some(i);
            let highlighted = is_hovered || is_sel;

            if highlighted {
                highlight_dropdown_row(buf, rect, y);
            }

            let item_style = dropdown_item_style(highlighted);

            let inner_w = rect.width.saturating_sub(2) as usize;
            let label = &item.label;
            let label_trimmed: String = label.chars().take(inner_w.saturating_sub(3)).collect();

            buf.set_string(rect.x + 2, y, &label_trimmed, item_style);

            let label_end = rect.x + 2 + label_trimmed.len() as u16;

            if item.has_submenu() {
                let is_open = self.open_submenu.as_ref().is_some_and(|s| s.0 == i);
                let arrow = if is_open { "▾" } else { "▸" };
                buf.set_string(rect.x + rect.width - 3, y, arrow, item_style);
            } else if let Some(ref shortcut) = item.shortcut {
                let shortcut_x = rect.x + rect.width - 2 - shortcut.len() as u16;
                if shortcut_x > label_end + 1 {
                    buf.set_string(shortcut_x, y, shortcut, item_style);
                }
            }
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> Option<CommandAction> {
        if let Some(menu_idx) = self.open_menu {
            if menu_idx >= self.menus.len() {
                self.close_all();
                return None;
            }
            let items = &self.menus[menu_idx].items;
            let visible: Vec<(usize, &MenuItem)> = items
                .iter()
                .enumerate()
                .filter(|(_, it)| !it.is_separator())
                .collect();

            if let Some(ref mut sub) = self.open_submenu {
                let sub_items = &items[sub.0].submenu;
                let sub_visible: Vec<(usize, &MenuItem)> = sub_items
                    .iter()
                    .enumerate()
                    .filter(|(_, it)| !it.is_separator())
                    .collect();

                match key.code {
                    KeyCode::Up => {
                        if let Some(sel) = sub.1.selected {
                            let pos = sub_visible.iter().position(|(i, _)| *i == sel);
                            if let Some(p) = pos {
                                if p > 0 {
                                    sub.1.selected = Some(sub_visible[p - 1].0);
                                }
                            }
                        } else {
                            sub.1.selected = sub_visible.first().map(|(i, _)| *i);
                        }
                        return None;
                    }
                    KeyCode::Down => {
                        if let Some(sel) = sub.1.selected {
                            let pos = sub_visible.iter().position(|(i, _)| *i == sel);
                            if let Some(p) = pos {
                                if p + 1 < sub_visible.len() {
                                    sub.1.selected = Some(sub_visible[p + 1].0);
                                }
                            }
                        } else {
                            sub.1.selected = sub_visible.first().map(|(i, _)| *i);
                        }
                        return None;
                    }
                    KeyCode::Left | KeyCode::Esc => {
                        let parent_idx = sub.0;
                        self.open_submenu = None;
                        self.selected = Some(parent_idx);
                        return None;
                    }
                    KeyCode::Enter | KeyCode::Right => {
                        if let Some(sel) = sub.1.selected {
                            if sel < sub_items.len() {
                                let item = &sub_items[sel];
                                if let Some(ref action) = item.action {
                                    let a = action.clone();
                                    self.close_all();
                                    return Some(a);
                                }
                            }
                        }
                        return None;
                    }
                    _ => {}
                }
                return None;
            }

            match key.code {
                KeyCode::Up => {
                    if let Some(sel) = self.selected {
                        let pos = visible.iter().position(|(i, _)| *i == sel);
                        if let Some(p) = pos {
                            if p > 0 {
                                self.selected = Some(visible[p - 1].0);
                            }
                        }
                    } else {
                        self.selected = visible.first().map(|(i, _)| *i);
                    }
                    None
                }
                KeyCode::Down => {
                    if let Some(sel) = self.selected {
                        let pos = visible.iter().position(|(i, _)| *i == sel);
                        if let Some(p) = pos {
                            if p + 1 < visible.len() {
                                self.selected = Some(visible[p + 1].0);
                            }
                        }
                    } else {
                        self.selected = visible.first().map(|(i, _)| *i);
                    }
                    None
                }
                KeyCode::Right => {
                    if let Some(sel) = self.selected {
                        if sel < items.len() && items[sel].has_submenu() {
                            self.open_submenu = Some(Box::new((
                                sel,
                                SubmenuState {
                                    selected: None,
                                    hovered: None,
                                },
                            )));
                        }
                    }
                    None
                }
                KeyCode::Left => {
                    self.close_all();
                    None
                }
                KeyCode::Esc => {
                    self.close_all();
                    None
                }
                KeyCode::Enter => {
                    if let Some(sel) = self.selected {
                        if sel < items.len() {
                            let item = &items[sel];
                            if item.has_submenu() {
                                self.open_submenu = Some(Box::new((
                                    sel,
                                    SubmenuState {
                                        selected: None,
                                        hovered: None,
                                    },
                                )));
                                return None;
                            }
                            if let Some(ref action) = item.action {
                                let a = action.clone();
                                self.close_all();
                                return Some(a);
                            }
                        }
                    }
                    None
                }
                _ => None,
            }
        } else {
            match key.code {
                KeyCode::Char(c) if key.modifiers.contains(KeyModifiers::ALT) => {
                    let lower = c.to_ascii_lowercase();
                    for (i, menu) in self.menus.iter().enumerate() {
                        if menu.hotkey == lower {
                            if self.open_menu == Some(i) {
                                self.close_all();
                            } else {
                                self.open_menu = Some(i);
                                self.selected = None;
                                self.hovered = None;
                            }
                            return None;
                        }
                    }
                    None
                }
                _ => None,
            }
        }
    }

    pub fn handle_mouse(&mut self, mouse: MouseEvent, full_area: Rect) -> Option<CommandAction> {
        let y = mouse.row;
        let x = mouse.column;

        // Top bar interactions
        if y == full_area.y {
            self.hovered_label = None;
            for (i, menu) in self.menus.iter().enumerate() {
                if x >= menu.x_pos && x < menu.label_end() {
                    self.hovered_label = Some(i);
                    if matches!(mouse.kind, MouseEventKind::Down(MouseButton::Left)) {
                        if self.open_menu == Some(i) {
                            self.close_all();
                        } else {
                            self.open_menu = Some(i);
                            self.selected = None;
                            self.hovered = None;
                        }
                        return None;
                    }
                }
            }
            if matches!(mouse.kind, MouseEventKind::Down(MouseButton::Left)) {
                self.close_all();
            }
            return None;
        }

        // Dropdown area interactions
        if let Some(menu_idx) = self.open_menu {
            if menu_idx >= self.menus.len() {
                return None;
            }
            let menu = &self.menus[menu_idx];
            let items: Vec<&MenuItem> = menu.items.iter().collect();
            let dropdown_rect = self.dropdown_rect(full_area, menu_idx, &items);

            let in_dropdown_interior = x > dropdown_rect.x
                && x < dropdown_rect.x + dropdown_rect.width - 1
                && y > dropdown_rect.y
                && y < dropdown_rect.y + dropdown_rect.height - 1;

            if in_dropdown_interior {
                let row = (y - dropdown_rect.y - 1) as usize;
                if row < items.len() {
                    let item = items[row];
                    if item.is_separator() {
                        self.hovered = None;
                        return None;
                    }
                    self.hovered = Some(row);

                    if matches!(mouse.kind, MouseEventKind::Moved) {
                        if item.has_submenu() {
                            self.open_submenu = Some(Box::new((
                                row,
                                SubmenuState {
                                    selected: None,
                                    hovered: None,
                                },
                            )));
                        } else {
                            self.open_submenu = None;
                        }
                        return None;
                    }

                    if matches!(mouse.kind, MouseEventKind::Down(MouseButton::Left)) {
                        if item.has_submenu() {
                            self.open_submenu = Some(Box::new((
                                row,
                                SubmenuState {
                                    selected: None,
                                    hovered: None,
                                },
                            )));
                            return None;
                        }
                        if let Some(ref action) = item.action {
                            let a = action.clone();
                            self.close_all();
                            return Some(a);
                        }
                    }
                    return None;
                }
            }

            // Check submenu
            let mut in_submenu_interior = false;
            let submenu_stuff = self.open_submenu.as_ref().map(|sub| {
                let item = &menu.items[sub.0];
                (sub.0, item.has_submenu())
            });
            if let Some((sub_idx, has_sub)) = submenu_stuff {
                if has_sub {
                    let item = &menu.items[sub_idx];
                    let sub_items: Vec<&MenuItem> = item.submenu.iter().collect();
                    let sub_rect = self.submenu_rect(full_area, dropdown_rect, sub_idx, &sub_items);

                    if x > sub_rect.x
                        && x < sub_rect.x + sub_rect.width - 1
                        && y > sub_rect.y
                        && y < sub_rect.y + sub_rect.height - 1
                    {
                        in_submenu_interior = true;
                        let row = (y - sub_rect.y - 1) as usize;
                        if row < sub_items.len() {
                            let sub_item = sub_items[row];
                            if sub_item.is_separator() {
                                if let Some(ref mut sub) = self.open_submenu {
                                    sub.1.hovered = None;
                                }
                                return None;
                            }
                            if let Some(ref mut sub) = self.open_submenu {
                                sub.1.hovered = Some(row);
                            }

                            if matches!(mouse.kind, MouseEventKind::Down(MouseButton::Left)) {
                                if let Some(ref action) = sub_item.action {
                                    let a = action.clone();
                                    self.close_all();
                                    return Some(a);
                                }
                            }
                            return None;
                        }
                    }
                }
            }

            if !in_dropdown_interior {
                self.hovered = None;
            }
            if !in_submenu_interior {
                if let Some(ref mut sub) = self.open_submenu {
                    sub.1.hovered = None;
                }
            }

            if matches!(mouse.kind, MouseEventKind::Down(MouseButton::Left)) {
                self.close_all();
            }
            return None;
        }

        None
    }
}
