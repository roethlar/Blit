//! Performance-history toggles + summary read.
//!
//! Moved from `crates/blit-cli/src/diagnostics.rs` (perf path)
//! in A.0. Thin wrappers around `blit_core::perf_history` plus a
//! `query()` helper that bundles the three reads (enabled flag,
//! history path, recent records) into a single result struct.

use blit_core::perf_history;
use eyre::Result;
use std::path::PathBuf;

pub use blit_core::perf_history::PerformanceRecord;

/// Bundle returned by [`query`]. Callers (`blit diagnostics perf`,
/// future TUI F4 diagnostics screen) render this however they
/// like — JSON, text, table cells.
#[derive(Debug, Clone)]
pub struct PerfReport {
    pub enabled: bool,
    pub history_path: PathBuf,
    pub records: Vec<PerformanceRecord>,
}

/// Read the persisted "perf history enabled" flag. Separate
/// from [`query`] because the CLI's `diagnostics perf` verb
/// treats the post-toggle refresh as best-effort: a malformed
/// `settings.json` shows the startup warning and falls back to
/// the caller's existing value rather than failing the verb.
/// Callers that want the bundled read (TUI F4 pane, scripted
/// consumers) use [`query`] and propagate the error.
pub fn read_enabled() -> Result<bool> {
    perf_history::perf_history_enabled()
}

/// Path to `perf_local.jsonl`. Pre-A.0 callers used
/// `perf_history::config_dir()?.join("perf_local.jsonl")`
/// inline; centralizing the join here keeps the filename out of
/// presenter code.
pub fn history_path() -> Result<PathBuf> {
    Ok(perf_history::config_dir()?.join("perf_local.jsonl"))
}

/// Read up to `limit` most recent records. `0` means "all" per
/// `blit_core::perf_history::read_recent_records`'s contract.
pub fn read_records(limit: usize) -> Result<Vec<PerformanceRecord>> {
    perf_history::read_recent_records(limit)
}

/// One-call read of the three perf-history surfaces. Convenience
/// for callers that want one-shot reads and don't need the
/// best-effort split (TUI's F4 pane will likely use this; the
/// CLI uses the granular functions above so it can keep
/// pre-A.0's best-effort enabled-refresh semantics).
pub fn query(limit: usize) -> Result<PerfReport> {
    Ok(PerfReport {
        enabled: read_enabled()?,
        history_path: history_path()?,
        records: read_records(limit)?,
    })
}

/// Persist a new "perf history enabled" setting. Caller refreshes
/// any in-process mirror (e.g. `AppContext.perf_history_enabled`)
/// after this returns.
pub fn set_enabled(enabled: bool) -> Result<()> {
    perf_history::set_perf_history_enabled(enabled)
}

/// Remove the on-disk history log. Returns `true` if a log
/// existed (and was deleted), `false` if there was nothing to
/// clear.
pub fn clear() -> Result<bool> {
    perf_history::clear_history()
}
