use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Cell, Row, Table as RtTable, Widget},
};
use unicode_width::UnicodeWidthStr;

use crate::components::ScrollState;
use crate::theme::{self, ButtonVariant};

#[derive(Debug, Clone)]
pub struct RowErrorInfo {
    pub message: String,
    pub is_toml: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BadgeKind {
    AddEntry,
    Clear,
    MoveUp,
    MoveDown,
    Remove,
}

impl BadgeKind {
    /// Chip variant for this badge's role in the action language.
    pub fn variant(self) -> ButtonVariant {
        match self {
            BadgeKind::AddEntry | BadgeKind::MoveUp | BadgeKind::MoveDown => ButtonVariant::Default,
            BadgeKind::Clear => ButtonVariant::Warning,
            BadgeKind::Remove => ButtonVariant::Destructive,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Badge {
    pub text: &'static str,
    pub kind: BadgeKind,
}

#[derive(Debug, Clone)]
pub struct RowBadges {
    pub col: usize,
    pub gap: u16,
    pub badges: Vec<Badge>,
}

#[derive(Debug, Clone, Copy)]
pub struct BadgeSpan {
    pub kind: BadgeKind,
    pub text: &'static str,
    pub x0: u16,
    pub x1: u16,
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
    pub overlays: Vec<Option<RowBadges>>,
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
            overlays: Vec::new(),
        }
    }

    pub fn add_row(&mut self, row: Vec<String>) {
        self.rows.push(row);
        self.overlays.push(None);
        self.scroll.total_lines = self.rows.len();
    }

    pub fn set_row_badges(&mut self, row: usize, badges: RowBadges) {
        if row < self.overlays.len() {
            self.overlays[row] = Some(badges);
        }
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

    /// Column rectangles laid out with the table's constraints, starting at `area.x`.
    pub fn col_areas(&self, area: Rect) -> Vec<Rect> {
        Layout::horizontal(&self.column_widths)
            .split(Rect::new(area.x, area.y, area.width, area.height))
            .to_vec()
    }

    /// Absolute x of a column's left edge.
    pub fn col_x(&self, col: usize, area: Rect) -> u16 {
        self.col_areas(area).get(col).map(|r| r.x).unwrap_or(0)
    }

    /// Width of a column in the current layout.
    pub fn col_width(&self, col: usize, area: Rect) -> u16 {
        self.col_areas(area).get(col).map(|r| r.width).unwrap_or(0)
    }

    /// Absolute screen x-ranges of the action badges trailing a cell's text.
    /// Mirrors the clipping used at render time so a clipped badge is not clickable.
    pub fn badge_spans(&self, row: usize, area: Rect) -> Vec<BadgeSpan> {
        let mut spans = Vec::new();
        let Some(ov) = self.overlays.get(row).and_then(|o| o.as_ref()) else {
            return spans;
        };
        let value = &self.rows[row][ov.col];
        let mut x = self
            .col_x(ov.col, area)
            .saturating_add(1)
            .saturating_add(UnicodeWidthStr::width(value.as_str()) as u16);
        let clip = area.x.saturating_add(area.width);
        for badge in &ov.badges {
            x = x.saturating_add(ov.gap);
            // Rendered chip includes one space of padding each side.
            let w = UnicodeWidthStr::width(badge.text) as u16 + 2;
            if x.saturating_add(w) > clip {
                break;
            }
            spans.push(BadgeSpan {
                kind: badge.kind,
                text: badge.text,
                x0: x,
                x1: x + w,
            });
            x = x.saturating_add(w);
        }
        spans
    }

    pub fn render_table(&mut self, area: Rect, buf: &mut Buffer) {
        if area.width < 4 || area.height < 2 {
            return;
        }

        let visible = area.height.saturating_sub(1) as usize;
        self.update_scroll(visible);
        let offset = self.scroll.offset;

        // Header background across the full width.
        for x in area.x..area.x + area.width {
            if let Some(cell) = buf.cell_mut((x, area.y)) {
                cell.set_bg(theme::SURFACE);
            }
        }
        let header_style = Style::default()
            .fg(theme::PRIMARY)
            .bg(theme::SURFACE)
            .add_modifier(Modifier::BOLD);
        let col_areas = self.col_areas(area);
        for (i, header) in self.headers.iter().enumerate() {
            if let Some(col_area) = col_areas.get(i) {
                buf.set_stringn(
                    col_area.x,
                    area.y,
                    format!(" {}", header),
                    col_area.width as usize,
                    header_style,
                );
            }
        }

        let body = Rect::new(
            area.x,
            area.y + 1,
            area.width,
            area.height.saturating_sub(1),
        );
        let mut rows: Vec<Row> = Vec::new();
        let mut active_tooltip: Option<(u16, u16, String, bool)> = None;

        for i in 0..visible {
            let idx = offset + i;
            if idx >= self.rows.len() {
                break;
            }

            let err_info = self.row_errors.get(&idx).cloned();
            let has_error = err_info.is_some();
            let is_selected = Some(idx) == self.selected;
            let is_hovered = self.hovered == Some(idx);

            let error_fg = |err: &RowErrorInfo| {
                if err.is_toml {
                    theme::DANGER
                } else {
                    theme::WARNING
                }
            };

            let text_fg = if let Some(ref err) = err_info {
                error_fg(err)
            } else if self.muted {
                theme::FG_MUTED
            } else {
                theme::FG
            };

            let row_style = if let Some(ref err) = err_info {
                Style::default().fg(error_fg(err)).bg(theme::DANGER_BG)
            } else if is_selected {
                theme::selected_row()
            } else if is_hovered {
                Style::default().fg(text_fg).bg(theme::HOVER)
            } else {
                let mut style = Style::default().fg(text_fg);
                if i % 2 == 1 {
                    style = style.bg(theme::BG_DARK);
                }
                style
            };

            let symbol_style = if let Some(ref err) = err_info {
                Style::default()
                    .fg(error_fg(err))
                    .add_modifier(Modifier::BOLD)
            } else if is_selected {
                Style::default().fg(theme::BG).add_modifier(Modifier::BOLD)
            } else if is_hovered {
                Style::default().fg(theme::FG_MUTED)
            } else {
                Style::default()
            };

            let symbol = if err_info.is_some() {
                "⚠ "
            } else if is_selected {
                "▸ "
            } else if is_hovered {
                "· "
            } else {
                "  "
            };

            if has_error && (is_hovered || (self.hovered.is_none() && is_selected)) {
                if let Some(ref err) = err_info {
                    active_tooltip = Some((
                        area.x + 2,
                        body.y + i as u16,
                        err.message.clone(),
                        err.is_toml,
                    ));
                }
            }

            rows.push(
                Row::new(self.row_cells(idx, symbol, symbol_style, row_style)).style(row_style),
            );
        }

        let widths: Vec<Constraint> = self.column_widths.to_vec();
        RtTable::new(rows, widths)
            .column_spacing(0)
            .render(body, buf);

        // Inline action chips drawn as overlays at their hit-test positions,
        // so they render as distinct shell buttons and never get clipped
        // mid-chip by the table cell layout.
        for i in 0..visible {
            let idx = offset + i;
            if idx >= self.rows.len() {
                break;
            }
            let y = body.y + i as u16;
            for span in self.badge_spans(idx, area) {
                let label = format!(" {} ", span.text);
                let width = UnicodeWidthStr::width(label.as_str()) as usize;
                buf.set_stringn(
                    span.x0,
                    y,
                    label.as_str(),
                    width,
                    theme::chip_style(span.kind.variant(), span.kind == BadgeKind::AddEntry),
                );
            }
        }

        // Tooltip popup overlay for hovered error rows.
        if let Some((tx, ty, msg, is_toml)) = active_tooltip {
            crate::components::TooltipPopup::render(buf, area, tx, ty, &msg, is_toml);
        }
    }

    fn row_cells(
        &self,
        idx: usize,
        symbol: &str,
        symbol_style: Style,
        row_style: Style,
    ) -> Vec<Cell<'_>> {
        let is_array_header = self.rows[idx]
            .get(1)
            .is_some_and(|v| v.starts_with("(array"));
        self.rows[idx]
            .iter()
            .enumerate()
            .map(|(col, value)| {
                let cell_style = if col == 0 && is_array_header {
                    row_style.fg(theme::PRIMARY).add_modifier(Modifier::BOLD)
                } else {
                    row_style
                };
                if col == 0 {
                    Cell::from(Line::from(vec![
                        Span::styled(symbol.to_string(), symbol_style),
                        Span::styled(format!(" {value}"), cell_style),
                    ]))
                } else {
                    Cell::from(Line::from(vec![Span::styled(
                        format!(" {value}"),
                        cell_style,
                    )]))
                }
            })
            .collect()
    }
}
