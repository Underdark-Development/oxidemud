use mud_core::templates::TemplateRegistry;
use ratatui::{
    buffer::Buffer,
    crossterm::event::{KeyCode, KeyEvent, MouseEvent, MouseEventKind},
    layout::{Constraint, Rect},
    style::{Color, Style},
    widgets::Widget,
};

use super::Screen;
use crate::components::{ScrollState, Table};

pub struct ValidationPanelScreen {
    registry: TemplateRegistry,
    table: Table,
    scrollbar: ScrollState,
    error_count: usize,
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

    fn handle_key(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Up => self.table.select_prev(),
            KeyCode::Down => self.table.select_next(),
            _ => {}
        }
        true
    }

    fn handle_mouse(&mut self, mouse: MouseEvent, _area: Rect) {
        match mouse.kind {
            MouseEventKind::ScrollUp => self.table.select_prev(),
            MouseEventKind::ScrollDown => self.table.select_next(),
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
            format!(" {} error(s) found ", self.error_count)
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
        self.table.render(table_area, buf);

        let scrollbar_area = Rect::new(
            area.x + area.width - 1,
            area.y + 1,
            1,
            area.height.saturating_sub(2),
        );
        self.scrollbar.render(scrollbar_area, buf);
    }
}
