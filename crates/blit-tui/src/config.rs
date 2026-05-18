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
