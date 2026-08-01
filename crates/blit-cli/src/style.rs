//! Dracula colouring for the live row and the failure block (clp-3).
//!
//! # Two rules this module exists to enforce
//!
//! 1. **Colour is applied after layout, never before.** Escape sequences are
//!    zero-width on screen but not zero-length in a `String`, so styling text
//!    before the row's width arithmetic would corrupt the layout. Callers
//!    truncate plain text, then style the finished pieces.
//! 2. **Disabled means byte-identical.** [`Palette::disabled`] returns the
//!    input unchanged, so piped output, `--json`, `NO_COLOR` and every
//!    existing test see exactly the bytes they saw before clp-3.
//!
//! Styling goes through `console::Style` rather than hand-written escapes so
//! it inherits console's capability detection; hand-rolled sequences would
//! bypass the very checks that make rule 2 hold.

use console::{Color, Style};

/// How much colour the terminal can express.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ColorDepth {
    /// No colour at all — output must be byte-identical to the plain form.
    None,
    /// 256-colour: Dracula approximations.
    Ansi256,
    /// 24-bit: exact Dracula.
    TrueColor,
}

/// Dracula, as the palette defines it.
mod dracula {
    pub(super) const COMMENT: (u8, u8, u8) = (0x62, 0x72, 0xA4);
    pub(super) const CYAN: (u8, u8, u8) = (0x8B, 0xE9, 0xFD);
    pub(super) const GREEN: (u8, u8, u8) = (0x50, 0xFA, 0x7B);
    pub(super) const ORANGE: (u8, u8, u8) = (0xFF, 0xB8, 0x6C);
    pub(super) const PURPLE: (u8, u8, u8) = (0xBD, 0x93, 0xF9);
    pub(super) const RED: (u8, u8, u8) = (0xFF, 0x55, 0x55);

    /// Standard 256-colour approximations, used when the terminal does not
    /// advertise 24-bit.
    pub(super) const COMMENT_256: u8 = 61;
    pub(super) const CYAN_256: u8 = 117;
    pub(super) const GREEN_256: u8 = 84;
    pub(super) const ORANGE_256: u8 = 215;
    pub(super) const PURPLE_256: u8 = 141;
    pub(super) const RED_256: u8 = 203;
}

/// One palette role. Deliberately named for MEANING, not for colour, so a
/// later palette change cannot silently alter what a colour communicates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Role {
    /// Source-side walk.
    PhaseEnumerating,
    /// Destination diff.
    PhaseComparing,
    /// Payload apply.
    PhaseCopying,
    /// Mirror delete pass. ORANGE, not red: this is expected destructive
    /// work, and red is reserved for something having gone wrong. Spending
    /// red here would cost red its meaning in the failure block below.
    PhaseDeleting,
    /// Secondary text: separators, truncated paths, the re-run hint.
    Muted,
    /// Something went wrong. The only red in the CLI.
    Failure,
}

impl Role {
    fn rgb(self) -> (u8, u8, u8) {
        match self {
            Role::PhaseEnumerating => dracula::PURPLE,
            Role::PhaseComparing => dracula::CYAN,
            Role::PhaseCopying => dracula::GREEN,
            Role::PhaseDeleting => dracula::ORANGE,
            Role::Muted => dracula::COMMENT,
            Role::Failure => dracula::RED,
        }
    }

    fn ansi256(self) -> u8 {
        match self {
            Role::PhaseEnumerating => dracula::PURPLE_256,
            Role::PhaseComparing => dracula::CYAN_256,
            Role::PhaseCopying => dracula::GREEN_256,
            Role::PhaseDeleting => dracula::ORANGE_256,
            Role::Muted => dracula::COMMENT_256,
            Role::Failure => dracula::RED_256,
        }
    }
}

/// The colouring decision for one output stream, resolved once.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Palette {
    depth: ColorDepth,
}

impl Palette {
    /// Never colours. Returned whenever colour is unwanted or unsupported,
    /// and used directly by every non-terminal caller.
    pub(crate) fn disabled() -> Self {
        Self {
            depth: ColorDepth::None,
        }
    }

    pub(crate) fn with_depth(depth: ColorDepth) -> Self {
        Self { depth }
    }

    /// Resolve from the environment.
    ///
    /// `colors_allowed` is the caller's own gate — for the live row it is
    /// "the bar can actually draw", which already accounts for a
    /// non-terminal stderr. This function adds the colour-specific
    /// conventions on top of it and never overrides a `false`.
    pub(crate) fn detect(colors_allowed: bool) -> Self {
        Self::detect_with(colors_allowed, |name| std::env::var(name).ok())
    }

    fn detect_with(colors_allowed: bool, mut read: impl FnMut(&str) -> Option<String>) -> Self {
        // NO_COLOR: set to ANY value, including empty, means no colour.
        if !colors_allowed || read("NO_COLOR").is_some() {
            return Self::disabled();
        }
        if read("TERM").is_some_and(|term| term.eq_ignore_ascii_case("dumb")) {
            return Self::disabled();
        }
        let truecolor = read("COLORTERM").is_some_and(|value| {
            let value = value.to_ascii_lowercase();
            value.contains("truecolor") || value.contains("24bit")
        });
        Self::with_depth(if truecolor {
            ColorDepth::TrueColor
        } else {
            ColorDepth::Ansi256
        })
    }

    #[cfg(test)]
    pub(crate) fn depth(self) -> ColorDepth {
        self.depth
    }

    pub(crate) fn is_enabled(self) -> bool {
        self.depth != ColorDepth::None
    }

    /// Paint `text` for `role`.
    ///
    /// When disabled this returns `text` unchanged — the byte-identity
    /// property the whole design rests on.
    pub(crate) fn paint(self, role: Role, text: &str) -> String {
        match self.depth {
            ColorDepth::None => text.to_string(),
            ColorDepth::Ansi256 => Style::new()
                .fg(Color::Color256(role.ansi256()))
                .force_styling(true)
                .apply_to(text)
                .to_string(),
            ColorDepth::TrueColor => {
                let (r, g, b) = role.rgb();
                Style::new()
                    .fg(Color::TrueColor(r, g, b))
                    .force_styling(true)
                    .apply_to(text)
                    .to_string()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env<'a>(pairs: &'a [(&'a str, &'a str)]) -> impl FnMut(&str) -> Option<String> + 'a {
        move |name| {
            pairs
                .iter()
                .find(|(key, _)| *key == name)
                .map(|(_, value)| value.to_string())
        }
    }

    #[test]
    fn disabled_is_byte_identical() {
        // The property every piped consumer and every pre-clp-3 test
        // depends on. Asserted on the exact bytes, not on "contains".
        let palette = Palette::disabled();
        for role in [
            Role::PhaseEnumerating,
            Role::PhaseComparing,
            Role::PhaseCopying,
            Role::PhaseDeleting,
            Role::Muted,
            Role::Failure,
        ] {
            assert_eq!(
                palette.paint(role, "copying • 3/9 files"),
                "copying • 3/9 files"
            );
            assert_eq!(palette.paint(role, ""), "");
        }
    }

    #[test]
    fn truecolor_emits_the_exact_dracula_sequence() {
        // Exact bytes per capability tier. "output is non-empty" or
        // "contains an escape" would pass for any colour and prove nothing.
        let palette = Palette::with_depth(ColorDepth::TrueColor);
        assert_eq!(
            palette.paint(Role::PhaseCopying, "copying"),
            "\u{1b}[38;2;80;250;123mcopying\u{1b}[0m",
            "copying must be Dracula green #50FA7B"
        );
        assert_eq!(
            palette.paint(Role::PhaseDeleting, "deleting"),
            "\u{1b}[38;2;255;184;108mdeleting\u{1b}[0m",
            "deleting must be Dracula orange #FFB86C — NOT red"
        );
        assert_eq!(
            palette.paint(Role::Failure, "boom"),
            "\u{1b}[38;2;255;85;85mboom\u{1b}[0m",
            "failures must be Dracula red #FF5555"
        );
        assert_eq!(
            palette.paint(Role::Muted, "…/path"),
            "\u{1b}[38;2;98;114;164m…/path\u{1b}[0m"
        );
    }

    #[test]
    fn ansi256_emits_the_approximation_sequence() {
        let palette = Palette::with_depth(ColorDepth::Ansi256);
        assert_eq!(
            palette.paint(Role::PhaseCopying, "copying"),
            "\u{1b}[38;5;84mcopying\u{1b}[0m"
        );
        assert_eq!(
            palette.paint(Role::Failure, "boom"),
            "\u{1b}[38;5;203mboom\u{1b}[0m"
        );
    }

    #[test]
    fn deleting_and_failure_never_share_a_colour() {
        // The one semantic invariant in the palette: an expected destructive
        // phase must not look like an error. If a later edit points both at
        // red this fails, whatever the hex happens to be.
        for depth in [ColorDepth::TrueColor, ColorDepth::Ansi256] {
            let palette = Palette::with_depth(depth);
            assert_ne!(
                palette.paint(Role::PhaseDeleting, "x"),
                palette.paint(Role::Failure, "x"),
                "{depth:?}: deleting must not render as failure"
            );
        }
    }

    #[test]
    fn no_color_wins_over_everything() {
        // Set to empty string, which is the case implementations most often
        // get wrong — the convention is "set at all", not "set to a value".
        let palette =
            Palette::detect_with(true, env(&[("NO_COLOR", ""), ("COLORTERM", "truecolor")]));
        assert!(!palette.is_enabled());
    }

    #[test]
    fn dumb_terminals_and_denied_callers_get_nothing() {
        assert!(!Palette::detect_with(true, env(&[("TERM", "dumb")])).is_enabled());
        assert!(!Palette::detect_with(true, env(&[("TERM", "DUMB")])).is_enabled());
        // The caller's own gate — "the bar can draw" — is never overridden.
        assert!(!Palette::detect_with(false, env(&[("COLORTERM", "truecolor")])).is_enabled());
    }

    #[test]
    fn colorterm_selects_the_depth() {
        assert_eq!(
            Palette::detect_with(true, env(&[("COLORTERM", "truecolor")])).depth(),
            ColorDepth::TrueColor
        );
        assert_eq!(
            Palette::detect_with(true, env(&[("COLORTERM", "24bit")])).depth(),
            ColorDepth::TrueColor
        );
        // Unknown or absent COLORTERM falls back rather than guessing 24-bit.
        assert_eq!(
            Palette::detect_with(true, env(&[("COLORTERM", "8bit")])).depth(),
            ColorDepth::Ansi256
        );
        assert_eq!(
            Palette::detect_with(true, env(&[])).depth(),
            ColorDepth::Ansi256
        );
    }
}
