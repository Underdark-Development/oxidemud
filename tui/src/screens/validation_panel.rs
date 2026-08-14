use oxide_core::templates::TemplateRegistry;
use ratatui::{
    buffer::Buffer,
    crossterm::event::{KeyCode, KeyEvent, MouseButton, MouseEvent, MouseEventKind},
    layout::{Constraint, Rect},
    style::{Color, Style},
    widgets::Widget,
};

use super::{Screen, ScreenAction};
use crate::components::{ScrollState, Table};

pub struct ValidationPanelScreen {
    registry: TemplateRegistry,
    table: Table,
    scrollbar: ScrollState,
    error_count: usize,
    pending_action: ScreenAction,
}

impl ValidationPanelScreen {
    pub fn new(registry: TemplateRegistry) -> Self {
        let mut screen = ValidationPanelScreen {
            registry,
            table: Table::new(vec![
                "Type".into(),
                "ID".into(),
                "Field".into(),
                "Message".into(),
            ]),
            scrollbar: ScrollState::new(),
            error_count: 0,
            pending_action: ScreenAction::None,
        };
        screen.run_validation();
        screen
    }

    fn run_validation(&mut self) {
        let errors = self.registry.validate();
        self.error_count = errors.len();

        let mut table = Table::new(vec![
            "Type".into(),
            "ID".into(),
            "Field".into(),
            "Message".into(),
        ]);
        table.column_widths = vec![
            Constraint::Length(8),
            Constraint::Length(20),
            Constraint::Length(24),
            Constraint::Fill(1),
        ];

        for err in &errors {
            table.add_row(vec![
                err.template_type.to_string(),
                err.template_id.clone(),
                err.field.clone(),
                err.message.clone(),
            ]);
        }

        if errors.is_empty() {
            table.add_row(vec![
                "✓".into(),
                "All good".into(),
                "no errors".into(),
                "template registry validated successfully".into(),
            ]);
        }

        self.table = table;
    }

    fn inspect_selected_error(&mut self) {
        if self.error_count == 0 {
            return;
        }
        if let Some(idx) = self.table.selected {
            if idx < self.table.rows.len() {
                let row = &self.table.rows[idx];
                let t_type = row[0].as_str();
                let t_id = &row[1];
                let category = match t_type {
                    "room" | "area" => "rooms",
                    "mob" => "mobs",
                    "item" => "items",
                    "quest" => "quests",
                    "recipe" => "recipes",
                    "faction" => "factions",
                    "race" => "races",
                    "class" => "classes",
                    other => other,
                };
                self.pending_action = ScreenAction::Inspect(category.to_string(), t_id.clone());
            }
        }
    }
}

impl Screen for ValidationPanelScreen {
    fn name(&self) -> &str {
        "Validation Panel"
    }

    fn reload(&mut self) {
        self.run_validation();
    }

    fn update_registry(&mut self, registry: &TemplateRegistry) {
        self.registry = registry.clone();
        self.run_validation();
    }

    fn take_action(&mut self) -> ScreenAction {
        std::mem::replace(&mut self.pending_action, ScreenAction::None)
    }

    fn handle_key(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Up => self.table.select_prev(),
            KeyCode::Down => self.table.select_next(),
            KeyCode::Enter => self.inspect_selected_error(),
            _ => {}
        }
        true
    }

    fn handle_mouse(&mut self, mouse: MouseEvent, area: Rect) {
        match mouse.kind {
            MouseEventKind::ScrollUp => self.table.select_prev(),
            MouseEventKind::ScrollDown => self.table.select_next(),
            MouseEventKind::Down(MouseButton::Left) => {
                let table_top = area.y + 2; // top status line + table header row
                let content_lines = area.height.saturating_sub(2) as usize;
                if mouse.row >= table_top && mouse.row < table_top + content_lines as u16 {
                    let clicked_row = (mouse.row - table_top) as usize + self.table.scroll.offset;
                    if clicked_row < self.table.rows.len() {
                        self.table.selected = Some(clicked_row);
                        self.inspect_selected_error();
                    }
                }
            }
            _ => {}
        }
    }

    fn render(&mut self, area: Rect, buf: &mut Buffer, _mouse_pos: Option<(u16, u16)>) {
        if area.width < 4 || area.height < 2 {
            return;
        }

        let color = if self.error_count == 0 {
            Color::Green
        } else {
            Color::LightRed
        };
        let status = if self.error_count == 0 {
            " validation passed — no errors ".to_string()
        } else {
            format!(
                " {} error(s) found  │  💡 Click any validation error (or press Enter) to open it in the entity editor ",
                self.error_count
            )
        };
        buf.set_string(area.x, area.y, &status, Style::default().fg(color));

        let content_lines = area.height.saturating_sub(2) as usize;
        self.table.update_scroll(content_lines);
        self.scrollbar = ScrollState {
            offset: self.table.scroll.offset,
            visible_lines: self.table.scroll.visible_lines,
            total_lines: self.table.scroll.total_lines,
        };

        let table_area = Rect::new(
            area.x,
            area.y + 1,
            area.width.saturating_sub(1),
            area.height.saturating_sub(2),
        );
        self.table.render_table(table_area, buf);

        let scrollbar_area = Rect::new(
            area.x + area.width - 1,
            area.y + 1,
            1,
            area.height.saturating_sub(2),
        );
        self.scrollbar.render(scrollbar_area, buf);
    }
}
