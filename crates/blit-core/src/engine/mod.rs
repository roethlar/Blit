//! The unified transfer engine (`ue-r2-1c`, REV4 Design §1).
//!
//! `TransferEngine` owns transfer execution: strategy selection
//! (`journal_no_work`, `no_work`, `tiny_manifest`, `single_huge_file`,
//! the single-file shortcut, streaming pipeline), the streaming leg
//! (plan tuning -> scan -> diff/plan -> sink pipeline -> mirror
//! deletions), and the perf-history/predictor accounting hooks. Path
//! adapters construct the source, sink, and options, then call
//! [`TransferEngine::execute`]; `TransferOrchestrator` is the local
//! adapter today, and push/pull converge here at `ue-r2-1f`/`1g`.
//! Dial creation and streaming plans arrive with `ue-r2-1d`/`1e`
//! (REV4 "Slice dependencies").
//!
//! The option/summary types keep their `LocalMirror*` names until the
//! remote paths converge -- renaming ahead of those slices would churn
//! every caller twice.

mod history;
mod journal;
mod mirror;
mod options;
mod single_file;
mod strategy;
mod summary;
mod tuning;

pub use options::{LocalCompareMode, LocalMirrorDeleteScope, LocalMirrorOptions};
pub use summary::{LocalMirrorSummary, TransferOutcome};

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use eyre::{bail, Context, Result};

use crate::auto_tune::derive_local_plan_tuning;
use crate::change_journal::{ChangeState, ChangeTracker, ProbeToken};
use crate::local_worker::{copy_large_blocking, copy_paths_blocking};
use crate::perf_history::{read_recent_records, TransferMode};
use crate::perf_predictor::PerformancePredictor;
use crate::remote::transfer::diff_planner::{plan_local_mirror, LocalDiffInputs};
use crate::remote::transfer::payload::DEFAULT_PAYLOAD_PREFETCH;
use crate::remote::transfer::pipeline::execute_sink_pipeline;
use crate::remote::transfer::sink::TransferSink;
use crate::remote::transfer::source::TransferSource;
use crate::transfer_plan::PlanOptions;
use crate::CopyConfig;

use self::history::{record_performance_history, update_predictor};
use self::journal::{log_probe, persist_journal_checkpoints};
use self::mirror::apply_mirror_deletions;
use self::single_file::execute_single_file_copy;
use self::strategy::{maybe_select_fast_path, FastPathDecision};
use self::tuning::select_tuning_window_from_history;

/// Everything the engine needs to run one transfer. The adapter owns
/// path-specific construction (REV4 Design §1): it resolves roots,
/// builds the (already filter-wrapped) source and the sink, translates
/// its option surface, and hands over execution.
pub struct EngineRequest {
    pub src_root: PathBuf,
    pub dest_root: PathBuf,
    /// Filter-wrapped source; used by the streaming strategy's scan.
    pub source: Arc<dyn TransferSource>,
    /// Destination sink for the streaming strategy (`FsTransferSink`
    /// or `NullSink` locally). Fast-path strategies use their own
    /// blocking executors, exactly as before the engine existed.
    pub sink: Arc<dyn TransferSink>,
    pub options: LocalMirrorOptions,
}

/// The unified transfer engine. Stateless today (all state is
/// per-execute); the live dial (`ue-r2-1e`) is the first field that
/// will change that.
pub struct TransferEngine;

impl TransferEngine {
    pub fn new() -> Self {
        Self
    }

    /// Execute one transfer: select a strategy (single-file, journal
    /// no-work, fast path, or streaming pipeline) and run it to a
    /// summary. Behavior moved verbatim from
    /// `TransferOrchestrator::execute_local_mirror_async` at
    /// ue-r2-1c; the caller-visible contract is unchanged.
    pub async fn execute(&self, request: EngineRequest) -> Result<LocalMirrorSummary> {
        let EngineRequest {
            src_root,
            dest_root,
            source,
            sink,
            options,
        } = request;
        let src_root = src_root.as_path();
        let dest_root = dest_root.as_path();

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
            self::strategy::FastPathOutcome::streaming()
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
            // R59 finding #5: tuning window keys on full compare_mode,
            // not just options.checksum. Translate via the same enum
            // the history snapshot uses so the bucket lookup matches
            // what the writer recorded.
            let query_compare_mode = options
                .compare_mode
                .resolve_compare_snapshot(options.checksum);
            if let Some(filtered) = select_tuning_window_from_history(
                read_recent_records,
                target_mode,
                query_compare_mode,
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
        let compare_mode = options
            .compare_mode
            .resolve_comparison_mode(options.checksum);

        // 1. Scan source via FsTransferSource, wrapped in FilteredSource so
        //    the user filter applies through the universal pipeline chokepoint
        //    (identical to push/pull/remote-remote behavior — full parity).
        // ue-r2-1c: the adapter built the (filter-wrapped) source; the
        // engine owns running the scan.
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

        // 5. Execute the unified pipeline against the adapter-built
        // sink (FsTransferSink with the translated compare_mode, or
        // NullSink -- see TransferOrchestrator).

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
            Some(self::summary::PredictorEstimate {
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

impl Default for TransferEngine {
    fn default() -> Self {
        Self::new()
    }
}
