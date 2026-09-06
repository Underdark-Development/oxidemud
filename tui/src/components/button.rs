//! Chip-style button widget.
//!
//! Follows the "Spade UI/UX Design Guide": a ` Label ` chip with one space of
//! padding on each side, styled through `theme::button_style`. Buttons are the
//! shared building block for dialog action rows and inline table action chips.

use ratatui::{buffer::Buffer, layout::Rect};

use crate::theme::{self, ButtonState, ButtonVariant};

/// A chip-style button rendered from the semantic button tokens.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Button {
    label: String,
    variant: ButtonVariant,
}

impl Button {
    /// Create a button with the given label and visual role.
    pub fn new(label: impl Into<String>, variant: ButtonVariant) -> Self {
        Button {
            label: label.into(),
            variant,
        }
    }

    /// The button's label text.
    pub fn label(&self) -> &str {
        &self.label
    }

    /// The button's visual role.
    pub fn variant(&self) -> ButtonVariant {
        self.variant
    }

    /// Number of columns the rendered chip occupies (label plus one space of
    /// padding on each side).
    pub fn width(&self) -> u16 {
        self.label.chars().count() as u16 + 2
    }

    /// Render the chip at `(x, y)` in the given state, returning the occupied
    /// rect (used for hit-testing and layout).
    pub fn render(&self, buf: &mut Buffer, x: u16, y: u16, state: ButtonState) -> Rect {
        let style = theme::button_style(self.variant, state);
        let label = format!(" {} ", self.label);
        let width = label.chars().count() as u16;
        buf.set_string(x, y, &label, style);
        Rect::new(x, y, width, 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::buffer::Buffer;

    #[test]
    fn width_includes_padding() {
        let btn = Button::new("Allow", ButtonVariant::Default);
        assert_eq!(btn.width(), 7); // 5 chars + 2 padding
    }

    #[test]
    fn render_draws_label_with_padding() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 20, 1));
        let btn = Button::new("OK", ButtonVariant::Default);
        let rect = btn.render(&mut buf, 0, 0, ButtonState::Inactive);
        assert_eq!(rect.width, 4);
        assert_eq!(buf[(2, 0)].symbol(), "K");
        assert_eq!(buf[(0, 0)].symbol(), " ");
        assert_eq!(buf[(1, 0)].symbol(), "O");
        assert_eq!(buf[(3, 0)].symbol(), " ");
    }
}
