//! `profile` — local performance history summary + predictor
//! coefficients.
//!
//! Moved from `crates/blit-cli/src/profile.rs` in A.0. No RPC;
//! reads `~/.config/blit/perf_local.jsonl` and the predictor
//! state file directly. The CLI keeps both formatters (JSON +
//! text); this module owns the data assembly.
//!
//! Preserves the pre-A.0 "predictor loaded but empty" vs
//! "predictor failed to load" distinction — see `ProfileReport`
//! field doc for why that matters for JSON shape parity.

use blit_core::perf_history;
use blit_core::perf_history::TransferMode;
use blit_core::perf_predictor::PerformancePredictor;
use eyre::Result;
use std::path::PathBuf;

pub use blit_core::perf_history::PerformanceRecord;
pub use blit_core::perf_predictor::DurationCoefficients;

/// One side of the predictor's mode-keyed profile (Copy / Mirror).
/// `coefficients` is `None` when no training data exists for the
/// queried key; `observations` and `fallback_depth` describe how
/// the predictor reached the answer (or what it didn't find).
#[derive(Debug, Clone)]
pub struct ProfileSummary {
    pub coefficients: Option<DurationCoefficients>,
    pub observations: u64,
    pub fallback_depth: usize,
}

/// Per-mode predictor coefficients. Always carries both sides;
/// the CLI's text path prints them as two blocks and the JSON
/// path serializes as a two-field object.
#[derive(Debug, Clone)]
pub struct PredictorReport {
    pub copy: ProfileSummary,
    pub mirror: ProfileSummary,
}

/// What `query()` returns. The `predictor` field is wrapped in
/// `Option` so that "predictor file failed to load" stays
/// distinguishable from "predictor file loaded but has no
/// training data" — the JSON output emits `"predictor": null`
/// in the first case and `"predictor": { "copy": ..., "mirror":
/// ... }` (with possibly-null inner coefficients) in the second.
/// Pre-A.0 the CLI's `predictor_summary: Option<(_, _)>` local
/// carried the same distinction.
#[derive(Debug, Clone)]
pub struct ProfileReport {
    pub enabled: bool,
    pub records: Vec<PerformanceRecord>,
    pub predictor_path: Option<PathBuf>,
    pub predictor: Option<PredictorReport>,
}

/// Build a `ProfileReport` from the on-disk perf history and
/// predictor state. `limit` matches the CLI's `--limit N` arg —
/// `0` means "all records" per `read_recent_records`'s contract.
pub fn query(limit: usize) -> Result<ProfileReport> {
    let enabled = perf_history::perf_history_enabled()?;
    let records = perf_history::read_recent_records(limit)?;
    let predictor = PerformancePredictor::load().ok();
    let predictor_path = predictor
        .as_ref()
        .map(|pred| pred.path().to_path_buf())
        .filter(|p| p.exists());

    let predictor = predictor.as_ref().map(|pred| PredictorReport {
        copy: summarize_mode(pred, TransferMode::Copy),
        mirror: summarize_mode(pred, TransferMode::Mirror),
    });

    Ok(ProfileReport {
        enabled,
        records,
        predictor_path,
        predictor,
    })
}

/// Probe the predictor for a mode with all secondary key
/// components defaulted. Exercises the same fallback chain
/// `predict()` walks, looking for the broadest profile that has
/// training data.
fn summarize_mode(predictor: &PerformancePredictor, mode: TransferMode) -> ProfileSummary {
    match predictor.coefficients_for(mode, None, None, None, true, false) {
        Some((coeffs, obs, depth)) => ProfileSummary {
            coefficients: Some(coeffs),
            observations: obs,
            fallback_depth: depth,
        },
        None => ProfileSummary {
            coefficients: None,
            observations: 0,
            fallback_depth: 0,
        },
    }
}
