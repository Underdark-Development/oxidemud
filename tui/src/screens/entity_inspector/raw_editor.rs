use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    Structured,
    RawToml,
}

#[derive(Debug, Clone)]
pub struct RawEditorError {
    pub line: usize,
    pub col: usize,
    pub message: String,
    pub is_syntax: bool,
}

pub struct RawTomlEditor {
    pub lines: Vec<String>,
    pub cursor_line: usize,
    pub cursor_col: usize,
    pub scroll: usize,
    pub error: Option<RawEditorError>,
    pub dirty: bool,
    pub undo_stack: Vec<(Vec<String>, usize, usize)>,
    pub redo_stack: Vec<(Vec<String>, usize, usize)>,
    pub selection_start: Option<(usize, usize)>,
    pub selection_end: Option<(usize, usize)>,
    pub is_selecting: bool,
    pub clipboard: String,
}

impl Default for RawTomlEditor {
    fn default() -> Self {
        Self::new()
    }
}

impl RawTomlEditor {
    pub fn new() -> Self {
        RawTomlEditor {
            lines: vec![String::new()],
            cursor_line: 0,
            cursor_col: 0,
            scroll: 0,
            error: None,
            dirty: false,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            selection_start: None,
            selection_end: None,
            is_selecting: false,
            clipboard: String::new(),
        }
    }

    pub fn set_content(&mut self, content: &str) {
        let lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
        self.lines = if lines.is_empty() {
            vec![String::new()]
        } else {
            lines
        };
        self.cursor_line = 0;
        self.cursor_col = 0;
        self.scroll = 0;
        self.error = None;
        self.dirty = false;
        self.undo_stack.clear();
        self.redo_stack.clear();
        self.selection_start = None;
        self.selection_end = None;
        self.is_selecting = false;
    }

    pub fn to_string_content(&self) -> String {
        self.lines.join("\n")
    }

    pub fn push_undo(&mut self) {
        if self.undo_stack.len() >= 50 {
            self.undo_stack.remove(0);
        }
        self.undo_stack
            .push((self.lines.clone(), self.cursor_line, self.cursor_col));
        self.redo_stack.clear();
    }

    pub fn undo(&mut self) {
        if let Some((lines, cl, cc)) = self.undo_stack.pop() {
            self.redo_stack
                .push((self.lines.clone(), self.cursor_line, self.cursor_col));
            self.lines = lines;
            self.cursor_line = cl;
            self.cursor_col = cc;
            self.dirty = true;
            self.ensure_cursor_visible();
        }
    }

    pub fn redo(&mut self) {
        if let Some((lines, cl, cc)) = self.redo_stack.pop() {
            self.undo_stack
                .push((self.lines.clone(), self.cursor_line, self.cursor_col));
            self.lines = lines;
            self.cursor_line = cl;
            self.cursor_col = cc;
            self.dirty = true;
            self.ensure_cursor_visible();
        }
    }

    pub fn get_selection_range(&self) -> Option<((usize, usize), (usize, usize))> {
        let start = self.selection_start?;
        let end = self.selection_end?;
        if start == end {
            return None;
        }
        if start <= end {
            Some((start, end))
        } else {
            Some((end, start))
        }
    }

    pub fn copy_selection(&mut self) {
        if let Some((start, end)) = self.get_selection_range() {
            let mut extracted = Vec::new();
            for l in start.0..=end.0 {
                if l >= self.lines.len() {
                    break;
                }
                let line_str = &self.lines[l];
                let c_start = if l == start.0 {
                    start.1.min(line_str.len())
                } else {
                    0
                };
                let c_end = if l == end.0 {
                    end.1.min(line_str.len())
                } else {
                    line_str.len()
                };
                if c_start < c_end {
                    extracted.push(line_str[c_start..c_end].to_string());
                }
            }
            self.clipboard = extracted.join("\n");
        } else if self.cursor_line < self.lines.len() {
            self.clipboard = self.lines[self.cursor_line].clone();
        }
    }

    pub fn cut_selection(&mut self) {
        self.copy_selection();
        if let Some((start, end)) = self.get_selection_range() {
            self.push_undo();
            if start.0 == end.0 {
                let l_str = &mut self.lines[start.0];
                let s = start.1.min(l_str.len());
                let e = end.1.min(l_str.len());
                l_str.drain(s..e);
                self.cursor_col = s;
            } else {
                let remaining =
                    self.lines[start.0][..start.1.min(self.lines[start.0].len())].to_string();
                let end_rem = self.lines[end.0.min(self.lines.len() - 1)]
                    [end.1.min(self.lines[end.0.min(self.lines.len() - 1)].len())..]
                    .to_string();
                let mut joined = remaining;
                joined.push_str(&end_rem);
                for _ in start.0..end.0 {
                    if start.0 + 1 < self.lines.len() {
                        self.lines.remove(start.0 + 1);
                    }
                }
                self.lines[start.0] = joined;
                self.cursor_line = start.0;
                self.cursor_col = start.1;
            }
            self.selection_start = None;
            self.selection_end = None;
            self.dirty = true;
        }
    }

    pub fn paste_clipboard(&mut self) {
        if self.clipboard.is_empty() {
            return;
        }
        self.push_undo();
        let clip_lines: Vec<&str> = self.clipboard.lines().collect();
        if clip_lines.len() == 1 {
            self.lines[self.cursor_line].insert_str(self.cursor_col, clip_lines[0]);
            self.cursor_col += clip_lines[0].len();
        } else if !clip_lines.is_empty() {
            let remainder = self.lines[self.cursor_line].split_off(self.cursor_col);
            self.lines[self.cursor_line].push_str(clip_lines[0]);
            for cl in clip_lines.iter().skip(1) {
                self.cursor_line += 1;
                self.lines.insert(self.cursor_line, cl.to_string());
            }
            self.cursor_col = self.lines[self.cursor_line].len();
            self.lines[self.cursor_line].push_str(&remainder);
        }
        self.dirty = true;
        self.ensure_cursor_visible();
    }

    pub fn handle_key(&mut self, key: ratatui::crossterm::event::KeyEvent) -> bool {
        use ratatui::crossterm::event::{KeyCode, KeyModifiers};

        let mods = key.modifiers;

        if mods.contains(KeyModifiers::CONTROL) || mods.contains(KeyModifiers::SUPER) {
            match key.code {
                KeyCode::Char('z') | KeyCode::Char('Z') | KeyCode::Char('\u{1a}') => {
                    if mods.contains(KeyModifiers::SHIFT) || key.code == KeyCode::Char('Z') {
                        self.redo();
                    } else {
                        self.undo();
                    }
                    return true;
                }
                KeyCode::Char('y')
                | KeyCode::Char('Y')
                | KeyCode::Char('\u{19}')
                | KeyCode::Char('r')
                | KeyCode::Char('R')
                | KeyCode::Char('\u{12}') => {
                    self.redo();
                    return true;
                }
                KeyCode::Char('c') | KeyCode::Char('C') => {
                    self.copy_selection();
                    return true;
                }
                KeyCode::Char('x') | KeyCode::Char('X') => {
                    self.cut_selection();
                    return true;
                }
                KeyCode::Char('v') | KeyCode::Char('V') => {
                    self.paste_clipboard();
                    return true;
                }
                _ => {}
            }
        }

        match key.code {
            KeyCode::Up => {
                if self.cursor_line > 0 {
                    self.cursor_line -= 1;
                    self.clamp_cursor_col();
                    self.ensure_cursor_visible();
                }
                true
            }
            KeyCode::Down => {
                if self.cursor_line + 1 < self.lines.len() {
                    self.cursor_line += 1;
                    self.clamp_cursor_col();
                    self.ensure_cursor_visible();
                }
                true
            }
            KeyCode::Left => {
                if self.cursor_col > 0 {
                    self.cursor_col -= 1;
                } else if self.cursor_line > 0 {
                    self.cursor_line -= 1;
                    self.cursor_col = self.lines[self.cursor_line].len();
                    self.ensure_cursor_visible();
                }
                true
            }
            KeyCode::Right => {
                let line_len = self.lines[self.cursor_line].len();
                if self.cursor_col < line_len {
                    self.cursor_col += 1;
                } else if self.cursor_line + 1 < self.lines.len() {
                    self.cursor_line += 1;
                    self.cursor_col = 0;
                    self.ensure_cursor_visible();
                }
                true
            }
            KeyCode::Home => {
                self.cursor_col = 0;
                true
            }
            KeyCode::End => {
                self.cursor_col = self.lines[self.cursor_line].len();
                true
            }
            KeyCode::PageUp => {
                self.cursor_line = self.cursor_line.saturating_sub(10);
                self.clamp_cursor_col();
                self.ensure_cursor_visible();
                true
            }
            KeyCode::PageDown => {
                self.cursor_line = (self.cursor_line + 10).min(self.lines.len().saturating_sub(1));
                self.clamp_cursor_col();
                self.ensure_cursor_visible();
                true
            }
            KeyCode::Char(c) => {
                self.push_undo();
                self.lines[self.cursor_line].insert(self.cursor_col, c);
                self.cursor_col += 1;
                self.dirty = true;
                true
            }
            KeyCode::Backspace => {
                if self.cursor_col > 0 {
                    self.push_undo();
                    self.lines[self.cursor_line].remove(self.cursor_col - 1);
                    self.cursor_col -= 1;
                    self.dirty = true;
                } else if self.cursor_line > 0 {
                    self.push_undo();
                    let cur_line = self.lines.remove(self.cursor_line);
                    self.cursor_line -= 1;
                    self.cursor_col = self.lines[self.cursor_line].len();
                    self.lines[self.cursor_line].push_str(&cur_line);
                    self.dirty = true;
                    self.ensure_cursor_visible();
                }
                true
            }
            KeyCode::Delete => {
                let line_len = self.lines[self.cursor_line].len();
                if self.cursor_col < line_len {
                    self.push_undo();
                    self.lines[self.cursor_line].remove(self.cursor_col);
                    self.dirty = true;
                } else if self.cursor_line + 1 < self.lines.len() {
                    self.push_undo();
                    let next_line = self.lines.remove(self.cursor_line + 1);
                    self.lines[self.cursor_line].push_str(&next_line);
                    self.dirty = true;
                }
                true
            }
            KeyCode::Enter => {
                self.push_undo();
                let remainder = self.lines[self.cursor_line].split_off(self.cursor_col);
                self.cursor_line += 1;
                self.lines.insert(self.cursor_line, remainder);
                self.cursor_col = 0;
                self.dirty = true;
                self.ensure_cursor_visible();
                true
            }
            _ => false,
        }
    }

    fn clamp_cursor_col(&mut self) {
        let line_len = self.lines[self.cursor_line].len();
        if self.cursor_col > line_len {
            self.cursor_col = line_len;
        }
    }

    fn ensure_cursor_visible(&mut self) {
        if self.cursor_line < self.scroll {
            self.scroll = self.cursor_line;
        }
    }

    pub fn handle_mouse(
        &mut self,
        mouse: ratatui::crossterm::event::MouseEvent,
        area: Rect,
    ) -> bool {
        use ratatui::crossterm::event::{MouseButton, MouseEventKind};

        if mouse.column < area.x
            || mouse.column >= area.x + area.width
            || mouse.row < area.y
            || mouse.row >= area.y + area.height
        {
            return false;
        }

        match mouse.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                let rel_row = (mouse.row as usize).saturating_sub(area.y as usize);
                let target_line = self.scroll + rel_row;
                if target_line < self.lines.len() {
                    self.cursor_line = target_line;
                    let num_width = 4;
                    let text_start_x = area.x as usize + num_width + 1;
                    let rel_col = (mouse.column as usize).saturating_sub(text_start_x);
                    self.cursor_col = rel_col.min(self.lines[self.cursor_line].len());
                    self.selection_start = Some((self.cursor_line, self.cursor_col));
                    self.selection_end = Some((self.cursor_line, self.cursor_col));
                    self.is_selecting = true;
                    self.ensure_cursor_visible();
                }
                true
            }
            MouseEventKind::Drag(MouseButton::Left) => {
                if self.is_selecting {
                    let rel_row = (mouse.row as usize).saturating_sub(area.y as usize);
                    let target_line =
                        (self.scroll + rel_row).min(self.lines.len().saturating_sub(1));
                    let num_width = 4;
                    let text_start_x = area.x as usize + num_width + 1;
                    let rel_col = (mouse.column as usize).saturating_sub(text_start_x);
                    let target_col = rel_col.min(self.lines[target_line].len());
                    self.selection_end = Some((target_line, target_col));
                    self.cursor_line = target_line;
                    self.cursor_col = target_col;
                }
                true
            }
            MouseEventKind::Up(MouseButton::Left) => {
                self.is_selecting = false;
                true
            }
            MouseEventKind::ScrollUp => {
                self.scroll = self.scroll.saturating_sub(3);
                true
            }
            MouseEventKind::ScrollDown => {
                if self.scroll + 3 < self.lines.len() {
                    self.scroll += 3;
                }
                true
            }
            _ => false,
        }
    }

    pub fn validate_with_schema<F>(&mut self, schema_validator: F)
    where
        F: FnOnce(&str) -> Result<(), toml::de::Error>,
    {
        let content = self.to_string_content();
        match toml::from_str::<toml::Value>(&content) {
            Ok(_) => {
                if let Err(e) = schema_validator(&content) {
                    let (line, col) = if let Some(span) = e.span() {
                        byte_offset_to_line_col(&content, span.start)
                    } else {
                        (0, 0)
                    };
                    self.error = Some(RawEditorError {
                        line,
                        col,
                        message: e.to_string(),
                        is_syntax: false,
                    });
                } else {
                    self.error = None;
                }
            }
            Err(e) => {
                let (line, col) = if let Some(span) = e.span() {
                    byte_offset_to_line_col(&content, span.start)
                } else {
                    (0, 0)
                };
                self.error = Some(RawEditorError {
                    line,
                    col,
                    message: e.to_string(),
                    is_syntax: true,
                });
            }
        }
    }

    pub fn validate_and_update_error(&mut self) {
        self.validate_with_schema(|_| Ok(()));
    }

    pub fn render(&mut self, area: Rect, buf: &mut Buffer, is_focused: bool) {
        if area.width < 5 || area.height < 2 {
            return;
        }

        let num_width = 4;
        let editor_width = area.width.saturating_sub(num_width + 1);

        let visible_lines = (area.height as usize).saturating_sub(1); // reserve 1 row for status line
        if self.cursor_line >= self.scroll + visible_lines {
            self.scroll = self.cursor_line.saturating_sub(visible_lines - 1);
        }

        for r in 0..visible_lines {
            let line_idx = self.scroll + r;
            let y = area.y + r as u16;

            if line_idx >= self.lines.len() {
                break;
            }

            let is_cur_line = line_idx == self.cursor_line;
            let line_text = &self.lines[line_idx];

            // Render line number gutter
            let num_str = format!("{:>3} │", line_idx + 1);
            let num_style = if is_cur_line {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Indexed(242))
            };
            buf.set_string(area.x, y, &num_str, num_style);

            // Render TOML syntax highlighted content
            let x_start = area.x + num_width + 1;
            let mut spans = highlight_toml_line(line_text);

            // Check if error is on this line
            let mut line_error = None;
            if let Some(ref err) = self.error {
                if err.line == line_idx {
                    line_error = Some(err.clone());
                }
            }

            if let Some(err) = line_error {
                let err_style = if err.is_syntax {
                    Style::default()
                        .fg(Color::LightRed)
                        .add_modifier(Modifier::UNDERLINED | Modifier::BOLD)
                } else {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::UNDERLINED | Modifier::BOLD)
                };

                // Underline the offending column / word or entire line if column is 0
                for span in spans.iter_mut() {
                    span.style = span.style.patch(err_style);
                }

                let badge_style = if err.is_syntax {
                    Style::default().fg(Color::White).bg(Color::Red)
                } else {
                    Style::default().fg(Color::Black).bg(Color::Yellow)
                };

                spans.push(Span::styled(format!("  [ {} ]", err.message), badge_style));
            }

            let line = Line::from(spans);
            buf.set_line(x_start, y, &line, editor_width);

            // Render selection highlight overlay
            if let Some((s_pos, e_pos)) = self.get_selection_range() {
                if line_idx >= s_pos.0 && line_idx <= e_pos.0 {
                    let s_col = if line_idx == s_pos.0 { s_pos.1 } else { 0 };
                    let e_col = if line_idx == e_pos.0 {
                        e_pos.1
                    } else {
                        line_text.len()
                    };
                    for col in s_col..e_col {
                        let sel_x = x_start + col as u16;
                        if sel_x < area.x + area.width {
                            if let Some(cell) = buf.cell_mut((sel_x, y)) {
                                cell.set_bg(Color::Indexed(242));
                                cell.set_fg(Color::White);
                            }
                        }
                    }
                }
            }

            // Render text cursor if focused
            if is_focused && is_cur_line {
                let cursor_x = x_start + self.cursor_col as u16;
                if cursor_x < area.x + area.width {
                    if let Some(cell) = buf.cell_mut((cursor_x, y)) {
                        cell.set_bg(Color::White);
                        cell.set_fg(Color::Black);
                    }
                }
            }
        }

        // Render editor footer line
        let footer_y = area.y + area.height - 1;
        let status_msg = if let Some(ref err) = self.error {
            format!(
                " Line {}, Col {} | ERR: {} ",
                self.cursor_line + 1,
                self.cursor_col + 1,
                err.message
            )
        } else {
            format!(
                " Line {}, Col {} | TOML Valid ",
                self.cursor_line + 1,
                self.cursor_col + 1
            )
        };
        let footer_style = if self.error.is_some() {
            Style::default().fg(Color::White).bg(Color::Red)
        } else {
            Style::default()
                .fg(Color::Indexed(245))
                .bg(Color::Indexed(236))
        };
        buf.set_string(area.x, footer_y, &status_msg, footer_style);

        // Render TooltipPopup overlay if cursor is on error line
        if let Some(ref err) = self.error {
            let vis_line = err.line.saturating_sub(self.scroll);
            if err.line >= self.scroll && vis_line < visible_lines {
                let err_y = area.y + vis_line as u16;
                if self.cursor_line == err.line {
                    crate::components::TooltipPopup::render(
                        buf,
                        area,
                        area.x + 4,
                        err_y,
                        &err.message,
                        err.is_syntax,
                    );
                }
            }
        }
    }
}

fn byte_offset_to_line_col(text: &str, target_offset: usize) -> (usize, usize) {
    let mut current_offset = 0;
    for (line_idx, line) in text.lines().enumerate() {
        let line_len = line.len() + 1; // +1 for newline
        if current_offset + line_len > target_offset {
            let col = target_offset.saturating_sub(current_offset);
            return (line_idx, col);
        }
        current_offset += line_len;
    }
    (0, 0)
}

fn highlight_toml_line(line: &str) -> Vec<Span<'static>> {
    let trimmed = line.trim_start();
    if trimmed.starts_with('#') {
        return vec![Span::styled(
            line.to_string(),
            Style::default().fg(Color::Indexed(242)),
        )];
    }
    if trimmed.starts_with('[') && trimmed.ends_with(']') {
        return vec![Span::styled(
            line.to_string(),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )];
    }

    if let Some((key, val)) = line.split_once('=') {
        vec![
            Span::styled(key.to_string(), Style::default().fg(Color::Yellow)),
            Span::styled("=", Style::default().fg(Color::Indexed(245))),
            highlight_toml_value(val),
        ]
    } else {
        vec![Span::styled(
            line.to_string(),
            Style::default().fg(Color::White),
        )]
    }
}

fn highlight_toml_value(val: &str) -> Span<'static> {
    let trimmed = val.trim();
    if trimmed.starts_with('"') || trimmed.starts_with('\'') {
        Span::styled(val.to_string(), Style::default().fg(Color::Green))
    } else if trimmed == "true" || trimmed == "false" {
        Span::styled(val.to_string(), Style::default().fg(Color::LightBlue))
    } else if trimmed.parse::<f64>().is_ok() || trimmed.parse::<i64>().is_ok() {
        Span::styled(val.to_string(), Style::default().fg(Color::Magenta))
    } else {
        Span::styled(val.to_string(), Style::default().fg(Color::White))
    }
}
