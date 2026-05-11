//! Auto-tuning and adaptive defaults for network transfers
//!
//! Provides warmup probes and heuristics for chunk sizing, stream counts,
//! and buffer allocation based on network characteristics.

/// Tuning parameters determined by warmup and workload analysis
#[derive(Debug, Clone)]
pub struct TuningParams {
    /// Chunk size in bytes for network I/O
    pub chunk_bytes: usize,
    /// Initial number of parallel streams
    pub initial_streams: usize,
    /// Maximum parallel streams
    pub max_streams: usize,
    /// Detected bandwidth (if warmup succeeded)
    pub warmup_gbps: Option<f64>,
    /// TCP buffer size (SO_SNDBUF/SO_RCVBUF)
    pub tcp_buffer_size: Option<usize>,
    /// Number of payloads to prefetch
    pub prefetch_count: Option<usize>,
}

/// Analyze warmup results and determine optimal chunk size
///
/// Helper for interpreting warmup probe bandwidth measurements.
pub fn analyze_warmup_result(gbps: f64) -> usize {
    if gbps >= 6.0 {
        32 * 1024 * 1024 // High bandwidth
    } else {
        16 * 1024 * 1024 // Standard
    }
}

/// Determine tuning parameters based on plan and optional warmup
pub fn determine_tuning(
    default_chunk_bytes: usize,
    warmup_result: Option<(f64, usize)>,
) -> TuningParams {
    let (warmup_gbps, chunk_bytes) = match warmup_result {
        Some((gbps, chunk)) => (Some(gbps), chunk),
        None => (None, default_chunk_bytes),
    };

    // Initial streams based on detected bandwidth
    let initial_streams = if let Some(gbps) = warmup_gbps {
        if gbps > 8.0 {
            6 // 10GbE or better
        } else if gbps > 3.0 {
            4 // Multi-gigabit
        } else {
            2 // Gigabit
        }
    } else {
        2
    };

    let (tcp_buffer_size, prefetch_count) = if let Some(gbps) = warmup_gbps {
        if gbps > 8.0 {
            (Some(8 * 1024 * 1024), Some(32)) // 10GbE
        } else if gbps > 3.0 {
            (Some(4 * 1024 * 1024), Some(16)) // Multi-gig
        } else {
            (Some(1024 * 1024), Some(8)) // Gigabit
        }
    } else {
        (None, None)
    };

    TuningParams {
        chunk_bytes,
        initial_streams,
        max_streams: 8,
        warmup_gbps,
        tcp_buffer_size,
        prefetch_count,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tuning_with_high_bandwidth() {
        let params = determine_tuning(16 * 1024 * 1024, Some((9.5, 32 * 1024 * 1024)));
        assert_eq!(params.chunk_bytes, 32 * 1024 * 1024);
        assert_eq!(params.initial_streams, 6);
        assert_eq!(params.max_streams, 8);
    }

    #[test]
    fn test_tuning_fallback() {
        let params = determine_tuning(16 * 1024 * 1024, None);
        assert_eq!(params.chunk_bytes, 16 * 1024 * 1024);
        assert_eq!(params.initial_streams, 2);
    }
}

/// Local plan tuning derived from historical performance records.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LocalPlanTuning {
    pub small_target_bytes: u64,
    pub small_count_target: usize,
    pub medium_target_bytes: u64,
}

/// Derive local planning thresholds from recent performance history.
///
/// R56-F1: only real-transfer records contribute to the aggregate.
/// Dry-run records have zero write bytes by design, null-sink
/// records have zero write bytes by definition, and bench records
/// represent diagnostic workloads — including any of them would
/// teach the tuner that destination writes are cheap or free, and
/// that's exactly the contamination the run_kind lane was added
/// to prevent.
pub fn derive_local_plan_tuning(
    records: &[crate::perf_history::PerformanceRecord],
) -> Option<LocalPlanTuning> {
    if records.is_empty() {
        return None;
    }

    let mut tar_tasks: u64 = 0;
    let mut tar_bytes: u128 = 0;
    let mut tar_files: u64 = 0;
    let mut raw_tasks: u64 = 0;
    let mut raw_bytes: u128 = 0;

    for record in records {
        if !record.run_kind.is_real_transfer() {
            continue;
        }
        tar_tasks = tar_tasks.saturating_add(record.tar_shard_tasks as u64);
        tar_bytes = tar_bytes.saturating_add(record.tar_shard_bytes as u128);
        tar_files = tar_files.saturating_add(record.tar_shard_files as u64);
        raw_tasks = raw_tasks.saturating_add(record.raw_bundle_tasks as u64);
        raw_bytes = raw_bytes.saturating_add(record.raw_bundle_bytes as u128);
    }

    if tar_tasks == 0 && raw_tasks == 0 {
        return None;
    }

    let mut small_target_bytes = 8 * 1024 * 1024;
    let mut small_count_target: usize = 2048;
    if tar_tasks > 0 && tar_bytes > 0 {
        let avg_bytes = (tar_bytes / u128::from(tar_tasks)).min(u128::from(u64::MAX)) as u64;
        small_target_bytes = avg_bytes.clamp(4 * 1024 * 1024, 128 * 1024 * 1024);
    }
    if tar_tasks > 0 && tar_files > 0 {
        let avg_files = (tar_files / tar_tasks) as usize;
        small_count_target = avg_files.clamp(128, 4096);
    }

    let mut medium_target_bytes = 128 * 1024 * 1024;
    if raw_tasks > 0 && raw_bytes > 0 {
        let avg_bytes = (raw_bytes / u128::from(raw_tasks)).min(u128::from(u64::MAX)) as u64;
        medium_target_bytes = avg_bytes.clamp(64 * 1024 * 1024, 512 * 1024 * 1024);
    }

    Some(LocalPlanTuning {
        small_target_bytes,
        small_count_target,
        medium_target_bytes,
    })
}

#[cfg(test)]
mod local_tests {
    use super::*;
    use crate::perf_history::{OptionSnapshot, PerformanceRecord, TransferMode};

    fn sample_record(
        tar_tasks: u32,
        tar_files: u32,
        tar_bytes: u64,
        raw_tasks: u32,
        raw_bytes: u64,
    ) -> PerformanceRecord {
        let mut record = PerformanceRecord::new(
            TransferMode::Copy,
            None,
            None,
            10,
            1024,
            OptionSnapshot {
                dry_run: false,
                preserve_symlinks: true,
                include_symlinks: true,
                skip_unchanged: true,
                checksum: false,
                workers: 4,
            },
            None,
            100,
            1_000,
            0,
            0,
        );
        record.tar_shard_tasks = tar_tasks;
        record.tar_shard_files = tar_files;
        record.tar_shard_bytes = tar_bytes;
        record.raw_bundle_tasks = raw_tasks;
        record.raw_bundle_bytes = raw_bytes;
        record
    }

    #[test]
    fn derive_tuning_from_history() {
        let records = vec![
            sample_record(4, 4000, 32 * 1024 * 1024, 2, 256 * 1024 * 1024),
            sample_record(2, 1800, 20 * 1024 * 1024, 1, 128 * 1024 * 1024),
        ];
        let tuning = derive_local_plan_tuning(&records).expect("tuning");
        assert!(tuning.small_target_bytes >= 4 * 1024 * 1024);
        assert!(tuning.small_target_bytes <= 128 * 1024 * 1024);
        assert!(tuning.small_count_target >= 128 && tuning.small_count_target <= 4096);
        assert!(tuning.medium_target_bytes >= 64 * 1024 * 1024);
    }

    #[test]
    fn returns_none_without_tasks() {
        let records = vec![sample_record(0, 0, 0, 0, 0)];
        assert!(derive_local_plan_tuning(&records).is_none());
    }

    // ── R56-F1: run_kind filtering ─────────────────────────────────────

    /// Records carry `run_kind` lane. Only `Real` lane contributes to
    /// the tuning aggregates. A pollutant DryRun / NullSink / Bench
    /// record sitting in the history must NOT shift the bucket
    /// targets — that was the latent bug `derive_local_plan_tuning`
    /// shipped with before R56-F1.
    fn record_in_lane(
        kind: crate::perf_history::RunKind,
        tar_tasks: u32,
        tar_bytes: u64,
        raw_tasks: u32,
        raw_bytes: u64,
    ) -> PerformanceRecord {
        let mut r = sample_record(tar_tasks, tar_tasks * 100, tar_bytes, raw_tasks, raw_bytes);
        r.run_kind = kind;
        r
    }

    #[test]
    fn tuning_ignores_dry_run_records() {
        use crate::perf_history::RunKind;
        // One real record + ten dry-run records with absurd byte
        // counts. The dry-runs must NOT pull the small_target_bytes
        // toward their bogus values.
        let mut records = vec![record_in_lane(
            RunKind::Real,
            4,
            32 * 1024 * 1024,
            2,
            256 * 1024 * 1024,
        )];
        for _ in 0..10 {
            // Massive byte counts in a dry-run — if these leaked
            // into the average we'd see them in the output.
            records.push(record_in_lane(
                RunKind::DryRun,
                4,
                512 * 1024 * 1024,
                2,
                4 * 1024 * 1024 * 1024,
            ));
        }
        let tuning = derive_local_plan_tuning(&records).expect("tuning");
        // Real record alone gives small_target_bytes = 32 MiB / 4 = 8 MiB.
        // If dry-runs leaked in, the avg would be much higher.
        assert_eq!(
            tuning.small_target_bytes,
            8 * 1024 * 1024,
            "dry-run records must not contribute to tuning aggregates"
        );
    }

    #[test]
    fn tuning_ignores_null_sink_records() {
        use crate::perf_history::RunKind;
        let mut records = vec![record_in_lane(
            RunKind::Real,
            2,
            16 * 1024 * 1024,
            1,
            128 * 1024 * 1024,
        )];
        for _ in 0..5 {
            records.push(record_in_lane(
                RunKind::NullSink,
                10,
                4 * 1024 * 1024 * 1024,
                0,
                0,
            ));
        }
        let tuning = derive_local_plan_tuning(&records).expect("tuning");
        // Real-only would give small_target_bytes = 16 MiB / 2 = 8 MiB.
        assert_eq!(tuning.small_target_bytes, 8 * 1024 * 1024);
    }

    #[test]
    fn tuning_ignores_bench_lane_records() {
        use crate::perf_history::RunKind;
        let mut records = vec![record_in_lane(
            RunKind::Real,
            1,
            8 * 1024 * 1024,
            1,
            64 * 1024 * 1024,
        )];
        for _ in 0..3 {
            records.push(record_in_lane(
                RunKind::BenchTransfer,
                100,
                256 * 1024 * 1024,
                100,
                512 * 1024 * 1024,
            ));
        }
        for _ in 0..3 {
            records.push(record_in_lane(
                RunKind::BenchWire,
                100,
                256 * 1024 * 1024,
                100,
                512 * 1024 * 1024,
            ));
        }
        let tuning = derive_local_plan_tuning(&records).expect("tuning");
        // Real-only would give 8 MiB / 1 = 8 MiB; clamp pushes to ≥4 MiB.
        assert_eq!(tuning.small_target_bytes, 8 * 1024 * 1024);
    }

    #[test]
    fn tuning_returns_none_when_only_non_real_records_present() {
        use crate::perf_history::RunKind;
        let records = vec![
            record_in_lane(RunKind::DryRun, 4, 32 * 1024 * 1024, 2, 64 * 1024 * 1024),
            record_in_lane(RunKind::NullSink, 4, 32 * 1024 * 1024, 2, 64 * 1024 * 1024),
            record_in_lane(
                RunKind::BenchTransfer,
                4,
                32 * 1024 * 1024,
                2,
                64 * 1024 * 1024,
            ),
        ];
        // With all non-Real records filtered out, tar_tasks + raw_tasks
        // both end up zero → None.
        assert!(
            derive_local_plan_tuning(&records).is_none(),
            "history with no Real records must yield no tuning"
        );
    }
}
