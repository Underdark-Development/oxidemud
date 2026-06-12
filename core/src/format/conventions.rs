//! Color and style conventions for common MUD display elements.
//!
//! Follows the conventions table in ARCHITECTURE.md (Text Formatting & Color).

use super::color::Color;
use super::rich_text::{Modifier, RichText, Segment};

fn bold() -> Modifier {
    let mut m = Modifier::new();
    m.set(Modifier::BOLD);
    m
}

/// Room name: brightwhite bold.
pub fn room_name(text: impl Into<String>) -> RichText {
    Segment::styled(text, Color::BrightWhite, Color::Default, bold()).into()
}

/// Exits/portals header: cyan.
pub fn exit_dir(text: impl Into<String>) -> RichText {
    Segment::colored(text, Color::Cyan).into()
}

/// Player name as a standalone line: yellow bold.
pub fn player_name(text: impl Into<String>) -> RichText {
    player_name_segment(text).into()
}

/// Player name as a segment for composing into larger messages: yellow bold.
pub fn player_name_segment(text: impl Into<String>) -> Segment {
    Segment::styled(text, Color::Yellow, Color::Default, bold())
}

/// Say text: default formatting.
pub fn say_text(text: impl Into<String>) -> RichText {
    Segment::new(text).into()
}

/// Error message: brightred.
pub fn error(text: impl Into<String>) -> RichText {
    Segment::colored(text, Color::BrightRed).into()
}

/// Emphasized text.
pub fn highlight(text: impl Into<String>) -> RichText {
    Segment::styled(text, Color::White, Color::Default, Modifier::new()).into()
}

/// Visual separator line.
pub fn separator(text: impl Into<String>) -> RichText {
    Segment::styled(text, Color::BrightBlack, Color::Default, Modifier::new()).into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_conventions_room_name() {
        let t = room_name("Tavern");
        assert_eq!(t.segments()[0].fg, Color::BrightWhite);
        assert!(t.segments()[0].modifiers.has(Modifier::BOLD));
        assert_eq!(t.segments()[0].text, "Tavern");
    }

    #[test]
    fn test_conventions_player_name() {
        let t = player_name("Alice");
        assert_eq!(t.segments()[0].fg, Color::Yellow);
        assert!(t.segments()[0].modifiers.has(Modifier::BOLD));
    }

    #[test]
    fn test_conventions_player_name_segment() {
        let s = player_name_segment("Alice");
        assert_eq!(s.fg, Color::Yellow);
        assert!(s.modifiers.has(Modifier::BOLD));
    }

    #[test]
    fn test_conventions_say_text_default() {
        let t = say_text("hello");
        assert_eq!(t.segments()[0].fg, Color::Default);
        assert_eq!(t.segments()[0].modifiers.bits(), 0);
    }

    #[test]
    fn test_conventions_error() {
        let t = error("error");
        assert_eq!(t.segments()[0].fg, Color::BrightRed);
    }

    #[test]
    fn test_conventions_exit_dir() {
        let t = exit_dir("[Exits: n e]");
        assert_eq!(t.segments()[0].fg, Color::Cyan);
    }
}
