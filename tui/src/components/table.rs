use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::Widget,
};

use crate::components::ScrollState;

#[derive(Debug, Clone)]
pub struct RowErrorInfo {
    pub message: String,
    pub is_toml: bool,
}

pub struct Table {
    pub headers: Vec<String>,
    pub rows: Vec<Vec<String>>,
    pub column_widths: Vec<Constraint>,
    pub scroll: ScrollState,
    pub selected: Option<usize>,
    pub hovered: Option<usize>,
    pub highlight_symbol: String,
    pub muted: bool,
    pub row_errors: std::collections::HashMap<usize, RowErrorInfo>,
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
            row_errors: std::collections::HashMap::new(),
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
        if idx == 0 {
            self.selected = Some(self.rows.len() - 1);
        } else {
            self.selected = Some(idx - 1);
        }
        self.ensure_selected_visible();
    }

    pub fn select_first(&mut self) {
        if !self.rows.is_empty() {
            self.selected = Some(0);
            self.ensure_selected_visible();
        }
    }

    pub fn select_last(&mut self) {
        if !self.rows.is_empty() {
            self.selected = Some(self.rows.len() - 1);
            self.ensure_selected_visible();
        }
    }

    pub fn page_up(&mut self) {
        if self.rows.is_empty() {
            return;
        }
        let step = self.scroll.visible_lines.max(1);
        let current = self.selected.unwrap_or(0);
        self.selected = Some(current.saturating_sub(step));
        self.ensure_selected_visible();
    }

    pub fn page_down(&mut self) {
        if self.rows.is_empty() {
            return;
        }
        let step = self.scroll.visible_lines.max(1);
        let current = self.selected.unwrap_or(0);
        let last = self.rows.len() - 1;
        self.selected = Some((current + step).min(last));
        self.ensure_selected_visible();
    }

    pub fn ensure_selected_visible(&mut self) {
        if let Some(selected) = self.selected {
            if self.scroll.visible_lines == 0 {
                return;
            }
            if selected < self.scroll.offset {
                self.scroll.offset = selected;
            } else if selected >= self.scroll.offset + self.scroll.visible_lines {
                self.scroll.offset = selected.saturating_sub(self.scroll.visible_lines - 1);
            }
        }
    }

    pub fn set_cell(&mut self, row: usize, col: usize, value: String) {
        if let Some(r) = self.rows.get_mut(row) {
            if let Some(c) = r.get_mut(col) {
                *c = value;
            }
        }
    }

    pub fn update_scroll(&mut self, visible_lines: usize) {
        self.scroll.visible_lines = visible_lines;
        self.scroll.total_lines = self.rows.len();
    }

    pub fn col_x(&self, col: usize, area: Rect) -> u16 {
        let layout = Layout::horizontal(&self.column_widths).split(area);
        if col < layout.len() {
            layout[col].x - area.x
        } else {
            0
        }
    }

    pub fn render_table(&mut self, area: Rect, buf: &mut Buffer) {
        if area.width < 4 || area.height < 2 {
            return;
        }

        let visible = area.height.saturating_sub(1) as usize;
        self.update_scroll(visible);

        let offset = self.scroll.offset;
        let col_areas = Layout::horizontal(&self.column_widths).split(Rect::new(
            area.x + 2,
            area.y,
            area.width.saturating_sub(2),
            area.height,
        ));

        // Render header background
        for x in area.x..area.x + area.width {
            if let Some(cell) = buf.cell_mut((x, area.y)) {
                cell.set_bg(Color::Indexed(238));
            }
        }

        // Render header strings
        let header_style = Style::default()
            .fg(Color::Cyan)
            .bg(Color::Indexed(238))
            .add_modifier(Modifier::BOLD);
        for (i, header) in self.headers.iter().enumerate() {
            if let Some(col_area) = col_areas.get(i) {
                let text = format!(" {}", header);
                buf.set_stringn(
                    col_area.x,
                    area.y,
                    &text,
                    col_area.width as usize,
                    header_style,
                );
            }
        }

        let mut active_tooltip: Option<(u16, u16, String, bool)> = None;

        // Render rows
        for i in 0..visible {
            let idx = offset + i;
            if idx >= self.rows.len() {
                break;
            }
            let y = area.y + 1 + i as u16;
            if y >= area.y + area.height {
                break;
            }

            let err_info = self.row_errors.get(&idx).cloned();
            let has_error = err_info.is_some();
            let is_selected = Some(idx) == self.selected;
            let is_hovered = self.hovered == Some(idx);
            let bg_color = if has_error {
                Color::Indexed(52)
            } else if is_selected && is_hovered {
                Color::Indexed(242)
            } else if is_selected {
                Color::Indexed(239)
            } else if is_hovered {
                Color::Indexed(236)
            } else if i % 2 == 1 {
                Color::Indexed(235)
            } else {
                Color::Reset
            };

            for x in area.x..area.x + area.width {
                if let Some(cell) = buf.cell_mut((x, y)) {
                    cell.set_bg(bg_color);
                }
            }

            let text_fg = if let Some(ref err) = err_info {
                if err.is_toml {
                    Color::LightRed
                } else {
                    Color::Yellow
                }
            } else if self.muted {
                Color::Indexed(245)
            } else {
                Color::White
            };
            let row_style = if is_selected {
                Style::default()
                    .fg(text_fg)
                    .bg(bg_color)
                    .add_modifier(Modifier::BOLD)
            } else if is_hovered {
                Style::default().fg(text_fg).bg(Color::Indexed(238))
            } else {
                Style::default().fg(text_fg).bg(bg_color)
            };

            // Selection symbol / Error symbol / Hover symbol
            let (symbol, symbol_style) = if let Some(ref err) = err_info {
                if is_hovered || (self.hovered.is_none() && is_selected) {
                    active_tooltip = Some((area.x + 2, y, err.message.clone(), err.is_toml));
                }
                let color = if err.is_toml {
                    Color::LightRed
                } else {
                    Color::Yellow
                };
                (
                    "⚠ ",
                    Style::default().fg(color).add_modifier(Modifier::BOLD),
                )
            } else if is_selected {
                (
                    &self.highlight_symbol[..],
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                )
            } else if is_hovered {
                ("· ", Style::default().fg(Color::Indexed(244)))
            } else {
                ("  ", Style::default())
            };
            buf.set_stringn(area.x, y, symbol, 2, symbol_style);

            // Columns data
            for (col, value) in self.rows[idx].iter().enumerate() {
                if let Some(col_area) = col_areas.get(col) {
                    let text = format!(" {}", value);
                    let is_array_header = col == 0
                        && self.rows[idx]
                            .get(1)
                            .is_some_and(|v| v.starts_with("(array"));
                    let cell_style = if is_array_header {
                        row_style.fg(Color::Cyan).add_modifier(Modifier::BOLD)
                    } else {
                        row_style
                    };
                    buf.set_stringn(col_area.x, y, &text, col_area.width as usize, cell_style);

                    // Overlay colored button badges on column 1 ("Value")
                    if col == 1 {
                        if let Some(pos) = value.find("[ + Add Entry ]") {
                            let btn_x = col_area.x + 1 + pos as u16;
                            buf.set_string(
                                btn_x,
                                y,
                                "[ + Add Entry ]",
                                Style::default()
                                    .fg(Color::Cyan)
                                    .bg(bg_color)
                                    .add_modifier(Modifier::BOLD),
                            );
                        }
                        if let Some(pos) = value.find("[ 🗑 Clear ]") {
                            let btn_x = col_area.x + 1 + pos as u16;
                            buf.set_string(
                                btn_x,
                                y,
                                "[ 🗑 Clear ]",
                                Style::default().fg(Color::Yellow).bg(bg_color),
                            );
                        }
                        if let Some(pos) = value.find("[ ▲ ]") {
                            let btn_x = col_area.x + 1 + pos as u16;
                            buf.set_string(
                                btn_x,
                                y,
                                "[ ▲ ]",
                                Style::default()
                                    .fg(Color::Cyan)
                                    .bg(bg_color)
                                    .add_modifier(Modifier::BOLD),
                            );
                        }
                        if let Some(pos) = value.find("[ ▼ ]") {
                            let btn_x = col_area.x + 1 + pos as u16;
                            buf.set_string(
                                btn_x,
                                y,
                                "[ ▼ ]",
                                Style::default()
                                    .fg(Color::Cyan)
                                    .bg(bg_color)
                                    .add_modifier(Modifier::BOLD),
                            );
                        }
                        if let Some(pos) = value.rfind("[ ✕ ]") {
                            let btn_x = col_area.x + 1 + pos as u16;
                            buf.set_string(
                                btn_x,
                                y,
                                "[ ✕ ]",
                                Style::default()
                                    .fg(Color::LightRed)
                                    .bg(bg_color)
                                    .add_modifier(Modifier::BOLD),
                            );
                        }
                    }
                }
            }
        }

        // Render tooltip popup overlay if active
        if let Some((tx, ty, msg, is_toml)) = active_tooltip {
            crate::components::TooltipPopup::render(buf, area, tx, ty, &msg, is_toml);
        }
    }
}

impl Widget for &Table {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let mut t = Table {
            headers: self.headers.clone(),
            rows: self.rows.clone(),
            column_widths: self.column_widths.clone(),
            scroll: self.scroll.clone(),
            selected: self.selected,
            hovered: self.hovered,
            highlight_symbol: self.highlight_symbol.clone(),
            muted: self.muted,
            row_errors: self.row_errors.clone(),
        };
        t.render_table(area, buf);
    }
}
