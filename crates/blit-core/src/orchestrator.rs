use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{anyhow, Context, Result};
use tokio::runtime::Builder;

use crate::enumeration::{EntryKind, FileEnumerator};
use crate::fs_enum::{CopyJob, FileEntry, FileFilter};
use crate::mirror_planner::MirrorPlanner;
use crate::perf_history::{append_local_record, OptionSnapshot, PerformanceRecord, TransferMode};
use crate::perf_predictor::PerformancePredictor;
use crate::transfer_engine::{
    create_task_stream, execute_streaming_plan, SchedulerOptions, TaskStreamSender,
};
use crate::transfer_facade::{PlannerEvent, TransferFacade};
use crate::transfer_plan::PlanOptions;
use crate::{
    local_worker::{copy_large_blocking, copy_paths_blocking, LocalWorkerFactory},
    CopyConfig,
};

/// Options for executing a local mirror/copy operation.
#[derive(Clone, Debug)]
pub struct LocalMirrorOptions {
    pub filter: FileFilter,
    pub mirror: bool,
    pub dry_run: bool,
    pub progress: bool,
    pub verbose: bool,
    pub ludicrous_speed: bool,
    pub force_tar: bool,
    pub preserve_symlinks: bool,
    pub include_symlinks: bool,
    pub skip_unchanged: bool,
    pub checksum: bool,
    pub workers: usize,
    pub preserve_times: bool,
}

impl Default for LocalMirrorOptions {
    fn default() -> Self {
        Self {
            filter: FileFilter::default(),
            mirror: false,
            dry_run: false,
            progress: false,
            verbose: false,
            ludicrous_speed: false,
            force_tar: false,
            preserve_symlinks: true,
            include_symlinks: true,
            skip_unchanged: true,
            checksum: false,
            workers: num_cpus::get().max(1),
            preserve_times: true,
        }
    }
}

/// Summary of a local transfer execution.
#[derive(Clone, Debug, Default)]
pub struct LocalMirrorSummary {
    pub planned_files: usize,
    pub copied_files: usize,
    pub total_bytes: u64,
    pub deleted_files: usize,
    pub deleted_dirs: usize,
    pub dry_run: bool,
    pub duration: Duration,
}

const TINY_FILE_LIMIT: usize = 8;
const TINY_TOTAL_BYTES: u64 = 100 * 1024 * 1024;
const HUGE_SINGLE_BYTES: u64 = 1024 * 1024 * 1024;
const PREDICT_STREAMING_THRESHOLD_MS: f64 = 1_000.0;

#[derive(Clone, Debug)]
enum FastPathDecision {
    NoWork,
    Tiny { files: Vec<(PathBuf, u64)> },
    Huge { file: PathBuf, size: u64 },
}

#[derive(Clone, Debug, Default)]
struct FastPathOutcome {
    decision: Option<FastPathDecision>,
    prediction: Option<(f64, u64)>,
}

impl FastPathOutcome {
    fn fast_path(decision: FastPathDecision, prediction: Option<(f64, u64)>) -> Self {
        Self {
            decision: Some(decision),
            prediction,
        }
    }

    fn streaming(prediction: Option<(f64, u64)>) -> Self {
        Self {
            decision: None,
            prediction,
        }
    }
}

#[derive(Debug)]
struct FastPathAbort;

impl std::fmt::Display for FastPathAbort {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "fast-path aborted")
    }
}

impl std::error::Error for FastPathAbort {}

pub struct TransferOrchestrator;

fn maybe_select_fast_path(
    src_root: &Path,
    dest_root: &Path,
    options: &LocalMirrorOptions,
    predictor: Option<&PerformancePredictor>,
) -> Result<FastPathOutcome> {
    if options.mirror || options.checksum || options.force_tar {
        return Ok(FastPathOutcome::streaming(None));
    }

    let mut enumerator = FileEnumerator::new(options.filter.clone_without_cache());
    if !options.preserve_symlinks {
        enumerator = enumerator.follow_symlinks(true);
    }
    if options.include_symlinks {
        enumerator = enumerator.include_symlinks(true);
    }

    let mode = if options.mirror {
        TransferMode::Mirror
    } else {
        TransferMode::Copy
    };

    let planner = MirrorPlanner::new(options.checksum);
    let mut files: Vec<(PathBuf, u64)> = Vec::new();
    let mut total_bytes: u64 = 0;
    let mut aborted = false;
    let mut huge_candidate: Option<(PathBuf, u64)> = None;

    let scan_result = enumerator.enumerate_local_streaming(src_root, |entry| {
        if let EntryKind::File { size } = entry.kind {
            let should_copy = if options.skip_unchanged {
                let job = CopyJob {
                    entry: FileEntry {
                        path: entry.absolute_path.clone(),
                        size,
                        is_directory: false,
                    },
                };
                planner.should_copy_entry(&job, src_root, dest_root)
            } else {
                true
            };

            if should_copy {
                if files.is_empty() {
                    huge_candidate = Some((entry.relative_path.clone(), size));
                } else {
                    huge_candidate = None;
                }

                files.push((entry.relative_path.clone(), size));
                total_bytes += size;

                if files.len() > TINY_FILE_LIMIT {
                    aborted = true;
                    return Err(FastPathAbort.into());
                }

                if total_bytes > TINY_TOTAL_BYTES && files.len() > 1 {
                    aborted = true;
                    return Err(FastPathAbort.into());
                }
            }
        }

        Ok(())
    });

    match scan_result {
        Ok(()) => {}
        Err(err) => {
            if err.downcast_ref::<FastPathAbort>().is_none() {
                return Err(err);
            }
        }
    }

    if aborted {
        return Ok(FastPathOutcome::streaming(None));
    }

    if files.is_empty() {
        return Ok(FastPathOutcome::fast_path(FastPathDecision::NoWork, None));
    }

    let prediction =
        predictor.and_then(|p| p.predict_planner_ms(mode.clone(), None, files.len(), total_bytes));

    if files.len() <= TINY_FILE_LIMIT && total_bytes <= TINY_TOTAL_BYTES {
        let use_fast_path = prediction
            .map(|(ms, observations)| observations == 0 || ms > PREDICT_STREAMING_THRESHOLD_MS)
            .unwrap_or(true);
        if use_fast_path {
            return Ok(FastPathOutcome::fast_path(
                FastPathDecision::Tiny { files },
                prediction,
            ));
        }
    }

    if let Some((file, size)) = huge_candidate {
        if size >= HUGE_SINGLE_BYTES {
            return Ok(FastPathOutcome::fast_path(
                FastPathDecision::Huge { file, size },
                None,
            ));
        }
    }

    Ok(FastPathOutcome::streaming(prediction))
}

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
            return Err(anyhow!(
                "source path does not exist: {}",
                src_root.display()
            ));
        }

        if !options.dry_run {
            if let Some(parent) = dest_root.parent() {
                std::fs::create_dir_all(parent).with_context(|| {
                    format!("failed to create destination parent {}", parent.display())
                })?;
            }
        }

        let start_time = Instant::now();

        let mut predictor = PerformancePredictor::load().ok();

        let mut copy_config = CopyConfig::default();
        copy_config.workers = options.workers.max(1);
        copy_config.preserve_times = options.preserve_times;
        copy_config.dry_run = options.dry_run;
        copy_config.checksum = if options.checksum {
            Some(crate::checksum::ChecksumType::Blake3)
        } else {
            None
        };

        let fast_path_outcome =
            maybe_select_fast_path(src_root, dest_root, &options, predictor.as_ref())?;
        let streaming_prediction = fast_path_outcome.prediction.clone();
        if let Some(decision) = fast_path_outcome.decision.clone() {
            let summary = match decision {
                FastPathDecision::NoWork => {
                    if options.verbose {
                        eprintln!("Fast-path routing: no work required (all files up to date)");
                    }
                    let summary = LocalMirrorSummary {
                        dry_run: options.dry_run,
                        duration: start_time.elapsed(),
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

        if options.verbose {
            if let Some((pred_ms, observations)) = streaming_prediction {
                if pred_ms > 0.0 {
                    eprintln!(
                        "Predictor estimate: planning ≈ {:.0} ms ({} observation{})",
                        pred_ms,
                        observations,
                        if observations == 1 { "" } else { "s" }
                    );
                }
            }
        }

        let mut filter = options.filter.clone_without_cache();
        let planner_for_stream = MirrorPlanner::new(options.checksum);
        let plan_options = PlanOptions {
            ludicrous: options.ludicrous_speed,
            force_tar: options.force_tar,
        };

        let planning_start = Instant::now();

        let stream = TransferFacade::stream_local_plan(
            src_root,
            dest_root,
            &mut filter,
            options.preserve_symlinks,
            options.include_symlinks,
            options.skip_unchanged,
            planner_for_stream,
            plan_options,
        )?;

        let (events, plan_handle) = stream.into_parts();

        // Prepare task queue for the transfer engine.
        let (task_sender, task_receiver) = create_task_stream(1024);
        let remaining = task_sender.remaining();
        let closed_flag = task_sender.closed_flag();

        let worker_factory = LocalWorkerFactory {
            src_root: src_root.to_path_buf(),
            dest_root: dest_root.to_path_buf(),
            config: copy_config.clone(),
        };

        let scheduler_opts = SchedulerOptions {
            ludicrous_speed: options.ludicrous_speed,
            progress: options.progress || options.verbose,
            byte_drain: None,
            initial_streams: Some(options.workers.min(12).max(1)),
            max_streams: Some(options.workers.max(1)),
        };

        // Default chunk size mirrors historic plan logic.
        let chunk_bytes = if options.ludicrous_speed {
            32 * 1024 * 1024
        } else {
            16 * 1024 * 1024
        };

        let runtime = Builder::new_multi_thread()
            .worker_threads(options.workers.max(1))
            .enable_all()
            .build()
            .context("build tokio runtime")?;

        let transfer_future = execute_streaming_plan(
            &worker_factory,
            chunk_bytes,
            scheduler_opts,
            task_receiver,
            Arc::clone(&remaining),
            Arc::clone(&closed_flag),
        );

        let stall_timeout = Duration::from_secs(10);
        let heartbeat = Duration::from_millis(500);

        let planner_future = drive_planner_events(
            &options,
            events,
            task_sender,
            Arc::clone(&remaining),
            Arc::clone(&closed_flag),
            stall_timeout,
            heartbeat,
        );

        let (transfer_result, planner_stats) =
            runtime.block_on(async { tokio::join!(transfer_future, planner_future) });

        transfer_result?;
        let drive_summary = planner_stats?;
        let plan_final = plan_handle.wait()?;
        let planner_duration_ms = planning_start.elapsed().as_millis();

        let mut summary = LocalMirrorSummary {
            planned_files: plan_final.copy_jobs.len(),
            copied_files: plan_final.copy_jobs.len(),
            total_bytes: plan_final.total_bytes,
            dry_run: options.dry_run,
            duration: start_time.elapsed(),
            ..Default::default()
        };

        if options.mirror {
            let deletion_planner = MirrorPlanner::new(options.checksum);
            let deletions = apply_local_deletions(
                &plan_final.entries,
                dest_root,
                &deletion_planner,
                &options.filter,
                !options.dry_run,
                options.verbose,
            )?;
            summary.deleted_files = deletions.0;
            summary.deleted_dirs = deletions.1;
        }

        if options.verbose {
            eprintln!(
                "Planning enumerated {} file(s), {} bytes",
                drive_summary.enumerated_files, drive_summary.total_bytes
            );
            eprintln!(
                "Completed local {}: {} file(s), {} bytes in {:.2?}",
                if options.mirror { "mirror" } else { "copy" },
                summary.copied_files,
                summary.total_bytes,
                summary.duration
            );
        }

        if let Some(record) = record_performance_history(
            &summary,
            &options,
            None,
            planner_duration_ms,
            summary.duration.as_millis(),
        ) {
            update_predictor(&mut predictor, &record, options.verbose);
        }

        Ok(summary)
    }
}

struct PlannerDriveSummary {
    enumerated_files: usize,
    total_bytes: u64,
}

async fn drive_planner_events(
    options: &LocalMirrorOptions,
    mut events: tokio::sync::mpsc::UnboundedReceiver<PlannerEvent>,
    task_sender: TaskStreamSender,
    remaining: Arc<AtomicUsize>,
    closed_flag: Arc<AtomicBool>,
    stall_timeout: Duration,
    heartbeat: Duration,
) -> Result<PlannerDriveSummary> {
    let mut last_planner_activity = Instant::now();
    let mut last_worker_remaining = remaining.load(Ordering::Relaxed);
    let mut last_worker_activity = Instant::now();
    let mut enumerated_files = 0usize;
    let mut total_bytes = 0u64;

    let mut ticker = tokio::time::interval(heartbeat);
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    let mut sender = Some(task_sender);

    loop {
        tokio::select! {
            maybe_event = events.recv() => {
                match maybe_event {
                    Some(PlannerEvent::Task(task)) => {
                        if let Some(ref s) = sender {
                            s.send(task).await?;
                        }
                        last_planner_activity = Instant::now();
                    }
                    Some(PlannerEvent::Progress { enumerated_files: files, total_bytes: bytes }) => {
                        enumerated_files = files;
                        total_bytes = bytes;
                        last_planner_activity = Instant::now();
                        if options.verbose {
                            eprintln!("Planning… {} file(s), {} bytes", files, bytes);
                        }
                    }
                    None => {
                        break;
                    }
                }
            }
            _ = ticker.tick() => {
                let now = Instant::now();
                let current_remaining = remaining.load(Ordering::Relaxed);
                if current_remaining < last_worker_remaining {
                    last_worker_remaining = current_remaining;
                    last_worker_activity = now;
                }

                if now.duration_since(last_planner_activity) >= stall_timeout
                    && now.duration_since(last_worker_activity) >= stall_timeout
                    && (!closed_flag.load(Ordering::SeqCst) || current_remaining > 0)
                {
                    return Err(anyhow!("planner or workers stalled for > {:?}", stall_timeout));
                }
            }
        }
    }

    // Close the task sender to signal no further work.
    drop(sender.take());

    Ok(PlannerDriveSummary {
        enumerated_files,
        total_bytes,
    })
}

fn apply_local_deletions(
    entries: &[crate::enumeration::EnumeratedEntry],
    dest_root: &Path,
    planner: &MirrorPlanner,
    filter: &FileFilter,
    perform: bool,
    verbose: bool,
) -> Result<(usize, usize)> {
    let delete_plan = planner.plan_local_deletions_from_entries(entries, dest_root, filter)?;
    let mut deleted_files = 0usize;
    let mut deleted_dirs = 0usize;

    for path in delete_plan.files {
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
                    if verbose {
                        eprintln!("Failed to delete file {}: {}", path.display(), err);
                    }
                }
            }
        } else {
            deleted_files += 1;
        }
    }

    for path in delete_plan.dirs {
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
                    if verbose {
                        eprintln!("Failed to delete directory {}: {}", path.display(), err);
                    }
                }
            }
        } else {
            deleted_dirs += 1;
        }
    }

    Ok((deleted_files, deleted_dirs))
}

fn record_performance_history(
    summary: &LocalMirrorSummary,
    options: &LocalMirrorOptions,
    fast_path: Option<&'static str>,
    planner_duration_ms: u128,
    transfer_duration_ms: u128,
) -> Option<PerformanceRecord> {
    let options_snapshot = OptionSnapshot {
        dry_run: options.dry_run,
        preserve_symlinks: options.preserve_symlinks,
        include_symlinks: options.include_symlinks,
        skip_unchanged: options.skip_unchanged,
        checksum: options.checksum,
        workers: options.workers.max(1),
    };

    let record = PerformanceRecord::new(
        if options.mirror {
            TransferMode::Mirror
        } else {
            TransferMode::Copy
        },
        None,
        None,
        summary.planned_files,
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

fn update_predictor(
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
    use super::*;
    use crate::perf_history::{OptionSnapshot, PerformanceRecord, TransferMode};
    use crate::perf_predictor::PerformancePredictor;
    use anyhow::Result;
    use tempfile::tempdir;

    struct EnvGuard {
        key: &'static str,
        prev: Option<String>,
    }

    impl EnvGuard {
        fn set(key: &'static str, value: &str) -> Self {
            let prev = std::env::var(key).ok();
            std::env::set_var(key, value);
            Self { key, prev }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            if let Some(prev) = &self.prev {
                std::env::set_var(self.key, prev);
            } else {
                std::env::remove_var(self.key);
            }
        }
    }

    #[test]
    fn tiny_fast_path_without_history_prefers_fastpath() -> Result<()> {
        let _guard = EnvGuard::set("BLIT_DISABLE_PERF_HISTORY", "1");
        let temp = tempdir()?;
        let src = temp.path().join("src");
        let dest = temp.path().join("dest");
        std::fs::create_dir_all(&src)?;
        std::fs::create_dir_all(&dest)?;
        std::fs::write(src.join("file.txt"), b"hello")?;

        let options = LocalMirrorOptions::default();
        let outcome = maybe_select_fast_path(&src, &dest, &options, None)?;
        assert!(matches!(
            outcome.decision,
            Some(FastPathDecision::Tiny { .. })
        ));
        Ok(())
    }

    #[test]
    fn tiny_fast_path_uses_predictor_when_history_exists() -> Result<()> {
        let _guard = EnvGuard::set("BLIT_DISABLE_PERF_HISTORY", "1");
        let temp = tempdir()?;
        let src = temp.path().join("src");
        let dest = temp.path().join("dest");
        std::fs::create_dir_all(&src)?;
        std::fs::create_dir_all(&dest)?;
        std::fs::write(src.join("file.txt"), b"hello")?;

        let mut predictor = PerformancePredictor::for_tests(temp.path());
        let snapshot = OptionSnapshot {
            dry_run: false,
            preserve_symlinks: true,
            include_symlinks: true,
            skip_unchanged: true,
            checksum: false,
            workers: 4,
        };
        let record = PerformanceRecord::new(
            TransferMode::Copy,
            None,
            None,
            2,
            256,
            snapshot,
            None,
            100,
            1_000,
            0,
            0,
        );
        predictor.observe(&record);

        let options = LocalMirrorOptions::default();
        let outcome = maybe_select_fast_path(&src, &dest, &options, Some(&predictor))?;
        assert!(
            outcome.decision.is_none(),
            "predictor should keep streaming path when predicted planning is fast"
        );
        let (pred_ms, _) = outcome.prediction.expect("expected prediction");
        assert!(pred_ms <= PREDICT_STREAMING_THRESHOLD_MS);
        Ok(())
    }
}
