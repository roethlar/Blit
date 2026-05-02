use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use eyre::{eyre, Context, Result};
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

    pub fn execute_local_mirror(
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
                                eprintln!(
                                    "Journal probe dest unsupported; cannot take fast-path"
                                );
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
            super::fast_path::FastPathOutcome { decision: None }
        } else {
            maybe_select_fast_path(src_root, dest_root, &options)?
        };
        if let Some(decision) = fast_path_outcome.decision {
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
                    let summary = LocalMirrorSummary {
                        planned_files: examined,
                        dry_run: options.dry_run,
                        duration: start_time.elapsed(),
                        outcome,
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
                    let summary = LocalMirrorSummary {
                        planned_files: files.len(),
                        copied_files: files.len(),
                        total_bytes,
                        dry_run: options.dry_run,
                        duration: start_time.elapsed(),
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
                    let summary = LocalMirrorSummary {
                        planned_files: 1,
                        copied_files: 1,
                        total_bytes: size,
                        dry_run: options.dry_run,
                        duration: start_time.elapsed(),
                        large_tasks: 1,
                        large_bytes: size,
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

        let runtime = Builder::new_multi_thread()
            .worker_threads(options.workers.max(1))
            .enable_all()
            .build()
            .context("build tokio runtime")?;

        let src_root_buf = src_root.to_path_buf();
        let dest_root_buf = dest_root.to_path_buf();
        let filter = options.filter.clone_without_cache();
        let skip_unchanged = options.skip_unchanged;
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

        let pipeline_result = runtime.block_on(async {
            // 1. Scan source via FsTransferSource, wrapped in FilteredSource so
            //    the user filter applies through the universal pipeline chokepoint
            //    (identical to push/pull/remote-remote behavior — full parity).
            let inner: Arc<dyn TransferSource> =
                Arc::new(FsTransferSource::new(src_root_buf.clone()));
            let source: Arc<dyn TransferSource> = Arc::new(FilteredSource::new(inner, filter));
            let unreadable = Arc::new(Mutex::new(Vec::new()));
            let (mut header_rx, scan_handle) = source.scan(None, unreadable);

            // 2. Collect all headers
            let mut all_headers = Vec::new();
            while let Some(h) = header_rx.recv().await {
                all_headers.push(h);
            }
            let _total_scanned = scan_handle.await
                .context("scan task panicked")?
                .context("scan failed")?;

            // 3. Diff + plan via the shared DiffPlanner stage. Combines
            //    the comparison-filter and payload-planning steps that
            //    were previously inline. Behavior preserved bit-for-bit
            //    (size+mtime or Blake3 hash, then tar/large/raw planning).
            let src = src_root_buf.clone();
            let dst = dest_root_buf.clone();
            let plan_opts = plan_options.clone();
            let headers = all_headers.clone();
            let planned = tokio::task::spawn_blocking(move || {
                plan_local_mirror(
                    headers,
                    LocalDiffInputs {
                        src_root: &src,
                        dst_root: &dst,
                        compare_mode,
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

            let outcome = execute_sink_pipeline(
                source,
                vec![sink],
                planned.payloads,
                DEFAULT_PAYLOAD_PREFETCH,
                None,
            )
            .await
            .context("transfer pipeline failed")?;

            Ok::<_, eyre::Report>((all_headers, outcome))
        })?;

        let (all_headers, pipeline_outcome) = pipeline_result;
        let planner_duration_ms = planning_start.elapsed().as_millis();

        let total_bytes: u64 = all_headers.iter().map(|h| h.size).sum();
        let mut summary = LocalMirrorSummary {
            planned_files: pipeline_outcome.files_written,
            copied_files: pipeline_outcome.files_written,
            total_bytes,
            dry_run: options.dry_run,
            duration: start_time.elapsed(),
            ..Default::default()
        };

        if options.mirror {
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
                all_headers.len(), total_bytes
            );
            eprintln!(
                "Completed local {}: {} file(s), {} bytes in {:.2?}",
                if options.mirror { "mirror" } else { "copy" },
                summary.copied_files,
                summary.total_bytes,
                summary.duration
            );
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
            summary.duration.as_millis(),
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
                }
            }
        } else {
            deleted_dirs += 1;
        }
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
            let _ = filetime::set_file_mtime(dest_root, ft);
        }
    }

    Ok(LocalMirrorSummary {
        planned_files: 1,
        copied_files: if did_copy { 1 } else { 0 },
        total_bytes: bytes_copied,
        duration: start_time.elapsed(),
        outcome: if did_copy {
            TransferOutcome::Transferred
        } else {
            TransferOutcome::UpToDate
        },
        ..Default::default()
    })
}
