use crate::perf_history::{
    append_local_record, CompareModeSnapshot, OptionSnapshot, PerformanceRecord, TransferMode,
};
use crate::perf_predictor::PerformancePredictor;

use super::{LocalMirrorOptions, LocalMirrorSummary};

/// Map the orchestrator's `LocalCompareMode` onto the perf-history
/// snapshot enum so tuning records preserve the user's full intent
/// (not just `checksum: bool`).
fn snapshot_compare_mode(options: &LocalMirrorOptions) -> CompareModeSnapshot {
    options
        .compare_mode
        .resolve_compare_snapshot(options.checksum)
}

pub(super) fn record_performance_history(
    summary: &LocalMirrorSummary,
    options: &LocalMirrorOptions,
    fast_path: Option<&str>,
    planner_duration_ms: u128,
    transfer_duration_ms: u128,
) -> Option<PerformanceRecord> {
    if !options.perf_history {
        return None;
    }

    let record = build_performance_record(
        summary,
        options,
        fast_path,
        planner_duration_ms,
        transfer_duration_ms,
    );

    if let Err(err) = append_local_record(&record) {
        if options.verbose {
            eprintln!("Failed to update performance history: {err:?}");
        }
    }
    Some(record)
}

/// Construct the `PerformanceRecord` from a summary without
/// touching disk. Split out from `record_performance_history` so
/// the record-shape contract — specifically R44-F1's "train and
/// query against the same feature vector" invariant — is
/// unit-testable without writing to the global perf history file.
fn build_performance_record(
    summary: &LocalMirrorSummary,
    options: &LocalMirrorOptions,
    fast_path: Option<&str>,
    planner_duration_ms: u128,
    transfer_duration_ms: u128,
) -> PerformanceRecord {
    let options_snapshot = OptionSnapshot {
        dry_run: options.dry_run,
        preserve_symlinks: options.preserve_symlinks,
        include_symlinks: options.include_symlinks,
        skip_unchanged: options.skip_unchanged,
        checksum: options.checksum,
        compare_mode: snapshot_compare_mode(options),
        workers: options.workers,
    };

    let mode = if options.mirror {
        TransferMode::Mirror
    } else {
        TransferMode::Copy
    };

    // R44-F1: train against scanned features so the predictor's
    // training inputs match its query inputs. The orchestrator
    // queries `predict(...)` with `all_headers.len()` (scanned
    // count) and `total_bytes` (scanned bytes); pre-fix the record
    // was populated with `summary.copied_files`, so the predictor
    // saw a different feature vector at training time than at
    // query time, and predictions drifted on every incremental
    // workload. The `total_bytes` field on the record was already
    // scanned-bytes by accident; this aligns both axes deliberately.
    //
    // `summary.copied_files` and the per-bucket counts
    // (tar_shard_files / raw_bundle_files / large_tasks) still
    // reflect actual writes — they're the load-bearing inputs for
    // `derive_local_plan_tuning`'s bucket-target heuristics, which
    // are computed from observed apply behavior, not scan size.
    let mut record = PerformanceRecord::new(
        mode,
        None,
        None,
        summary.scanned_files,
        summary.scanned_bytes,
        options_snapshot,
        fast_path.map(|s| s.to_string()),
        planner_duration_ms,
        transfer_duration_ms,
        0,
        0,
    );
    record.tar_shard_tasks = summary.tar_shard_tasks as u32;
    record.tar_shard_files = summary.tar_shard_files as u32;
    record.tar_shard_bytes = summary.tar_shard_bytes;
    record.raw_bundle_tasks = summary.raw_bundle_tasks as u32;
    record.raw_bundle_files = summary.raw_bundle_files as u32;
    record.raw_bundle_bytes = summary.raw_bundle_bytes;
    record.large_tasks = summary.large_tasks as u32;
    record.large_bytes = summary.large_bytes;

    record
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

#[cfg(test)]
mod tests {
    use super::super::summary::TransferOutcome;
    use super::*;
    use std::time::Duration;

    fn options_with_mirror(mirror: bool) -> LocalMirrorOptions {
        LocalMirrorOptions {
            mirror,
            ..LocalMirrorOptions::default()
        }
    }

    /// R44-F1 contract: the record's `(file_count, total_bytes)`
    /// must mirror the orchestrator's predictor-query features.
    /// Pre-fix this assertion would have failed: the record was
    /// populated from `summary.copied_files` and `summary.total_bytes`
    /// while the query used scanned values, so on this incremental
    /// scenario (1000 scanned, 5 actually written) the predictor
    /// trained on (5, 100KB) but was queried with
    /// (1000, ~10MB).
    #[test]
    fn record_uses_scanned_features_not_copied() {
        let summary = LocalMirrorSummary {
            // Mostly-unchanged incremental run: 1000 files scanned,
            // only 5 actually written.
            scanned_files: 1000,
            scanned_bytes: 10 * 1024 * 1024,
            planned_files: 5,
            copied_files: 5,
            total_bytes: 100 * 1024,
            duration: Duration::from_millis(200),
            outcome: TransferOutcome::Transferred,
            ..LocalMirrorSummary::default()
        };
        let options = options_with_mirror(false);
        let record = build_performance_record(&summary, &options, Some("streaming"), 150, 50);

        assert_eq!(
            record.file_count, 1000,
            "record.file_count must reflect scanned (planner-side) workload, not copied count"
        );
        assert_eq!(
            record.total_bytes, summary.scanned_bytes,
            "record.total_bytes must reflect scanned bytes, not transferred bytes"
        );
        assert_eq!(record.planner_duration_ms, 150);
        assert_eq!(record.transfer_duration_ms, 50);
    }

    /// Bucket-shape fields (tar_shard_*, raw_bundle_*, large_*)
    /// must continue to reflect actual write activity — they feed
    /// `derive_local_plan_tuning` which heuristically sizes
    /// destination buckets from past apply behavior.
    #[test]
    fn bucket_counts_still_reflect_actual_writes() {
        let summary = LocalMirrorSummary {
            scanned_files: 100,
            scanned_bytes: 1_000_000,
            copied_files: 10,
            total_bytes: 50_000,
            tar_shard_tasks: 2,
            tar_shard_files: 7,
            tar_shard_bytes: 30_000,
            raw_bundle_tasks: 1,
            raw_bundle_files: 2,
            raw_bundle_bytes: 15_000,
            large_tasks: 1,
            large_bytes: 5_000,
            ..LocalMirrorSummary::default()
        };
        let options = options_with_mirror(true);
        let record = build_performance_record(&summary, &options, Some("streaming"), 30, 70);

        assert_eq!(record.tar_shard_tasks, 2);
        assert_eq!(record.tar_shard_files, 7);
        assert_eq!(record.tar_shard_bytes, 30_000);
        assert_eq!(record.raw_bundle_tasks, 1);
        assert_eq!(record.raw_bundle_files, 2);
        assert_eq!(record.raw_bundle_bytes, 15_000);
        assert_eq!(record.large_tasks, 1);
        assert_eq!(record.large_bytes, 5_000);
    }
}
