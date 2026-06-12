/// Standard 16 ANSI colors plus terminal default and 256-color indexed.
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
    /// Extended 256-color palette index.
    Indexed(u8),
}

/// Approximate RGB values for the 16 standard ANSI colors (xterm defaults).
const ANSI_16_RGB: [(Color, (u8, u8, u8)); 16] = [
    (Color::Black, (0, 0, 0)),
    (Color::Red, (205, 0, 0)),
    (Color::Green, (0, 205, 0)),
    (Color::Yellow, (205, 205, 0)),
    (Color::Blue, (0, 0, 238)),
    (Color::Magenta, (205, 0, 205)),
    (Color::Cyan, (0, 205, 205)),
    (Color::White, (229, 229, 229)),
    (Color::BrightBlack, (127, 127, 127)),
    (Color::BrightRed, (255, 0, 0)),
    (Color::BrightGreen, (0, 255, 0)),
    (Color::BrightYellow, (255, 255, 0)),
    (Color::BrightBlue, (92, 92, 255)),
    (Color::BrightMagenta, (255, 0, 255)),
    (Color::BrightCyan, (0, 255, 255)),
    (Color::BrightWhite, (255, 255, 255)),
];

/// Color cube axis levels for indices 16-231 (6x6x6 cube).
const CUBE_LEVELS: [u8; 6] = [0, 95, 135, 175, 215, 255];

fn squared_distance(a: (u8, u8, u8), b: (u8, u8, u8)) -> u32 {
    let dr = i32::from(a.0) - i32::from(b.0);
    let dg = i32::from(a.1) - i32::from(b.1);
    let db = i32::from(a.2) - i32::from(b.2);
    (dr * dr + dg * dg + db * db) as u32
}

fn nearest_16(rgb: (u8, u8, u8)) -> Color {
    ANSI_16_RGB
        .iter()
        .min_by_key(|(_, candidate)| squared_distance(rgb, *candidate))
        .map(|(color, _)| *color)
        .expect("ANSI_16_RGB is non-empty")
}

impl Color {
    /// Nearest 16-color equivalent for basic-ANSI clients.
    ///
    /// Mapping for `Indexed(n)`:
    /// - 0-7: standard colors
    /// - 8-15: bright variants
    /// - 16-231: 6x6x6 cube, nearest of the 16 by RGB distance
    /// - 232-255: grayscale, nearest to Black or White
    ///
    /// Non-indexed colors return themselves.
    pub fn fallback_16(self) -> Self {
        let Color::Indexed(n) = self else {
            return self;
        };
        match n {
            0 => Color::Black,
            1 => Color::Red,
            2 => Color::Green,
            3 => Color::Yellow,
            4 => Color::Blue,
            5 => Color::Magenta,
            6 => Color::Cyan,
            7 => Color::White,
            8 => Color::BrightBlack,
            9 => Color::BrightRed,
            10 => Color::BrightGreen,
            11 => Color::BrightYellow,
            12 => Color::BrightBlue,
            13 => Color::BrightMagenta,
            14 => Color::BrightCyan,
            15 => Color::BrightWhite,
            16..=231 => {
                let i = n - 16;
                let r = CUBE_LEVELS[usize::from(i / 36)];
                let g = CUBE_LEVELS[usize::from((i / 6) % 6)];
                let b = CUBE_LEVELS[usize::from(i % 6)];
                nearest_16((r, g, b))
            }
            232..=255 => {
                let level = 8 + 10 * u16::from(n - 232);
                if level < 128 {
                    Color::Black
                } else {
                    Color::White
                }
            }
        }
    }

    pub(crate) fn fg_code(self) -> &'static str {
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
            // Basic-ANSI rendering: indexed colors degrade to nearest 16.
            // Full 256-color output arrives with terminal capability
            // detection (MTTS) in Phase 6.
            Color::Indexed(_) => self.fallback_16().fg_code(),
        }
    }

    pub(crate) fn bg_code(self) -> &'static str {
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
            Color::Indexed(_) => self.fallback_16().bg_code(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fallback_16_standard_range() {
        assert_eq!(Color::Indexed(0).fallback_16(), Color::Black);
        assert_eq!(Color::Indexed(1).fallback_16(), Color::Red);
        assert_eq!(Color::Indexed(7).fallback_16(), Color::White);
    }

    #[test]
    fn test_fallback_16_bright_range() {
        assert_eq!(Color::Indexed(8).fallback_16(), Color::BrightBlack);
        assert_eq!(Color::Indexed(9).fallback_16(), Color::BrightRed);
        assert_eq!(Color::Indexed(15).fallback_16(), Color::BrightWhite);
    }

    #[test]
    fn test_fallback_16_cube() {
        // 196 = pure red corner of the cube (255, 0, 0)
        assert_eq!(Color::Indexed(196).fallback_16(), Color::BrightRed);
        // 46 = pure green corner (0, 255, 0)
        assert_eq!(Color::Indexed(46).fallback_16(), Color::BrightGreen);
        // 16 = cube black (0, 0, 0)
        assert_eq!(Color::Indexed(16).fallback_16(), Color::Black);
        // 231 = cube white (255, 255, 255)
        assert_eq!(Color::Indexed(231).fallback_16(), Color::BrightWhite);
    }

    #[test]
    fn test_fallback_16_grayscale() {
        // 232 = darkest gray (8) -> Black
        assert_eq!(Color::Indexed(232).fallback_16(), Color::Black);
        // 255 = lightest gray (238) -> White
        assert_eq!(Color::Indexed(255).fallback_16(), Color::White);
        // 243 = gray 118 -> Black, 244 = gray 128 -> White
        assert_eq!(Color::Indexed(243).fallback_16(), Color::Black);
        assert_eq!(Color::Indexed(244).fallback_16(), Color::White);
    }

    #[test]
    fn test_fallback_16_non_indexed_identity() {
        assert_eq!(Color::Red.fallback_16(), Color::Red);
        assert_eq!(Color::Default.fallback_16(), Color::Default);
    }

    #[test]
    fn test_indexed_fg_code_degrades() {
        assert_eq!(Color::Indexed(196).fg_code(), "91");
        assert_eq!(Color::Indexed(232).bg_code(), "40");
    }
}
