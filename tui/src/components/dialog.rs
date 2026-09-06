use ratatui::{
    buffer::Buffer,
    crossterm::event::{KeyCode, KeyEvent, MouseButton, MouseEvent, MouseEventKind},
    layout::Rect,
    style::{Modifier, Style},
    widgets::{Block, BorderType, Borders, Clear, Widget},
};

use crate::components::Button;
use crate::theme::{self, ButtonState, ButtonVariant, DialogTone};

/// Modal confirmation/informational dialog.
///
/// Rendered per the Spade UI/UX Design Guide: a rounded border tinted by tone,
/// an inline bold `PRIMARY` title line, a word-wrapped message, and a
/// right-aligned row of chip buttons. Keyboard focus defaults to the safe
/// option (Cancel/No/Dismiss) and `Esc` activates it; `Enter`/`Space` activate
/// the focused button; `Tab`/arrows cycle focus; single-letter accelerator
/// hints (e.g. `Allow (a)`) activate their button.
pub struct Dialog {
    tone: DialogTone,
    title: String,
    message: String,
    buttons: Vec<Button>,
    selected: usize,
    button_rects: Vec<Rect>,
    wrapped_lines: Vec<String>,
}

impl Dialog {
    pub fn new(tone: DialogTone, title: &str, message: &str, buttons: &[String]) -> Self {
        let buttons: Vec<Button> = buttons
            .iter()
            .enumerate()
            .map(|(i, label)| {
                let variant = dialog_button_variant(tone, i, buttons.len());
                Button::new(label.clone(), variant)
            })
            .collect();
        let selected = buttons
            .iter()
            .position(|b| is_safe_label(b.label()))
            .unwrap_or(0);
        Dialog {
            tone,
            title: title.to_string(),
            message: message.to_string(),
            buttons,
            selected,
            button_rects: Vec::new(),
            wrapped_lines: Vec::new(),
        }
    }

    /// Buttons currently shown by this dialog, in order.
    pub fn buttons(&self) -> &[Button] {
        &self.buttons
    }

    pub fn render(&mut self, area: Rect, buf: &mut Buffer, mouse_pos: Option<(u16, u16)>) {
        if self.buttons.is_empty() {
            return;
        }

        // Button row width: chip widths joined by two-space gaps.
        let btn_gap = 2u16;
        let btn_row_w = self.buttons.iter().map(Button::width).sum::<u16>()
            + btn_gap * (self.buttons.len().saturating_sub(1)) as u16;

        // Start with a content width that fits the button row and title.
        let title_width = self.title.chars().count() as u16;
        let mut inner_w = btn_row_w.max(title_width).max(40);

        // Word-wrap the message to the available width, then widen if needed.
        self.wrapped_lines = word_wrap(&self.message, (inner_w.saturating_sub(4)).max(1) as usize);
        let max_line_w = self
            .wrapped_lines
            .iter()
            .map(|l| l.chars().count())
            .max()
            .unwrap_or(0) as u16;
        if max_line_w + 4 > inner_w {
            inner_w = max_line_w + 4;
            let wrap = (inner_w.saturating_sub(4)).max(1) as usize;
            self.wrapped_lines = word_wrap(&self.message, wrap);
        }

        let msg_lines = self.wrapped_lines.len() as u16;
        // Inner rows: pad, title, gap, message, gap, buttons, pad.
        let inner_h = msg_lines + 6;
        let width = inner_w + 2;
        let height = inner_h + 2;
        let x = area.x + area.width.saturating_sub(width) / 2;
        let y = area.y + area.height.saturating_sub(height) / 2;
        let overlay = Rect::new(x, y, width, height);

        Clear.render(overlay, buf);
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme::dialog_border_color(self.tone)))
            .style(theme::canvas_style());
        let inner = block.inner(overlay);
        block.render(overlay, buf);

        let title_style = Style::default()
            .fg(theme::PRIMARY)
            .add_modifier(Modifier::BOLD);
        buf.set_string(inner.x + 1, inner.y + 1, &self.title, title_style);

        let msg_style = Style::default().fg(theme::FG);
        for (i, line) in self.wrapped_lines.iter().enumerate() {
            buf.set_string(inner.x + 1, inner.y + 3 + i as u16, line, msg_style);
        }

        // Right-aligned button row.
        let btn_row_y = inner.y + inner.height - 2;
        let mut btn_x = inner.x + inner.width.saturating_sub(btn_row_w);
        self.button_rects.clear();
        for (i, button) in self.buttons.iter().enumerate() {
            let is_hovered = mouse_pos.is_some_and(|(col, row)| {
                row == btn_row_y && col >= btn_x && col < btn_x + button.width()
            });
            let state = if self.selected == i || is_hovered {
                ButtonState::Active
            } else {
                ButtonState::Inactive
            };
            let rect = button.render(buf, btn_x, btn_row_y, state);
            self.button_rects.push(rect);
            btn_x += button.width() + btn_gap;
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> Option<usize> {
        if self.buttons.is_empty() {
            return None;
        }
        let len = self.buttons.len();
        match key.code {
            KeyCode::Tab | KeyCode::Right => {
                self.selected = (self.selected + 1) % len;
                None
            }
            KeyCode::BackTab | KeyCode::Left => {
                self.selected = (self.selected + len - 1) % len;
                None
            }
            KeyCode::Enter | KeyCode::Char(' ') => Some(self.selected),
            KeyCode::Esc => Some(self.safe_index()),
            KeyCode::Char(c) => self.accelerator_index(c),
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

    /// Index of the safe (non-destructive) option: Cancel/No/Dismiss/Close.
    fn safe_index(&self) -> usize {
        self.buttons
            .iter()
            .position(|b| is_safe_label(b.label()))
            .unwrap_or(0)
    }

    /// Index of the button whose accelerator hint matches `ch` (e.g. `(d)`).
    fn accelerator_index(&self, ch: char) -> Option<usize> {
        let needle = ch.to_ascii_lowercase();
        self.buttons.iter().position(|b| {
            b.label()
                .split('(')
                .nth(1)
                .and_then(|tail| tail.chars().next())
                .is_some_and(|c| c.to_ascii_lowercase() == needle)
        })
    }
}

fn is_safe_label(label: &str) -> bool {
    matches!(label, "Cancel" | "No" | "Dismiss" | "Close")
}

/// Assign a visual role to a dialog button by position: the first button is a
/// neutral escape hatch (Cancel), the final button is the confirm action
/// (destructive when the dialog is destructive), and middle buttons are
/// primary actions.
fn dialog_button_variant(tone: DialogTone, index: usize, len: usize) -> ButtonVariant {
    if index == 0 {
        ButtonVariant::Neutral
    } else if index == len - 1 && tone == DialogTone::Destructive {
        ButtonVariant::Destructive
    } else {
        ButtonVariant::Default
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
            if !current.is_empty() && current.chars().count() + 1 + word.chars().count() > max_width
            {
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

    fn dialog(tone: DialogTone) -> Dialog {
        Dialog::new(
            tone,
            "Are you sure?",
            "This will delete the record.",
            &["Cancel".into(), "Delete".into()],
        )
    }

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

    #[test]
    fn destructive_dialog_variants() {
        let d = dialog(DialogTone::Destructive);
        assert_eq!(d.buttons[0].variant(), ButtonVariant::Neutral);
        assert_eq!(d.buttons[1].variant(), ButtonVariant::Destructive);
    }

    #[test]
    fn default_focus_is_safe_option() {
        let d = dialog(DialogTone::Destructive);
        assert_eq!(d.selected, 0); // Cancel
    }

    #[test]
    fn escape_activates_safe_option() {
        let mut d = dialog(DialogTone::Destructive);
        d.selected = 1;
        let key = KeyEvent::new(KeyCode::Esc, ratatui::crossterm::event::KeyModifiers::NONE);
        assert_eq!(d.handle_key(key), Some(0));
    }

    #[test]
    fn tab_cycles_focus() {
        let mut d = dialog(DialogTone::Destructive);
        d.selected = 1;
        let tab = KeyEvent::new(KeyCode::Tab, ratatui::crossterm::event::KeyModifiers::NONE);
        assert_eq!(d.handle_key(tab), None);
        assert_eq!(d.selected, 0);
    }

    #[test]
    fn accelerator_activates_matching_button() {
        let mut d = Dialog::new(
            DialogTone::Confirm,
            "Allow?",
            "Grant access?",
            &["Deny (n)".into(), "Allow (y)".into()],
        );
        let key = KeyEvent::new(
            KeyCode::Char('y'),
            ratatui::crossterm::event::KeyModifiers::NONE,
        );
        assert_eq!(d.handle_key(key), Some(1));
    }

    #[test]
    fn inactive_button_uses_primary_text() {
        let style = crate::theme::button_style(ButtonVariant::Default, ButtonState::Inactive);
        assert_eq!(style.fg, Some(crate::theme::PRIMARY));
    }
}
