//! audit-7d5: `Ctrl+R` config hot-reload helpers extracted from `main.rs`
//! (behavior-preserving — verbatim move, no logic change). `reload_tui_config`
//! is the thin I/O wrapper (`config::load`); `classify_reload` is the pure
//! keep-vs-adopt decision. `ReloadBanner` itself stays in `main.rs` (it is an
//! AppState field with its own impl), referenced here via the crate-root path.

use crate::{config, ReloadBanner};
use std::time::Instant;

/// d-36: re-read `tui.toml` for a `Ctrl+R` hot-reload.
/// Returns the config to use plus the banner to show.
///
/// On a parse error, the CURRENT config is kept (the
/// loader returns defaults on failure, which would
/// silently wipe the operator's settings) and the banner
/// carries the error. On success — including a missing
/// file, which legitimately means "use defaults" — the
/// freshly-loaded config is adopted.
pub(crate) fn reload_tui_config(
    current: &config::TuiConfig,
    now: Instant,
) -> (config::TuiConfig, ReloadBanner) {
    let mut warning: Option<String> = None;
    let loaded = config::load(|msg| warning = Some(msg));
    classify_reload(loaded, warning, current, now)
}

/// Pure core of [`reload_tui_config`] — splits the I/O
/// (`config::load`) from the keep-vs-adopt decision so
/// the decision is unit-testable without touching the
/// process-global config dir (which would race under
/// parallel tests).
pub(crate) fn classify_reload(
    loaded: config::TuiConfig,
    warning: Option<String>,
    current: &config::TuiConfig,
    now: Instant,
) -> (config::TuiConfig, ReloadBanner) {
    match warning {
        Some(message) => (
            current.clone(),
            ReloadBanner {
                message: format!("reload failed: {message} — kept previous"),
                ok: false,
                shown_at: now,
            },
        ),
        None => (
            loaded,
            ReloadBanner {
                message: "config reloaded".to_string(),
                ok: true,
                shown_at: now,
            },
        ),
    }
}
