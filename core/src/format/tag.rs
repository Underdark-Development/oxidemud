use super::color::Color;
use super::rich_text::{Modifier, RichText, Segment};

pub(crate) fn color_from_name(name: &str) -> Option<Color> {
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

pub(crate) fn modifier_from_name(name: &str) -> Option<u8> {
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
pub fn parse_tags(input: &str) -> RichText {
    let mut text = RichText::new();
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
                    text.push(Segment {
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
                text.push(Segment {
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
        text.push(Segment {
            text: buf,
            fg,
            bg,
            modifiers,
        });
    }

    text
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn test_render_with_parse_full_flow() {
        let t = parse_tags("{red}hello{/} {green}world{/}");
        let rendered = t.render(true, true);
        assert!(rendered.starts_with("\x1b[0;31;49mhello"));
        assert!(rendered.contains("\x1b[0;32;49mworld"));
        assert!(rendered.ends_with("\x1b[0m"));
        assert_eq!(t.plain(), "hello world");
    }
}
