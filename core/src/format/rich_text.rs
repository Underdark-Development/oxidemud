use std::fmt;

use super::color::Color;

/// Bitmask of text modifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Modifier(u8);

impl Default for Modifier {
    fn default() -> Self {
        Self::new()
    }
}

impl Modifier {
    pub const BOLD: u8 = 0b0000_0001;
    pub const DIM: u8 = 0b0000_0010;
    pub const ITALIC: u8 = 0b0000_0100;
    pub const UNDERLINE: u8 = 0b0000_1000;
    pub const BLINK: u8 = 0b0001_0000;
    pub const REVERSE: u8 = 0b0010_0000;
    pub const HIDDEN: u8 = 0b0100_0000;
    pub const STRIKE: u8 = 0b1000_0000;

    pub const fn new() -> Self {
        Modifier(0)
    }

    pub fn set(&mut self, bits: u8) {
        self.0 |= bits;
    }

    pub fn remove(&mut self, bits: u8) {
        self.0 &= !bits;
    }

    pub fn has(self, bits: u8) -> bool {
        self.0 & bits != 0
    }

    pub fn bits(self) -> u8 {
        self.0
    }

    fn codes(self, allow_blink: bool) -> Vec<&'static str> {
        let mut v = Vec::new();
        if self.has(Self::BOLD) {
            v.push("1");
        }
        if self.has(Self::DIM) {
            v.push("2");
        }
        if self.has(Self::ITALIC) {
            v.push("3");
        }
        if self.has(Self::UNDERLINE) {
            v.push("4");
        }
        if allow_blink && self.has(Self::BLINK) {
            v.push("5");
        }
        if self.has(Self::REVERSE) {
            v.push("7");
        }
        if self.has(Self::HIDDEN) {
            v.push("8");
        }
        if self.has(Self::STRIKE) {
            v.push("9");
        }
        v
    }
}

impl fmt::Display for Modifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let codes = self.codes(true);
        write!(f, "{}", codes.join(";"))
    }
}

/// A styled segment of text.
#[derive(Debug, Clone)]
pub struct Segment {
    pub text: String,
    pub fg: Color,
    pub bg: Color,
    pub modifiers: Modifier,
}

impl Segment {
    pub fn new(text: impl Into<String>) -> Self {
        Segment {
            text: text.into(),
            fg: Color::Default,
            bg: Color::Default,
            modifiers: Modifier::new(),
        }
    }

    pub fn styled(text: impl Into<String>, fg: Color, bg: Color, modifiers: Modifier) -> Self {
        Segment {
            text: text.into(),
            fg,
            bg,
            modifiers,
        }
    }

    pub fn colored(text: impl Into<String>, fg: Color) -> Self {
        Segment {
            text: text.into(),
            fg,
            bg: Color::Default,
            modifiers: Modifier::new(),
        }
    }
}

/// Ordered sequence of styled text segments.
#[derive(Debug, Clone)]
pub struct RichText(Vec<Segment>);

impl RichText {
    pub fn new() -> Self {
        RichText(Vec::new())
    }

    pub fn push(&mut self, segment: Segment) {
        self.0.push(segment);
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn segments(&self) -> &[Segment] {
        &self.0
    }

    pub fn into_segments(self) -> Vec<Segment> {
        self.0
    }

    pub fn plain(self) -> String {
        self.0.into_iter().map(|s| s.text).collect()
    }

    pub fn as_plain(&self) -> String {
        self.0.iter().map(|s| s.text.as_str()).collect()
    }

    /// Render to an ANSI-escaped string if `ansi` is true, else plain text.
    /// `allow_blink` gates blink output (client capability or user preference).
    pub fn render(&self, ansi: bool, allow_blink: bool) -> String {
        if !ansi {
            return self.as_plain();
        }
        if self.is_empty() {
            return String::new();
        }

        let mut out = String::new();
        for seg in self.segments() {
            let mut params: Vec<&str> = Vec::new();

            params.push("0");
            params.extend(seg.modifiers.codes(allow_blink));
            params.push(seg.fg.fg_code());
            params.push(seg.bg.bg_code());

            out.push_str("\x1b[");
            out.push_str(&params.join(";"));
            out.push('m');
            out.push_str(&seg.text);
        }
        out.push_str("\x1b[0m");
        out
    }
}

impl Default for RichText {
    fn default() -> Self {
        Self::new()
    }
}

impl From<String> for RichText {
    fn from(s: String) -> Self {
        RichText(vec![Segment::new(s)])
    }
}

impl From<&str> for RichText {
    fn from(s: &str) -> Self {
        RichText(vec![Segment::new(s)])
    }
}

impl From<Segment> for RichText {
    fn from(seg: Segment) -> Self {
        RichText(vec![seg])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_plain_text() {
        let t = RichText::from("hello");
        assert_eq!(t.render(true, true), "\x1b[0;39;49mhello\x1b[0m");
    }

    #[test]
    fn test_render_colored() {
        let t = RichText::from(Segment::colored("red", Color::Red));
        assert_eq!(t.render(true, true), "\x1b[0;31;49mred\x1b[0m");
    }

    #[test]
    fn test_render_bold_colored() {
        let mut m = Modifier::new();
        m.set(Modifier::BOLD);
        let t = RichText::from(Segment::styled("bold red", Color::Red, Color::Default, m));
        assert_eq!(t.render(true, true), "\x1b[0;1;31;49mbold red\x1b[0m");
    }

    #[test]
    fn test_render_multiple_segments() {
        let mut t = RichText::new();
        t.push(Segment::colored("hello", Color::Green));
        t.push(Segment::new(" world"));
        let result = t.render(true, true);
        assert!(result.contains("\x1b[0;32;49mhello"));
        assert!(result.contains("\x1b[0;39;49m world"));
    }

    #[test]
    fn test_render_empty_text() {
        let t = RichText::new();
        assert_eq!(t.render(true, true), "");
    }

    #[test]
    fn test_render_no_ansi_returns_plain() {
        let t = RichText::from(Segment::colored("plain", Color::Red));
        assert_eq!(t.render(false, true), "plain");
    }

    #[test]
    fn test_render_blink_gated() {
        let mut m = Modifier::new();
        m.set(Modifier::BLINK);
        let t = RichText::from(Segment::styled("alert", Color::Default, Color::Default, m));
        assert_eq!(t.render(true, true), "\x1b[0;5;39;49malert\x1b[0m");
        assert_eq!(t.render(true, false), "\x1b[0;39;49malert\x1b[0m");
    }

    #[test]
    fn test_text_plain() {
        let mut t = RichText::new();
        t.push(Segment::colored("hello", Color::Red));
        t.push(Segment::new(" world"));
        assert_eq!(t.plain(), "hello world");
    }

    #[test]
    fn test_render_color_is_red() {
        let t = RichText::from(Segment::colored("test", Color::Red));
        let r = t.render(true, true);
        assert!(r.starts_with("\x1b[0;31;49m"));
        assert!(r.ends_with("\x1b[0m"));
    }
}
