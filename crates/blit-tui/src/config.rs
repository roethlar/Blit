//! e-3: optional TUI config loaded from
//! `<config_dir>/tui.toml` at startup. The CLI shares
//! `blit_core::config::config_dir()` for path resolution
//! (honors `BLIT_CONFIG_DIR` overrides used by tests).
//!
//! Missing file → defaults. Parse errors → warn on
//! stderr (visible after TUI exit) and use defaults. We
//! never crash the TUI on a misconfigured `tui.toml`.
//!
//! Current schema (grown through e-3 / e-4 / e-5 / e-6 / e-7 / d-24):
//!
//! ```toml
//! [verify]
//! default_use_checksum = false  # `H` toggle's startup value
//! default_one_way = false       # `O` toggle's startup value
//! default_source = ""           # e-6: launch-time Source prefill
//! default_destination = ""      # e-6: launch-time Destination prefill
//!
//! [tab_strip]
//! show_counts = true            # e-4: right-edge counts column
//!
//! [live_tick]
//! interval_ms = 500             # e-5: render-wakeup cadence (clamped to [50, 5000])
//!
//! [theme]
//! accent_color = "cyan"         # e-7: active-tab background
//!
//! [transfer]
//! cancel_status_ttl_ms = 5000   # d-24: F2 cancel-fragment TTL (clamped to [250, 60000])
//! ```
//!
//! Future slices can grow the schema (per-pane tick
//! intervals, runtime save-back of edited Verify paths,
//! more themable surfaces). Every new field must have
//! `#[serde(default)]` so older configs continue to
//! parse without surprises.

use serde::Deserialize;
use std::path::Path;

/// Filename the loader looks for inside `config_dir()`.
pub const CONFIG_FILENAME: &str = "tui.toml";

#[derive(Debug, Default, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TuiConfig {
    pub verify: VerifyDefaults,
    pub tab_strip: TabStripDefaults,
    pub live_tick: LiveTickDefaults,
    pub theme: ThemeDefaults,
    pub transfer: TransferDefaults,
}

#[derive(Debug, Default, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct VerifyDefaults {
    /// Default value of `VerifyState::use_checksum` when
    /// the TUI starts. `false` matches the rsync /
    /// `blit check` default.
    pub default_use_checksum: bool,
    /// Default value of `VerifyState::one_way`. `false`
    /// matches `blit check`'s two-way default.
    pub default_one_way: bool,
    /// e-6: prefill the Verify form's Source field at
    /// TUI launch. Empty string (default) leaves the
    /// field empty — operator types as before. Useful
    /// when the operator runs the same compare repeatedly
    /// (e.g. nightly backup verification against a
    /// known target).
    pub default_source: String,
    /// e-6: prefill the Verify form's Destination field
    /// at TUI launch. Same shape as `default_source`.
    pub default_destination: String,
}

/// e-4: tab-strip rendering preferences.
#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TabStripDefaults {
    /// Render the right-side "N daemons · N active · N
    /// recent · ? help" counts. `true` (default) matches
    /// d-15. `false` collapses the tab strip to just the
    /// F1..F4 labels — useful on narrow terminals or for
    /// operators who find the counts distracting.
    pub show_counts: bool,
}

impl Default for TabStripDefaults {
    fn default() -> Self {
        Self { show_counts: true }
    }
}

/// e-5: live-tick wakeup cadence. Higher values reduce
/// redraw cost on slow / high-latency terminals at the
/// expense of choppier elapsed counters.
#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct LiveTickDefaults {
    /// Milliseconds between wakeups while a Running
    /// transfer, Running verify, or pane with a freshness
    /// footer is on screen. Clamped to
    /// `[MIN_INTERVAL_MS, MAX_INTERVAL_MS]` after load —
    /// anything outside is silently snapped to the bound
    /// rather than refused.
    pub interval_ms: u64,
}

impl LiveTickDefaults {
    /// Default 500ms — matches d-9's hardcoded value, so
    /// upgrading without writing a `tui.toml` keeps the
    /// existing cadence.
    pub const DEFAULT_INTERVAL_MS: u64 = 500;
    /// Floor — 50ms is already 20Hz, fast enough that
    /// human eyes can't tell the difference and the
    /// terminal would burn CPU on cells that aren't
    /// changing.
    pub const MIN_INTERVAL_MS: u64 = 50;
    /// Ceiling — 5s. Beyond that the "live" tick stops
    /// looking live (a 12.3s timer that updates once
    /// every 5s looks frozen most of the time).
    pub const MAX_INTERVAL_MS: u64 = 5000;

    /// Clamped accessor. The loader applies this once,
    /// so the runtime always sees a sane value even when
    /// the TOML file specifies 0 or u64::MAX.
    pub fn interval_ms_clamped(&self) -> u64 {
        self.interval_ms
            .clamp(Self::MIN_INTERVAL_MS, Self::MAX_INTERVAL_MS)
    }
}

impl Default for LiveTickDefaults {
    fn default() -> Self {
        Self {
            interval_ms: Self::DEFAULT_INTERVAL_MS,
        }
    }
}

/// e-7: themable colors. Renderer-facing surfaces that
/// historically hardcoded `Color::Cyan` / `Color::Magenta`
/// now read from this struct so an operator with red-green
/// colorblindness or a custom terminal palette can swap
/// the highlight to something more legible.
#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ThemeDefaults {
    /// Active-tab background in the tab strip. Default
    /// `"cyan"` matches the d-15 visual baseline.
    pub accent_color: String,
}

impl ThemeDefaults {
    pub const DEFAULT_ACCENT: &'static str = "cyan";

    /// Parse the accent string into a renderer-ready
    /// color. Returns the parsed color when the string
    /// matches a known name, or `None` on unknown values
    /// so callers can warn + fall back to the default.
    /// Case-insensitive.
    pub fn parse_accent(&self) -> Option<RawColor> {
        accent_color_from_str(&self.accent_color)
    }
}

impl Default for ThemeDefaults {
    fn default() -> Self {
        Self {
            accent_color: Self::DEFAULT_ACCENT.to_string(),
        }
    }
}

/// d-24: transfer-related preferences. Currently just
/// the d-23 cancel-status TTL.
#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TransferDefaults {
    /// Milliseconds the F2 cancel-status fragment stays
    /// on screen after a CancelJob reply lands. Sending
    /// has no TTL — only Done / Error variants expire.
    /// Clamped to `[MIN_CANCEL_TTL_MS, MAX_CANCEL_TTL_MS]`
    /// — 0 (always-hidden) or 600000 (10 minutes) are
    /// silently snapped to the bounds rather than refused.
    pub cancel_status_ttl_ms: u64,
}

impl TransferDefaults {
    /// d-23 baseline: 5 seconds. Long enough to read
    /// "cancelled abc-123", short enough not to clutter.
    pub const DEFAULT_CANCEL_TTL_MS: u64 = 5_000;
    /// Floor — at 250ms the operator barely sees the
    /// fragment before it disappears. Lower values
    /// effectively mean "don't show me cancel outcomes."
    pub const MIN_CANCEL_TTL_MS: u64 = 250;
    /// Ceiling — 60 seconds. Beyond that the footer feels
    /// permanently cluttered. Operators who want
    /// truly-permanent retention can re-run the cancel.
    pub const MAX_CANCEL_TTL_MS: u64 = 60_000;

    /// Clamped accessor; the renderer reads this once per
    /// frame so out-of-range config values are silently
    /// normalized.
    pub fn cancel_status_ttl_ms_clamped(&self) -> u64 {
        self.cancel_status_ttl_ms
            .clamp(Self::MIN_CANCEL_TTL_MS, Self::MAX_CANCEL_TTL_MS)
    }
}

impl Default for TransferDefaults {
    fn default() -> Self {
        Self {
            cancel_status_ttl_ms: Self::DEFAULT_CANCEL_TTL_MS,
        }
    }
}

/// e-7: the subset of ratatui colors we expose for the
/// accent setting. Names match the standard ANSI palette
/// terminal users recognize.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RawColor {
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    Gray,
    DarkGray,
    LightRed,
    LightGreen,
    LightYellow,
    LightBlue,
    LightMagenta,
    LightCyan,
    White,
}

fn accent_color_from_str(name: &str) -> Option<RawColor> {
    match name.trim().to_ascii_lowercase().as_str() {
        "black" => Some(RawColor::Black),
        "red" => Some(RawColor::Red),
        "green" => Some(RawColor::Green),
        "yellow" => Some(RawColor::Yellow),
        "blue" => Some(RawColor::Blue),
        "magenta" => Some(RawColor::Magenta),
        "cyan" => Some(RawColor::Cyan),
        "gray" | "grey" => Some(RawColor::Gray),
        "darkgray" | "darkgrey" | "dark_gray" | "dark_grey" => Some(RawColor::DarkGray),
        "lightred" | "light_red" => Some(RawColor::LightRed),
        "lightgreen" | "light_green" => Some(RawColor::LightGreen),
        "lightyellow" | "light_yellow" => Some(RawColor::LightYellow),
        "lightblue" | "light_blue" => Some(RawColor::LightBlue),
        "lightmagenta" | "light_magenta" => Some(RawColor::LightMagenta),
        "lightcyan" | "light_cyan" => Some(RawColor::LightCyan),
        "white" => Some(RawColor::White),
        _ => None,
    }
}

/// Read + parse `<config_dir>/tui.toml`. Any failure
/// (missing dir, missing file, parse error) falls back
/// to `TuiConfig::default()`. The `on_warn` callback
/// gets a single string describing the failure for the
/// missing-file case it's NOT called (that's the
/// expected happy default).
pub fn load(on_warn: impl FnOnce(String)) -> TuiConfig {
    let Ok(dir) = blit_core::config::config_dir() else {
        // No platform-resolvable config dir — defaults
        // are the only sane fallback. Silent: this isn't
        // a user-actionable warning.
        return TuiConfig::default();
    };
    let path = dir.join(CONFIG_FILENAME);
    load_from_path(&path, on_warn)
}

/// Path-explicit loader. Exposed so tests can point at
/// a tempdir without relying on the global
/// `config_dir()` resolution. Missing file is not a
/// warning — that's the expected default config. Parse
/// errors are.
pub fn load_from_path(path: &Path, on_warn: impl FnOnce(String)) -> TuiConfig {
    let Ok(text) = std::fs::read_to_string(path) else {
        // Missing or unreadable — default config. No
        // warning: a fresh install legitimately has no
        // `tui.toml`.
        return TuiConfig::default();
    };
    match toml::from_str::<TuiConfig>(&text) {
        Ok(cfg) => cfg,
        Err(err) => {
            on_warn(format!(
                "failed to parse {}: {err} — using defaults",
                path.display()
            ));
            TuiConfig::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_file_returns_defaults_silently() {
        let tmp = tempfile::tempdir().expect("tmp");
        let path = tmp.path().join("tui.toml");
        let mut warned = None;
        let cfg = load_from_path(&path, |msg| warned = Some(msg));
        assert!(warned.is_none(), "missing file is not a warning");
        assert!(!cfg.verify.default_use_checksum);
        assert!(!cfg.verify.default_one_way);
    }

    #[test]
    fn empty_file_returns_defaults() {
        let tmp = tempfile::tempdir().expect("tmp");
        let path = tmp.path().join("tui.toml");
        std::fs::write(&path, "").expect("write");
        let cfg = load_from_path(&path, |_| panic!("empty file should not warn"));
        assert!(!cfg.verify.default_use_checksum);
        assert!(!cfg.verify.default_one_way);
    }

    #[test]
    fn populated_verify_section_overrides_defaults() {
        let tmp = tempfile::tempdir().expect("tmp");
        let path = tmp.path().join("tui.toml");
        std::fs::write(
            &path,
            "[verify]\ndefault_use_checksum = true\ndefault_one_way = true\n",
        )
        .expect("write");
        let cfg = load_from_path(&path, |msg| panic!("unexpected warn: {msg}"));
        assert!(cfg.verify.default_use_checksum);
        assert!(cfg.verify.default_one_way);
    }

    /// e-6: path-prefill fields parse + default to empty.
    #[test]
    fn verify_path_prefill_round_trip() {
        let tmp = tempfile::tempdir().expect("tmp");
        let path = tmp.path().join("tui.toml");
        std::fs::write(
            &path,
            "[verify]\n\
             default_source = \"/backups/src\"\n\
             default_destination = \"/backups/dst\"\n",
        )
        .expect("write");
        let cfg = load_from_path(&path, |msg| panic!("unexpected warn: {msg}"));
        assert_eq!(cfg.verify.default_source, "/backups/src");
        assert_eq!(cfg.verify.default_destination, "/backups/dst");
        // Other verify fields untouched.
        assert!(!cfg.verify.default_use_checksum);
        assert!(!cfg.verify.default_one_way);
    }

    #[test]
    fn verify_path_prefill_defaults_to_empty() {
        // Verify section absent entirely — path fields
        // must default to empty so the operator's typing
        // workflow is unchanged.
        let cfg = TuiConfig::default();
        assert_eq!(cfg.verify.default_source, "");
        assert_eq!(cfg.verify.default_destination, "");
    }

    #[test]
    fn partial_verify_section_keeps_other_defaults() {
        let tmp = tempfile::tempdir().expect("tmp");
        let path = tmp.path().join("tui.toml");
        std::fs::write(&path, "[verify]\ndefault_use_checksum = true\n").expect("write");
        let cfg = load_from_path(&path, |msg| panic!("unexpected warn: {msg}"));
        assert!(cfg.verify.default_use_checksum);
        assert!(
            !cfg.verify.default_one_way,
            "unspecified fields take serde defaults"
        );
    }

    #[test]
    fn malformed_toml_emits_warning_returns_defaults() {
        let tmp = tempfile::tempdir().expect("tmp");
        let path = tmp.path().join("tui.toml");
        std::fs::write(&path, "this is not toml = = =").expect("write");
        let mut warned = None;
        let cfg = load_from_path(&path, |msg| warned = Some(msg));
        let warning = warned.expect("malformed toml must warn");
        assert!(warning.contains("parse"));
        // Defaults intact.
        assert!(!cfg.verify.default_use_checksum);
    }

    /// e-3 R2: warnings flow through the caller-provided
    /// callback, not via direct stderr writes. This is
    /// what lets `main` buffer them and flush AFTER the
    /// TUI guard restores the terminal. The test
    /// captures the same end-to-end shape `main` uses:
    /// load → push to Vec → check after.
    #[test]
    fn warnings_route_through_callback_not_stderr() {
        let tmp = tempfile::tempdir().expect("tmp");
        let path = tmp.path().join("tui.toml");
        std::fs::write(&path, "this is not valid toml").expect("write");

        let mut collected: Vec<String> = Vec::new();
        let cfg = load_from_path(&path, |msg| collected.push(msg));
        assert_eq!(collected.len(), 1, "exactly one warning from a parse error");
        assert!(collected[0].contains("parse"));
        // Buffer is owned by the caller — caller can
        // flush after restoring the terminal.
        assert!(!cfg.verify.default_use_checksum);
    }

    // e-5: live-tick interval clamp + parse.

    #[test]
    fn live_tick_default_is_500ms() {
        let cfg = TuiConfig::default();
        assert_eq!(cfg.live_tick.interval_ms, 500);
        assert_eq!(cfg.live_tick.interval_ms_clamped(), 500);
    }

    #[test]
    fn live_tick_parses_from_toml() {
        let tmp = tempfile::tempdir().expect("tmp");
        let path = tmp.path().join("tui.toml");
        std::fs::write(&path, "[live_tick]\ninterval_ms = 1200\n").expect("write");
        let cfg = load_from_path(&path, |msg| panic!("unexpected warn: {msg}"));
        assert_eq!(cfg.live_tick.interval_ms, 1200);
        assert_eq!(cfg.live_tick.interval_ms_clamped(), 1200);
    }

    #[test]
    fn live_tick_clamp_floor() {
        let cfg = LiveTickDefaults { interval_ms: 0 };
        assert_eq!(cfg.interval_ms_clamped(), LiveTickDefaults::MIN_INTERVAL_MS);
        let cfg = LiveTickDefaults { interval_ms: 1 };
        assert_eq!(cfg.interval_ms_clamped(), LiveTickDefaults::MIN_INTERVAL_MS);
    }

    #[test]
    fn live_tick_clamp_ceiling() {
        let cfg = LiveTickDefaults {
            interval_ms: u64::MAX,
        };
        assert_eq!(cfg.interval_ms_clamped(), LiveTickDefaults::MAX_INTERVAL_MS);
        let cfg = LiveTickDefaults {
            interval_ms: 10_000,
        };
        assert_eq!(cfg.interval_ms_clamped(), LiveTickDefaults::MAX_INTERVAL_MS);
    }

    #[test]
    fn live_tick_passes_through_when_in_range() {
        for ms in [50, 250, 500, 1000, 3000, 5000] {
            let cfg = LiveTickDefaults { interval_ms: ms };
            assert_eq!(
                cfg.interval_ms_clamped(),
                ms,
                "in-range value passes through"
            );
        }
    }

    // e-7: [theme] accent color.

    #[test]
    fn theme_default_is_cyan() {
        let cfg = TuiConfig::default();
        assert_eq!(cfg.theme.accent_color, "cyan");
        assert_eq!(cfg.theme.parse_accent(), Some(RawColor::Cyan));
    }

    #[test]
    fn theme_parses_each_supported_color() {
        for (name, expected) in [
            ("black", RawColor::Black),
            ("red", RawColor::Red),
            ("green", RawColor::Green),
            ("yellow", RawColor::Yellow),
            ("blue", RawColor::Blue),
            ("magenta", RawColor::Magenta),
            ("cyan", RawColor::Cyan),
            ("gray", RawColor::Gray),
            ("grey", RawColor::Gray),
            ("darkgray", RawColor::DarkGray),
            ("dark_gray", RawColor::DarkGray),
            ("lightblue", RawColor::LightBlue),
            ("light_blue", RawColor::LightBlue),
            ("white", RawColor::White),
        ] {
            let theme = ThemeDefaults {
                accent_color: name.to_string(),
            };
            assert_eq!(theme.parse_accent(), Some(expected), "color name {name:?}");
        }
    }

    #[test]
    fn theme_parse_is_case_insensitive() {
        let theme = ThemeDefaults {
            accent_color: "CyAn".to_string(),
        };
        assert_eq!(theme.parse_accent(), Some(RawColor::Cyan));
    }

    #[test]
    fn theme_parse_unknown_color_returns_none() {
        let theme = ThemeDefaults {
            accent_color: "fuchsia".to_string(),
        };
        assert!(theme.parse_accent().is_none());
    }

    #[test]
    fn theme_round_trips_through_toml() {
        let tmp = tempfile::tempdir().expect("tmp");
        let path = tmp.path().join("tui.toml");
        std::fs::write(&path, "[theme]\naccent_color = \"magenta\"\n").expect("write");
        let cfg = load_from_path(&path, |msg| panic!("unexpected warn: {msg}"));
        assert_eq!(cfg.theme.accent_color, "magenta");
        assert_eq!(cfg.theme.parse_accent(), Some(RawColor::Magenta));
    }

    #[test]
    fn unknown_fields_emit_warning() {
        // deny_unknown_fields catches typos early so
        // operators don't sit on dead config they think is
        // applying.
        let tmp = tempfile::tempdir().expect("tmp");
        let path = tmp.path().join("tui.toml");
        std::fs::write(&path, "[verify]\ndefalut_use_checksum = true\n").expect("write");
        let mut warned = None;
        let _cfg = load_from_path(&path, |msg| warned = Some(msg));
        let warning = warned.expect("typo'd field must warn");
        assert!(warning.contains("defalut_use_checksum") || warning.contains("unknown"));
    }

    // d-24: [transfer] cancel_status_ttl_ms clamp + parse.

    #[test]
    fn transfer_default_cancel_ttl_is_5000ms() {
        let cfg = TuiConfig::default();
        assert_eq!(
            cfg.transfer.cancel_status_ttl_ms,
            TransferDefaults::DEFAULT_CANCEL_TTL_MS
        );
        assert_eq!(
            cfg.transfer.cancel_status_ttl_ms_clamped(),
            TransferDefaults::DEFAULT_CANCEL_TTL_MS
        );
    }

    #[test]
    fn transfer_cancel_ttl_parses_from_toml() {
        let tmp = tempfile::tempdir().expect("tmp");
        let path = tmp.path().join("tui.toml");
        std::fs::write(&path, "[transfer]\ncancel_status_ttl_ms = 2500\n").expect("write");
        let cfg = load_from_path(&path, |msg| panic!("unexpected warn: {msg}"));
        assert_eq!(cfg.transfer.cancel_status_ttl_ms, 2500);
        assert_eq!(cfg.transfer.cancel_status_ttl_ms_clamped(), 2500);
    }

    #[test]
    fn transfer_cancel_ttl_clamp_floor() {
        let cfg = TransferDefaults {
            cancel_status_ttl_ms: 0,
        };
        assert_eq!(
            cfg.cancel_status_ttl_ms_clamped(),
            TransferDefaults::MIN_CANCEL_TTL_MS
        );
        let cfg = TransferDefaults {
            cancel_status_ttl_ms: 100,
        };
        assert_eq!(
            cfg.cancel_status_ttl_ms_clamped(),
            TransferDefaults::MIN_CANCEL_TTL_MS
        );
    }

    #[test]
    fn transfer_cancel_ttl_clamp_ceiling() {
        let cfg = TransferDefaults {
            cancel_status_ttl_ms: u64::MAX,
        };
        assert_eq!(
            cfg.cancel_status_ttl_ms_clamped(),
            TransferDefaults::MAX_CANCEL_TTL_MS
        );
        let cfg = TransferDefaults {
            cancel_status_ttl_ms: 120_000,
        };
        assert_eq!(
            cfg.cancel_status_ttl_ms_clamped(),
            TransferDefaults::MAX_CANCEL_TTL_MS
        );
    }

    #[test]
    fn transfer_cancel_ttl_passes_through_when_in_range() {
        for ms in [250, 1_000, 5_000, 10_000, 30_000, 60_000] {
            let cfg = TransferDefaults {
                cancel_status_ttl_ms: ms,
            };
            assert_eq!(
                cfg.cancel_status_ttl_ms_clamped(),
                ms,
                "in-range value passes through"
            );
        }
    }

    #[test]
    fn transfer_cancel_ttl_round_trips_through_toml() {
        let tmp = tempfile::tempdir().expect("tmp");
        let path = tmp.path().join("tui.toml");
        std::fs::write(&path, "[transfer]\ncancel_status_ttl_ms = 750\n").expect("write");
        let cfg = load_from_path(&path, |msg| panic!("unexpected warn: {msg}"));
        assert_eq!(cfg.transfer.cancel_status_ttl_ms, 750);
        assert_eq!(cfg.transfer.cancel_status_ttl_ms_clamped(), 750);
    }

    #[test]
    fn transfer_unknown_field_warns() {
        // deny_unknown_fields on [transfer] catches typos
        // so the operator's intended TTL override doesn't
        // silently fall back to the default.
        let tmp = tempfile::tempdir().expect("tmp");
        let path = tmp.path().join("tui.toml");
        std::fs::write(
            &path,
            "[transfer]\ncancel_status_ttl = 1000\n", // missing _ms suffix
        )
        .expect("write");
        let mut warned = None;
        let _cfg = load_from_path(&path, |msg| warned = Some(msg));
        let warning = warned.expect("unknown [transfer] field must warn");
        assert!(warning.contains("cancel_status_ttl") || warning.contains("unknown"));
    }
}
