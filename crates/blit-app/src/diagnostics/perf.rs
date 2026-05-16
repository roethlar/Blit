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

/// One-call read of the three perf-history surfaces the CLI's
/// `diagnostics perf` verb cares about. `limit` matches the CLI's
/// `--limit N` arg; `0` means "all records" per
/// `read_recent_records`'s contract.
pub fn query(limit: usize) -> Result<PerfReport> {
    let enabled = perf_history::perf_history_enabled()?;
    let history_path = perf_history::config_dir()?.join("perf_local.jsonl");
    let records = perf_history::read_recent_records(limit)?;
    Ok(PerfReport {
        enabled,
        history_path,
        records,
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
