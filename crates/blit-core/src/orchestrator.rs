use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{anyhow, Context, Result};
use tokio::runtime::Builder;

use crate::fs_enum::FileFilter;
use crate::mirror_planner::MirrorPlanner;
use crate::transfer_engine::{
    create_task_stream, execute_streaming_plan, SchedulerOptions, TaskStreamSender,
};
use crate::transfer_facade::{PlannerEvent, TransferFacade};
use crate::transfer_plan::PlanOptions;
use crate::{local_worker::LocalWorkerFactory, CopyConfig};

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

        let mut filter = options.filter.clone_without_cache();
        let planner_for_stream = MirrorPlanner::new(options.checksum);
        let plan_options = PlanOptions {
            ludicrous: options.ludicrous_speed,
            force_tar: options.force_tar,
        };

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

        let mut copy_config = CopyConfig::default();
        copy_config.workers = options.workers.max(1);
        copy_config.preserve_times = options.preserve_times;
        copy_config.dry_run = options.dry_run;
        copy_config.checksum = if options.checksum {
            Some(crate::checksum::ChecksumType::Blake3)
        } else {
            None
        };

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
