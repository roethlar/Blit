//! audit-7d: pure throughput + du-total helpers extracted from `main.rs`.
//!
//! w6-1: the three per-direction `ProgressEvent` folding rules that used
//! to live here (`accumulate_pull_progress`, `accumulate_push_progress`,
//! `accumulate_delegated_progress`) are gone — all producers now emit one
//! contract (bytes ride `Payload` only; files count once via either
//! `FileComplete` or `Payload.files`) and every forwarder folds through
//! the shared `blit_core::remote::transfer::ProgressTotals` accumulator.

/// d-39: average pull throughput in bytes/sec.
///
/// Suppressed (returns 0) until at least one second has elapsed —
/// `bytes / tiny_elapsed` produces meaningless multi-GiB/s spikes in the
/// first moments of a transfer, and the footer reads better with no rate
/// than a wrong one. After the warm-up it's a simple cumulative average
/// (`bytes / elapsed`), matching the "is it moving" intent of the footer
/// rather than an instantaneous rate.
pub(crate) fn pull_throughput(bytes: u64, elapsed_secs: f64) -> u64 {
    if elapsed_secs >= 1.0 {
        (bytes as f64 / elapsed_secs) as u64
    } else {
        0
    }
}

/// Fold one `DiskUsageEntry`'s `(bytes, files)` into the running best
/// total, keeping the entry with the largest byte count (the F3 du
/// summary reports the deepest/total measurement).
pub(crate) fn du_total_from_entries(
    acc: Option<(u64, u64)>,
    bytes: u64,
    files: u64,
) -> Option<(u64, u64)> {
    match acc {
        Some((best_bytes, _)) if best_bytes >= bytes => acc,
        _ => Some((bytes, files)),
    }
}
