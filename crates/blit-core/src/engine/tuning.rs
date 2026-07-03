//! History-derived plan tuning for the engine's streaming strategy.
//! Moved verbatim from `orchestrator/orchestrator.rs` at ue-r2-1c.

use eyre::Result;

use crate::perf_history::TransferMode;

/// Maximum number of recent eligible records the local tuner looks
/// at. The cap exists so a recent regime change (new disk, fresh
/// install) propagates into tuning within ~20 transfers instead of
/// being diluted by older history.
const TUNING_WINDOW_SIZE: usize = 20;

/// R56-F2: select the window of recent records that should feed
/// `derive_local_plan_tuning`. Filters on `run_kind.is_real_transfer()`
/// FIRST, then the per-operation discriminants, THEN takes the
/// last `TUNING_WINDOW_SIZE`. Pre-fix the take() ran before the
/// run_kind filter, so 20 recent dry-run / null-sink records with
/// matching mode could fill the window and force tuning to fall
/// back to defaults even when older real records existed.
///
/// Extracted so the contract is unit-testable without touching
/// the global perf-history JSONL.
pub(super) fn select_tuning_window(
    history: &[crate::perf_history::PerformanceRecord],
    target_mode: TransferMode,
    compare_mode: crate::perf_history::CompareModeSnapshot,
    skip_unchanged: bool,
) -> Vec<crate::perf_history::PerformanceRecord> {
    history
        .iter()
        .rev()
        .filter(|record| record.run_kind.is_real_transfer())
        .filter(|record| record.mode == target_mode)
        // R59 finding #5: key on the full comparison policy
        // (not just `checksum: bool`) so SizeMtime / SizeOnly /
        // Force / IgnoreTimes runs don't mix into the same tuning
        // bucket. Pre-fix a session of `--size-only` runs trained
        // the SizeMtime bucket (and vice versa).
        .filter(|record| record.options.compare_mode == compare_mode)
        .filter(|record| record.options.skip_unchanged == skip_unchanged)
        .filter(|record| record.fast_path.as_deref() != Some("tiny_manifest"))
        // R58-followup: require a tuning signal. `derive_local_plan_tuning`
        // only aggregates `tar_shard_*` + `raw_bundle_*`; records with
        // `tar_shard_tasks == 0 && raw_bundle_tasks == 0` (no_work,
        // journal_no_work, single_huge_file, streaming no-ops) are
        // RunKind::Real and pass every other gate but contribute
        // nothing. Pre-fix they could fill the 20-slot window and
        // hide older bucket-bearing records. If the tuner ever
        // starts consuming `large_tasks`, add it here too.
        .filter(|record| record.tar_shard_tasks > 0 || record.raw_bundle_tasks > 0)
        .take(TUNING_WINDOW_SIZE)
        .cloned()
        .collect()
}

/// R57-F1: wrapper that always reads the FULL history before
/// applying the run_kind filter. The caller used to pass
/// `read_recent_records(50)`, which pre-capped the input slice
/// at 50 records — so 50 recent non-real records could hide
/// older real records before `select_tuning_window` ever saw
/// them. Baking the "ask for all records" invariant into the
/// wrapper means the limit can't drift back to a finite value.
/// The history file is already size-capped at ~1 MiB upstream
/// (DEFAULT_MAX_BYTES in perf_history.rs), so reading all
/// records is bounded.
///
/// Generic over the reader so unit tests can inject a synthetic
/// history; production passes `read_recent_records` directly.
/// Returns `None` if the reader errored OR no eligible records
/// were found; the caller treats either case as "fall back to
/// defaults."
pub(super) fn select_tuning_window_from_history<F>(
    reader: F,
    target_mode: TransferMode,
    compare_mode: crate::perf_history::CompareModeSnapshot,
    skip_unchanged: bool,
) -> Option<Vec<crate::perf_history::PerformanceRecord>>
where
    F: FnOnce(usize) -> Result<Vec<crate::perf_history::PerformanceRecord>>,
{
    // `0` means "all records" per read_recent_records' contract
    // (see read_records_from_path in perf_history.rs:298). This
    // is the load-bearing literal — passing anything else
    // reintroduces R57-F1.
    let history = reader(0).ok()?;
    let window = select_tuning_window(&history, target_mode, compare_mode, skip_unchanged);
    if window.is_empty() {
        None
    } else {
        Some(window)
    }
}

#[cfg(test)]
mod select_tuning_window_tests {
    //! R56-F2: ensure non-real records are filtered BEFORE the
    //! 20-record window, not after. Pre-fix, recent
    //! dry-run/null-sink records with matching mode could fill the
    //! window and force tuning to fall back to defaults even when
    //! older real records existed.

    use super::*;
    use crate::auto_tune::derive_local_plan_tuning;
    use crate::perf_history::{
        CompareModeSnapshot, OptionSnapshot, PerformanceRecord, RunKind, TransferMode,
    };
    use eyre::eyre;

    fn record(
        kind: RunKind,
        mode: TransferMode,
        tar_tasks: u32,
        tar_bytes: u64,
        timestamp_ms: u128,
    ) -> PerformanceRecord {
        let mut r = PerformanceRecord::new(
            mode,
            None,
            None,
            10,
            1024,
            OptionSnapshot {
                dry_run: false,
                preserve_symlinks: true,
                include_symlinks: false,
                skip_unchanged: true,
                checksum: false,
                compare_mode: CompareModeSnapshot::SizeMtime,
                workers: 4,
            },
            None,
            10,
            100,
            0,
            0,
        );
        r.run_kind = kind;
        r.tar_shard_tasks = tar_tasks;
        r.tar_shard_files = tar_tasks * 100;
        r.tar_shard_bytes = tar_bytes;
        r.timestamp_epoch_ms = timestamp_ms;
        r
    }

    /// 30 recent NullSink records (matching the target operation
    /// shape) followed by 5 older Real records. Pre-fix .take(20)
    /// ran first, grabbed 20 NullSinks, derive_local_plan_tuning
    /// skipped them all internally and returned None — tuning
    /// fell back to defaults despite real history being available.
    /// Post-fix, the filter eats the NullSinks before the take, so
    /// the 5 Real records make it through and tuning succeeds.
    #[test]
    fn null_sink_records_do_not_crowd_out_older_real_records() {
        let mut history = Vec::new();
        // Older real records (timestamps lowest = oldest).
        for i in 0..5 {
            history.push(record(
                RunKind::Real,
                TransferMode::Copy,
                4,
                16 * 1024 * 1024,
                100 + i,
            ));
        }
        // Recent null-sink records (higher timestamps = more recent).
        for i in 0..30 {
            history.push(record(
                RunKind::NullSink,
                TransferMode::Copy,
                4,
                512 * 1024 * 1024,
                10_000 + i,
            ));
        }

        let window = select_tuning_window(
            &history,
            TransferMode::Copy,
            CompareModeSnapshot::SizeMtime,
            true,
        );
        assert!(
            !window.is_empty(),
            "real records must reach the window; 30 NullSink records crowded them out pre-R56-F2"
        );
        assert!(
            window.iter().all(|r| r.run_kind.is_real_transfer()),
            "only Real records should land in the tuning window"
        );
        // derive_local_plan_tuning succeeds → tuner sees its 5 Real
        // records with 16 MiB tar bytes / 4 tar tasks = 4 MiB avg
        // (clamped to the 4 MiB floor).
        let tuning = derive_local_plan_tuning(&window).expect("tuning must succeed");
        assert!(tuning.small_target_bytes >= 4 * 1024 * 1024);
        assert!(tuning.small_target_bytes <= 16 * 1024 * 1024);
    }

    #[test]
    fn dry_run_records_do_not_crowd_out_real_records() {
        let mut history = Vec::new();
        for i in 0..3 {
            history.push(record(
                RunKind::Real,
                TransferMode::Copy,
                2,
                8 * 1024 * 1024,
                100 + i,
            ));
        }
        for i in 0..25 {
            history.push(record(
                RunKind::DryRun,
                TransferMode::Copy,
                10,
                1024 * 1024 * 1024,
                10_000 + i,
            ));
        }
        let window = select_tuning_window(
            &history,
            TransferMode::Copy,
            CompareModeSnapshot::SizeMtime,
            true,
        );
        assert_eq!(
            window.len(),
            3,
            "expected the 3 real records, got {} entries",
            window.len()
        );
        assert!(derive_local_plan_tuning(&window).is_some());
    }

    #[test]
    fn bench_records_do_not_crowd_out_real_records() {
        let mut history = Vec::new();
        for i in 0..2 {
            history.push(record(
                RunKind::Real,
                TransferMode::Copy,
                1,
                4 * 1024 * 1024,
                100 + i,
            ));
        }
        for i in 0..50 {
            history.push(record(
                RunKind::BenchTransfer,
                TransferMode::Copy,
                100,
                512 * 1024 * 1024,
                10_000 + i,
            ));
        }
        for i in 0..50 {
            history.push(record(
                RunKind::BenchWire,
                TransferMode::Copy,
                100,
                512 * 1024 * 1024,
                20_000 + i,
            ));
        }
        let window = select_tuning_window(
            &history,
            TransferMode::Copy,
            CompareModeSnapshot::SizeMtime,
            true,
        );
        assert_eq!(window.len(), 2);
        assert!(window.iter().all(|r| r.run_kind == RunKind::Real));
    }

    /// Sanity: with abundant real records, the window caps at 20.
    #[test]
    fn window_caps_at_20_real_records() {
        let history: Vec<_> = (0..50)
            .map(|i| {
                record(
                    RunKind::Real,
                    TransferMode::Copy,
                    2,
                    8 * 1024 * 1024,
                    100 + i,
                )
            })
            .collect();
        let window = select_tuning_window(
            &history,
            TransferMode::Copy,
            CompareModeSnapshot::SizeMtime,
            true,
        );
        assert_eq!(window.len(), 20, "expected the 20 most recent real records");
    }

    /// R57-F1 regression: the call site is now
    /// `select_tuning_window_from_history` which bakes the
    /// "ask for all records" invariant into the wrapper — see
    /// the dedicated tests below for the synthetic-reader
    /// regression that catches a future drift back to a finite
    /// limit. The pure-helper test below verifies that the
    /// in-function logic copes with arbitrarily large histories
    /// even if the wrapper were bypassed.
    #[test]
    fn handles_large_history_with_non_real_records_at_the_front() {
        let mut history = Vec::new();
        // 200 recent NullSink records (would have fit inside the
        // old 50-record pre-cap with room to spare).
        for i in 0..200 {
            history.push(record(
                RunKind::NullSink,
                TransferMode::Copy,
                4,
                512 * 1024 * 1024,
                10_000 + i,
            ));
        }
        // 5 older Real records (would never have been seen with
        // pre-cap=50, since the 200 NullSinks alone exceed it).
        for i in 0..5 {
            history.push(record(
                RunKind::Real,
                TransferMode::Copy,
                4,
                16 * 1024 * 1024,
                100 + i,
            ));
        }
        // Real records were appended last (highest timestamps);
        // select_tuning_window iterates .rev() so they come first.
        let window = select_tuning_window(
            &history,
            TransferMode::Copy,
            CompareModeSnapshot::SizeMtime,
            true,
        );
        assert_eq!(
            window.len(),
            5,
            "expected the 5 real records to survive a flood of non-real history"
        );
        assert!(window.iter().all(|r| r.run_kind.is_real_transfer()));
        assert!(derive_local_plan_tuning(&window).is_some());
    }

    /// R58-followup: Real records with no tuning signal
    /// (`tar_shard_tasks == 0 && raw_bundle_tasks == 0`) must not
    /// crowd out older bucket-bearing records. These exist when a
    /// run took the no_work / journal_no_work / single_huge_file
    /// fast-path or was a streaming run that copied nothing — they
    /// pass `is_real_transfer`, pass the per-operation discriminants,
    /// pass the !=tiny_manifest gate, but contribute zero to
    /// `derive_local_plan_tuning`. Pre-fix the 20-record window
    /// could fill with them and the tuner fell back to defaults.
    #[test]
    fn no_signal_real_records_do_not_crowd_out_bucket_bearing_records() {
        let mut history = Vec::new();
        // 5 older Real records WITH bucket signal (timestamps lowest).
        for i in 0..5 {
            history.push(record(
                RunKind::Real,
                TransferMode::Copy,
                4,
                16 * 1024 * 1024,
                100 + i,
            ));
        }
        // 30 recent Real records WITHOUT bucket signal: tar_tasks=0,
        // bytes=0 — same shape `single_huge_file` / `no_work` /
        // `journal_no_work` / streaming-no-op records produce.
        for i in 0..30 {
            let mut r = record(RunKind::Real, TransferMode::Copy, 0, 0, 10_000 + i);
            // Vary fast_path across the no-signal categories to
            // mirror real history. None of these exclude the record
            // from the existing gates.
            r.fast_path = match i % 4 {
                0 => Some("no_work".to_string()),
                1 => Some("journal_no_work".to_string()),
                2 => Some("single_huge_file".to_string()),
                _ => None,
            };
            history.push(r);
        }

        let window = select_tuning_window(
            &history,
            TransferMode::Copy,
            CompareModeSnapshot::SizeMtime,
            true,
        );
        assert!(
            !window.is_empty(),
            "older bucket-bearing records must reach the window; \
             30 no-signal Real records crowded them out pre-fix"
        );
        assert!(
            window
                .iter()
                .all(|r| r.tar_shard_tasks > 0 || r.raw_bundle_tasks > 0),
            "every record in the window must carry a tuning signal"
        );
        assert!(
            derive_local_plan_tuning(&window).is_some(),
            "tuner must return a value, not fall back to defaults"
        );
    }

    // ── R57-F1: wrapper's "ask for all records" invariant ────────────
    //
    // The bug class isn't about what `select_tuning_window` does
    // with a slice; it's about which slice the caller passes in.
    // `select_tuning_window_from_history` wraps the reader call so
    // a future maintainer can't drift the limit back to a finite
    // value. These tests catch that drift by asserting on the
    // limit value the wrapper passes to its reader.

    use std::cell::Cell;
    use std::rc::Rc;

    /// Captures the `limit` argument every call to the reader.
    /// The reader returns a fixed slice; we just want to see what
    /// the wrapper asks for.
    fn recording_reader(
        captured_limit: Rc<Cell<Option<usize>>>,
        records: Vec<PerformanceRecord>,
    ) -> impl FnOnce(usize) -> Result<Vec<PerformanceRecord>> {
        move |limit| {
            captured_limit.set(Some(limit));
            Ok(records)
        }
    }

    #[test]
    fn wrapper_passes_zero_to_reader() {
        let captured: Rc<Cell<Option<usize>>> = Rc::new(Cell::new(None));
        let reader = recording_reader(captured.clone(), vec![]);
        let _ = select_tuning_window_from_history(
            reader,
            TransferMode::Copy,
            CompareModeSnapshot::SizeMtime,
            true,
        );
        assert_eq!(
            captured.get(),
            Some(0),
            "R57-F1: the wrapper must ask for all records (limit=0); any \
             finite limit reintroduces the JSONL-layer crowd-out bug"
        );
    }

    #[test]
    fn wrapper_returns_none_when_reader_errors() {
        let reader = |_limit: usize| -> Result<Vec<PerformanceRecord>> {
            Err(eyre!("simulated read failure"))
        };
        let result = select_tuning_window_from_history(
            reader,
            TransferMode::Copy,
            CompareModeSnapshot::SizeMtime,
            true,
        );
        assert!(result.is_none());
    }

    #[test]
    fn wrapper_returns_none_when_no_eligible_records() {
        let captured: Rc<Cell<Option<usize>>> = Rc::new(Cell::new(None));
        let reader = recording_reader(
            captured,
            vec![
                record(RunKind::DryRun, TransferMode::Copy, 4, 1024 * 1024, 100),
                record(RunKind::NullSink, TransferMode::Copy, 4, 1024 * 1024, 200),
            ],
        );
        let result = select_tuning_window_from_history(
            reader,
            TransferMode::Copy,
            CompareModeSnapshot::SizeMtime,
            true,
        );
        assert!(result.is_none());
    }

    #[test]
    fn wrapper_returns_some_window_when_real_records_present() {
        let captured: Rc<Cell<Option<usize>>> = Rc::new(Cell::new(None));
        let reader = recording_reader(
            captured.clone(),
            vec![
                record(RunKind::Real, TransferMode::Copy, 4, 16 * 1024 * 1024, 100),
                record(RunKind::DryRun, TransferMode::Copy, 4, 1024 * 1024, 200),
            ],
        );
        let result = select_tuning_window_from_history(
            reader,
            TransferMode::Copy,
            CompareModeSnapshot::SizeMtime,
            true,
        )
        .unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].run_kind, RunKind::Real);
        assert_eq!(captured.get(), Some(0));
    }

    /// Sanity: mode and option filters still apply post-R56-F2.
    /// A Real record with the wrong mode/checksum/skip_unchanged
    /// must NOT land in the window.
    #[test]
    fn mode_and_option_filters_still_apply() {
        let mut history = Vec::new();
        // Real Mirror records (wrong mode).
        for i in 0..10 {
            history.push(record(
                RunKind::Real,
                TransferMode::Mirror,
                4,
                16 * 1024 * 1024,
                100 + i,
            ));
        }
        // Real Copy record.
        history.push(record(
            RunKind::Real,
            TransferMode::Copy,
            2,
            8 * 1024 * 1024,
            500,
        ));
        let window = select_tuning_window(
            &history,
            TransferMode::Copy,
            CompareModeSnapshot::SizeMtime,
            true,
        );
        assert_eq!(window.len(), 1);
        assert_eq!(window[0].mode, TransferMode::Copy);
    }

    /// R59 finding #5: SizeOnly / Force / IgnoreTimes runs must
    /// not contaminate the SizeMtime tuning bucket. Pre-fix the
    /// window filtered on `options.checksum == checksum_bool`, so a
    /// `--size-only` run (checksum=false) landed in the same bucket
    /// as a default `SizeMtime` run.
    #[test]
    fn compare_mode_buckets_are_separate() {
        let mut history = Vec::new();
        // 10 SizeOnly Real records (signal-bearing).
        for i in 0..10 {
            let mut r = record(
                RunKind::Real,
                TransferMode::Copy,
                4,
                16 * 1024 * 1024,
                100 + i,
            );
            r.options.compare_mode = CompareModeSnapshot::SizeOnly;
            history.push(r);
        }
        // One SizeMtime Real record.
        let mut sm = record(RunKind::Real, TransferMode::Copy, 2, 8 * 1024 * 1024, 500);
        sm.options.compare_mode = CompareModeSnapshot::SizeMtime;
        history.push(sm);

        // Querying SizeMtime must NOT pick up the 10 SizeOnly records.
        let window = select_tuning_window(
            &history,
            TransferMode::Copy,
            CompareModeSnapshot::SizeMtime,
            true,
        );
        assert_eq!(window.len(), 1);
        assert_eq!(
            window[0].options.compare_mode,
            CompareModeSnapshot::SizeMtime
        );

        // Querying SizeOnly returns the SizeOnly records.
        let window = select_tuning_window(
            &history,
            TransferMode::Copy,
            CompareModeSnapshot::SizeOnly,
            true,
        );
        assert_eq!(window.len(), 10);
        assert!(window
            .iter()
            .all(|r| r.options.compare_mode == CompareModeSnapshot::SizeOnly));
    }
}
