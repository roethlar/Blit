use crate::perf_history::{append_local_record, OptionSnapshot, PerformanceRecord, TransferMode};
use crate::perf_predictor::PerformancePredictor;

use super::{LocalMirrorOptions, LocalMirrorSummary};

pub(super) fn record_performance_history(
    summary: &LocalMirrorSummary,
    options: &LocalMirrorOptions,
    fast_path: Option<&str>,
    planner_duration_ms: u128,
    transfer_duration_ms: u128,
) -> Option<PerformanceRecord> {
    if std::env::var("BLIT_DISABLE_PERF_HISTORY")
        .map(|v| matches!(v.trim(), "1" | "true" | "TRUE"))
        .unwrap_or(false)
    {
        return None;
    }

    let options_snapshot = OptionSnapshot {
        dry_run: options.dry_run,
        preserve_symlinks: options.preserve_symlinks,
        include_symlinks: options.include_symlinks,
        skip_unchanged: options.skip_unchanged,
        checksum: options.checksum,
        workers: options.workers,
    };

    let mode = if options.mirror {
        TransferMode::Mirror
    } else {
        TransferMode::Copy
    };

    let record = PerformanceRecord::new(
        mode,
        None,
        None,
        summary.copied_files,
        summary.total_bytes,
        options_snapshot,
        fast_path.map(|s| s.to_string()),
        planner_duration_ms,
        transfer_duration_ms,
        0,
        0,
    );

    if let Err(err) = append_local_record(&record) {
        if options.verbose {
            eprintln!("Failed to update performance history: {err:?}");
        }
    }
    Some(record)
}

pub(super) fn update_predictor(
    predictor: &mut Option<PerformancePredictor>,
    record: &PerformanceRecord,
    verbose: bool,
) {
    if let Some(ref mut predictor) = predictor {
        predictor.observe(record);
        if let Err(err) = predictor.save() {
            if verbose {
                eprintln!("Failed to persist predictor state: {err:?}");
            }
        }
    }
}
