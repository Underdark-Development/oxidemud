//! Semantic design tokens and style helpers for the spade TUI.
//!
//! This module is the single source of truth for the spade visual language
//! (see the "Spade UI/UX Design Guide" section of `ARCHITECTURE.md`). Components
//! and screens must reference these tokens and helpers instead of raw colors so
//! the palette and styling stay consistent across the application.
//!
//! The palette intentionally mirrors spade's established 256-indexed look; the
//! interaction style (chip buttons, inline bold titles, filled selection,
//! rounded borders, right-aligned dialog buttons) follows the opencode TUI.

use ratatui::style::{Color, Modifier, Style};

// ---------------------------------------------------------------------------
// Color tokens
// ---------------------------------------------------------------------------

/// Main canvas background (the surface screens draw on).
pub const BG: Color = Color::Black;
/// Panel/chrome surface (menu bar, sidebar, status bar, overlay interiors).
pub const PANEL: Color = Color::Indexed(236);
/// Deepest surface, used as a darker base (e.g. dashboard gauges).
pub const BG_DARK: Color = Color::Indexed(235);
/// Elevated surface (headers, table rows, overlay rows).
pub const SURFACE: Color = Color::Indexed(238);

/// Primary foreground text.
pub const FG: Color = Color::White;
/// Secondary/muted foreground text.
pub const FG_MUTED: Color = Color::Indexed(245);
/// De-emphasized foreground text (e.g. dimmed overlay content).
pub const FG_FAINT: Color = Color::Indexed(242);
/// Emphasized foreground text.
pub const FG_BRIGHT: Color = Color::Indexed(250);

/// Primary accent: focus, selection fill, active borders, prompt, links.
pub const PRIMARY: Color = Color::Cyan;
/// Destructive actions, errors, validation failures.
pub const DANGER: Color = Color::Red;
/// Risk-tinted surface background (e.g. rows with validation errors).
pub const DANGER_BG: Color = Color::Indexed(52);
/// Warnings and transient status messages.
pub const WARNING: Color = Color::Yellow;
/// Success states and connected indicators.
pub const POSITIVE: Color = Color::Green;

/// Hover / dim highlight background.
pub const HOVER: Color = Color::Indexed(240);

/// Normal (idle) border color.
pub const BORDER: Color = FG_MUTED;
/// Focused border color.
pub const BORDER_FOCUS: Color = PRIMARY;

// ---------------------------------------------------------------------------
// Style helpers
// ---------------------------------------------------------------------------

/// Standard panel style: foreground text on the panel surface.
pub fn panel_style() -> Style {
    Style::default().fg(FG).bg(PANEL)
}

/// Canvas style: foreground text on the main background.
pub fn canvas_style() -> Style {
    Style::default().fg(FG).bg(BG)
}

/// Primary text style.
pub fn text() -> Style {
    Style::default().fg(FG)
}

/// Muted text style.
pub fn text_muted() -> Style {
    Style::default().fg(FG_MUTED)
}

/// Faint (de-emphasized) text style.
pub fn text_faint() -> Style {
    Style::default().fg(FG_FAINT)
}

/// Bright/emphasized text style.
pub fn text_bright() -> Style {
    Style::default().fg(FG_BRIGHT)
}

/// Hover highlight row background.
pub fn hovered_row() -> Style {
    Style::default().bg(HOVER)
}

/// Focused/selected row treatment: dark text on the primary fill.
pub fn selected_row() -> Style {
    Style::default()
        .fg(BG)
        .bg(PRIMARY)
        .add_modifier(Modifier::BOLD)
}

/// Accent (focused) border style.
pub fn border_accent() -> Style {
    Style::default().fg(BORDER_FOCUS)
}

/// Muted (idle) border style.
pub fn border_muted() -> Style {
    Style::default().fg(BORDER)
}

/// Destructive border style.
pub fn border_danger() -> Style {
    Style::default().fg(DANGER)
}

/// Command prompt marker style: bold primary glyph (e.g. `>`).
pub fn prompt_style() -> Style {
    Style::default().fg(PRIMARY).add_modifier(Modifier::BOLD)
}

// ---------------------------------------------------------------------------
// Buttons
// ---------------------------------------------------------------------------

/// Visual role of a button.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonVariant {
    /// Primary action (confirm, submit).
    Default,
    /// Destructive confirmation (delete, quit without saving).
    Destructive,
    /// Non-emphasized action (cancel, dismiss).
    Neutral,
    /// Warning-tinted action (e.g. clear).
    Warning,
    /// Unavailable action.
    Disabled,
}

/// Focus state of a button.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonState {
    /// Focused/hovered/selected; rendered as a filled chip.
    Active,
    /// Idle; rendered as an outlined chip.
    Inactive,
}

/// Style a button chip for the given variant and state.
pub fn button_style(variant: ButtonVariant, state: ButtonState) -> Style {
    match variant {
        ButtonVariant::Default => match state {
            ButtonState::Active => Style::default()
                .fg(BG)
                .bg(PRIMARY)
                .add_modifier(Modifier::BOLD),
            ButtonState::Inactive => Style::default().fg(PRIMARY).bg(BG),
        },
        ButtonVariant::Destructive => match state {
            ButtonState::Active => Style::default()
                .fg(FG)
                .bg(DANGER)
                .add_modifier(Modifier::BOLD),
            ButtonState::Inactive => Style::default().fg(DANGER).bg(BG),
        },
        ButtonVariant::Neutral => match state {
            ButtonState::Active => Style::default()
                .fg(BG)
                .bg(FG_MUTED)
                .add_modifier(Modifier::BOLD),
            ButtonState::Inactive => Style::default().fg(FG_MUTED).bg(BG),
        },
        ButtonVariant::Warning => match state {
            ButtonState::Active => Style::default()
                .fg(BG)
                .bg(WARNING)
                .add_modifier(Modifier::BOLD),
            ButtonState::Inactive => Style::default().fg(WARNING).bg(BG),
        },
        ButtonVariant::Disabled => Style::default().fg(FG_FAINT).bg(BG),
    }
}

// ---------------------------------------------------------------------------
// Inline chips
// ---------------------------------------------------------------------------

/// Foreground color for a button variant (used by chip outlines).
pub fn variant_fg(variant: ButtonVariant) -> Color {
    match variant {
        ButtonVariant::Default | ButtonVariant::Disabled => PRIMARY,
        ButtonVariant::Destructive => DANGER,
        ButtonVariant::Neutral => FG_MUTED,
        ButtonVariant::Warning => WARNING,
    }
}

/// Style an inline table chip: kind-colored text on a subtle surface shell.
/// When `emphasized` is true (e.g. Add Entry CTA), the label is bold.
pub fn chip_style(variant: ButtonVariant, emphasized: bool) -> Style {
    let mut style = Style::default().fg(variant_fg(variant)).bg(SURFACE);
    if emphasized {
        style = style.add_modifier(Modifier::BOLD);
    }
    style
}

// ---------------------------------------------------------------------------
// Dialogs
// ---------------------------------------------------------------------------

/// Emotional/severity intent of a dialog, carried by its border color.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DialogTone {
    /// Informational (e.g. notification history).
    Info,
    /// Non-destructive confirmation.
    Confirm,
    /// Destructive confirmation (e.g. delete, quit with unsaved changes).
    Destructive,
}

/// Border color for a dialog's tone.
pub fn dialog_border_color(tone: DialogTone) -> Color {
    match tone {
        DialogTone::Info | DialogTone::Confirm => BORDER,
        DialogTone::Destructive => DANGER,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokens_match_established_palette() {
        assert_eq!(BG, Color::Black);
        assert_eq!(PANEL, Color::Indexed(236));
        assert_eq!(BG_DARK, Color::Indexed(235));
        assert_eq!(SURFACE, Color::Indexed(238));
        assert_eq!(FG, Color::White);
        assert_eq!(FG_MUTED, Color::Indexed(245));
        assert_eq!(FG_FAINT, Color::Indexed(242));
        assert_eq!(FG_BRIGHT, Color::Indexed(250));
        assert_eq!(PRIMARY, Color::Cyan);
        assert_eq!(DANGER, Color::Red);
        assert_eq!(DANGER_BG, Color::Indexed(52));
        assert_eq!(WARNING, Color::Yellow);
        assert_eq!(POSITIVE, Color::Green);
        assert_eq!(HOVER, Color::Indexed(240));
        assert_eq!(BORDER, FG_MUTED);
        assert_eq!(BORDER_FOCUS, PRIMARY);
    }

    #[test]
    fn button_style_fills_only_active_state() {
        let cases = [
            (ButtonVariant::Default, ButtonState::Active, BG, PRIMARY),
            (ButtonVariant::Default, ButtonState::Inactive, PRIMARY, BG),
            (ButtonVariant::Destructive, ButtonState::Active, FG, DANGER),
            (
                ButtonVariant::Destructive,
                ButtonState::Inactive,
                DANGER,
                BG,
            ),
            (ButtonVariant::Neutral, ButtonState::Active, BG, FG_MUTED),
            (ButtonVariant::Neutral, ButtonState::Inactive, FG_MUTED, BG),
            (ButtonVariant::Warning, ButtonState::Active, BG, WARNING),
            (ButtonVariant::Warning, ButtonState::Inactive, WARNING, BG),
        ];
        for (variant, state, expect_fg, expect_bg) in cases {
            let style = button_style(variant, state);
            assert_eq!(style.fg, Some(expect_fg), "{variant:?} {state:?}");
            assert_eq!(style.bg, Some(expect_bg), "{variant:?} {state:?}");
        }
    }

    #[test]
    fn button_style_bolds_active_state() {
        for variant in [
            ButtonVariant::Default,
            ButtonVariant::Destructive,
            ButtonVariant::Neutral,
            ButtonVariant::Warning,
        ] {
            assert!(button_style(variant, ButtonState::Active)
                .add_modifier
                .contains(Modifier::BOLD));
            assert!(!button_style(variant, ButtonState::Inactive)
                .add_modifier
                .contains(Modifier::BOLD));
        }
    }

    #[test]
    fn disabled_button_ignores_state() {
        assert_eq!(
            button_style(ButtonVariant::Disabled, ButtonState::Active),
            button_style(ButtonVariant::Disabled, ButtonState::Inactive)
        );
    }

    #[test]
    fn dialog_tone_border_mapping() {
        assert_eq!(dialog_border_color(DialogTone::Info), BORDER);
        assert_eq!(dialog_border_color(DialogTone::Confirm), BORDER);
        assert_eq!(dialog_border_color(DialogTone::Destructive), DANGER);
    }

    #[test]
    fn selected_row_uses_primary_fill() {
        let style = selected_row();
        assert_eq!(style.fg, Some(BG));
        assert_eq!(style.bg, Some(PRIMARY));
        assert!(style.add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn prompt_marker_is_bold_primary() {
        let style = prompt_style();
        assert_eq!(style.fg, Some(PRIMARY));
        assert!(style.add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn chip_style_uses_surface_bg_and_variant_fg() {
        let cases = [
            (ButtonVariant::Default, false, PRIMARY),
            (ButtonVariant::Destructive, false, DANGER),
            (ButtonVariant::Neutral, false, FG_MUTED),
            (ButtonVariant::Warning, false, WARNING),
        ];
        for (variant, bold, expect_fg) in cases {
            let style = chip_style(variant, bold);
            assert_eq!(style.fg, Some(expect_fg), "{variant:?}");
            assert_eq!(style.bg, Some(SURFACE), "{variant:?}");
            assert!(!style.add_modifier.contains(Modifier::BOLD), "{variant:?}");
        }
        let emphasized = chip_style(ButtonVariant::Default, true);
        assert!(emphasized.add_modifier.contains(Modifier::BOLD));
    }
}
