use std::fs;
use std::time::Instant;

use ratatui::{
    buffer::Buffer,
    crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind},
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem, Paragraph, Widget},
};

use crate::screens::{Screen, ScreenAction};
use crate::theme;

pub struct SpadeLogsScreen {
    pub logs: Vec<String>,
    pub scroll_offset: usize,
    pub paused: bool,
    pub toast: Option<(String, Instant)>,
    pub pause_chip_rect: Rect,
    pub clear_chip_rect: Rect,
    pub export_chip_rect: Rect,
    pub logs_rect: Rect,
    action: ScreenAction,
}

impl Default for SpadeLogsScreen {
    fn default() -> Self {
        Self::new()
    }
}

impl SpadeLogsScreen {
    pub fn new() -> Self {
        SpadeLogsScreen {
            logs: Vec::new(),
            scroll_offset: 0,
            paused: false,
            toast: None,
            pause_chip_rect: Rect::default(),
            clear_chip_rect: Rect::default(),
            export_chip_rect: Rect::default(),
            logs_rect: Rect::default(),
            action: ScreenAction::None,
        }
    }

    pub fn add_log(&mut self, log: String) {
        let total_secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let secs = total_secs % 60;
        let mins = (total_secs / 60) % 60;
        let hours = (total_secs / 3600) % 24;
        let timestamp = format!("{hours:02}:{mins:02}:{secs:02}");
        let formatted = if log.starts_with('[') {
            format!("[{timestamp}] {log}")
        } else {
            format!("[{timestamp}] [INFO] {log}")
        };

        self.logs.push(formatted);
        if self.logs.len() > 5000 {
            self.logs.remove(0);
        }
        if !self.paused && self.scroll_offset == 0 {
            self.scroll_offset = 0;
        }
    }

    pub fn clear(&mut self) {
        self.logs.clear();
        self.scroll_offset = 0;
        self.toast = Some(("Logs cleared".to_string(), Instant::now()));
    }

    pub fn export_logs(&mut self) {
        if self.logs.is_empty() {
            self.toast = Some(("No logs to export".to_string(), Instant::now()));
            return;
        }

        let total_secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let filename = format!("spade-client-logs-{total_secs}.log");
        let content = self.logs.join("\n");

        match fs::write(&filename, content) {
            Ok(_) => {
                self.toast = Some((
                    format!("Exported {} lines to {}", self.logs.len(), filename),
                    Instant::now(),
                ));
            }
            Err(e) => {
                self.toast = Some((format!("Export failed: {e}"), Instant::now()));
            }
        }
    }

    fn scroll_up(&mut self, amount: usize) {
        self.paused = true;
        self.scroll_offset = self.scroll_offset.saturating_add(amount);
    }

    fn scroll_down(&mut self, amount: usize) {
        self.scroll_offset = self.scroll_offset.saturating_sub(amount);
        if self.scroll_offset == 0 {
            self.paused = false;
        }
    }
}

fn format_client_log_line(line: &str) -> Line<'static> {
    let mut spans = Vec::new();

    let mut rest = line;
    // 1. Extract first bracket [HH:MM:SS]
    if rest.starts_with('[') {
        if let Some(end) = rest.find(']') {
            let ts = &rest[..=end];
            spans.push(Span::styled(
                ts.to_string(),
                Style::default().fg(theme::FG_MUTED),
            ));
            spans.push(Span::raw(" "));
            rest = rest[end + 1..].trim_start();
        }
    }

    // 2. Extract tag e.g. [NETWORK], [RPC], [ERROR], [WARN], etc.
    if rest.starts_with('[') {
        if let Some(end) = rest.find(']') {
            let tag = &rest[..=end];
            let upper = tag.to_uppercase();
            let style = if upper.contains("ERROR") || upper.contains("FAIL") {
                Style::default()
                    .fg(theme::DANGER)
                    .add_modifier(Modifier::BOLD)
            } else if upper.contains("WARN") {
                Style::default()
                    .fg(theme::WARNING)
                    .add_modifier(Modifier::BOLD)
            } else if upper.contains("RPC") {
                Style::default()
                    .fg(theme::PRIMARY)
                    .add_modifier(Modifier::BOLD)
            } else if upper.contains("NETWORK") {
                Style::default()
                    .fg(theme::POSITIVE)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
                    .fg(theme::FG_BRIGHT)
                    .add_modifier(Modifier::BOLD)
            };

            spans.push(Span::styled(tag.to_string(), style));
            spans.push(Span::raw(" "));
            rest = rest[end + 1..].trim_start();
        }
    }

    spans.push(Span::styled(
        rest.to_string(),
        Style::default().fg(theme::FG),
    ));

    Line::from(spans)
}

impl Screen for SpadeLogsScreen {
    fn name(&self) -> &str {
        "Spade Logs"
    }

    fn reload(&mut self) {}

    fn handle_key(&mut self, key: KeyEvent) -> bool {
        if key.modifiers == KeyModifiers::CONTROL && key.code == KeyCode::Char('s') {
            self.export_logs();
            return true;
        }

        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                self.scroll_up(1);
                true
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.scroll_down(1);
                true
            }
            KeyCode::PageUp => {
                self.scroll_up(10);
                true
            }
            KeyCode::PageDown => {
                self.scroll_down(10);
                true
            }
            KeyCode::Home => {
                self.scroll_up(self.logs.len());
                true
            }
            KeyCode::End => {
                self.scroll_offset = 0;
                self.paused = false;
                true
            }
            KeyCode::Char(' ') => {
                self.paused = !self.paused;
                if !self.paused {
                    self.scroll_offset = 0;
                }
                true
            }
            KeyCode::Char('c') | KeyCode::Char('C') => {
                self.clear();
                true
            }
            _ => false,
        }
    }

    fn handle_mouse(&mut self, mouse: MouseEvent, _area: Rect) {
        let (col, row) = (mouse.column, mouse.row);

        match mouse.kind {
            MouseEventKind::ScrollUp => {
                self.scroll_up(3);
            }
            MouseEventKind::ScrollDown => {
                self.scroll_down(3);
            }
            MouseEventKind::Down(MouseButton::Left) => {
                let p = self.pause_chip_rect;
                if p.width > 0
                    && col >= p.x
                    && col < p.x + p.width
                    && row >= p.y
                    && row < p.y + p.height
                {
                    self.paused = !self.paused;
                    if !self.paused {
                        self.scroll_offset = 0;
                    }
                    return;
                }

                let c = self.clear_chip_rect;
                if c.width > 0
                    && col >= c.x
                    && col < c.x + c.width
                    && row >= c.y
                    && row < c.y + c.height
                {
                    self.clear();
                }

                let e = self.export_chip_rect;
                if e.width > 0
                    && col >= e.x
                    && col < e.x + e.width
                    && row >= e.y
                    && row < e.y + e.height
                {
                    self.export_logs();
                }
            }
            _ => {}
        }
    }

    fn render(&mut self, area: Rect, buf: &mut Buffer, mouse_pos: Option<(u16, u16)>) {
        if area.width < 10 || area.height < 6 {
            return;
        }

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Top title & status bar
                Constraint::Min(4),    // Log stream
                Constraint::Length(1), // Footer bar
            ])
            .split(area);

        self.logs_rect = chunks[1];

        // 1. Header block
        let total = self.logs.len();
        let header_title = format!(" SPADE CLIENT LOGS ({total}) ");
        let header_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .title(header_title)
            .title_style(
                Style::default()
                    .fg(theme::PRIMARY)
                    .add_modifier(Modifier::BOLD),
            )
            .border_style(theme::border_accent());
        let header_inner = header_block.inner(chunks[0]);
        Widget::render(header_block, chunks[0], buf);

        // Header controls (Pause chip, Clear chip, Export chip)
        let mut curr_x = header_inner.x;
        let chip_y = header_inner.y;

        // Status indicator
        let status_text = if self.paused {
            " ● PAUSED "
        } else {
            " ● LIVE STREAM "
        };
        let status_color = if self.paused {
            theme::WARNING
        } else {
            theme::POSITIVE
        };
        buf.set_string(
            curr_x,
            chip_y,
            status_text,
            Style::default()
                .fg(status_color)
                .add_modifier(Modifier::BOLD),
        );
        curr_x += status_text.len() as u16 + 2;

        // Pause / Resume Chip
        let pause_label = if self.paused {
            " [ Resume ] "
        } else {
            " [ Pause ] "
        };
        let pause_w = pause_label.len() as u16;
        let is_pause_hover =
            mouse_pos.is_some_and(|(c, r)| r == chip_y && c >= curr_x && c < curr_x + pause_w);
        let pause_style = if is_pause_hover {
            Style::default()
                .bg(theme::SURFACE)
                .fg(theme::PRIMARY)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme::FG_MUTED)
        };
        buf.set_string(curr_x, chip_y, pause_label, pause_style);
        self.pause_chip_rect = Rect::new(curr_x, chip_y, pause_w, 1);
        curr_x += pause_w + 1;

        // Clear Chip
        let clear_label = " [ Clear ] ";
        let clear_w = clear_label.len() as u16;
        let is_clear_hover =
            mouse_pos.is_some_and(|(c, r)| r == chip_y && c >= curr_x && c < curr_x + clear_w);
        let clear_style = if is_clear_hover {
            Style::default()
                .bg(theme::SURFACE)
                .fg(theme::DANGER)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme::FG_MUTED)
        };
        buf.set_string(curr_x, chip_y, clear_label, clear_style);
        self.clear_chip_rect = Rect::new(curr_x, chip_y, clear_w, 1);
        curr_x += clear_w + 1;

        // Export Chip
        let export_label = " [ Export ] ";
        let export_w = export_label.len() as u16;
        let is_export_hover =
            mouse_pos.is_some_and(|(c, r)| r == chip_y && c >= curr_x && c < curr_x + export_w);
        let export_style = if is_export_hover {
            Style::default()
                .bg(theme::SURFACE)
                .fg(theme::PRIMARY)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme::FG_MUTED)
        };
        buf.set_string(curr_x, chip_y, export_label, export_style);
        self.export_chip_rect = Rect::new(curr_x, chip_y, export_w, 1);

        // Toast message on the right of header
        if let Some((ref msg, t)) = self.toast {
            if t.elapsed().as_secs() < 4 {
                let msg_w = msg.len() as u16;
                let toast_x = header_inner.x + header_inner.width.saturating_sub(msg_w);
                buf.set_string(
                    toast_x,
                    chip_y,
                    msg,
                    Style::default()
                        .fg(theme::PRIMARY)
                        .add_modifier(Modifier::BOLD),
                );
            }
        }

        // 2. Log viewport
        let log_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(theme::border_accent());
        let log_inner = log_block.inner(chunks[1]);
        Widget::render(log_block, chunks[1], buf);

        let visible_h = log_inner.height as usize;
        let max_scroll = total.saturating_sub(visible_h);
        self.scroll_offset = self.scroll_offset.min(max_scroll);

        let slice: &[String] = if total == 0 {
            &[]
        } else if self.scroll_offset == 0 {
            let start = total.saturating_sub(visible_h);
            &self.logs[start..total]
        } else {
            let end = total - self.scroll_offset;
            let start = end.saturating_sub(visible_h);
            &self.logs[start..end]
        };

        let items: Vec<ListItem> = slice
            .iter()
            .map(|l| ListItem::new(format_client_log_line(l)))
            .collect();
        Widget::render(List::new(items), log_inner, buf);

        // 3. Footer Bar
        let footer_text = " [F1-F7] Switch Screens • [↑↓/PgUp/PgDn] Scroll • [Space] Pause • [C] Clear • [Ctrl+S] Export ";
        Paragraph::new(footer_text)
            .style(Style::default().bg(theme::BG_DARK).fg(theme::FG_BRIGHT))
            .render(chunks[2], buf);
    }

    fn take_action(&mut self) -> ScreenAction {
        std::mem::replace(&mut self.action, ScreenAction::None)
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn modal_overlay_active(&self) -> bool {
        false
    }
}
