use std::fmt;

/// Standard 16 ANSI colors plus terminal default.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Color {
    Default,
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    White,
    BrightBlack,
    BrightRed,
    BrightGreen,
    BrightYellow,
    BrightBlue,
    BrightMagenta,
    BrightCyan,
    BrightWhite,
}

impl Color {
    fn fg_code(self) -> &'static str {
        match self {
            Color::Default => "39",
            Color::Black => "30",
            Color::Red => "31",
            Color::Green => "32",
            Color::Yellow => "33",
            Color::Blue => "34",
            Color::Magenta => "35",
            Color::Cyan => "36",
            Color::White => "37",
            Color::BrightBlack => "90",
            Color::BrightRed => "91",
            Color::BrightGreen => "92",
            Color::BrightYellow => "93",
            Color::BrightBlue => "94",
            Color::BrightMagenta => "95",
            Color::BrightCyan => "96",
            Color::BrightWhite => "97",
        }
    }

    fn bg_code(self) -> &'static str {
        match self {
            Color::Default => "49",
            Color::Black => "40",
            Color::Red => "41",
            Color::Green => "42",
            Color::Yellow => "43",
            Color::Blue => "44",
            Color::Magenta => "45",
            Color::Cyan => "46",
            Color::White => "47",
            Color::BrightBlack => "100",
            Color::BrightRed => "101",
            Color::BrightGreen => "102",
            Color::BrightYellow => "103",
            Color::BrightBlue => "104",
            Color::BrightMagenta => "105",
            Color::BrightCyan => "106",
            Color::BrightWhite => "107",
        }
    }
}

/// Bitmask of text modifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Modifier(u8);

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

    fn codes(self) -> Vec<&'static str> {
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
        if self.has(Self::BLINK) {
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
        let codes = self.codes();
        write!(f, "{}", codes.join(";"))
    }
}

/// A styled segment of text.
#[derive(Debug, Clone)]
pub struct StyledText {
    pub text: String,
    pub fg: Color,
    pub bg: Color,
    pub modifiers: Modifier,
}

impl StyledText {
    pub fn new(text: impl Into<String>) -> Self {
        StyledText {
            text: text.into(),
            fg: Color::Default,
            bg: Color::Default,
            modifiers: Modifier::new(),
        }
    }

    pub fn styled(text: impl Into<String>, fg: Color, bg: Color, modifiers: Modifier) -> Self {
        StyledText {
            text: text.into(),
            fg,
            bg,
            modifiers,
        }
    }

    pub fn colored(text: impl Into<String>, fg: Color) -> Self {
        StyledText {
            text: text.into(),
            fg,
            bg: Color::Default,
            modifiers: Modifier::new(),
        }
    }
}

/// Ordered sequence of styled text segments.
#[derive(Debug, Clone)]
pub struct Text(Vec<StyledText>);

impl Text {
    pub fn new() -> Self {
        Text(Vec::new())
    }

    pub fn push(&mut self, segment: StyledText) {
        self.0.push(segment);
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn segments(&self) -> &[StyledText] {
        &self.0
    }

    pub fn into_segments(self) -> Vec<StyledText> {
        self.0
    }

    pub fn plain(self) -> String {
        self.0.into_iter().map(|s| s.text).collect()
    }
}

impl Default for Text {
    fn default() -> Self {
        Self::new()
    }
}

impl From<String> for Text {
    fn from(s: String) -> Self {
        Text(vec![StyledText::new(s)])
    }
}

impl From<&str> for Text {
    fn from(s: &str) -> Self {
        Text(vec![StyledText::new(s)])
    }
}

impl From<StyledText> for Text {
    fn from(seg: StyledText) -> Self {
        Text(vec![seg])
    }
}

/// Render formatted text to an ANSI-escaped string.
pub fn render(text: &Text) -> String {
    if text.is_empty() {
        return String::new();
    }

    let mut out = String::new();
    for seg in text.segments() {
        let mut params: Vec<&str> = Vec::new();

        params.push("0");
        params.extend(seg.modifiers.codes());
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

fn color_from_name(name: &str) -> Option<Color> {
    match name.to_lowercase().as_str() {
        "default" => Some(Color::Default),
        "black" => Some(Color::Black),
        "red" => Some(Color::Red),
        "green" => Some(Color::Green),
        "yellow" => Some(Color::Yellow),
        "blue" => Some(Color::Blue),
        "magenta" => Some(Color::Magenta),
        "cyan" => Some(Color::Cyan),
        "white" => Some(Color::White),
        "bright-black" | "grey" | "gray" => Some(Color::BrightBlack),
        "bright-red" => Some(Color::BrightRed),
        "bright-green" => Some(Color::BrightGreen),
        "bright-yellow" => Some(Color::BrightYellow),
        "bright-blue" => Some(Color::BrightBlue),
        "bright-magenta" => Some(Color::BrightMagenta),
        "bright-cyan" => Some(Color::BrightCyan),
        "bright-white" => Some(Color::BrightWhite),
        _ => None,
    }
}

fn modifier_from_name(name: &str) -> Option<u8> {
    match name.to_lowercase().as_str() {
        "bold" => Some(Modifier::BOLD),
        "dim" => Some(Modifier::DIM),
        "italic" => Some(Modifier::ITALIC),
        "underline" => Some(Modifier::UNDERLINE),
        "blink" => Some(Modifier::BLINK),
        "reverse" => Some(Modifier::REVERSE),
        "hidden" => Some(Modifier::HIDDEN),
        "strike" => Some(Modifier::STRIKE),
        _ => None,
    }
}

/// Parse markup tags into formatted text.
///
/// Supported tags:
/// - `{color}` — set foreground color (`red`, `green`, `blue`, `bright-red`, etc.)
/// - `{bg:color}` — set background color
/// - `{modifier}` — set modifier (`bold`, `italic`, `underline`, etc.)
/// - `{/}` — reset all formatting
/// - `{/modifier}` — clear a specific modifier
/// - `{{` — literal `{`
pub fn parse_tags(input: &str) -> Text {
    let mut text = Text::new();
    let mut fg = Color::Default;
    let mut bg = Color::Default;
    let mut modifiers = Modifier::new();
    let mut buf = String::new();
    let mut chars = input.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '{' {
            if chars.peek() == Some(&'{') {
                chars.next();
                buf.push('{');
                continue;
            }
            if chars.peek() == Some(&'/') {
                chars.next();
                let mut tag = String::new();
                for c in chars.by_ref() {
                    if c == '}' {
                        break;
                    }
                    tag.push(c);
                }
                if !buf.is_empty() {
                    text.push(StyledText {
                        text: buf.clone(),
                        fg,
                        bg,
                        modifiers,
                    });
                    buf.clear();
                }
                if tag.is_empty() {
                    fg = Color::Default;
                    bg = Color::Default;
                    modifiers = Modifier::new();
                } else if let Some(bits) = modifier_from_name(&tag) {
                    modifiers.remove(bits);
                }
                continue;
            }
            let mut tag = String::new();
            for c in chars.by_ref() {
                if c == '}' {
                    break;
                }
                tag.push(c);
            }
            if !buf.is_empty() {
                text.push(StyledText {
                    text: buf.clone(),
                    fg,
                    bg,
                    modifiers,
                });
                buf.clear();
            }
            if let Some(color) = color_from_name(&tag) {
                fg = color;
            } else if let Some(bits) = modifier_from_name(&tag) {
                modifiers.set(bits);
            } else if let Some(bg_tag) = tag.strip_prefix("bg:") {
                if let Some(color) = color_from_name(bg_tag) {
                    bg = color;
                }
            }
            continue;
        }
        buf.push(ch);
    }

    if !buf.is_empty() {
        text.push(StyledText {
            text: buf,
            fg,
            bg,
            modifiers,
        });
    }

    text
}

/// Color and style conventions for common MUD display elements.
pub mod conventions {
    use super::*;

    pub fn room_name(text: impl Into<String>) -> Text {
        StyledText::styled(text, Color::Yellow, Color::Default, Modifier::new()).into()
    }

    pub fn exit_dir(text: impl Into<String>) -> Text {
        StyledText::colored(text, Color::Cyan).into()
    }

    pub fn player_name(text: impl Into<String>) -> Text {
        StyledText::colored(text, Color::Green).into()
    }

    pub fn say_text(text: impl Into<String>) -> Text {
        let mut m = Modifier::new();
        m.set(Modifier::ITALIC);
        StyledText::styled(text, Color::Default, Color::Default, m).into()
    }

    pub fn error(text: impl Into<String>) -> Text {
        StyledText::colored(text, Color::Red).into()
    }

    pub fn highlight(text: impl Into<String>) -> Text {
        StyledText::styled(text, Color::White, Color::Default, Modifier::new()).into()
    }

    pub fn separator(text: impl Into<String>) -> Text {
        StyledText::styled(text, Color::BrightBlack, Color::Default, Modifier::new()).into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_plain_text() {
        let t = Text::from("hello");
        assert_eq!(render(&t), "\x1b[0;39;49mhello\x1b[0m");
    }

    #[test]
    fn test_render_colored() {
        let t = Text::from(StyledText::colored("red", Color::Red));
        assert_eq!(render(&t), "\x1b[0;31;49mred\x1b[0m");
    }

    #[test]
    fn test_render_bold_colored() {
        let mut m = Modifier::new();
        m.set(Modifier::BOLD);
        let t = Text::from(StyledText::styled("bold red", Color::Red, Color::Default, m));
        assert_eq!(render(&t), "\x1b[0;1;31;49mbold red\x1b[0m");
    }

    #[test]
    fn test_render_multiple_segments() {
        let mut t = Text::new();
        t.push(StyledText::colored("hello", Color::Green));
        t.push(StyledText::new(" world"));
        let result = render(&t);
        assert!(result.contains("\x1b[0;32;49mhello"));
        assert!(result.contains("\x1b[0;39;49m world"));
    }

    #[test]
    fn test_render_empty_text() {
        let t = Text::new();
        assert_eq!(render(&t), "");
    }

    #[test]
    fn test_parse_tags_plain() {
        let t = parse_tags("hello world");
        assert_eq!(t.segments().len(), 1);
        assert_eq!(t.segments()[0].text, "hello world");
    }

    #[test]
    fn test_parse_tags_color() {
        let t = parse_tags("{red}hello");
        assert_eq!(t.segments().len(), 1);
        assert_eq!(t.segments()[0].fg, Color::Red);
        assert_eq!(t.segments()[0].text, "hello");
    }

    #[test]
    fn test_parse_tags_reset() {
        let t = parse_tags("{red}hello{/} world");
        assert_eq!(t.segments().len(), 2);
        assert_eq!(t.segments()[0].fg, Color::Red);
        assert_eq!(t.segments()[0].text, "hello");
        assert_eq!(t.segments()[1].fg, Color::Default);
        assert_eq!(t.segments()[1].text, " world");
    }

    #[test]
    fn test_parse_tags_modifier() {
        let t = parse_tags("{bold}hello");
        assert_eq!(t.segments().len(), 1);
        assert!(t.segments()[0].modifiers.has(Modifier::BOLD));
        assert_eq!(t.segments()[0].text, "hello");
    }

    #[test]
    fn test_parse_tags_bg() {
        let t = parse_tags("{bg:red}hello");
        assert_eq!(t.segments().len(), 1);
        assert_eq!(t.segments()[0].bg, Color::Red);
    }

    #[test]
    fn test_parse_tags_nested() {
        let t = parse_tags("{red}{bold}hello{/} world");
        assert_eq!(t.segments().len(), 2);
        assert_eq!(t.segments()[0].fg, Color::Red);
        assert!(t.segments()[0].modifiers.has(Modifier::BOLD));
        assert_eq!(t.segments()[0].text, "hello");
        assert_eq!(t.segments()[1].fg, Color::Default);
        assert!(!t.segments()[1].modifiers.has(Modifier::BOLD));
        assert_eq!(t.segments()[1].text, " world");
    }

    #[test]
    fn test_parse_tags_clear_modifier() {
        let t = parse_tags("{bold}{italic}hello{/italic} world");
        assert_eq!(t.segments().len(), 2);
        assert!(t.segments()[0].modifiers.has(Modifier::BOLD));
        assert!(t.segments()[0].modifiers.has(Modifier::ITALIC));
        assert!(t.segments()[1].modifiers.has(Modifier::BOLD));
        assert!(!t.segments()[1].modifiers.has(Modifier::ITALIC));
    }

    #[test]
    fn test_parse_tags_escaped_brace() {
        let t = parse_tags("{{hello");
        assert_eq!(t.segments().len(), 1);
        assert_eq!(t.segments()[0].text, "{hello");
    }

    #[test]
    fn test_parse_tags_empty() {
        let t = parse_tags("");
        assert!(t.is_empty());
    }

    #[test]
    fn test_parse_tags_unknown_tag() {
        let t = parse_tags("{unknown}hello");
        assert_eq!(t.segments().len(), 1);
        assert_eq!(t.segments()[0].text, "hello");
        assert_eq!(t.segments()[0].fg, Color::Default);
    }

    #[test]
    fn test_text_plain() {
        let mut t = Text::new();
        t.push(StyledText::colored("hello", Color::Red));
        t.push(StyledText::new(" world"));
        assert_eq!(t.plain(), "hello world");
    }

    #[test]
    fn test_conventions_room_name() {
        let t = conventions::room_name("Tavern");
        assert_eq!(t.segments()[0].fg, Color::Yellow);
        assert_eq!(t.segments()[0].text, "Tavern");
    }

    #[test]
    fn test_conventions_player_name() {
        let t = conventions::player_name("Alice");
        assert_eq!(t.segments()[0].fg, Color::Green);
    }

    #[test]
    fn test_conventions_error() {
        let t = conventions::error("error");
        assert_eq!(t.segments()[0].fg, Color::Red);
    }

    #[test]
    fn test_render_with_parse_full_flow() {
        let t = parse_tags("{red}hello{/} {green}world{/}");
        let rendered = render(&t);
        assert!(rendered.starts_with("\x1b[0;31;49mhello"));
        assert!(rendered.contains("\x1b[0;32;49mworld"));
        assert!(rendered.ends_with("\x1b[0m"));
        assert_eq!(t.plain(), "hello world");
    }

    #[test]
    fn test_render_color_is_red() {
        let t = Text::from(StyledText::colored("test", Color::Red));
        let r = render(&t);
        assert!(r.starts_with("\x1b[0;31;49m"));
        assert!(r.ends_with("\x1b[0m"));
    }
}
