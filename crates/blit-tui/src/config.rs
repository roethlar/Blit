//! e-3: optional TUI config loaded from
//! `<config_dir>/tui.toml` at startup. The CLI shares
//! `blit_core::config::config_dir()` for path resolution
//! (honors `BLIT_CONFIG_DIR` overrides used by tests).
//!
//! Missing file → defaults. Parse errors → warn on
//! stderr (visible after TUI exit) and use defaults. We
//! never crash the TUI on a misconfigured `tui.toml`.
//!
//! d-36: the config is also reloadable at runtime via
//! `Ctrl+R` (see `main::reload_tui_config`), which swaps
//! the live config without restarting. A reload parse
//! error keeps the current config rather than reverting
//! to defaults.
//!
//! Current schema (grown through e-3 / e-4 / e-5 / e-6 / e-7 / e-8 / d-24 / d-40 / d-52 / d-64):
//!
//! ```toml
//! [daemon]
//! default_remote = ""           # e-8: fallback remote when no --remote flag (CLI flag wins)
//!
//! [keys]
//! quit = "q"                    # keys-1: quit key (Esc + Ctrl+C always quit too)
//! refresh = "r"                 # keys-2: refresh key (Ctrl+R reload unaffected)
//! pane_f1 = "1"                 # keys-3: pane-switch digit aliases (F1-F4 keys also navigate)
//! pane_f2 = "2"
//! pane_f3 = "3"
//! pane_f4 = "4"
//!
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
//! confirm_cancel = false        # d-29: opt-in `K` confirm prompt (y/N)
//! pull_status_ttl_ms = 5000     # d-40: F3 pull-outcome TTL (clamped to [250, 60000])
//! delete_status_ttl_ms = 5000   # d-52: F3 batch-delete outcome TTL (clamped to [250, 60000])
//! push_status_ttl_ms = 5000     # d-64: F1 push-outcome TTL (clamped to [250, 60000])
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
    pub daemon: DaemonDefaults,
    pub keys: KeysDefaults,
}

/// keys-1/2/3: operator key remapping for the global keys. Covers quit
/// (keys-1), refresh (keys-2), and the pane-switch digit aliases
/// (keys-3); later slices extend to per-screen keys. Each binding is a
/// single character claimed only on a PLAIN press (no Ctrl/Alt) so it
/// can't hijack a chord for that character. See [`Self::resolved`] for
/// the collision policy.
#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct KeysDefaults {
    /// The primary quit key — a single character (default `"q"`). `Esc`
    /// and `Ctrl+C` always quit regardless, as failsafes, so a typo here
    /// can never lock the operator in. A value that isn't exactly one
    /// character is ignored (warn + fall back to `q`).
    pub quit: String,
    /// The refresh key — a single character (default `"r"`). Re-runs the
    /// active pane's fetch. A non-single-character value is ignored
    /// (warn + fall back to `r`). `Ctrl+R` (config reload) is unaffected.
    pub refresh: String,
    /// keys-3: the digit-alias pane-switch keys (defaults `"1"`/`"2"`/
    /// `"3"`/`"4"` → F1/F2/F3/F4). The function keys F1-F4 always
    /// navigate too (conventional, not remappable); these are the
    /// remappable plain-char aliases for terminals that drop F-keys.
    /// Non-single-char values fall back to the default digit.
    pub pane_f1: String,
    pub pane_f2: String,
    pub pane_f3: String,
    pub pane_f4: String,
}

/// keys-3: the effective global key bindings after the collision policy
/// is applied. `None` means the configured binding collided with a
/// higher-precedence one and is disabled (see [`KeysDefaults::resolved`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedKeys {
    pub quit: char,
    /// Pane-switch digit aliases, F1..F4 order.
    pub nav: [Option<char>; 4],
    pub refresh: Option<char>,
}

impl KeysDefaults {
    pub const DEFAULT_QUIT: char = 'q';
    pub const DEFAULT_REFRESH: char = 'r';
    /// Default pane-switch digit aliases, F1..F4 order.
    pub const DEFAULT_PANE: [char; 4] = ['1', '2', '3', '4'];

    /// The configured quit key as a single `char`, or `None` when the
    /// value isn't exactly one character (caller warns + falls back to
    /// [`Self::DEFAULT_QUIT`]).
    pub fn quit_char(&self) -> Option<char> {
        single_char(&self.quit)
    }

    /// The configured refresh key as a single `char`, or `None` (caller
    /// warns + falls back to [`Self::DEFAULT_REFRESH`]).
    pub fn refresh_char(&self) -> Option<char> {
        single_char(&self.refresh)
    }

    /// The configured pane-switch keys (F1..F4), each a single `char` or
    /// `None` when not a single character (caller warns + falls back to
    /// the corresponding [`Self::DEFAULT_PANE`] digit).
    pub fn pane_chars(&self) -> [Option<char>; 4] {
        [
            single_char(&self.pane_f1),
            single_char(&self.pane_f2),
            single_char(&self.pane_f3),
            single_char(&self.pane_f4),
        ]
    }

    /// Resolve the effective global bindings, applying the **collision
    /// policy**. `key_action` dispatches in a fixed order — quit, then
    /// the pane-switch aliases, then refresh — so a lower-precedence
    /// binding sharing a character with a higher one would be silently
    /// unreachable. Policy: process bindings in that precedence order;
    /// the first claim on a character wins, and any later binding that
    /// collides is **disabled** (`None`). Each char is the configured
    /// value, or its default when not a single character. A startup
    /// warning flags every disabled binding.
    pub fn resolved(&self) -> ResolvedKeys {
        let quit = self.quit_char().unwrap_or(Self::DEFAULT_QUIT);
        let pane_cfg = self.pane_chars();
        let refresh_cfg = self.refresh_char().unwrap_or(Self::DEFAULT_REFRESH);

        // Greedy claim in dispatch-precedence order: quit > nav1..4 >
        // refresh. `claimed.insert` returns false when the char is
        // already taken → that binding is disabled.
        let mut claimed = std::collections::HashSet::new();
        claimed.insert(quit);

        let mut nav = [None; 4];
        for (i, slot) in nav.iter_mut().enumerate() {
            let c = pane_cfg[i].unwrap_or(Self::DEFAULT_PANE[i]);
            if claimed.insert(c) {
                *slot = Some(c);
            }
        }

        let refresh = if claimed.insert(refresh_cfg) {
            Some(refresh_cfg)
        } else {
            None
        };

        ResolvedKeys { quit, nav, refresh }
    }
}

/// A config string that must be exactly one character → that `char`,
/// else `None` (empty or multi-character).
fn single_char(value: &str) -> Option<char> {
    let mut chars = value.chars();
    match (chars.next(), chars.next()) {
        (Some(c), None) => Some(c),
        _ => None,
    }
}

impl Default for KeysDefaults {
    fn default() -> Self {
        Self {
            quit: Self::DEFAULT_QUIT.to_string(),
            refresh: Self::DEFAULT_REFRESH.to_string(),
            pane_f1: Self::DEFAULT_PANE[0].to_string(),
            pane_f2: Self::DEFAULT_PANE[1].to_string(),
            pane_f3: Self::DEFAULT_PANE[2].to_string(),
            pane_f4: Self::DEFAULT_PANE[3].to_string(),
        }
    }
}

/// e-8: launch-time daemon defaults.
#[derive(Debug, Default, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct DaemonDefaults {
    /// The remote the TUI points F2/F3 at when no `--remote`
    /// CLI flag is given (`host` or `host:port`, optionally
    /// with a `:/module/path`). Empty (default) means
    /// "no default remote" — the TUI launches mDNS-only, as
    /// before. The CLI flag, when present, always wins; this
    /// only fills the gap so an operator who always targets
    /// the same daemon doesn't retype it every launch.
    pub default_remote: String,
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

/// d-24 / d-29: transfer-related preferences. Currently
/// the d-23 cancel-status TTL and the d-29 opt-in
/// cancel-confirm prompt.
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
    /// d-29: when `true`, pressing `K` on F2 opens a
    /// `cancel <id>? y/N` prompt instead of firing the
    /// CancelJob RPC immediately. `y` confirms, `n` or
    /// `Esc` aborts. Default `false` keeps the d-22
    /// behavior of one-keystroke cancel — cancel is
    /// reversible (the daemon stops sending bytes; data
    /// already on disk stays) so the prompt is opt-in
    /// safety for operators who type `K` reflexively.
    pub confirm_cancel: bool,
    /// d-40: milliseconds the F3 pull outcome fragment
    /// (`pulled N · X → <dest>` / `pull failed: <msg>`)
    /// stays on screen after the pull finishes, before
    /// the d-38 auto-hide sweep clears it back to Idle.
    /// A sibling of `cancel_status_ttl_ms`, kept separate
    /// so the two outcome fragments tune independently.
    /// Clamped to `[MIN_PULL_TTL_MS, MAX_PULL_TTL_MS]`.
    pub pull_status_ttl_ms: u64,
    /// d-52: milliseconds a *batch* F3 delete outcome
    /// (`deleted N item(s)`) stays on screen before the
    /// d-50 auto-hide sweep clears it. (Single-row deletes
    /// self-hide on cursor move, so they ignore this.) A
    /// sibling of `pull_status_ttl_ms`. Clamped to
    /// `[MIN_DELETE_TTL_MS, MAX_DELETE_TTL_MS]`.
    pub delete_status_ttl_ms: u64,
    /// d-64: milliseconds the F1 push outcome fragment
    /// (`pushed N · X → <dest>` / `push failed: <msg>`) stays in
    /// the F1 footer after the push finishes, before the d-64
    /// auto-hide sweep clears it back to Idle. A sibling of
    /// `pull_status_ttl_ms`. Clamped to
    /// `[MIN_PUSH_TTL_MS, MAX_PUSH_TTL_MS]`.
    pub push_status_ttl_ms: u64,
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

    /// d-40 baseline: 5 seconds, mirroring the d-38
    /// hardcoded `f3pull::F3PullState::TERMINAL_TTL` this
    /// field now overrides.
    pub const DEFAULT_PULL_TTL_MS: u64 = 5_000;
    /// Floor — same rationale as [`Self::MIN_CANCEL_TTL_MS`].
    pub const MIN_PULL_TTL_MS: u64 = 250;
    /// Ceiling — same rationale as [`Self::MAX_CANCEL_TTL_MS`].
    pub const MAX_PULL_TTL_MS: u64 = 60_000;

    /// Clamped accessor; the renderer reads this once per
    /// frame so out-of-range config values are silently
    /// normalized.
    pub fn cancel_status_ttl_ms_clamped(&self) -> u64 {
        self.cancel_status_ttl_ms
            .clamp(Self::MIN_CANCEL_TTL_MS, Self::MAX_CANCEL_TTL_MS)
    }

    /// d-40: clamped F3 pull outcome TTL. The loop reads
    /// this each frame and feeds it to
    /// `clear_terminal_if_expired`.
    pub fn pull_status_ttl_ms_clamped(&self) -> u64 {
        self.pull_status_ttl_ms
            .clamp(Self::MIN_PULL_TTL_MS, Self::MAX_PULL_TTL_MS)
    }

    /// d-52 baseline: 5s, mirroring the d-50 hardcoded
    /// `F3DelState::BATCH_TERMINAL_TTL` this field overrides.
    pub const DEFAULT_DELETE_TTL_MS: u64 = 5_000;
    /// Floor — same rationale as [`Self::MIN_CANCEL_TTL_MS`].
    pub const MIN_DELETE_TTL_MS: u64 = 250;
    /// Ceiling — same rationale as [`Self::MAX_CANCEL_TTL_MS`].
    pub const MAX_DELETE_TTL_MS: u64 = 60_000;

    /// d-52: clamped batch-delete outcome TTL. The loop reads
    /// this each frame and feeds it to the d-50
    /// `clear_terminal_if_expired` sweep.
    pub fn delete_status_ttl_ms_clamped(&self) -> u64 {
        self.delete_status_ttl_ms
            .clamp(Self::MIN_DELETE_TTL_MS, Self::MAX_DELETE_TTL_MS)
    }

    /// d-64 baseline: 5s, matching the pull/delete outcome TTLs.
    pub const DEFAULT_PUSH_TTL_MS: u64 = 5_000;
    /// Floor — same rationale as [`Self::MIN_CANCEL_TTL_MS`].
    pub const MIN_PUSH_TTL_MS: u64 = 250;
    /// Ceiling — same rationale as [`Self::MAX_CANCEL_TTL_MS`].
    pub const MAX_PUSH_TTL_MS: u64 = 60_000;

    /// d-64: clamped F1 push outcome TTL. The loop reads this each
    /// frame and feeds it to `clear_terminal_if_expired`.
    pub fn push_status_ttl_ms_clamped(&self) -> u64 {
        self.push_status_ttl_ms
            .clamp(Self::MIN_PUSH_TTL_MS, Self::MAX_PUSH_TTL_MS)
    }
}

impl Default for TransferDefaults {
    fn default() -> Self {
        Self {
            cancel_status_ttl_ms: Self::DEFAULT_CANCEL_TTL_MS,
            confirm_cancel: false,
            pull_status_ttl_ms: Self::DEFAULT_PULL_TTL_MS,
            delete_status_ttl_ms: Self::DEFAULT_DELETE_TTL_MS,
            push_status_ttl_ms: Self::DEFAULT_PUSH_TTL_MS,
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

    /// keys-1: `[keys] quit` parses and defaults to "q"; `quit_char`
    /// accepts a single char and rejects multi-char / empty values.
    #[test]
    fn keys_quit_parses_and_defaults() {
        let cfg = TuiConfig::default();
        assert_eq!(cfg.keys.quit, "q");
        assert_eq!(cfg.keys.quit_char(), Some('q'));

        let tmp = tempfile::tempdir().expect("tmp");
        let path = tmp.path().join("tui.toml");
        std::fs::write(&path, "[keys]\nquit = \"x\"\n").expect("write");
        let cfg = load_from_path(&path, |msg| panic!("unexpected warn: {msg}"));
        assert_eq!(cfg.keys.quit_char(), Some('x'));

        // Multi-char / empty values are rejected by quit_char (caller
        // warns + falls back).
        let multi = KeysDefaults {
            quit: "esc".to_string(),
            ..KeysDefaults::default()
        };
        assert_eq!(multi.quit_char(), None);
        let empty = KeysDefaults {
            quit: String::new(),
            ..KeysDefaults::default()
        };
        assert_eq!(empty.quit_char(), None);
    }

    /// keys-2: `[keys] refresh` parses and defaults to "r"; rejects
    /// multi-char / empty like quit.
    #[test]
    fn keys_refresh_parses_and_defaults() {
        let cfg = TuiConfig::default();
        assert_eq!(cfg.keys.refresh, "r");
        assert_eq!(cfg.keys.refresh_char(), Some('r'));

        let tmp = tempfile::tempdir().expect("tmp");
        let path = tmp.path().join("tui.toml");
        std::fs::write(&path, "[keys]\nrefresh = \"R\"\n").expect("write");
        let cfg = load_from_path(&path, |msg| panic!("unexpected warn: {msg}"));
        assert_eq!(cfg.keys.refresh_char(), Some('R'));
        // quit keeps its default when only refresh is set.
        assert_eq!(cfg.keys.quit_char(), Some('q'));

        let multi = KeysDefaults {
            refresh: "rr".to_string(),
            ..KeysDefaults::default()
        };
        assert_eq!(multi.refresh_char(), None);
    }

    /// keys-2 R2 / keys-3: the collision policy in dispatch precedence
    /// (quit > nav1..4 > refresh) — the first claim on a char wins,
    /// later collisions are disabled (`None`).
    #[test]
    fn keys_resolved_collision_policy() {
        // quit + refresh, default nav.
        let qr = |quit: &str, refresh: &str| KeysDefaults {
            quit: quit.to_string(),
            refresh: refresh.to_string(),
            ..KeysDefaults::default()
        };
        // Distinct → both active; nav defaults 1-4.
        let r = qr("q", "r").resolved();
        assert_eq!(r.quit, 'q');
        assert_eq!(r.refresh, Some('r'));
        assert_eq!(r.nav, [Some('1'), Some('2'), Some('3'), Some('4')]);
        // refresh == quit → refresh disabled.
        assert_eq!(qr("r", "r").resolved().refresh, None);
        // refresh = default quit char → disabled.
        assert_eq!(qr("q", "q").resolved().refresh, None);
        // Multi-char refresh falls back to 'r' (distinct from quit 'q').
        assert_eq!(qr("q", "rr").resolved().refresh, Some('r'));
        // ...but with quit 'r', the fallback 'r' collides → disabled.
        assert_eq!(qr("r", "rr").resolved().refresh, None);

        // keys-3: a nav key colliding with quit is disabled; refresh is
        // lowest precedence so it loses to a nav key on the same char.
        let cfg = KeysDefaults {
            quit: "1".to_string(),    // collides with default pane_f1 "1"
            refresh: "2".to_string(), // collides with default pane_f2 "2"
            ..KeysDefaults::default()
        };
        let r = cfg.resolved();
        assert_eq!(r.quit, '1');
        // pane_f1 "1" collides with quit → disabled; others survive.
        assert_eq!(r.nav, [None, Some('2'), Some('3'), Some('4')]);
        // refresh "2" loses to nav pane_f2 "2" (higher precedence) → None.
        assert_eq!(r.refresh, None);

        // Two nav keys set to the same char → the later one disabled.
        let cfg = KeysDefaults {
            pane_f2: "1".to_string(), // collides with default pane_f1 "1"
            ..KeysDefaults::default()
        };
        let r = cfg.resolved();
        assert_eq!(r.nav, [Some('1'), None, Some('3'), Some('4')]);
    }

    /// e-8: `[daemon] default_remote` parses, and defaults to empty
    /// (mDNS-only launch) when the section is absent.
    #[test]
    fn daemon_default_remote_parses_and_defaults_empty() {
        assert_eq!(
            TuiConfig::default().daemon.default_remote,
            "",
            "no [daemon] section → no default remote"
        );
        let tmp = tempfile::tempdir().expect("tmp");
        let path = tmp.path().join("tui.toml");
        std::fs::write(&path, "[daemon]\ndefault_remote = \"nas:9444:/m/\"\n").expect("write");
        let cfg = load_from_path(&path, |msg| panic!("unexpected warn: {msg}"));
        assert_eq!(cfg.daemon.default_remote, "nas:9444:/m/");
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
            ..TransferDefaults::default()
        };
        assert_eq!(
            cfg.cancel_status_ttl_ms_clamped(),
            TransferDefaults::MIN_CANCEL_TTL_MS
        );
        let cfg = TransferDefaults {
            cancel_status_ttl_ms: 100,
            ..TransferDefaults::default()
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
            ..TransferDefaults::default()
        };
        assert_eq!(
            cfg.cancel_status_ttl_ms_clamped(),
            TransferDefaults::MAX_CANCEL_TTL_MS
        );
        let cfg = TransferDefaults {
            cancel_status_ttl_ms: 120_000,
            ..TransferDefaults::default()
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
                ..TransferDefaults::default()
            };
            assert_eq!(
                cfg.cancel_status_ttl_ms_clamped(),
                ms,
                "in-range value passes through"
            );
        }
    }

    // d-40: [transfer] pull_status_ttl_ms clamp + parse.

    #[test]
    fn transfer_default_pull_ttl_is_5000ms() {
        let cfg = TuiConfig::default();
        assert_eq!(
            cfg.transfer.pull_status_ttl_ms,
            TransferDefaults::DEFAULT_PULL_TTL_MS
        );
        assert_eq!(
            cfg.transfer.pull_status_ttl_ms_clamped(),
            TransferDefaults::DEFAULT_PULL_TTL_MS
        );
    }

    #[test]
    fn transfer_pull_ttl_parses_from_toml() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("tui.toml");
        std::fs::write(&path, "[transfer]\npull_status_ttl_ms = 3000\n").expect("write");
        let cfg = load_from_path(&path, |_| {});
        assert_eq!(cfg.transfer.pull_status_ttl_ms, 3000);
        assert_eq!(cfg.transfer.pull_status_ttl_ms_clamped(), 3000);
    }

    // d-52: [transfer] delete_status_ttl_ms — batch-delete TTL.

    #[test]
    fn transfer_default_delete_ttl_is_5000ms() {
        let cfg = TuiConfig::default();
        assert_eq!(
            cfg.transfer.delete_status_ttl_ms,
            TransferDefaults::DEFAULT_DELETE_TTL_MS
        );
        assert_eq!(
            cfg.transfer.delete_status_ttl_ms_clamped(),
            TransferDefaults::DEFAULT_DELETE_TTL_MS
        );
    }

    #[test]
    fn transfer_delete_ttl_parses_and_clamps() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("tui.toml");
        std::fs::write(&path, "[transfer]\ndelete_status_ttl_ms = 2000\n").expect("write");
        let cfg = load_from_path(&path, |_| {});
        assert_eq!(cfg.transfer.delete_status_ttl_ms_clamped(), 2000);

        // Floor + ceiling clamp.
        let floor = TransferDefaults {
            delete_status_ttl_ms: 0,
            ..TransferDefaults::default()
        };
        assert_eq!(
            floor.delete_status_ttl_ms_clamped(),
            TransferDefaults::MIN_DELETE_TTL_MS
        );
        let ceiling = TransferDefaults {
            delete_status_ttl_ms: u64::MAX,
            ..TransferDefaults::default()
        };
        assert_eq!(
            ceiling.delete_status_ttl_ms_clamped(),
            TransferDefaults::MAX_DELETE_TTL_MS
        );
    }

    // d-64: [transfer] push_status_ttl_ms — F1 push outcome TTL.

    #[test]
    fn transfer_default_push_ttl_is_5000ms() {
        let cfg = TuiConfig::default();
        assert_eq!(
            cfg.transfer.push_status_ttl_ms,
            TransferDefaults::DEFAULT_PUSH_TTL_MS
        );
        assert_eq!(
            cfg.transfer.push_status_ttl_ms_clamped(),
            TransferDefaults::DEFAULT_PUSH_TTL_MS
        );
    }

    #[test]
    fn transfer_push_ttl_parses_and_clamps() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("tui.toml");
        std::fs::write(&path, "[transfer]\npush_status_ttl_ms = 2000\n").expect("write");
        let cfg = load_from_path(&path, |_| {});
        assert_eq!(cfg.transfer.push_status_ttl_ms_clamped(), 2000);

        let floor = TransferDefaults {
            push_status_ttl_ms: 0,
            ..TransferDefaults::default()
        };
        assert_eq!(
            floor.push_status_ttl_ms_clamped(),
            TransferDefaults::MIN_PUSH_TTL_MS
        );
        let ceiling = TransferDefaults {
            push_status_ttl_ms: u64::MAX,
            ..TransferDefaults::default()
        };
        assert_eq!(
            ceiling.push_status_ttl_ms_clamped(),
            TransferDefaults::MAX_PUSH_TTL_MS
        );
    }

    #[test]
    fn transfer_pull_ttl_clamp_floor() {
        let cfg = TransferDefaults {
            pull_status_ttl_ms: 0,
            ..TransferDefaults::default()
        };
        assert_eq!(
            cfg.pull_status_ttl_ms_clamped(),
            TransferDefaults::MIN_PULL_TTL_MS
        );
    }

    #[test]
    fn transfer_pull_ttl_clamp_ceiling() {
        let cfg = TransferDefaults {
            pull_status_ttl_ms: u64::MAX,
            ..TransferDefaults::default()
        };
        assert_eq!(
            cfg.pull_status_ttl_ms_clamped(),
            TransferDefaults::MAX_PULL_TTL_MS
        );
    }

    #[test]
    fn transfer_pull_and_cancel_ttls_are_independent() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("tui.toml");
        std::fs::write(
            &path,
            "[transfer]\ncancel_status_ttl_ms = 1000\npull_status_ttl_ms = 8000\n",
        )
        .expect("write");
        let cfg = load_from_path(&path, |_| {});
        assert_eq!(cfg.transfer.cancel_status_ttl_ms_clamped(), 1000);
        assert_eq!(cfg.transfer.pull_status_ttl_ms_clamped(), 8000);
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

    // d-29: [transfer] confirm_cancel opt-in flag.

    #[test]
    fn transfer_default_confirm_cancel_is_false() {
        let cfg = TuiConfig::default();
        assert!(
            !cfg.transfer.confirm_cancel,
            "default keeps d-22 one-keystroke cancel behavior"
        );
    }

    #[test]
    fn transfer_confirm_cancel_parses_true() {
        let tmp = tempfile::tempdir().expect("tmp");
        let path = tmp.path().join("tui.toml");
        std::fs::write(&path, "[transfer]\nconfirm_cancel = true\n").expect("write");
        let cfg = load_from_path(&path, |msg| panic!("unexpected warn: {msg}"));
        assert!(cfg.transfer.confirm_cancel);
    }

    #[test]
    fn transfer_confirm_cancel_parses_false_explicitly() {
        let tmp = tempfile::tempdir().expect("tmp");
        let path = tmp.path().join("tui.toml");
        std::fs::write(&path, "[transfer]\nconfirm_cancel = false\n").expect("write");
        let cfg = load_from_path(&path, |msg| panic!("unexpected warn: {msg}"));
        assert!(!cfg.transfer.confirm_cancel);
    }

    /// d-29 + d-24 compose: the two `[transfer]` fields
    /// are independent — TTL override doesn't change the
    /// confirm_cancel default, and vice versa.
    #[test]
    fn transfer_confirm_cancel_and_ttl_compose() {
        let tmp = tempfile::tempdir().expect("tmp");
        let path = tmp.path().join("tui.toml");
        std::fs::write(
            &path,
            "[transfer]\nconfirm_cancel = true\ncancel_status_ttl_ms = 1500\n",
        )
        .expect("write");
        let cfg = load_from_path(&path, |msg| panic!("unexpected warn: {msg}"));
        assert!(cfg.transfer.confirm_cancel);
        assert_eq!(cfg.transfer.cancel_status_ttl_ms_clamped(), 1500);
    }
}
