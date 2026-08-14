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
    sort_column: usize,
    sort_ascending: bool,
}

fn classify_error_kind(msg: &str, field: &str) -> &'static str {
    let msg_lower = msg.to_lowercase();
    let field_lower = field.to_lowercase();
    if msg_lower.contains("does not exist")
        || msg_lower.contains("referential")
        || field_lower.contains("exit")
        || field_lower.contains("destination")
    {
        "Reference"
    } else if msg_lower.contains("enum")
        || msg_lower.contains("invalid option")
        || field_lower.contains("hands")
        || field_lower.contains("slot")
        || field_lower.contains("quality")
        || field_lower.contains("item_type")
    {
        "Enum"
    } else if msg_lower.contains("schema")
        || msg_lower.contains("type")
        || msg_lower.contains("expected")
    {
        "Schema"
    } else if msg_lower.contains("syntax") || msg_lower.contains("parse") {
        "Syntax"
    } else {
        "Validation"
    }
}

impl ValidationPanelScreen {
    pub fn new(registry: TemplateRegistry) -> Self {
        let mut screen = ValidationPanelScreen {
            registry,
            table: Table::new(vec![
                "Category".into(),
                "Entity ID".into(),
                "Error Kind".into(),
                "Field".into(),
                "Message".into(),
            ]),
            scrollbar: ScrollState::new(),
            error_count: 0,
            pending_action: ScreenAction::None,
            sort_column: 0,
            sort_ascending: true,
        };
        screen.run_validation();
        screen
    }

    fn run_validation(&mut self) {
        let errors = self.registry.validate();
        self.error_count = errors.len();

        let mut headers = vec![
            "Category".to_string(),
            "Entity ID".to_string(),
            "Error Kind".to_string(),
            "Field".to_string(),
            "Message".to_string(),
        ];
        if self.sort_column < headers.len() {
            headers[self.sort_column]
                .push_str(if self.sort_ascending { " ▲" } else { " ▼" });
        }

        let mut table = Table::new(headers);
        table.column_widths = vec![
            Constraint::Length(12),
            Constraint::Length(22),
            Constraint::Length(16),
            Constraint::Length(24),
            Constraint::Fill(1),
        ];

        let mut row_data: Vec<[String; 5]> = Vec::new();

        for err in &errors {
            let category = match err.template_type {
                "room" | "area" => "rooms".to_string(),
                "mob" => "mobs".to_string(),
                "item" => "items".to_string(),
                "quest" => "quests".to_string(),
                "recipe" => "recipes".to_string(),
                "faction" => "factions".to_string(),
                "race" => "races".to_string(),
                "class" => "classes".to_string(),
                other => format!("{}s", other),
            };
            let kind = classify_error_kind(&err.message, &err.field).to_string();

            row_data.push([
                category,
                err.template_id.clone(),
                kind,
                err.field.clone(),
                err.message.clone(),
            ]);
        }

        let sort_col = self.sort_column.min(4);
        let sort_asc = self.sort_ascending;
        row_data.sort_by(|a, b| {
            let cmp = a[sort_col].to_lowercase().cmp(&b[sort_col].to_lowercase());
            if sort_asc {
                cmp
            } else {
                cmp.reverse()
            }
        });

        for row in row_data {
            table.add_row(row.to_vec());
        }

        if errors.is_empty() {
            table.add_row(vec![
                "✓".into(),
                "All good".into(),
                "Clean".into(),
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
                let category = &row[0];
                let mut target_id = row[1].clone();
                let field = &row[3];

                // Handle nested room field prefix (e.g. rooms.north_gate.exits)
                if field.starts_with("rooms.") {
                    let parts: Vec<&str> = field.split('.').collect();
                    if parts.len() >= 2 {
                        target_id = parts[1].to_string();
                    }
                }

                self.pending_action =
                    ScreenAction::Inspect(category.clone(), target_id);
            }
        }
    }

    fn handle_header_click(&mut self, col: usize) {
        if col == self.sort_column {
            self.sort_ascending = !self.sort_ascending;
        } else {
            self.sort_column = col.min(4);
            self.sort_ascending = true;
        }
        self.run_validation();
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
            KeyCode::Home => self.table.select_first(),
            KeyCode::End => self.table.select_last(),
            KeyCode::PageUp => self.table.page_up(),
            KeyCode::PageDown => self.table.page_down(),
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
                let header_y = area.y + 1;
                let table_top = area.y + 2;
                let content_lines = area.height.saturating_sub(2) as usize;

                if mouse.row == header_y {
                    let rel_x = mouse.column.saturating_sub(area.x + 2);
                    // Column width bounds:
                    // Category: 0..12, Entity ID: 12..34, Error Kind: 34..50, Field: 50..74, Message: 74+
                    let col = if rel_x < 12 {
                        0
                    } else if rel_x < 34 {
                        1
                    } else if rel_x < 50 {
                        2
                    } else if rel_x < 74 {
                        3
                    } else {
                        4
                    };
                    self.handle_header_click(col);
                } else if mouse.row >= table_top && mouse.row < table_top + content_lines as u16 {
                    let clicked_row =
                        (mouse.row - table_top) as usize + self.table.scroll.offset;
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
                " {} error(s) found  │  💡 Click any header to sort, click error to edit ",
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
