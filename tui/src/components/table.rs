use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Widget,
};

use crate::components::ScrollState;

pub struct Table {
    pub headers: Vec<String>,
    pub rows: Vec<Vec<String>>,
    pub column_widths: Vec<Constraint>,
    pub scroll: ScrollState,
    pub selected: Option<usize>,
    pub hovered: Option<usize>,
    pub highlight_symbol: String,
    pub muted: bool,
}

impl Table {
    pub fn new(headers: Vec<String>) -> Self {
        let count = headers.len();
        Table {
            headers,
            rows: Vec::new(),
            column_widths: vec![Constraint::Length(20); count],
            scroll: ScrollState::new(),
            selected: None,
            hovered: None,
            highlight_symbol: "▸ ".to_string(),
            muted: false,
        }
    }

    pub fn add_row(&mut self, row: Vec<String>) {
        self.rows.push(row);
        self.scroll.total_lines = self.rows.len();
    }

    pub fn scroll_up(&mut self) {
        if self.scroll.offset > 0 {
            self.scroll.offset -= 1;
        }
    }

    pub fn scroll_down(&mut self) {
        let max = self
            .scroll
            .total_lines
            .saturating_sub(self.scroll.visible_lines);
        if self.scroll.offset < max {
            self.scroll.offset += 1;
        }
    }

    pub fn select_next(&mut self) {
        if self.rows.is_empty() {
            return;
        }
        let idx = self.selected.unwrap_or(0);
        self.selected = Some((idx + 1) % self.rows.len());
        self.ensure_selected_visible();
    }

    pub fn select_prev(&mut self) {
        if self.rows.is_empty() {
            return;
        }
        let idx = self.selected.unwrap_or(0);
        self.selected = Some(if idx == 0 {
            self.rows.len() - 1
        } else {
            idx - 1
        });
        self.ensure_selected_visible();
    }

    pub fn ensure_selected_visible(&mut self) {
        let idx = match self.selected {
            Some(i) => i,
            None => return,
        };
        if idx < self.scroll.offset {
            self.scroll.offset = idx;
        } else if idx >= self.scroll.offset + self.scroll.visible_lines {
            self.scroll.offset = idx
                .saturating_add(1)
                .saturating_sub(self.scroll.visible_lines);
        }
    }

    pub fn set_cell(&mut self, row: usize, col: usize, value: String) {
        if let Some(r) = self.rows.get_mut(row) {
            if let Some(c) = r.get_mut(col) {
                *c = value;
            }
        }
    }

    pub fn update_scroll(&mut self, area_height: usize) {
        self.scroll.total_lines = self.rows.len();
        self.scroll.visible_lines = area_height.saturating_sub(1);
        self.ensure_selected_visible();
    }

    pub fn col_x(&self, col: usize) -> u16 {
        let mut x = 0;
        for (i, w) in self.column_widths.iter().enumerate() {
            if i >= col {
                break;
            }
            match w {
                Constraint::Length(len) => x += len + 2,
                Constraint::Min(len) => x += len + 2,
                Constraint::Max(len) => x += len + 2,
                Constraint::Percentage(p) => x += p / 10 + 2,
                Constraint::Ratio(num, den) => x += ((num * 20 / den) + 2) as u16,
                Constraint::Fill(_) => x += 20 + 2,
            }
        }
        x
    }
}

impl Widget for &Table {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.height < 2 || self.headers.is_empty() {
            return;
        }

        let offset = self.scroll.offset;
        let visible = self.scroll.visible_lines.max(1);

        let header_style = Style::default()
            .fg(Color::Black)
            .bg(Color::White)
            .add_modifier(Modifier::BOLD);

        let mut header_spans = Vec::new();
        for (i, header) in self.headers.iter().enumerate() {
            let width = match self.column_widths.get(i) {
                Some(Constraint::Length(len)) => *len as usize,
                _ => 20,
            };
            header_spans.push(Span::styled(
                format!(" {:width$} ", header, width = width),
                header_style,
            ));
        }
        buf.set_line(area.x, area.y, &Line::from(header_spans), area.width);

        for i in 0..visible {
            let idx = offset + i;
            if idx >= self.rows.len() {
                break;
            }
            let y = area.y + 1 + i as u16;
            if y >= area.y + area.height {
                break;
            }

            let is_selected = Some(idx) == self.selected;
            let is_hovered = self.hovered == Some(idx);
            if is_selected || is_hovered {
                for x in area.x..area.x + area.width {
                    if let Some(cell) = buf.cell_mut((x, y)) {
                        cell.set_bg(Color::Indexed(240));
                    }
                }
            }
            let text_fg = if self.muted {
                Color::Indexed(245)
            } else {
                Color::White
            };
            let row_style = if is_selected {
                Style::default()
                    .fg(text_fg)
                    .bg(Color::Indexed(240))
                    .add_modifier(Modifier::BOLD)
            } else if is_hovered {
                Style::default().fg(text_fg).bg(Color::Indexed(240))
            } else if i % 2 == 0 {
                Style::default().fg(text_fg)
            } else {
                Style::default().fg(if self.muted { text_fg } else { Color::Gray })
            };

            let mut row_spans = Vec::new();

            if is_selected {
                row_spans.push(Span::styled(
                    self.highlight_symbol.clone(),
                    Style::default().fg(Color::Cyan),
                ));
            } else {
                row_spans.push(Span::raw("  "));
            }

            for (col, value) in self.rows[idx].iter().enumerate() {
                let width = match self.column_widths.get(col) {
                    Some(Constraint::Length(len)) => *len as usize,
                    _ => 20,
                };
                row_spans.push(Span::styled(
                    format!(" {:width$} ", value, width = width),
                    row_style,
                ));
            }
            buf.set_line(area.x, y, &Line::from(row_spans), area.width);
        }
    }
}
