use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use eyre::{bail, eyre, Context, Result};
use tokio::runtime::Builder;

use crate::auto_tune::derive_local_plan_tuning;
use crate::change_journal::{ChangeState, ChangeTracker, ProbeToken, StoredSnapshot};
use crate::fs_enum::FileFilter;
use crate::generated::ComparisonMode;
use crate::local_worker::{copy_large_blocking, copy_paths_blocking};
use crate::perf_history::{read_recent_records, TransferMode};
use crate::perf_predictor::PerformancePredictor;
use crate::remote::transfer::diff_planner::{plan_local_mirror, LocalDiffInputs};
use crate::remote::transfer::payload::DEFAULT_PAYLOAD_PREFETCH;
use crate::remote::transfer::pipeline::execute_sink_pipeline;
use crate::remote::transfer::sink::{FsSinkConfig, FsTransferSink, NullSink, TransferSink};
use crate::remote::transfer::source::{FilteredSource, FsTransferSource, TransferSource};
use crate::transfer_plan::PlanOptions;
use crate::CopyConfig;

use super::fast_path::{maybe_select_fast_path, FastPathDecision};
use super::history::{record_performance_history, update_predictor};
use super::options::LocalMirrorOptions;
use super::summary::{LocalMirrorSummary, TransferOutcome};

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
fn select_tuning_window(
    history: &[crate::perf_history::PerformanceRecord],
    target_mode: TransferMode,
    checksum: bool,
    skip_unchanged: bool,
) -> Vec<crate::perf_history::PerformanceRecord> {
    history
        .iter()
        .rev()
        .filter(|record| record.run_kind.is_real_transfer())
        .filter(|record| record.mode == target_mode)
        .filter(|record| record.options.checksum == checksum)
        .filter(|record| record.options.skip_unchanged == skip_unchanged)
        .filter(|record| record.fast_path.as_deref() != Some("tiny_manifest"))
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
fn select_tuning_window_from_history<F>(
    reader: F,
    target_mode: TransferMode,
    checksum: bool,
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
    let window = select_tuning_window(&history, target_mode, checksum, skip_unchanged);
    if window.is_empty() {
        None
    } else {
        Some(window)
    }
}

pub struct TransferOrchestrator;

impl TransferOrchestrator {
    pub fn new() -> Self {
        Self
    }

    /// Sync wrapper around [`execute_local_mirror_async`]. Builds a
    /// new multi-thread Tokio runtime and blocks on it. Use this from
    /// non-async callers (CLI commands, tests). Callers already
    /// inside an async runtime must use `execute_local_mirror_async`
    /// directly — calling this from inside a Tokio context will
    /// panic at `Runtime::new` (closes F9 of
    /// `docs/reviews/codebase_review_2026-05-01.md`).
    pub fn execute_local_mirror(
        &self,
        src_root: &Path,
        dest_root: &Path,
        options: LocalMirrorOptions,
    ) -> Result<LocalMirrorSummary> {
        let workers = options.workers.max(1);
        let runtime = Builder::new_multi_thread()
            .worker_threads(workers)
            .enable_all()
            .build()
            .context("build tokio runtime")?;
        runtime.block_on(self.execute_local_mirror_async(src_root, dest_root, options))
    }

    /// Async core of the local-mirror orchestrator. Callable from
    /// any async context. Closes F9 of the 2026-05-01 baseline
    /// review: previously `execute_local_mirror` built and owned its
    /// own Tokio runtime, which panicked when called from an async
    /// caller. The sync wrapper above is now a thin convenience for
    /// blocking callers.
    pub async fn execute_local_mirror_async(
        &self,
        src_root: &Path,
        dest_root: &Path,
        options: LocalMirrorOptions,
    ) -> Result<LocalMirrorSummary> {
        if !src_root.exists() {
            return Err(eyre!("source path does not exist: {}", src_root.display()));
        }

        if !options.dry_run {
            if let Some(parent) = dest_root.parent() {
                std::fs::create_dir_all(parent).with_context(|| {
                    format!("failed to create destination parent {}", parent.display())
                })?;
            }
        }

        let start_time = Instant::now();

        // Single-file source: bypass the enumerator/planner/pipeline machinery
        // entirely and copy the file directly. The destination resolver in the
        // CLI has already produced the exact target path (accounting for
        // trailing-slash / existing-dir semantics), so we just invoke copy_file.
        // Without this short-circuit, the enumerator would skip the depth-0
        // root entry and the fast-path would report NoWork — silent data loss.
        if src_root.is_file() {
            return execute_single_file_copy(src_root, dest_root, &options, start_time);
        }

        let mut journal_tracker = ChangeTracker::load().ok();
        let mut journal_tokens: Vec<ProbeToken> = Vec::new();
        let mut journal_skip = false;

        let mut predictor = PerformancePredictor::load().ok();

        let copy_config = CopyConfig {
            workers: options.workers.max(1),
            preserve_times: options.preserve_times,
            dry_run: options.dry_run,
            checksum: if options.checksum {
                Some(crate::checksum::ChecksumType::Blake3)
            } else {
                None
            },
            resume: options.resume,
            null_sink: options.null_sink,
        };

        // Journal fast-path requires BOTH source and destination to exist and
        // report "no changes". A missing destination obviously needs a full
        // transfer — treating it as unchanged would silently skip the work.
        if options.skip_unchanged
            && !options.checksum
            && !options.force_tar
            && !options.null_sink
            && dest_root.exists()
        {
            if let Some(tracker) = journal_tracker.as_ref() {
                match tracker.probe(src_root) {
                    Ok(src_probe) => {
                        let dest_probe = tracker.probe(dest_root).ok();

                        if src_probe.snapshot.is_some() {
                            journal_tokens.push(src_probe.clone());
                        }
                        if let Some(ref probe) = dest_probe {
                            if probe.snapshot.is_some() {
                                journal_tokens.push(probe.clone());
                            }
                        }

                        if options.verbose {
                            log_probe("src", &src_probe);
                            if let Some(probe) = dest_probe.as_ref() {
                                log_probe("dest", probe);
                            } else {
                                eprintln!("Journal probe dest unsupported; cannot take fast-path");
                            }
                        }

                        let src_no_change = matches!(src_probe.state, ChangeState::NoChanges);
                        // If dest_probe is None (unsupported FS), we cannot
                        // assert "no change" — fall through to full planner.
                        let dest_no_change = dest_probe
                            .as_ref()
                            .map(|probe| matches!(probe.state, ChangeState::NoChanges))
                            .unwrap_or(false);

                        if src_no_change && dest_no_change {
                            journal_skip = true;
                        }
                    }
                    Err(err) => {
                        if options.verbose {
                            eprintln!("Filesystem journal probe failed: {err:?}");
                        }
                    }
                }
            }
        }

        if journal_skip {
            if options.verbose {
                eprintln!(
                    "Filesystem journal fast-path: source/destination unchanged; skipping planner."
                );
            }
            if let Some(tracker) = journal_tracker.as_mut() {
                persist_journal_checkpoints(
                    tracker,
                    journal_tokens.as_mut_slice(),
                    options.verbose,
                );
            }

            // Journal said both sides match, so we never enumerated.
            // scanned_{files,bytes} stay 0 — predictor sees this as
            // "noop with no scan cost" which is what actually happened.
            let summary = LocalMirrorSummary {
                dry_run: options.dry_run,
                duration: start_time.elapsed(),
                outcome: TransferOutcome::JournalSkip,
                ..Default::default()
            };

            if let Some(record) = record_performance_history(
                &summary,
                &options,
                Some("journal_no_work"),
                0,
                summary.duration.as_millis(),
            ) {
                update_predictor(&mut predictor, &record, options.verbose);
            }

            return Ok(summary);
        }

        // Skip fast path when using null sink — it bypasses the sink abstraction.
        let fast_path_outcome = if options.null_sink {
            super::fast_path::FastPathOutcome::streaming()
        } else {
            maybe_select_fast_path(src_root, dest_root, &options)?
        };
        if let Some(decision) = fast_path_outcome.decision {
            // R47-F4: propagate the fast-path scan's suppressed
            // errors into the per-branch summary. Each fast-path
            // outcome below clones this into `unreadable_paths`
            // so the CLI's source-delete step can detect a
            // partial scan even on the Tiny/Huge/NoWork paths.
            let fast_path_unreadable = fast_path_outcome.unreadable_paths.clone();
            let summary = match decision {
                FastPathDecision::NoWork { examined } => {
                    let outcome = if examined == 0 {
                        TransferOutcome::SourceEmpty
                    } else {
                        TransferOutcome::UpToDate
                    };
                    if options.verbose {
                        match outcome {
                            TransferOutcome::SourceEmpty => {
                                eprintln!("Fast-path routing: source yielded no file entries")
                            }
                            _ => eprintln!(
                                "Fast-path routing: {} files examined, all up to date",
                                examined
                            ),
                        }
                    }
                    // NoWork ran a real fast-path scan but copied nothing.
                    // scanned_files = examined captures the planner-side
                    // workload; scanned_bytes is 0 because the fast-path
                    // scanner only resolves names + identity, not sizes.
                    let summary = LocalMirrorSummary {
                        planned_files: examined,
                        scanned_files: examined,
                        dry_run: options.dry_run,
                        duration: start_time.elapsed(),
                        outcome,
                        unreadable_paths: fast_path_unreadable.clone(),
                        ..Default::default()
                    };
                    if let Some(record) = record_performance_history(
                        &summary,
                        &options,
                        Some("no_work"),
                        0,
                        summary.duration.as_millis(),
                    ) {
                        update_predictor(&mut predictor, &record, options.verbose);
                    }
                    summary
                }
                FastPathDecision::Tiny { files } => {
                    let total_bytes: u64 = files.iter().map(|(_, size)| *size).sum();
                    if options.verbose {
                        eprintln!(
                            "Fast-path routing: tiny manifest ({} file(s), {} bytes)",
                            files.len(),
                            total_bytes
                        );
                    }
                    let rels: Vec<PathBuf> = files.iter().map(|(rel, _)| rel.clone()).collect();
                    copy_paths_blocking(src_root, dest_root, &rels, &copy_config)?;
                    // Tiny copies everything it scanned, so scanned ==
                    // copied here. Setting both lets the predictor
                    // train on the actual workload size for the
                    // tiny_manifest fast-path key.
                    let summary = LocalMirrorSummary {
                        planned_files: files.len(),
                        copied_files: files.len(),
                        total_bytes,
                        scanned_files: files.len(),
                        scanned_bytes: total_bytes,
                        dry_run: options.dry_run,
                        duration: start_time.elapsed(),
                        unreadable_paths: fast_path_unreadable.clone(),
                        ..Default::default()
                    };
                    if let Some(record) = record_performance_history(
                        &summary,
                        &options,
                        Some("tiny_manifest"),
                        0,
                        summary.duration.as_millis(),
                    ) {
                        update_predictor(&mut predictor, &record, options.verbose);
                    }
                    summary
                }
                FastPathDecision::Huge { file, size } => {
                    if options.verbose {
                        eprintln!(
                            "Fast-path routing: huge file {} ({} bytes)",
                            file.display(),
                            size
                        );
                    }
                    copy_large_blocking(src_root, dest_root, &file, &copy_config)?;
                    // Huge fast-path copies a single file: scan size
                    // and copy size are identical (one file, `size`
                    // bytes).
                    let summary = LocalMirrorSummary {
                        planned_files: 1,
                        copied_files: 1,
                        total_bytes: size,
                        scanned_files: 1,
                        scanned_bytes: size,
                        dry_run: options.dry_run,
                        duration: start_time.elapsed(),
                        large_tasks: 1,
                        large_bytes: size,
                        unreadable_paths: fast_path_unreadable.clone(),
                        ..Default::default()
                    };
                    if let Some(record) = record_performance_history(
                        &summary,
                        &options,
                        Some("single_huge_file"),
                        0,
                        summary.duration.as_millis(),
                    ) {
                        update_predictor(&mut predictor, &record, options.verbose);
                    }
                    summary
                }
            };

            if let Some(tracker) = journal_tracker.as_mut() {
                persist_journal_checkpoints(
                    tracker,
                    journal_tokens.as_mut_slice(),
                    options.verbose,
                );
            }

            if options.verbose {
                eprintln!(
                    "Completed local {} via fast-path: {} file(s), {} bytes in {:.2?}",
                    if options.mirror { "mirror" } else { "copy" },
                    summary.copied_files,
                    summary.total_bytes,
                    summary.duration
                );
            }

            return Ok(summary);
        }

        // --- Unified pipeline: same path as remote transfers ---
        let mut plan_options = PlanOptions {
            force_tar: options.force_tar,
            ..PlanOptions::default()
        };

        if options.perf_history {
            // R57-F1: read ALL history, not a pre-cap window. The
            // R56-F2 fix correctly filtered run_kind before the
            // 20-record cap inside `select_tuning_window`, but the
            // caller was still pre-capping at 50 records from the
            // JSONL — so 50 recent non-real records could still
            // hide older real records one layer up. The file is
            // already size-capped at ~1 MiB upstream
            // (DEFAULT_MAX_BYTES in perf_history.rs), so reading
            // all records is bounded; `read_recent_records(0)`
            // means "all" per its limit semantics.
            let target_mode = if options.mirror {
                TransferMode::Mirror
            } else {
                TransferMode::Copy
            };
            if let Some(filtered) = select_tuning_window_from_history(
                read_recent_records,
                target_mode,
                options.checksum,
                options.skip_unchanged,
            ) {
                if let Some(tuning) = derive_local_plan_tuning(&filtered) {
                    plan_options.small_target = Some(tuning.small_target_bytes);
                    plan_options.small_count_target = Some(tuning.small_count_target);
                    plan_options.medium_target = Some(tuning.medium_target_bytes);
                }
            }
        }

        let planning_start = Instant::now();

        let src_root_buf = src_root.to_path_buf();
        let dest_root_buf = dest_root.to_path_buf();
        let filter = options.filter.clone_without_cache();
        let skip_unchanged = options.skip_unchanged;
        let ignore_existing = options.ignore_existing;
        // R58-F7: translate the orchestrator's `compare_mode` (set by
        // the CLI from --size-only / --ignore-times / --force /
        // --checksum / default) onto the unified ComparisonMode enum.
        // Pre-fix this hardcoded a bool→Checksum-or-SizeMtime mapping
        // and ignored the other flags entirely; remote pull already
        // honored all five variants, so behavior diverged by direction.
        //
        // Backward-compat: the old `options.checksum` bool still
        // wins if it's set without `compare_mode` being explicitly
        // changed — preserves the existing `--checksum` behavior
        // for any caller that hasn't migrated yet.
        let compare_mode = match options.compare_mode {
            crate::orchestrator::LocalCompareMode::Checksum => ComparisonMode::Checksum,
            crate::orchestrator::LocalCompareMode::SizeOnly => ComparisonMode::SizeOnly,
            crate::orchestrator::LocalCompareMode::Force => ComparisonMode::Force,
            crate::orchestrator::LocalCompareMode::IgnoreTimes => ComparisonMode::IgnoreTimes,
            crate::orchestrator::LocalCompareMode::SizeMtime => {
                if options.checksum {
                    ComparisonMode::Checksum
                } else {
                    ComparisonMode::SizeMtime
                }
            }
        };

        // 1. Scan source via FsTransferSource, wrapped in FilteredSource so
        //    the user filter applies through the universal pipeline chokepoint
        //    (identical to push/pull/remote-remote behavior — full parity).
        let inner: Arc<dyn TransferSource> = Arc::new(FsTransferSource::new(src_root_buf.clone()));
        let source: Arc<dyn TransferSource> = Arc::new(FilteredSource::new(inner, filter));
        let unreadable = Arc::new(Mutex::new(Vec::new()));
        let (mut header_rx, scan_handle) = source.scan(None, unreadable.clone());

        // 2. Collect all headers
        let mut all_headers = Vec::new();
        while let Some(h) = header_rx.recv().await {
            all_headers.push(h);
        }
        let _total_scanned = scan_handle
            .await
            .context("scan task panicked")?
            .context("scan failed")?;

        // 3. Diff + plan via the shared DiffPlanner stage. Combines
        //    the comparison-filter and payload-planning steps that
        //    were previously inline. Behavior preserved bit-for-bit
        //    (size+mtime or Blake3 hash, then tar/large/raw planning).
        let src = src_root_buf.clone();
        let dst = dest_root_buf.clone();
        let plan_opts = plan_options;
        let headers = all_headers.clone();
        let planned = tokio::task::spawn_blocking(move || {
            plan_local_mirror(
                headers,
                LocalDiffInputs {
                    src_root: &src,
                    dst_root: &dst,
                    compare_mode,
                    ignore_existing,
                    plan_options: plan_opts,
                    skip_unchanged,
                },
            )
        })
        .await
        .context("diff_planner task panicked")??;

        // 5. Create sink and execute unified pipeline
        let sink: Arc<dyn TransferSink> = if copy_config.null_sink {
            Arc::new(NullSink::new())
        } else {
            Arc::new(FsTransferSink::new(
                src_root_buf.clone(),
                dest_root_buf.clone(),
                FsSinkConfig {
                    preserve_times: copy_config.preserve_times,
                    dry_run: copy_config.dry_run,
                    checksum: copy_config.checksum,
                    resume: copy_config.resume,
                },
            ))
        };

        // Boundary between planner and transfer phases. `planning_start`
        // covers scan + diff + plan; everything after this `Instant`
        // is the transfer pipeline. §2.8 phase 2 split: pre-fix the
        // record's `planner_duration_ms` field was set to whole-run
        // time, so the v1 predictor effectively trained on `planner =
        // total` for both targets and couldn't distinguish them.
        let plan_done = Instant::now();
        let planner_duration_ms = plan_done.duration_since(planning_start).as_millis();

        // §2.8 phase 2: query the predictor BEFORE running the
        // pipeline. Surfaces in summary.predictor_estimate so
        // `--verbose` and `blit profile --json` can compare
        // predicted vs actual.
        //
        // R44-F1: query and observation must use the same feature
        // vector. We query with `(scanned_files, scanned_bytes)`
        // here; `record_performance_history` populates the matching
        // `PerformanceRecord.{file_count,total_bytes}` from
        // `summary.{scanned_files,scanned_bytes}`. Pre-fix the
        // record was populated from `summary.copied_files`, so on
        // any incremental run the predictor was queried with one
        // workload size and trained against another.
        //
        // src_fs/dest_fs are left None for 0.1.0 — wiring
        // `fs_capability` per-path probes into the predictor query
        // is post-release work (see §3.3 / Phase 4.8.2 deferral).
        let scanned_files = all_headers.len();
        let scanned_bytes: u64 = all_headers.iter().map(|h| h.size).sum();
        // R45 follow-up to R44-F1: never alias `total_bytes` to
        // `scanned_bytes`. `summary.total_bytes` is the
        // pipeline-wrote-bytes contract (see `LocalMirrorSummary`
        // rustdoc); the predictor uses scan features only. Pre-fix
        // this aliased the two so `summary.total_bytes` reported
        // scanned bytes as bytes-written, overcounting throughput
        // on incremental runs.
        let predictor_estimate = predictor.as_ref().and_then(|p| {
            let kind_total = crate::perf_predictor::DurationKind::Total;
            let mode = if options.mirror {
                crate::perf_history::TransferMode::Mirror
            } else {
                crate::perf_history::TransferMode::Copy
            };
            let total_pred = p.predict(
                kind_total,
                mode.clone(),
                None,
                None,
                None,
                options.skip_unchanged,
                options.checksum,
                scanned_files,
                scanned_bytes,
            )?;
            // Pull planner + transfer separately too so the verbose
            // line and the JSON profile can break down the estimate.
            // All three predictor calls share the same
            // (scanned_files, scanned_bytes) feature vector — both
            // for consistency with the recording side, and so a
            // future maintainer can't accidentally reintroduce a
            // train/query mismatch by editing one branch and
            // missing another.
            let planner_pred = p
                .predict(
                    crate::perf_predictor::DurationKind::Planner,
                    mode.clone(),
                    None,
                    None,
                    None,
                    options.skip_unchanged,
                    options.checksum,
                    scanned_files,
                    scanned_bytes,
                )
                .map(|p| p.predicted_ms)
                .unwrap_or(0.0);
            let transfer_pred = p
                .predict(
                    crate::perf_predictor::DurationKind::Transfer,
                    mode,
                    None,
                    None,
                    None,
                    options.skip_unchanged,
                    options.checksum,
                    scanned_files,
                    scanned_bytes,
                )
                .map(|p| p.predicted_ms)
                .unwrap_or(0.0);
            Some(super::summary::PredictorEstimate {
                planner_ms: planner_pred.max(0.0) as u128,
                transfer_ms: transfer_pred.max(0.0) as u128,
                total_ms: total_pred.predicted_ms.max(0.0) as u128,
                observations: total_pred.observations,
                fallback_depth: total_pred.fallback_depth,
            })
        });
        if options.verbose {
            if let Some(est) = predictor_estimate.as_ref() {
                eprintln!(
                    "Predictor estimate: planner ~{} ms, transfer ~{} ms, \
                     total ~{} ms (n={}, fallback_depth={})",
                    est.planner_ms,
                    est.transfer_ms,
                    est.total_ms,
                    est.observations,
                    est.fallback_depth
                );
            } else {
                eprintln!("Predictor estimate: unavailable (no profile yet for this workload)");
            }
        }

        let pipeline_outcome = execute_sink_pipeline(
            source,
            vec![sink],
            planned.payloads,
            DEFAULT_PAYLOAD_PREFETCH,
            None,
        )
        .await
        .context("transfer pipeline failed")?;
        let transfer_duration_ms = plan_done.elapsed().as_millis();

        // R47-F4: snapshot unreadable paths so the CLI's source-
        // delete step (in `blit move`) can refuse to remove a
        // source it couldn't fully scan. The R46-F2 gate inside
        // the orchestrator only fires on `options.mirror`, but
        // move uses mirror=false — without this surface, an
        // unreadable source file would get skipped during the
        // copy and then silently deleted from the source by the
        // CLI's `remove_dir_all` step.
        let unreadable_snapshot: Vec<String> = unreadable
            .lock()
            .map(|guard| guard.clone())
            .unwrap_or_default();

        let mut summary = LocalMirrorSummary {
            planned_files: pipeline_outcome.files_written,
            copied_files: pipeline_outcome.files_written,
            // R45: bytes the pipeline actually wrote, not scanned
            // bytes. Distinct on incremental runs.
            total_bytes: pipeline_outcome.bytes_written,
            scanned_files,
            scanned_bytes,
            dry_run: options.dry_run,
            duration: start_time.elapsed(),
            predictor_estimate: predictor_estimate.clone(),
            unreadable_paths: unreadable_snapshot.clone(),
            ..Default::default()
        };

        if options.mirror {
            // R46-F2: refuse to mirror-delete when the source scan
            // was incomplete. The `unreadable_snapshot` captured
            // above (R47-F4) covers the per-file open path
            // (PermissionDenied / NotFound on individual files) and
            // the walkdir non-root error path (unreadable
            // subdirectories). Either case means the header set
            // we're about to use as the source-of-truth for "what
            // the destination should contain" is missing entries,
            // and a delete pass would silently remove matching
            // destination subtrees.
            if !unreadable_snapshot.is_empty() {
                bail!(
                    "refusing to mirror-delete from {}: source scan was \
                     incomplete ({} unreadable entr{}); the first {} \
                     reported: {}. Resolve the scan errors (typically \
                     permissions) or run as a non-mirror copy.",
                    dest_root.display(),
                    unreadable_snapshot.len(),
                    if unreadable_snapshot.len() == 1 {
                        "y"
                    } else {
                        "ies"
                    },
                    unreadable_snapshot.len().min(5),
                    unreadable_snapshot
                        .iter()
                        .take(5)
                        .cloned()
                        .collect::<Vec<_>>()
                        .join("; "),
                );
            }

            let source_paths: HashSet<String> = all_headers
                .iter()
                .map(|h| h.relative_path.clone())
                .collect();
            let deletions = apply_mirror_deletions(
                &source_paths,
                dest_root,
                &options.filter,
                options.delete_scope,
                !options.dry_run,
                options.verbose,
            )?;
            summary.deleted_files = deletions.0;
            summary.deleted_dirs = deletions.1;
        }

        if let Some(tracker) = journal_tracker.as_mut() {
            persist_journal_checkpoints(tracker, journal_tokens.as_mut_slice(), options.verbose);
        }

        if options.verbose {
            eprintln!(
                "Planning enumerated {} file(s), {} bytes",
                scanned_files, scanned_bytes
            );
            eprintln!(
                "Completed local {}: {} file(s), {} bytes in {:.2?} (plan {} ms, xfer {} ms)",
                if options.mirror { "mirror" } else { "copy" },
                summary.copied_files,
                summary.total_bytes,
                summary.duration,
                planner_duration_ms,
                transfer_duration_ms,
            );
            // §2.8: side-by-side predicted-vs-actual so operators
            // can audit the predictor against this run's actual
            // numbers. The bare percentage error per phase is the
            // most useful single number; we keep absolute ms in the
            // line above for context.
            if let Some(est) = summary.predictor_estimate.as_ref() {
                let pct = |predicted_ms: u128, actual_ms: u128| -> String {
                    if actual_ms == 0 {
                        "n/a".to_string()
                    } else {
                        let pred = predicted_ms as f64;
                        let act = actual_ms as f64;
                        format!("{:+.0}%", ((pred - act) / act) * 100.0)
                    }
                };
                eprintln!(
                    "Predictor delta: planner {} ({} vs {} ms), \
                     transfer {} ({} vs {} ms)",
                    pct(est.planner_ms, planner_duration_ms),
                    est.planner_ms,
                    planner_duration_ms,
                    pct(est.transfer_ms, transfer_duration_ms),
                    est.transfer_ms,
                    transfer_duration_ms,
                );
            }
        }

        let fast_path_label = if options.null_sink {
            Some("null_sink")
        } else {
            None
        };
        if let Some(record) = record_performance_history(
            &summary,
            &options,
            fast_path_label,
            planner_duration_ms,
            transfer_duration_ms,
        ) {
            // Don't update the predictor from null-sink runs — the zero
            // write cost would teach it that transfers are faster than
            // they really are.
            if !options.null_sink {
                update_predictor(&mut predictor, &record, options.verbose);
            }
        }

        Ok(summary)
    }
}

/// Delete destination files/dirs not present in the source header set.
///
/// R58-F6: `delete_scope` controls which destination entries are
/// even considered for deletion:
///   - `FilteredSubset` (default): enumerate the destination
///     *through the user's filter*, then delete entries not in
///     the source set. Excluded files (e.g. `*.log` when
///     `--exclude '*.log'`) are out of scope — they're not
///     candidates for deletion, and their parent directories are
///     therefore non-empty from the user's perspective. When
///     `remove_dir` fails with ENOTEMPTY on a parent whose only
///     remaining contents are out-of-scope, we treat it as
///     expected, not as an error.
///   - `All`: enumerate the destination *without* the filter so
///     every entry is in scope. ENOTEMPTY is a genuine error
///     here (we did walk everything, so something other than
///     filter-excluded content must be in the way).
fn apply_mirror_deletions(
    source_paths: &HashSet<String>,
    dest_root: &Path,
    filter: &FileFilter,
    delete_scope: crate::orchestrator::LocalMirrorDeleteScope,
    perform: bool,
    verbose: bool,
) -> Result<(usize, usize)> {
    use crate::enumeration::{EntryKind, FileEnumerator};
    use crate::orchestrator::LocalMirrorDeleteScope;

    // R58-F6: FilteredSubset uses the user's filter for the
    // enumeration (only in-scope entries become deletion
    // candidates). All bypasses the filter so every destination
    // entry is considered.
    let enum_filter = match delete_scope {
        LocalMirrorDeleteScope::FilteredSubset => filter.clone_without_cache(),
        LocalMirrorDeleteScope::All => FileFilter::default(),
    };
    let enumerator = FileEnumerator::new(enum_filter);
    let dest_entries = enumerator.enumerate_local(dest_root)?;

    // R48-F1: source.scan() only emits file headers, so
    // `source_paths` is a set of *files*. Pre-fix this meant every
    // destination directory was "not in source_paths" and got
    // queued for deletion. Combined with R46-F5's hard-error
    // policy on remove_* failures, a normal mirror containing
    // `sub/file.txt` would keep `sub/file.txt`, then try
    // `remove_dir("sub")` and fail the whole operation with
    // ENOTEMPTY. Derive `source_dirs` from each file's parent
    // chain so dest dirs that exist implicitly on the source
    // side (because they contain a source file) get preserved.
    let mut source_dirs: HashSet<String> = HashSet::new();
    for path in source_paths {
        let p = std::path::Path::new(path);
        let mut cur = p.parent();
        while let Some(parent) = cur {
            if parent.as_os_str().is_empty() {
                break;
            }
            let parent_str = parent.to_string_lossy().replace('\\', "/");
            // Insert and keep walking up; if already present every
            // shallower ancestor is too, so we could break — but
            // the walk is cheap and the eager form is simpler to
            // reason about.
            source_dirs.insert(parent_str);
            cur = parent.parent();
        }
    }

    let mut files_to_delete = Vec::new();
    let mut dirs_to_delete = Vec::new();

    for entry in &dest_entries {
        let rel = entry.relative_path.to_string_lossy().replace('\\', "/");
        let absent_at_source = match entry.kind {
            EntryKind::Directory => !source_dirs.contains(&rel),
            _ => !source_paths.contains(&rel),
        };
        if absent_at_source {
            let abs = dest_root.join(&entry.relative_path);
            match entry.kind {
                EntryKind::Directory => dirs_to_delete.push(abs),
                _ => files_to_delete.push(abs),
            }
        }
    }

    // Sort dirs deepest-first so children are deleted before parents.
    dirs_to_delete.sort_by_key(|b| std::cmp::Reverse(b.components().count()));

    let mut deleted_files = 0usize;
    let mut deleted_dirs = 0usize;
    // R46-F5: collect deletion failures and bail at the end. Pre-fix
    // each `remove_file` / `remove_dir` error was printed as a
    // warning and the function returned Ok, so a mirror could
    // succeed-on-paper while leaving stale destination content
    // behind. Now we still attempt every deletion (better partial
    // progress than abort-on-first-failure), but we bail with an
    // aggregated error if any failed — the caller's mirror operation
    // returns Err, the user sees the failed entries, and the summary
    // line doesn't claim "complete".
    let mut failures: Vec<String> = Vec::new();

    for path in files_to_delete {
        #[cfg(windows)]
        crate::win_fs::clear_readonly_recursive(&path);

        if perform {
            match std::fs::remove_file(&path) {
                Ok(_) => {
                    deleted_files += 1;
                    if verbose {
                        eprintln!("Deleted file: {}", path.display());
                    }
                }
                Err(err) => {
                    eprintln!("Failed to delete file {}: {}", path.display(), err);
                    failures.push(format!("{}: {}", path.display(), err));
                }
            }
        } else {
            deleted_files += 1;
        }
    }

    for path in dirs_to_delete {
        #[cfg(windows)]
        crate::win_fs::clear_readonly_recursive(&path);

        if perform {
            match std::fs::remove_dir(&path) {
                Ok(_) => {
                    deleted_dirs += 1;
                    if verbose {
                        eprintln!("Deleted directory: {}", path.display());
                    }
                }
                Err(err) => {
                    // R58-F6: in FilteredSubset mode, ENOTEMPTY on
                    // a destination dir means the dir contains
                    // out-of-scope content (files matching the
                    // user's exclude rules). Those files
                    // intentionally aren't candidates for
                    // deletion, so the dir genuinely can't be
                    // empty — that's not a failure, it's the
                    // expected behavior of the scope contract.
                    // Skip silently in that case; surface the
                    // error in `All` mode where the dir really
                    // should have been empty.
                    let is_not_empty = err.kind() == std::io::ErrorKind::DirectoryNotEmpty
                        || err.raw_os_error() == Some(66); // ENOTEMPTY on macOS/BSD
                    if matches!(delete_scope, LocalMirrorDeleteScope::FilteredSubset)
                        && is_not_empty
                    {
                        if verbose {
                            eprintln!(
                                "Kept directory {} (contains out-of-scope contents)",
                                path.display()
                            );
                        }
                    } else {
                        eprintln!("Failed to delete directory {}: {}", path.display(), err);
                        failures.push(format!("{}: {}", path.display(), err));
                    }
                }
            }
        } else {
            deleted_dirs += 1;
        }
    }

    if !failures.is_empty() {
        let preview = failures
            .iter()
            .take(5)
            .cloned()
            .collect::<Vec<_>>()
            .join("; ");
        bail!(
            "mirror-delete left {} entr{} in place at {} ({} succeeded): {}",
            failures.len(),
            if failures.len() == 1 { "y" } else { "ies" },
            dest_root.display(),
            deleted_files + deleted_dirs,
            preview
        );
    }

    Ok((deleted_files, deleted_dirs))
}

fn persist_journal_checkpoints(
    tracker: &mut ChangeTracker,
    tokens: &mut [ProbeToken],
    verbose: bool,
) {
    if tokens.is_empty() {
        return;
    }

    for token in tokens.iter_mut() {
        match tracker.reprobe_canonical(&token.canonical_path) {
            Ok(snapshot) => token.snapshot = snapshot,
            Err(err) => {
                token.snapshot = None;
                if verbose {
                    eprintln!(
                        "Failed to refresh journal snapshot for {}: {err:?}",
                        token.canonical_path.display()
                    );
                }
            }
        }
    }

    if let Err(err) = tracker.refresh_and_persist(tokens) {
        if verbose {
            eprintln!("Failed to update journal checkpoint: {err:?}");
        }
    }
}

fn log_probe(label: &str, probe: &ProbeToken) {
    eprintln!(
        "Journal probe {label} state={:?} snapshot={} path={}",
        probe.state,
        probe.snapshot.is_some(),
        probe.canonical_path.display()
    );

    if let Some(snapshot) = &probe.snapshot {
        match snapshot {
            StoredSnapshot::Windows(snap) => {
                eprintln!(
                    "  {label} windows: volume={} journal_id={} next_usn={} mtime={:?}",
                    snap.volume, snap.journal_id, snap.next_usn, snap.root_mtime_epoch_ms
                );
            }
            StoredSnapshot::MacOs(snap) => {
                eprintln!(
                    "  {label} macOS: fsid={} event_id={}",
                    snap.fsid, snap.event_id
                );
            }
            StoredSnapshot::Linux(snap) => {
                eprintln!(
                    "  {label} linux: device={} inode={} ctime={}s+{}ns mtime={:?}",
                    snap.device,
                    snap.inode,
                    snap.ctime_sec,
                    snap.ctime_nsec,
                    snap.root_mtime_epoch_ms
                );
            }
        }
    }
}

impl Default for TransferOrchestrator {
    fn default() -> Self {
        Self::new()
    }
}

/// Copy a single file source directly to `dest_root`, bypassing the
/// enumerator/planner/pipeline machinery which assumes `src_root` is a
/// directory. The CLI's destination resolver has already produced the final
/// target path, so this is a simple `copy_file` call.
fn execute_single_file_copy(
    src_root: &Path,
    dest_root: &Path,
    options: &LocalMirrorOptions,
    start_time: Instant,
) -> Result<LocalMirrorSummary> {
    use crate::buffer::BufferSizer;
    use crate::copy::{copy_file, file_needs_copy_with_checksum_type, resume_copy_file};
    use crate::logger::NoopLogger;
    use filetime::FileTime;

    let src_meta = std::fs::metadata(src_root)
        .with_context(|| format!("stat source file {}", src_root.display()))?;
    let size = src_meta.len();

    let checksum = if options.checksum {
        Some(crate::checksum::ChecksumType::Blake3)
    } else {
        None
    };

    // R58-F5: the single-file short-circuit (orchestrator.rs:125)
    // bypasses the enumerator + planner, which is where the
    // streaming-pipeline path checks filter / ignore_existing.
    // Apply both here so single-file copies honor the same
    // CLI contract.
    //
    // Filter: the source root is itself the only entry. Run
    // `filter.allows_entry` against the source name. If excluded,
    // return a "scanned 1 / copied 0" summary so the user sees
    // "no work performed" rather than the file being copied
    // anyway.
    let src_name = src_root.file_name().map(PathBuf::from);
    let allows = match src_name {
        Some(name) => {
            let mtime = src_meta.modified().ok();
            options
                .filter
                .allows_entry(Some(&name), src_root, size, mtime)
        }
        None => true,
    };
    if !allows {
        return Ok(LocalMirrorSummary {
            planned_files: 0,
            copied_files: 0,
            total_bytes: 0,
            scanned_files: 1,
            scanned_bytes: size,
            duration: start_time.elapsed(),
            outcome: TransferOutcome::UpToDate,
            ..Default::default()
        });
    }

    // ignore_existing: if the destination file already exists,
    // skip the copy entirely. Matches the diff_planner behavior
    // for the streaming-pipeline path (diff_planner.rs).
    if options.ignore_existing && dest_root.exists() {
        return Ok(LocalMirrorSummary {
            planned_files: 0,
            copied_files: 0,
            total_bytes: 0,
            scanned_files: 1,
            scanned_bytes: size,
            duration: start_time.elapsed(),
            outcome: TransferOutcome::UpToDate,
            ..Default::default()
        });
    }

    if options.dry_run {
        return Ok(LocalMirrorSummary {
            planned_files: 1,
            copied_files: 1,
            total_bytes: size,
            scanned_files: 1,
            scanned_bytes: size,
            dry_run: true,
            duration: start_time.elapsed(),
            ..Default::default()
        });
    }

    if options.null_sink {
        return Ok(LocalMirrorSummary {
            planned_files: 1,
            copied_files: 1,
            total_bytes: size,
            scanned_files: 1,
            scanned_bytes: size,
            duration: start_time.elapsed(),
            ..Default::default()
        });
    }

    let mut did_copy = false;
    let mut clone_succeeded = false;
    let mut bytes_copied = 0u64;

    if options.resume {
        let outcome = resume_copy_file(src_root, dest_root, 0)
            .with_context(|| format!("resume copy {}", src_root.display()))?;
        did_copy = outcome.bytes_transferred > 0;
        bytes_copied = outcome.bytes_transferred;
    } else {
        let needs_copy = !options.skip_unchanged
            || file_needs_copy_with_checksum_type(src_root, dest_root, checksum).unwrap_or(true);
        if needs_copy {
            let sizer = BufferSizer::default();
            let logger = NoopLogger;
            let outcome = copy_file(src_root, dest_root, &sizer, false, &logger)
                .with_context(|| format!("copy {}", src_root.display()))?;
            did_copy = true;
            clone_succeeded = outcome.clone_succeeded;
            bytes_copied = outcome.bytes_copied;
        }
    }

    if options.preserve_times && did_copy && !clone_succeeded {
        if let Ok(modified) = src_meta.modified() {
            let ft = FileTime::from_system_time(modified);
            // R42-F1: warn-don't-silence (was `let _ = ...`).
            if let Err(e) = filetime::set_file_mtime(dest_root, ft) {
                log::warn!("set mtime on {}: {}", dest_root.display(), e);
            }
        }
    }

    Ok(LocalMirrorSummary {
        planned_files: 1,
        copied_files: if did_copy { 1 } else { 0 },
        total_bytes: bytes_copied,
        // Single-file path always saw exactly one entry of `size`
        // bytes; whether we copied it or not is the
        // copied_files/total_bytes story, but the scan saw it.
        scanned_files: 1,
        scanned_bytes: size,
        duration: start_time.elapsed(),
        outcome: if did_copy {
            TransferOutcome::Transferred
        } else {
            TransferOutcome::UpToDate
        },
        ..Default::default()
    })
}

#[cfg(test)]
mod async_runtime_tests {
    //! F9 regression: `execute_local_mirror_async` must be callable
    //! from inside an existing Tokio runtime without panicking. The
    //! sync `execute_local_mirror` wrapper builds its own runtime
    //! and would panic with "Cannot start a runtime from within a
    //! runtime" if called from `#[tokio::test]`.
    use super::*;
    use tempfile::tempdir;

    fn write_file(path: &std::path::Path, body: &[u8]) {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(path, body).unwrap();
    }

    fn opts() -> LocalMirrorOptions {
        LocalMirrorOptions {
            workers: 2,
            preserve_times: false,
            dry_run: false,
            checksum: false,
            ..Default::default()
        }
    }

    #[tokio::test]
    async fn async_version_callable_from_async_context() {
        // The whole point of F9 — calling the async version from
        // within #[tokio::test]'s runtime must not build a nested
        // runtime or panic.
        let tmp = tempdir().unwrap();
        let src = tmp.path().join("src");
        let dst = tmp.path().join("dst");
        write_file(&src.join("a.txt"), b"hello");
        let orch = TransferOrchestrator::new();
        let summary = orch
            .execute_local_mirror_async(&src, &dst, opts())
            .await
            .unwrap();
        assert!(
            summary.copied_files >= 1,
            "expected at least one file copied, got {:?}",
            summary
        );
        assert!(dst.join("a.txt").exists());
    }

    #[test]
    fn sync_wrapper_still_works() {
        // The sync API must keep working for non-async callers
        // (CLI commands today).
        let tmp = tempdir().unwrap();
        let src = tmp.path().join("src");
        let dst = tmp.path().join("dst");
        write_file(&src.join("a.txt"), b"hello-sync");
        let orch = TransferOrchestrator::new();
        let summary = orch.execute_local_mirror(&src, &dst, opts()).unwrap();
        assert!(summary.copied_files >= 1);
        assert_eq!(std::fs::read(dst.join("a.txt")).unwrap(), b"hello-sync");
    }

    /// R45 regression: `summary.total_bytes` must report bytes the
    /// pipeline actually wrote, not bytes the source scan saw. The
    /// pre-fix R44 commit aliased `let total_bytes = scanned_bytes`
    /// and fed that into the summary — so on this skip-unchanged
    /// incremental run the second run would have reported the full
    /// scanned size as bytes-written even though zero bytes were
    /// actually written.
    ///
    /// The fast-path branches (NoWork / Tiny / Huge / JournalSkip)
    /// don't exhibit the bug because they construct their summary
    /// directly without going through the aliased local. We force
    /// the streaming-pipeline path by enabling `mirror = true`,
    /// which disables fast-path selection (see
    /// `maybe_select_fast_path`'s mirror short-circuit).
    #[tokio::test]
    async fn incremental_run_total_bytes_excludes_skipped_files() {
        let tmp = tempdir().unwrap();
        let src = tmp.path().join("src");
        let dst = tmp.path().join("dst");
        let body_a = vec![b'a'; 2 * 1024];
        let body_b = vec![b'b'; 2 * 1024];
        write_file(&src.join("a.txt"), &body_a);
        write_file(&src.join("b.txt"), &body_b);
        let total_payload = (body_a.len() + body_b.len()) as u64;

        // mirror=true forces the streaming pipeline (fast-path is
        // skipped for mirror runs); skip_unchanged=true means the
        // diff stage will mark both files unchanged on the second
        // run, so the pipeline writes 0 bytes.
        let mut run_opts = opts();
        run_opts.mirror = true;
        run_opts.skip_unchanged = true;

        let orch = TransferOrchestrator::new();
        let first = orch
            .execute_local_mirror_async(&src, &dst, run_opts.clone())
            .await
            .unwrap();
        assert_eq!(
            first.scanned_files, 2,
            "first run should hit streaming planner and scan both files (got summary {:?})",
            first
        );
        assert_eq!(first.scanned_bytes, total_payload);
        assert_eq!(
            first.total_bytes, total_payload,
            "from-scratch run: total_bytes equals bytes written"
        );
        assert_eq!(first.copied_files, 2);

        let second = orch
            .execute_local_mirror_async(&src, &dst, run_opts)
            .await
            .unwrap();
        assert_eq!(
            second.scanned_files, 2,
            "second run still scans both files in mirror mode (got summary {:?})",
            second
        );
        assert_eq!(second.scanned_bytes, total_payload);
        assert_eq!(
            second.total_bytes, 0,
            "incremental skip_unchanged run must report 0 bytes \
             written; R45 alias bug would have reported {} here \
             (full summary: {:?})",
            second.scanned_bytes, second
        );
        assert_eq!(second.copied_files, 0);
    }

    /// R46-F2 regression: a mirror with an unreadable source
    /// subdirectory must NOT delete the corresponding destination
    /// subtree. Pre-fix the walkdir error on the unreadable
    /// subdir was silently dropped at `enumeration.rs:90-95`, the
    /// orchestrator never checked `unreadable`, and
    /// `apply_mirror_deletions` would treat the unscanned subtree
    /// as "absent at source" and delete the matching destination
    /// path. Now the mirror branch refuses to delete with a clear
    /// error.
    ///
    /// Unix-only because we rely on `chmod 000` to make the
    /// subdirectory unreadable and that doesn't work the same way
    /// on Windows.
    #[cfg(unix)]
    #[tokio::test]
    async fn mirror_refuses_when_source_scan_incomplete() {
        use std::os::unix::fs::PermissionsExt;
        let tmp = tempdir().unwrap();
        let src = tmp.path().join("src");
        let dst = tmp.path().join("dst");

        // Source has a readable file and a subdirectory we'll make
        // unreadable so the walkdir can't enter it.
        write_file(&src.join("readable.txt"), b"keep");
        let blocked = src.join("blocked");
        std::fs::create_dir_all(&blocked).unwrap();
        write_file(&blocked.join("inner.txt"), b"unscannable");

        // Destination has the readable file already AND a
        // subdirectory matching the (now-unreadable) source
        // subdir. Pre-fix mirror would delete `dst/blocked/`
        // because the source scan never observed it.
        std::fs::create_dir_all(&dst).unwrap();
        write_file(&dst.join("readable.txt"), b"keep");
        std::fs::create_dir_all(dst.join("blocked")).unwrap();
        write_file(&dst.join("blocked/preserve_me.txt"), b"survivor");

        // Make src/blocked unreadable to the walkdir.
        let mut perms = std::fs::metadata(&blocked).unwrap().permissions();
        perms.set_mode(0o000);
        std::fs::set_permissions(&blocked, perms).unwrap();
        // Restore perms in a guard so cleanup works whatever the
        // assertion outcome.
        struct PermGuard(std::path::PathBuf);
        impl Drop for PermGuard {
            fn drop(&mut self) {
                let mut p = std::fs::metadata(&self.0).unwrap().permissions();
                p.set_mode(0o755);
                let _ = std::fs::set_permissions(&self.0, p);
            }
        }
        let _guard = PermGuard(blocked.clone());

        let mut opts = opts();
        opts.mirror = true;
        let orch = TransferOrchestrator::new();
        let result = orch.execute_local_mirror_async(&src, &dst, opts).await;

        // Mirror should refuse with an explicit error. Pre-fix it
        // would have returned Ok and deleted dst/blocked/.
        let err = match result {
            Err(e) => e,
            Ok(summary) => {
                panic!(
                    "expected mirror to refuse on incomplete scan, \
                     got Ok(summary={:?}); dst/blocked/preserve_me \
                     exists: {}",
                    summary,
                    dst.join("blocked/preserve_me.txt").exists()
                );
            }
        };
        let msg = format!("{err:#}");
        assert!(
            msg.contains("source scan was") && msg.contains("incomplete"),
            "expected scan-incomplete error, got: {msg}"
        );
        // The destination subtree must still be intact.
        assert!(
            dst.join("blocked/preserve_me.txt").exists(),
            "dst/blocked/preserve_me.txt was deleted (R46-F2 \
             incomplete-scan-mirror-delete regression)"
        );
    }

    /// R46-F5 regression: mirror-delete failures must surface as an
    /// Err on the orchestrator's return value, not be silently
    /// swallowed into a warning + Ok summary. We force a deletion
    /// failure by making the destination's "extra" file's parent
    /// non-writable; on unix `remove_file` then fails with EACCES.
    #[cfg(unix)]
    #[tokio::test]
    async fn mirror_delete_failure_propagates_as_error() {
        use std::os::unix::fs::PermissionsExt;
        let tmp = tempdir().unwrap();
        let src = tmp.path().join("src");
        let dst = tmp.path().join("dst");

        write_file(&src.join("kept.txt"), b"src");

        std::fs::create_dir_all(&dst).unwrap();
        write_file(&dst.join("kept.txt"), b"src");
        // The "extra" file mirror would try to delete. Lock its
        // parent dir so the unlink fails.
        let locked_parent = dst.join("locked_subdir");
        std::fs::create_dir_all(&locked_parent).unwrap();
        write_file(&locked_parent.join("extra.txt"), b"unwanted");
        let mut perms = std::fs::metadata(&locked_parent).unwrap().permissions();
        perms.set_mode(0o555); // r-xr-xr-x: contents listable, not writable
        std::fs::set_permissions(&locked_parent, perms).unwrap();

        struct PermGuard(std::path::PathBuf);
        impl Drop for PermGuard {
            fn drop(&mut self) {
                let mut p = std::fs::metadata(&self.0).unwrap().permissions();
                p.set_mode(0o755);
                let _ = std::fs::set_permissions(&self.0, p);
            }
        }
        let _g = PermGuard(locked_parent.clone());

        let mut opts = opts();
        opts.mirror = true;
        let orch = TransferOrchestrator::new();
        let result = orch.execute_local_mirror_async(&src, &dst, opts).await;

        let err = match result {
            Err(e) => e,
            Ok(summary) => panic!(
                "expected mirror-delete failure to propagate as Err, \
                 got Ok(summary={:?})",
                summary
            ),
        };
        let msg = format!("{err:#}");
        assert!(
            msg.contains("mirror-delete left") && msg.contains("in place"),
            "expected mirror-delete-left-in-place message, got: {msg}"
        );
    }

    /// R48-F1 regression: a normal mirror that contains a
    /// subdirectory with a source file must succeed. Pre-fix
    /// `source_paths` was a set of file paths only; every dest
    /// directory was "absent at source" and got queued for
    /// `remove_dir`, which (after R46-F5 promoted those failures
    /// to hard errors) failed the whole mirror with ENOTEMPTY on
    /// the parent dir that contained the freshly-copied file.
    #[tokio::test]
    async fn mirror_with_subdir_does_not_treat_parent_dir_as_absent() {
        let tmp = tempdir().unwrap();
        let src = tmp.path().join("src");
        let dst = tmp.path().join("dst");

        // Source: nested file under a subdir.
        write_file(&src.join("sub/file.txt"), b"payload");

        let mut opts = opts();
        opts.mirror = true;
        let orch = TransferOrchestrator::new();
        let summary = orch
            .execute_local_mirror_async(&src, &dst, opts)
            .await
            .unwrap_or_else(|e| panic!("mirror failed: {e:#}"));

        assert!(
            dst.join("sub/file.txt").exists(),
            "destination subdir file must exist after mirror, got summary: {:?}",
            summary
        );
        // The parent dir is implicitly in source — must not have
        // been counted as a deletion.
        assert_eq!(
            summary.deleted_dirs, 0,
            "mirror over a single nested file must not delete any \
             destination directory; got summary: {:?}",
            summary
        );
    }

    /// R48-F1 sibling: a destination dir that the source *doesn't*
    /// reference must still be deleted by mirror.
    #[tokio::test]
    async fn mirror_still_deletes_truly_unrelated_destination_dirs() {
        let tmp = tempdir().unwrap();
        let src = tmp.path().join("src");
        let dst = tmp.path().join("dst");

        write_file(&src.join("kept.txt"), b"src");
        // Pre-existing dest dir that's not part of source.
        std::fs::create_dir_all(dst.join("stale_dir")).unwrap();
        // Plus a stale file inside it, so the delete order has to
        // be deepest-first.
        write_file(&dst.join("stale_dir/extra.txt"), b"stale");

        let mut opts = opts();
        opts.mirror = true;
        let orch = TransferOrchestrator::new();
        let summary = orch
            .execute_local_mirror_async(&src, &dst, opts)
            .await
            .unwrap();

        assert!(dst.join("kept.txt").exists());
        assert!(
            !dst.join("stale_dir/extra.txt").exists(),
            "mirror must still delete files in unrelated dest subdirs"
        );
        assert!(
            !dst.join("stale_dir").exists(),
            "mirror must still delete unrelated dest dirs once empty"
        );
        assert!(summary.deleted_dirs >= 1);
        assert!(summary.deleted_files >= 1);
    }

    /// R58-F4 regression: local dry-run on a directory source must
    /// not create the destination directory. Pre-fix `blit copy
    /// src/ dst/ --dry-run` would create `dst/` on disk.
    #[tokio::test]
    async fn local_dry_run_does_not_create_destination() {
        let tmp = tempdir().unwrap();
        let src = tmp.path().join("src");
        let dst = tmp.path().join("brand_new_dst");
        write_file(&src.join("a.txt"), b"hello");

        let mut opts = opts();
        opts.dry_run = true;
        let orch = TransferOrchestrator::new();
        let _ = orch
            .execute_local_mirror_async(&src, &dst, opts)
            .await
            .unwrap();

        assert!(!dst.exists(), "dry-run must not create destination dir");
    }

    /// R58-F5 regression: single-file local copy must honor
    /// `options.filter`. Pre-fix `execute_single_file_copy`
    /// short-circuited around the enumerator/planner and copied
    /// regardless of filter rules.
    #[tokio::test]
    async fn single_file_copy_honors_filter_excludes() {
        let tmp = tempdir().unwrap();
        let src = tmp.path().join("src.txt");
        let dst = tmp.path().join("dst.txt");
        std::fs::write(&src, b"would-be-copied").unwrap();

        let mut opts = opts();
        // Build a filter that excludes `*.txt`. FileFilter has
        // private compiled-glob caches so we go through
        // clone_without_cache() to construct one cleanly.
        let mut filter = crate::fs_enum::FileFilter::default();
        filter.exclude_files = vec!["*.txt".to_string()];
        opts.filter = filter.clone_without_cache();

        let orch = TransferOrchestrator::new();
        let summary = orch
            .execute_local_mirror_async(&src, &dst, opts)
            .await
            .unwrap();

        assert_eq!(
            summary.copied_files, 0,
            "filter exclusion must skip the file"
        );
        assert!(!dst.exists(), "excluded file must not be copied (R58-F5)");
    }

    /// R58-F6 regression: local mirror with `--exclude '*.log'`
    /// must not try to remove an out-of-scope directory just
    /// because the filter hid its in-scope contents. Pre-fix
    /// `apply_mirror_deletions` enumerated the destination
    /// through the filter, saw the .log file as out-of-scope (so
    /// the dir looked empty), and queued the dir for
    /// `remove_dir` — which failed with ENOTEMPTY because the
    /// .log was actually still inside.
    #[tokio::test]
    async fn local_mirror_subset_keeps_excluded_only_directories() {
        let tmp = tempdir().unwrap();
        let src = tmp.path().join("src");
        let dst = tmp.path().join("dst");
        write_file(&src.join("keep.txt"), b"src-keep");
        // Pre-existing destination structure: a directory that
        // only contains an excluded `.log` file.
        std::fs::create_dir_all(dst.join("logs")).unwrap();
        write_file(&dst.join("logs/app.log"), b"excluded contents");

        let mut opts = opts();
        opts.mirror = true;
        // FilteredSubset is the default; spell it out for clarity.
        opts.delete_scope = crate::orchestrator::LocalMirrorDeleteScope::FilteredSubset;
        opts.filter.exclude_files = vec!["*.log".to_string()];

        let orch = TransferOrchestrator::new();
        let summary = orch
            .execute_local_mirror_async(&src, &dst, opts)
            .await
            .unwrap_or_else(|e| panic!("mirror failed: {e:#}"));

        // Mirror must succeed even though `dst/logs/` contains an
        // out-of-scope file. The `.log` survives, the dir
        // survives, the in-scope file transferred.
        assert!(dst.join("keep.txt").exists());
        assert!(
            dst.join("logs/app.log").exists(),
            "excluded .log file must not be deleted by mirror"
        );
        assert!(
            dst.join("logs").exists(),
            "dir containing only excluded files must survive mirror \
             (R58-F6 — pre-fix this failed with ENOTEMPTY)"
        );
        let _ = summary;
    }

    /// R58-F6 sibling: `--delete-scope=all` deletes through the
    /// filter, including dirs that only hold excluded files. The
    /// user explicitly opted out of subset semantics.
    #[tokio::test]
    async fn local_mirror_all_scope_deletes_through_filter() {
        let tmp = tempdir().unwrap();
        let src = tmp.path().join("src");
        let dst = tmp.path().join("dst");
        write_file(&src.join("keep.txt"), b"src-keep");
        std::fs::create_dir_all(dst.join("logs")).unwrap();
        write_file(&dst.join("logs/app.log"), b"deletable in All mode");

        let mut opts = opts();
        opts.mirror = true;
        opts.delete_scope = crate::orchestrator::LocalMirrorDeleteScope::All;
        opts.filter.exclude_files = vec!["*.log".to_string()];

        let orch = TransferOrchestrator::new();
        let _ = orch
            .execute_local_mirror_async(&src, &dst, opts)
            .await
            .unwrap();

        assert!(dst.join("keep.txt").exists());
        assert!(
            !dst.join("logs/app.log").exists(),
            "All scope must delete excluded files at destination"
        );
        assert!(
            !dst.join("logs").exists(),
            "All scope must delete the now-empty dir"
        );
    }

    /// R58-F7 regression: local copy honors `compare_mode =
    /// SizeOnly`. With a destination that has the same SIZE but
    /// different MTIME, default SizeMtime would re-copy
    /// (mtime differs); SizeOnly must skip.
    #[tokio::test]
    async fn local_copy_honors_size_only_compare_mode() {
        let tmp = tempdir().unwrap();
        let src = tmp.path().join("src");
        let dst = tmp.path().join("dst");
        write_file(&src.join("file.txt"), b"AAAA"); // 4 bytes
        write_file(&dst.join("file.txt"), b"BBBB"); // 4 bytes, different content

        // Bump the source mtime so SizeMtime would re-copy.
        let now = std::time::SystemTime::now();
        let later = now + std::time::Duration::from_secs(10);
        filetime::set_file_mtime(
            src.join("file.txt"),
            filetime::FileTime::from_system_time(later),
        )
        .unwrap();
        filetime::set_file_mtime(
            dst.join("file.txt"),
            filetime::FileTime::from_system_time(now),
        )
        .unwrap();

        let mut opts = opts();
        opts.compare_mode = crate::orchestrator::LocalCompareMode::SizeOnly;

        let orch = TransferOrchestrator::new();
        let _ = orch
            .execute_local_mirror_async(&src, &dst, opts)
            .await
            .unwrap();

        // SizeOnly: same size → skip → dst content unchanged.
        assert_eq!(
            std::fs::read(dst.join("file.txt")).unwrap(),
            b"BBBB",
            "SizeOnly compare must skip when sizes match (R58-F7)"
        );
    }

    /// R58-F7 regression: local copy honors `compare_mode = Force`.
    /// With matching size+mtime, default SizeMtime would skip;
    /// Force must always re-copy.
    #[tokio::test]
    async fn local_copy_honors_force_compare_mode() {
        let tmp = tempdir().unwrap();
        let src = tmp.path().join("src");
        let dst = tmp.path().join("dst");
        write_file(&src.join("file.txt"), b"CCCC");
        write_file(&dst.join("file.txt"), b"OLD!");

        // Match size+mtime so SizeMtime would skip.
        let t = filetime::FileTime::from_unix_time(1_700_000_000, 0);
        filetime::set_file_mtime(src.join("file.txt"), t).unwrap();
        filetime::set_file_mtime(dst.join("file.txt"), t).unwrap();

        let mut opts = opts();
        opts.compare_mode = crate::orchestrator::LocalCompareMode::Force;

        let orch = TransferOrchestrator::new();
        let _ = orch
            .execute_local_mirror_async(&src, &dst, opts)
            .await
            .unwrap();

        assert_eq!(
            std::fs::read(dst.join("file.txt")).unwrap(),
            b"CCCC",
            "Force compare must always copy even when size+mtime match (R58-F7)"
        );
    }

    /// R58-F5 regression: single-file local copy must honor
    /// `--ignore-existing`. Pre-fix the short-circuit overwrote
    /// the destination regardless.
    #[tokio::test]
    async fn single_file_copy_honors_ignore_existing() {
        let tmp = tempdir().unwrap();
        let src = tmp.path().join("src.txt");
        let dst = tmp.path().join("dst.txt");
        std::fs::write(&src, b"new-content").unwrap();
        std::fs::write(&dst, b"existing-pre-existing").unwrap();

        let mut opts = opts();
        opts.ignore_existing = true;

        let orch = TransferOrchestrator::new();
        let summary = orch
            .execute_local_mirror_async(&src, &dst, opts)
            .await
            .unwrap();

        assert_eq!(
            summary.copied_files, 0,
            "--ignore-existing must skip when destination exists"
        );
        assert_eq!(
            std::fs::read(&dst).unwrap(),
            b"existing-pre-existing",
            "destination content must be preserved (R58-F5)"
        );
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
    use crate::perf_history::{OptionSnapshot, PerformanceRecord, RunKind, TransferMode};

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

        let window = select_tuning_window(&history, TransferMode::Copy, false, true);
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
        let window = select_tuning_window(&history, TransferMode::Copy, false, true);
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
        let window = select_tuning_window(&history, TransferMode::Copy, false, true);
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
        let window = select_tuning_window(&history, TransferMode::Copy, false, true);
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
        let window = select_tuning_window(&history, TransferMode::Copy, false, true);
        assert_eq!(
            window.len(),
            5,
            "expected the 5 real records to survive a flood of non-real history"
        );
        assert!(window.iter().all(|r| r.run_kind.is_real_transfer()));
        assert!(derive_local_plan_tuning(&window).is_some());
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
        let _ = select_tuning_window_from_history(reader, TransferMode::Copy, false, true);
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
        let result = select_tuning_window_from_history(reader, TransferMode::Copy, false, true);
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
        let result = select_tuning_window_from_history(reader, TransferMode::Copy, false, true);
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
        let result =
            select_tuning_window_from_history(reader, TransferMode::Copy, false, true).unwrap();
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
        let window = select_tuning_window(&history, TransferMode::Copy, false, true);
        assert_eq!(window.len(), 1);
        assert_eq!(window[0].mode, TransferMode::Copy);
    }
}
