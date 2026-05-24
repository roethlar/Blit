//! audit-7d9: pure theme-color mapping helpers extracted from `main.rs`
//! (behavior-preserving — verbatim move, no logic change). They bridge the
//! config `RawColor`/base bg-fg into the ratatui types the renderer uses,
//! kept pure so the mapping is unit-testable. No AppState.

use crate::config;

/// dark-1: build the base frame style from the optional `[theme]`
/// background / foreground colors. Returns `None` when BOTH are unset —
/// so the caller skips painting a base layer and the terminal's own
/// colors show through (the historical default). Pure, so the
/// bg/fg → style mapping is unit-testable.
pub(crate) fn base_theme_style(
    bg: Option<ratatui::style::Color>,
    fg: Option<ratatui::style::Color>,
) -> Option<ratatui::style::Style> {
    if bg.is_none() && fg.is_none() {
        return None;
    }
    let mut style = ratatui::style::Style::default();
    if let Some(bg) = bg {
        style = style.bg(bg);
    }
    if let Some(fg) = fg {
        style = style.fg(fg);
    }
    Some(style)
}

/// e-7: bridge from the config's `RawColor` (which lives
/// in `config` to avoid leaking ratatui types into the
/// schema layer) to the ratatui color used by the
/// renderer.
pub(crate) fn raw_color_to_ratatui(c: config::RawColor) -> ratatui::style::Color {
    use ratatui::style::Color;
    match c {
        config::RawColor::Black => Color::Black,
        config::RawColor::Red => Color::Red,
        config::RawColor::Green => Color::Green,
        config::RawColor::Yellow => Color::Yellow,
        config::RawColor::Blue => Color::Blue,
        config::RawColor::Magenta => Color::Magenta,
        config::RawColor::Cyan => Color::Cyan,
        config::RawColor::Gray => Color::Gray,
        config::RawColor::DarkGray => Color::DarkGray,
        config::RawColor::LightRed => Color::LightRed,
        config::RawColor::LightGreen => Color::LightGreen,
        config::RawColor::LightYellow => Color::LightYellow,
        config::RawColor::LightBlue => Color::LightBlue,
        config::RawColor::LightMagenta => Color::LightMagenta,
        config::RawColor::LightCyan => Color::LightCyan,
        config::RawColor::White => Color::White,
    }
}
