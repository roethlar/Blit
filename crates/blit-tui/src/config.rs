//! e-3: optional TUI config loaded from
//! `<config_dir>/tui.toml` at startup. The CLI shares
//! `blit_core::config::config_dir()` for path resolution
//! (honors `BLIT_CONFIG_DIR` overrides used by tests).
//!
//! Missing file → defaults. Parse errors → warn on
//! stderr (visible after TUI exit) and use defaults. We
//! never crash the TUI on a misconfigured `tui.toml`.
//!
//! Initial schema (intentionally tiny):
//!
//! ```toml
//! [verify]
//! default_use_checksum = true
//! default_one_way = false
//! ```
//!
//! Future slices can grow the schema (color themes,
//! refresh intervals, persisted form fields). Every new
//! field must have `#[serde(default)]` so older configs
//! continue to parse without surprises.

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
    /// `[MIN_TICK_MS, MAX_TICK_MS]` after load — anything
    /// outside is silently snapped to the bound rather
    /// than refused.
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
}
