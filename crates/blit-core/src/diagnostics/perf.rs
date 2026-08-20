//! Performance-history toggles + granular reads.
//!
//! Moved from `crates/blit-cli/src/diagnostics.rs` (perf path)
//! in A.0. Thin wrappers around `crate::perf_history`.

use crate::perf_history;
use eyre::Result;
use std::path::PathBuf;

pub use crate::perf_history::{
    MergedHistory, MergedRecord, PerformanceRecord, RecordOrigin, RouteAggregate,
};

/// Read the persisted "perf history enabled" flag. Separate
/// so the CLI's `diagnostics perf` verb can treat the post-toggle
/// refresh as best-effort: a malformed
/// `settings.json` shows the startup warning and falls back to
/// the caller's existing value rather than failing the verb.
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
/// `crate::perf_history::read_recent_records`'s contract.
pub fn read_records(limit: usize) -> Result<Vec<PerformanceRecord>> {
    perf_history::read_recent_records(limit)
}

/// Operator + daemon stores merged with origin labels (ph-2, R5b);
/// `limit` caps the merged union to its newest rows (`0` = all).
pub fn read_merged(limit: usize) -> Result<MergedHistory> {
    perf_history::read_merged_recent_records(limit)
}

/// Per-key aggregate rows over merged records — one per
/// `(origin, topology, role, initiator, peer_key)` group, in
/// deterministic label order.
pub fn aggregate(records: &[MergedRecord]) -> Vec<RouteAggregate> {
    perf_history::aggregate_by_route(records)
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
