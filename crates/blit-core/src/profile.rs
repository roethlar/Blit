//! `profile` — local performance history summary.
//!
//! Moved from `crates/blit-cli/src/profile.rs` in A.0. No RPC;
//! reads the operator's `perf_local.jsonl` (plus the daemon store,
//! R5b) directly. The CLI keeps both formatters (JSON + text); this
//! module owns the data assembly.
//!
//! The gradient-descent predictor retired here in ph-3 of
//! `docs/plan/PERF_HISTORY_PLANNING.md` (R1, D-2026-08-20-2): it
//! predicted for no consumer, and the settled-dial seed store
//! (`crate::seed_store`) is the planning consumer instead. The
//! `predictor`/`predictor_path` JSON keys are gone with it — an
//! owner-ruled output change, not a slip.

use crate::perf_history;
use eyre::Result;

pub use crate::perf_history::{MergedRecord, PerformanceRecord, RecordOrigin, RouteAggregate};

/// What `query()` returns.
#[derive(Debug, Clone)]
pub struct ProfileReport {
    pub enabled: bool,
    /// Operator + daemon rows merged with origin labels (ph-2, R5b),
    /// oldest-first, capped to the newest `limit` of the union.
    pub records: Vec<MergedRecord>,
    /// Per-key aggregates over `records`: one row per
    /// `(origin, topology, role, initiator, peer_key)` group.
    pub aggregates: Vec<RouteAggregate>,
    /// The daemon store's history file when a distinct one was found
    /// and merged (R5b).
    pub daemon_history_path: Option<std::path::PathBuf>,
    /// Set when a daemon history file exists but could not be read.
    pub daemon_note: Option<String>,
}

/// Build a `ProfileReport` from the on-disk perf history. `limit`
/// matches the CLI's `--limit N` arg — `0` means "all records" per
/// `read_recent_records`'s contract.
pub fn query(limit: usize) -> Result<ProfileReport> {
    let enabled = perf_history::perf_history_enabled()?;
    let history = perf_history::read_merged_recent_records(limit)?;
    let aggregates = perf_history::aggregate_by_route(&history.records);

    Ok(ProfileReport {
        enabled,
        records: history.records,
        aggregates,
        daemon_history_path: history.daemon_path,
        daemon_note: history.daemon_note,
    })
}
