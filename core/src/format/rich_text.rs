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

    /// Push a plain text segment.
    pub fn push_str(&mut self, s: impl Into<String>) {
        self.0.push(Segment::new(s));
    }

    /// Append all segments from another `RichText`.
    pub fn extend(&mut self, other: RichText) {
        self.0.extend(other.0);
    }

    /// Append segments from an iterator.
    pub fn extend_iter(&mut self, iter: impl IntoIterator<Item = Segment>) {
        self.0.extend(iter);
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

    /// Render with word-wrapping at `width` columns.
    ///
    /// If `width` is 0, delegates to [`render`](Self::render) (no wrapping).
    /// Wrapping preserves per-segment styling across line breaks.
    /// Words longer than `width` overflow onto the current line (no hyphenation).
    pub fn render_wrapped(&self, width: usize, ansi: bool, allow_blink: bool) -> String {
        if width == 0 {
            return self.render(ansi, allow_blink);
        }
        if self.is_empty() {
            return String::new();
        }

        fn seg_ansi(seg: &Segment, blink: bool) -> String {
            let mut params: Vec<&str> = Vec::new();
            params.push("0");
            params.extend(seg.modifiers.codes(blink));
            params.push(seg.fg.fg_code());
            params.push(seg.bg.bg_code());
            format!("\x1b[{}m", params.join(";"))
        }

        let mut out = String::new();
        let mut line_width: usize = 0;

        for seg in self.segments() {
            let ansi_on = if ansi {
                Some(seg_ansi(seg, allow_blink))
            } else {
                None
            };

            if let Some(ref a) = ansi_on {
                out.push_str(a);
            }

            let mut word = String::new();

            for ch in seg.text.chars() {
                match ch {
                    '\n' => {
                        if !word.is_empty() {
                            out.push_str(&word);
                            word.clear();
                        }
                        if let Some(ref a) = ansi_on {
                            out.push_str("\x1b[0m\n");
                            out.push_str(a);
                        } else {
                            out.push('\n');
                        }
                        line_width = 0;
                    }
                    ' ' => {
                        // flush accumulated word
                        if !word.is_empty() {
                            if line_width + word.len() > width {
                                if let Some(ref a) = ansi_on {
                                    out.push_str("\x1b[0m\n");
                                    out.push_str(a);
                                } else {
                                    out.push('\n');
                                }
                                line_width = 0;
                            }
                            out.push_str(&word);
                            line_width += word.len();
                            word.clear();
                        }
                        // add space if it fits
                        if line_width < width {
                            out.push(' ');
                            line_width += 1;
                        }
                    }
                    _ => {
                        word.push(ch);
                    }
                }
            }

            // flush remaining word
            if !word.is_empty() {
                if line_width + word.len() > width {
                    if let Some(ref a) = ansi_on {
                        out.push_str("\x1b[0m\n");
                        out.push_str(a);
                    } else {
                        out.push('\n');
                    }
                    line_width = 0;
                }
                out.push_str(&word);
                line_width += word.len();
            }
        }

        if ansi {
            out.push_str("\x1b[0m");
        }
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

    // ── render_wrapped tests ──

    #[test]
    fn test_render_wrapped_width_zero() {
        let t = RichText::from(Segment::colored("hello world", Color::Red));
        // width=0 means no wrapping
        let r = t.render_wrapped(0, true, true);
        assert_eq!(r, "\x1b[0;31;49mhello world\x1b[0m");
    }

    #[test]
    fn test_render_wrapped_no_ansi() {
        let t = RichText::from(Segment::colored("hello world", Color::Red));
        let r = t.render_wrapped(80, false, true);
        assert_eq!(r, "hello world");
    }

    #[test]
    fn test_render_wrapped_shorter_than_width() {
        let t = RichText::from("hello");
        let r = t.render_wrapped(80, true, true);
        assert_eq!(r, "\x1b[0;39;49mhello\x1b[0m");
    }

    #[test]
    fn test_render_wrapped_word_boundary() {
        let t = RichText::from("one two three");
        // width=8: "one two " (8) fits, "three" wraps (space added before word known to overflow)
        let r = t.render_wrapped(8, true, true);
        assert_eq!(r, "\x1b[0;39;49mone two \x1b[0m\n\x1b[0;39;49mthree\x1b[0m");
    }

    #[test]
    fn test_render_wrapped_multi_segment() {
        let mut t = RichText::new();
        t.push(Segment::colored("hello ", Color::Red));
        t.push(Segment::colored("world", Color::Green));
        // width=10: "hello " (6) + "wor" fits on one line, "ld" wraps
        let r = t.render_wrapped(10, true, true);
        assert!(r.contains("\x1b[0;31;49mhello "));
        assert!(r.contains("\x1b[0;32;49mworld"));
        assert!(r.ends_with("\x1b[0m"));
    }

    #[test]
    fn test_render_wrapped_newlines() {
        let t = RichText::from("hello\nworld");
        let r = t.render_wrapped(80, true, true);
        assert_eq!(r, "\x1b[0;39;49mhello\x1b[0m\n\x1b[0;39;49mworld\x1b[0m");
    }

    #[test]
    fn test_render_wrapped_long_word_overflow() {
        // A word longer than width should overflow (no hyphenation)
        let t = RichText::from("a antidisestablishment");
        let r = t.render_wrapped(10, true, true);
        // "a " fits on first line, "antidisestablishment" is longer than 10
        assert!(r.contains("antidisestablishment"));
    }

    #[test]
    fn test_render_wrapped_skip_space_at_line_start() {
        let t = RichText::from("hello magnificent world");
        // width=8: "hello" + space = 6, then wrap before "magnificent"
        let r = t.render_wrapped(8, true, true);
        // "magnificent" overflows 8, then "world" wraps to its own line
        assert!(r.contains("hello"));
        assert!(r.contains("magnificent"));
        assert!(r.contains("world"));
        // No leading space on wrapped lines
        assert!(!r.contains("\nmagnificent")); // wouldn't have leading space
    }

    #[test]
    fn test_render_wrapped_empty() {
        let t = RichText::new();
        assert_eq!(t.render_wrapped(80, true, true), "");
        assert_eq!(t.render_wrapped(0, true, true), "");
    }
}
