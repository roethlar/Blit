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
            if let Ok(history) = read_recent_records(50) {
                let target_mode = if options.mirror {
                    TransferMode::Mirror
                } else {
                    TransferMode::Copy
                };
                let filtered: Vec<_> = history
                    .iter()
                    .rev()
                    .filter(|record| record.mode == target_mode)
                    .filter(|record| record.options.checksum == options.checksum)
                    .filter(|record| record.options.skip_unchanged == options.skip_unchanged)
                    .filter(|record| record.fast_path.as_deref() != Some("tiny_manifest"))
                    .take(20)
                    .cloned()
                    .collect();
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
        // Translate the orchestrator's bool `checksum` flag onto the
        // unified ComparisonMode enum. Other variants (SizeOnly,
        // IgnoreTimes, etc.) become first-class once pull_sync.rs
        // migrates in step 4 and brings their behavior into the
        // shared comparison primitives.
        let compare_mode = if options.checksum {
            ComparisonMode::Checksum
        } else {
            ComparisonMode::SizeMtime
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
fn apply_mirror_deletions(
    source_paths: &HashSet<String>,
    dest_root: &Path,
    filter: &FileFilter,
    perform: bool,
    verbose: bool,
) -> Result<(usize, usize)> {
    use crate::enumeration::{EntryKind, FileEnumerator};

    let enumerator = FileEnumerator::new(filter.clone_without_cache());
    let dest_entries = enumerator.enumerate_local(dest_root)?;

    let mut files_to_delete = Vec::new();
    let mut dirs_to_delete = Vec::new();

    for entry in &dest_entries {
        let rel = entry.relative_path.to_string_lossy().replace('\\', "/");
        if !source_paths.contains(&rel) {
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
                    eprintln!("Failed to delete directory {}: {}", path.display(), err);
                    failures.push(format!("{}: {}", path.display(), err));
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
}
