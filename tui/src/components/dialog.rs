use ratatui::{
    buffer::Buffer,
    crossterm::event::{KeyCode, KeyEvent, MouseButton, MouseEvent, MouseEventKind},
    layout::Rect,
    style::{Color, Modifier, Style},
    text::Span,
    widgets::{Block, Borders, Widget},
};

pub struct Dialog {
    border_color: Color,
    title: String,
    message: String,
    buttons: Vec<String>,
    selected: usize,
    button_rects: Vec<Rect>,
    wrapped_lines: Vec<String>,
}

impl Dialog {
    pub fn new(border_color: Color, title: &str, message: &str, buttons: &[String]) -> Self {
        Dialog {
            border_color,
            title: title.to_string(),
            message: message.to_string(),
            buttons: buttons.to_vec(),
            selected: 0,
            button_rects: Vec::new(),
            wrapped_lines: Vec::new(),
        }
    }

    pub fn render(&mut self, area: Rect, buf: &mut Buffer, mouse_pos: Option<(u16, u16)>) {
        // Compute layout
        let btn_group_w: u16 = self
            .buttons
            .iter()
            .map(|b| format!(" {} ", b).len() as u16)
            .sum::<u16>()
            + ((self.buttons.len().saturating_sub(1)) * 3) as u16;

        // Determine content width (before word-wrap so we can wrap properly)
        let min_content_w = btn_group_w.max(40).max(4);
        let content_w = min_content_w;

        // Word-wrap message
        let wrap_w = content_w as usize - 4;
        self.wrapped_lines = word_wrap(&self.message, wrap_w);
        let _msg_lines = self.wrapped_lines.len();

        // Adjust width if any wrapped line is too wide
        let max_line_w = self
            .wrapped_lines
            .iter()
            .map(|l| l.len())
            .max()
            .unwrap_or(0);
        let content_w_adj = (max_line_w + 4).max(content_w as usize) as u16;

        // Re-wrap if width grew
        let final_content_w = if content_w_adj > content_w {
            let new_wrap = content_w_adj as usize - 4;
            self.wrapped_lines = word_wrap(&self.message, new_wrap);
            content_w_adj
        } else {
            content_w
        };

        let width = final_content_w + 4; // +2 border each side
        let msg_lines = self.wrapped_lines.len();
        let inner_h = msg_lines + 4; // pad + msg + gap + buttons + pad
        let height = inner_h as u16 + 2; // +2 for top/bottom border

        let x = area.x + (area.width.saturating_sub(width)) / 2;
        let y = area.y + (area.height.saturating_sub(height)) / 2;
        let overlay = Rect::new(x, y, width, height);

        ratatui::widgets::Clear.render(overlay, buf);
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(self.border_color))
            .title(Span::styled(
                &self.title,
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ))
            .style(Style::default().bg(Color::Black));
        let inner = block.inner(overlay);
        block.render(overlay, buf);

        // Fill inner area
        for iy in inner.y..inner.y + inner.height {
            for ix in inner.x..inner.x + inner.width {
                if let Some(cell) = buf.cell_mut((ix, iy)) {
                    cell.set_char(' ');
                    cell.set_bg(Color::Black);
                }
            }
        }

        // Message lines (inner.y + 1 onward)
        for (i, line) in self.wrapped_lines.iter().enumerate() {
            buf.set_string(
                inner.x + 2,
                inner.y + 1 + i as u16,
                line,
                Style::default().fg(Color::White),
            );
        }

        // Buttons centered as a group at inner.y + msg_lines + 2
        let btn_y = inner.y + msg_lines as u16 + 2;
        self.button_rects.clear();
        let mut btn_x = inner.x + (inner.width.saturating_sub(btn_group_w)) / 2;

        for (i, btn) in self.buttons.iter().enumerate() {
            let label = format!(" {} ", btn);
            let is_selected = i == self.selected;
            let is_hovered = mouse_pos.is_some_and(|(col, row)| {
                row == btn_y && col >= btn_x && col < btn_x + label.len() as u16
            });
            let is_active = is_selected || is_hovered;

            let (fg, bg) = if self.border_color == Color::Red && !is_active {
                // Destructive red dialog: non-active buttons neutral
                (Color::White, Color::Indexed(240))
            } else if self.border_color == Color::Red && is_active {
                // Hovered/selected destructive button
                (Color::White, Color::Red)
            } else if is_active {
                (Color::White, Color::Indexed(240))
            } else {
                (Color::White, Color::Indexed(236))
            };

            let mut style = Style::default().fg(fg).bg(bg);
            if is_active && self.border_color == Color::Red && i == 1 {
                // The destructive button (convention: last button = confirm)
                style = style.add_modifier(Modifier::BOLD);
            }

            buf.set_string(btn_x, btn_y, &label, style);
            self.button_rects
                .push(Rect::new(btn_x, btn_y, label.len() as u16, 1));
            btn_x += label.len() as u16 + 3; // gap between buttons
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> Option<usize> {
        match key.code {
            KeyCode::Left => {
                if self.buttons.len() > 1 && self.selected > 0 {
                    self.selected -= 1;
                }
                None
            }
            KeyCode::Right => {
                if self.selected + 1 < self.buttons.len() {
                    self.selected += 1;
                }
                None
            }
            KeyCode::Enter => Some(self.selected),
            KeyCode::Esc => {
                if !self.buttons.is_empty() {
                    Some(0)
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    pub fn handle_mouse(&mut self, mouse: MouseEvent) -> Option<usize> {
        if matches!(mouse.kind, MouseEventKind::Down(MouseButton::Left)) {
            for (i, rect) in self.button_rects.iter().enumerate() {
                if mouse.column >= rect.x
                    && mouse.column < rect.x + rect.width
                    && mouse.row >= rect.y
                    && mouse.row < rect.y + rect.height
                {
                    self.selected = i;
                    return Some(i);
                }
            }
        }
        None
    }
}

fn word_wrap(text: &str, max_width: usize) -> Vec<String> {
    let max_width = max_width.max(1);
    let mut result = Vec::new();
    for paragraph in text.split('\n') {
        let mut current = String::new();
        for word in paragraph.split(' ') {
            if word.is_empty() {
                continue;
            }
            if !current.is_empty() && current.len() + 1 + word.len() > max_width {
                result.push(current.clone());
                current.clear();
            }
            if current.is_empty() {
                current.push_str(word);
            } else {
                current.push(' ');
                current.push_str(word);
            }
        }
        if !current.is_empty() {
            result.push(current);
        } else if text.contains('\n') {
            // Preserve empty lines from explicit newlines
            result.push(String::new());
        }
    }
    if result.is_empty() && !text.is_empty() {
        // Single word that didn't get split
        result.push(text.to_string());
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_word_wrap_no_wrap() {
        assert_eq!(word_wrap("hello world", 80), vec!["hello world"]);
    }

    #[test]
    fn test_word_wrap_forces_break() {
        let lines = word_wrap("hello world", 5);
        assert_eq!(lines, vec!["hello", "world"]);
    }

    #[test]
    fn test_word_wrap_exact_fit() {
        let lines = word_wrap("hello world", 11);
        assert_eq!(lines, vec!["hello world"]);
    }

    #[test]
    fn test_word_wrap_newlines() {
        let lines = word_wrap("line1\nline2", 80);
        assert_eq!(lines, vec!["line1", "line2"]);
    }

    #[test]
    fn test_word_wrap_email_like() {
        let lines = word_wrap("Hello player,\n\nWelcome to the game!", 30);
        assert_eq!(lines, vec!["Hello player,", "", "Welcome to the game!"]);
    }
}
