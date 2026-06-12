//! Color and style conventions for common MUD display elements.

use super::color::Color;
use super::rich_text::{Modifier, RichText, Segment};

pub fn room_name(text: impl Into<String>) -> RichText {
    Segment::styled(text, Color::Yellow, Color::Default, Modifier::new()).into()
}

pub fn exit_dir(text: impl Into<String>) -> RichText {
    Segment::colored(text, Color::Cyan).into()
}

pub fn player_name(text: impl Into<String>) -> RichText {
    Segment::colored(text, Color::Green).into()
}

pub fn say_text(text: impl Into<String>) -> RichText {
    let mut m = Modifier::new();
    m.set(Modifier::ITALIC);
    Segment::styled(text, Color::Default, Color::Default, m).into()
}

pub fn error(text: impl Into<String>) -> RichText {
    Segment::colored(text, Color::Red).into()
}

pub fn highlight(text: impl Into<String>) -> RichText {
    Segment::styled(text, Color::White, Color::Default, Modifier::new()).into()
}

pub fn separator(text: impl Into<String>) -> RichText {
    Segment::styled(text, Color::BrightBlack, Color::Default, Modifier::new()).into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_conventions_room_name() {
        let t = room_name("Tavern");
        assert_eq!(t.segments()[0].fg, Color::Yellow);
        assert_eq!(t.segments()[0].text, "Tavern");
    }

    #[test]
    fn test_conventions_player_name() {
        let t = player_name("Alice");
        assert_eq!(t.segments()[0].fg, Color::Green);
    }

    #[test]
    fn test_conventions_error() {
        let t = error("error");
        assert_eq!(t.segments()[0].fg, Color::Red);
    }
}
