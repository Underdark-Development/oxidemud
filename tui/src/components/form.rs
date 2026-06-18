use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Widget,
};

pub enum FieldType {
    Text,
    Number,
    Bool,
    Choice(Vec<String>),
    DiceNotation,
}

pub struct FormField {
    pub label: String,
    pub value: String,
    pub field_type: FieldType,
    pub error: Option<String>,
    pub cursor: usize,
    pub read_only: bool,
}

impl FormField {
    pub fn new(label: String, value: String, field_type: FieldType) -> Self {
        let cursor = value.len();
        FormField {
            label,
            value,
            field_type,
            error: None,
            cursor,
            read_only: false,
        }
    }
}

pub struct Form {
    pub fields: Vec<FormField>,
    pub focus: usize,
    pub label_width: u16,
}

impl Form {
    pub fn new(fields: Vec<FormField>) -> Self {
        let label_width = fields
            .iter()
            .map(|f| f.label.len() as u16)
            .max()
            .unwrap_or(15)
            .min(30);
        Form {
            fields,
            focus: 0,
            label_width,
        }
    }

    pub fn focus_next(&mut self) {
        if !self.fields.is_empty() {
            self.focus = (self.focus + 1) % self.fields.len();
        }
    }

    pub fn focus_prev(&mut self) {
        if self.fields.is_empty() {
            return;
        }
        self.focus = if self.focus == 0 {
            self.fields.len() - 1
        } else {
            self.focus - 1
        };
    }

    pub fn insert_char(&mut self, c: char) {
        let field = match self.fields.get_mut(self.focus) {
            Some(f) => f,
            None => return,
        };
        if field.read_only {
            return;
        }
        if let FieldType::Bool = field.field_type {
            self.toggle_bool();
            return;
        }
        field.value.insert(field.cursor, c);
        field.cursor += 1;
    }

    pub fn delete_char(&mut self) {
        let field = match self.fields.get_mut(self.focus) {
            Some(f) => f,
            None => return,
        };
        if field.read_only || field.cursor == 0 {
            return;
        }
        field.cursor -= 1;
        field.value.remove(field.cursor);
    }

    pub fn move_cursor_left(&mut self) {
        if let Some(field) = self.fields.get_mut(self.focus) {
            if field.cursor > 0 {
                field.cursor -= 1;
            }
        }
    }

    pub fn move_cursor_right(&mut self) {
        if let Some(field) = self.fields.get_mut(self.focus) {
            if field.cursor < field.value.len() {
                field.cursor += 1;
            }
        }
    }

    pub fn toggle_bool(&mut self) {
        let field = match self.fields.get_mut(self.focus) {
            Some(f) => f,
            None => return,
        };
        if field.read_only {
            return;
        }
        field.value = if field.value == "true" {
            "false".to_string()
        } else {
            "true".to_string()
        };
    }

    pub fn cycle_choice(&mut self, forward: bool) {
        let field = match self.fields.get_mut(self.focus) {
            Some(f) => f,
            None => return,
        };
        if field.read_only {
            return;
        }
        if let FieldType::Choice(options) = &field.field_type {
            if options.is_empty() {
                return;
            }
            let current = options.iter().position(|o| o == &field.value);
            let next = match (current, forward) {
                (Some(i), true) => (i + 1) % options.len(),
                (Some(i), false) => {
                    if i == 0 {
                        options.len() - 1
                    } else {
                        i - 1
                    }
                }
                (None, _) => 0,
            };
            field.value = options[next].clone();
            field.cursor = field.value.len();
        }
    }

    pub fn current_field(&self) -> Option<&FormField> {
        self.fields.get(self.focus)
    }

    pub fn set_error(&mut self, field: usize, error: String) {
        if let Some(f) = self.fields.get_mut(field) {
            f.error = Some(error);
        }
    }

    pub fn clear_errors(&mut self) {
        for field in &mut self.fields {
            field.error = None;
        }
    }
}

impl Widget for &Form {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.height < 1 || self.fields.is_empty() {
            return;
        }

        let value_start = area.x + self.label_width + 2;

        for (i, field) in self.fields.iter().enumerate() {
            let y = area.y + i as u16;
            if y >= area.y + area.height {
                break;
            }

            let is_focused = i == self.focus;
            let has_error = field.error.is_some();

            let label_style = if is_focused {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };

            let focus_marker = if is_focused { "▸ " } else { "  " };

            let label_line = Line::from(Span::styled(
                format!(
                    "{}{:width$}",
                    focus_marker,
                    field.label,
                    width = self.label_width as usize
                ),
                label_style,
            ));
            buf.set_line(area.x, y, &label_line, self.label_width + 2);

            let value_style = if has_error {
                Style::default().fg(Color::LightRed)
            } else if field.read_only {
                Style::default().fg(Color::Indexed(245))
            } else {
                Style::default().fg(Color::White)
            };

            let display_value = match &field.field_type {
                FieldType::Bool => {
                    format!("[{}]", if field.value == "true" { "x" } else { " " })
                }
                FieldType::Choice(options) => {
                    format!(
                        "{} ({}/{})",
                        field.value,
                        options.iter().position(|o| o == &field.value).unwrap_or(0) + 1,
                        options.len()
                    )
                }
                _ => field.value.clone(),
            };

            let mut value_spans = vec![Span::styled(display_value, value_style)];

            if is_focused && !field.read_only {
                if let FieldType::Text | FieldType::Number | FieldType::DiceNotation =
                    field.field_type
                {
                    value_spans.push(Span::styled(
                        " ",
                        Style::default().add_modifier(Modifier::REVERSED),
                    ));
                }
            }

            if has_error {
                if let Some(err) = &field.error {
                    value_spans.push(Span::styled(
                        format!("  ⚠ {}", err),
                        Style::default().fg(Color::LightRed),
                    ));
                }
            }

            let max_width = area.width.saturating_sub(value_start - area.x);
            buf.set_line(value_start, y, &Line::from(value_spans), max_width);
        }
    }
}
