use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use eyre::{eyre, Context, Result};
use tokio::runtime::Builder;

use crate::fs_enum::FileFilter;
use crate::mirror_planner::MirrorPlanner;
use crate::perf_predictor::PerformancePredictor;
use crate::transfer_engine::{create_task_stream, execute_streaming_plan, SchedulerOptions};
use crate::transfer_facade::TransferFacade;
use crate::transfer_plan::PlanOptions;
use crate::{
    local_worker::{copy_large_blocking, copy_paths_blocking, LocalWorkerFactory},
    CopyConfig,
};

mod fast_path;
mod history;
mod planner;

use fast_path::{maybe_select_fast_path, FastPathDecision};
use history::{record_performance_history, update_predictor};
use planner::drive_planner_events;

/// Options for executing a local mirror/copy operation.
#[derive(Clone, Debug)]
pub struct LocalMirrorOptions {
    pub filter: FileFilter,
    pub mirror: bool,
    pub dry_run: bool,
    pub progress: bool,
    pub verbose: bool,
    pub perf_history: bool,
    pub force_tar: bool,
    pub preserve_symlinks: bool,
    pub include_symlinks: bool,
    pub skip_unchanged: bool,
    pub checksum: bool,
    pub workers: usize,
    pub preserve_times: bool,
    pub debug_mode: bool,
}

impl Default for LocalMirrorOptions {
    fn default() -> Self {
        Self {
            filter: FileFilter::default(),
            mirror: false,
            dry_run: false,
            progress: false,
            verbose: false,
            perf_history: true,
            force_tar: false,
            preserve_symlinks: true,
            include_symlinks: true,
            skip_unchanged: true,
            checksum: false,
            workers: num_cpus::get().max(1),
            preserve_times: true,
            debug_mode: false,
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
    pub tar_shard_tasks: usize,
    pub tar_shard_files: usize,
    pub tar_shard_bytes: u64,
    pub raw_bundle_tasks: usize,
    pub raw_bundle_files: usize,
    pub raw_bundle_bytes: u64,
    pub large_tasks: usize,
    pub large_bytes: u64,
}

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
            progress: options.progress || options.verbose,
            byte_drain: None,
            initial_streams: Some(options.workers.min(12).max(1)),
            max_streams: Some(options.workers.max(1)),
        };

        // Default chunk size mirrors historic plan logic.
        let chunk_bytes = 16 * 1024 * 1024;

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
        summary.tar_shard_tasks = plan_final.task_stats.tar_shard_tasks;
        summary.tar_shard_files = plan_final.task_stats.tar_shard_files;
        summary.tar_shard_bytes = plan_final.task_stats.tar_shard_bytes;
        summary.raw_bundle_tasks = plan_final.task_stats.raw_bundle_tasks;
        summary.raw_bundle_files = plan_final.task_stats.raw_bundle_files;
        summary.raw_bundle_bytes = plan_final.task_stats.raw_bundle_bytes;
        summary.large_tasks = plan_final.task_stats.large_tasks;
        summary.large_bytes = plan_final.task_stats.large_bytes;

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

        fn apply_local_deletions(
            entries: &[crate::enumeration::EnumeratedEntry],
            dest_root: &Path,
            planner: &MirrorPlanner,
            filter: &FileFilter,
            perform: bool,
            verbose: bool,
        ) -> Result<(usize, usize)> {
            let delete_plan =
                planner.plan_local_deletions_from_entries(entries, dest_root, filter)?;
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
                            eprintln!("Failed to delete file {}: {}", path.display(), err);
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
                            eprintln!("Failed to delete directory {}: {}", path.display(), err);
                        }
                    }
                } else {
                    deleted_dirs += 1;
                }
            }

            Ok((deleted_files, deleted_dirs))
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
