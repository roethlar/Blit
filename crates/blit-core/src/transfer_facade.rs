use eyre::{eyre, Context, Result};
use rayon::prelude::*;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::thread;

use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};

use crate::enumeration::{EntryKind, EnumeratedEntry, FileEnumerator};
use crate::fs_enum::{CopyJob, FileEntry, FileFilter};
use crate::mirror_planner::{MirrorPlanner, RemoteEntryState};
use crate::transfer_plan::{PlanOptions, TransferTask};

#[derive(Debug)]
pub struct LocalTransferPlan {
    pub entries: Vec<EnumeratedEntry>,
    pub copy_jobs: Vec<CopyJob>,
}

#[derive(Debug)]
pub struct PullTransferPlan {
    pub files_to_fetch: Vec<FileEntry>,
    pub skipped_rel_paths: Vec<PathBuf>,
}

pub struct TransferFacade;

#[derive(Debug)]
pub enum PlannerEvent {
    Task(TransferTask),
    Progress {
        enumerated_files: usize,
        total_bytes: u64,
    },
}

#[derive(Debug)]
pub struct LocalPlanFinal {
    pub entries: Vec<EnumeratedEntry>,
    pub copy_jobs: Vec<CopyJob>,
    pub chunk_bytes: usize,
    pub total_bytes: u64,
    pub task_stats: PlanTaskStats,
}

#[derive(Debug, Clone, Default)]
pub struct PlanTaskStats {
    pub tar_shard_tasks: usize,
    pub tar_shard_files: usize,
    pub tar_shard_bytes: u64,
    pub raw_bundle_tasks: usize,
    pub raw_bundle_files: usize,
    pub raw_bundle_bytes: u64,
    pub large_tasks: usize,
    pub large_bytes: u64,
}

pub struct LocalPlanStream {
    pub events: UnboundedReceiver<PlannerEvent>,
    join_handle: thread::JoinHandle<Result<LocalPlanFinal>>,
}

impl LocalPlanStream {
    pub fn into_parts(self) -> (UnboundedReceiver<PlannerEvent>, PlanJoinHandle) {
        (
            self.events,
            PlanJoinHandle {
                handle: self.join_handle,
            },
        )
    }
}

pub struct PlanJoinHandle {
    handle: thread::JoinHandle<Result<LocalPlanFinal>>,
}

impl PlanJoinHandle {
    pub fn wait(self) -> Result<LocalPlanFinal> {
        match self.handle.join() {
            Ok(res) => res,
            Err(err) => Err(eyre!("planner thread panicked: {:?}", err)),
        }
    }
}

impl TransferFacade {
    const PARALLEL_FILE_THRESHOLD: usize = 4096;
    const PARALLEL_CHECKSUM_THRESHOLD: usize = 1024;
    const PARALLEL_BYTE_THRESHOLD: u64 = 8 * 1024 * 1024 * 1024; // 8 GiB

    #[allow(clippy::too_many_arguments)]
    pub fn stream_local_plan(
        src_root: &Path,
        dest_root: &Path,
        filter: &mut FileFilter,
        preserve_links: bool,
        include_symlinks: bool,
        skip_unchanged: bool,
        planner: MirrorPlanner,
        plan_options: PlanOptions,
    ) -> Result<LocalPlanStream> {
        let src_root = src_root.to_path_buf();
        let dest_root = dest_root.to_path_buf();

        let mut enumerator = FileEnumerator::new(filter.clone_without_cache());
        if !preserve_links {
            enumerator = enumerator.follow_symlinks(true);
        }
        if include_symlinks {
            enumerator = enumerator.include_symlinks(true);
        }

        let (tx, rx) = unbounded_channel();

        let handle = thread::spawn(move || -> Result<LocalPlanFinal> {
            let block_clone_same_volume = {
                #[cfg(windows)]
                {
                    crate::fs_capability::supports_block_clone_same_volume(&src_root, &dest_root)
                        .unwrap_or_else(|err| {
                            log::debug!(
                                "planner: block clone probe failed for {} -> {}: {err}",
                                src_root.display(),
                                dest_root.display()
                            );
                            false
                        })
                }
                #[cfg(not(windows))]
                {
                    let _ = (&src_root, &dest_root);
                    false
                }
            };

            let mut entries = Vec::new();
            let mut copy_jobs = Vec::new();
            let mut total_bytes = 0u64;
            let mut enumerated_files = 0usize;

            let mut aggregator = TaskAggregator::new(plan_options, block_clone_same_volume);

            enumerator.enumerate_local_streaming(&src_root, |entry| {
                let kind = entry.kind.clone();
                entries.push(entry.clone());
                match kind {
                    EntryKind::Directory | EntryKind::Symlink { .. } => {}
                    EntryKind::File { size } => {
                        let rel = entry.relative_path.clone();
                        let abs = entry.absolute_path.clone();

                        let job = CopyJob {
                            entry: FileEntry {
                                path: abs,
                                size,
                                is_directory: false,
                            },
                        };

                        if skip_unchanged && !planner.should_copy_entry(&job, &src_root, &dest_root)
                        {
                            return Ok(());
                        }

                        enumerated_files += 1;
                        total_bytes += size;
                        copy_jobs.push(job);

                        aggregator.push(rel, size, &tx)?;

                        #[allow(clippy::manual_is_multiple_of)]
                        if enumerated_files % 256 == 0 {
                            let _ = tx.send(PlannerEvent::Progress {
                                enumerated_files,
                                total_bytes,
                            });
                        }
                    }
                }
                Ok(())
            })?;

            aggregator.flush_remaining(&tx)?;

            Ok(LocalPlanFinal {
                entries,
                copy_jobs,
                chunk_bytes: aggregator.chunk_bytes,
                total_bytes,
                task_stats: aggregator.stats.clone(),
            })
        });

        Ok(LocalPlanStream {
            events: rx,
            join_handle: handle,
        })
    }

    pub fn build_local_plan(
        src_root: &Path,
        dest_root: &Path,
        filter: &mut FileFilter,
        preserve_links: bool,
        include_symlinks: bool,
        skip_unchanged: bool,
        planner: &MirrorPlanner,
    ) -> Result<LocalTransferPlan> {
        let mut enumerator = FileEnumerator::new(filter.clone_without_cache());
        if !preserve_links {
            enumerator = enumerator.follow_symlinks(true);
        }
        if include_symlinks {
            enumerator = enumerator.include_symlinks(true);
        }
        let entries = enumerator
            .enumerate_local(src_root)
            .with_context(|| format!("Failed to enumerate source {}", src_root.display()))?;

        let mut copy_jobs: Vec<CopyJob> = entries
            .iter()
            .filter_map(|entry| match entry.kind {
                EntryKind::File { .. } => entry
                    .clone()
                    .into_file_entry()
                    .map(|file_entry| CopyJob { entry: file_entry }),
                _ => None,
            })
            .collect();

        if skip_unchanged {
            let src_root = src_root.to_path_buf();
            let dest_root = dest_root.to_path_buf();
            let total_bytes: u64 = copy_jobs.iter().map(|job| job.entry.size).sum();
            let job_count = copy_jobs.len();
            let use_parallel = job_count >= Self::PARALLEL_FILE_THRESHOLD
                || (planner.checksum_enabled() && job_count >= Self::PARALLEL_CHECKSUM_THRESHOLD)
                || total_bytes >= Self::PARALLEL_BYTE_THRESHOLD;

            if use_parallel {
                copy_jobs = copy_jobs
                    .into_par_iter()
                    .filter(|job| planner.should_copy_entry(job, &src_root, &dest_root))
                    .collect();
            } else {
                copy_jobs.retain(|job| planner.should_copy_entry(job, &src_root, &dest_root));
            }
        }

        Ok(LocalTransferPlan { entries, copy_jobs })
    }

    pub fn normalized_rel_key(path: &Path) -> String {
        let replaced = path.to_string_lossy().replace('\\', "/");
        #[cfg(windows)]
        {
            replaced.to_ascii_lowercase()
        }
        #[cfg(not(windows))]
        {
            replaced
        }
    }

    pub fn build_pull_plan(
        remote_entries: &[FileEntry],
        remote_root: &Path,
        dest_root: &Path,
        remote_states: &HashMap<String, RemoteEntryState>,
        planner: &MirrorPlanner,
    ) -> PullTransferPlan {
        let mut files_to_fetch = Vec::new();
        let mut skipped_rel_paths = Vec::new();

        for entry in remote_entries {
            if entry.is_directory {
                continue;
            }
            let rel = entry
                .path
                .strip_prefix(remote_root)
                .unwrap_or(&entry.path)
                .to_path_buf();
            let key = Self::normalized_rel_key(&rel);
            let state = remote_states
                .get(&key)
                .cloned()
                .unwrap_or(RemoteEntryState {
                    size: entry.size,
                    mtime: 0,
                    hash: None,
                });
            let dest_path = dest_root.join(&rel);
            if planner.should_fetch_remote_file(&dest_path, &state) {
                files_to_fetch.push(entry.clone());
            } else {
                skipped_rel_paths.push(rel);
            }
        }

        PullTransferPlan {
            files_to_fetch,
            skipped_rel_paths,
        }
    }
}

struct TaskAggregator {
    small_paths: Vec<PathBuf>,
    small_bytes: u64,
    small_count: u64,
    small_target: u64,
    small_count_target: usize,
    small_profile: bool,
    total_small_bytes: u64,
    medium_paths: Vec<PathBuf>,
    medium_bytes: u64,
    medium_target: u64,
    total_medium_bytes: u64,
    medium_max: u64,
    chunk_bytes: usize,
    options: PlanOptions,
    stats: PlanTaskStats,
    block_clone_same_volume: bool,
}

impl TaskAggregator {
    fn new(options: PlanOptions, block_clone_same_volume: bool) -> Self {
        let small_target = options.small_target.unwrap_or(8 * 1024 * 1024);
        let medium_target = options.medium_target.unwrap_or(128 * 1024 * 1024);
        let medium_max = (medium_target as f64 * 1.25) as u64;
        let chunk_bytes = 16 * 1024 * 1024;

        Self {
            small_paths: Vec::new(),
            small_bytes: 0,
            small_count: 0,
            small_target,
            small_count_target: options.small_count_target.unwrap_or(2048),
            small_profile: false,
            total_small_bytes: 0,
            medium_paths: Vec::new(),
            medium_bytes: 0,
            medium_target,
            total_medium_bytes: 0,
            medium_max,
            chunk_bytes,
            options,
            stats: PlanTaskStats::default(),
            block_clone_same_volume,
        }
    }

    fn promote_small_strategy(&mut self) {
        if self.total_small_bytes >= 768 * 1024 * 1024 && self.small_target < 64 * 1024 * 1024 {
            self.small_target = 64 * 1024 * 1024;
        } else if self.total_small_bytes >= 256 * 1024 * 1024
            && self.small_target < 32 * 1024 * 1024
        {
            self.small_target = 32 * 1024 * 1024;
        }
        if self.total_small_bytes >= 1_000_000_000 {
            self.chunk_bytes = self.chunk_bytes.max(32 * 1024 * 1024);
        }
    }

    fn promote_medium_strategy(&mut self) {
        const PROMOTE_MEDIUM_THRESHOLD: u64 = 512 * 1024 * 1024; // 512 MiB of medium files
        if self.total_medium_bytes >= PROMOTE_MEDIUM_THRESHOLD
            && self.medium_target < 384 * 1024 * 1024
        {
            self.medium_target = 384 * 1024 * 1024;
            self.medium_max = (self.medium_target as f64 * 1.25) as u64;
            self.chunk_bytes = self.chunk_bytes.max(32 * 1024 * 1024);
        }
    }

    fn update_small_profile(&mut self) {
        if self.small_profile {
            return;
        }
        if self.small_count >= 64 {
            let avg = if self.small_count == 0 {
                0
            } else {
                self.total_small_bytes / self.small_count
            };
            if avg <= 64 * 1024 {
                self.small_profile = true;
                self.small_count_target = 1024;
                self.chunk_bytes = self.chunk_bytes.max(self.small_target as usize);
            }
        }
    }

    fn push(&mut self, rel: PathBuf, size: u64, tx: &UnboundedSender<PlannerEvent>) -> Result<()> {
        if self.block_clone_same_volume {
            self.chunk_bytes = self.chunk_bytes.max(8 * 1024 * 1024);
            self.stats.raw_bundle_tasks = self.stats.raw_bundle_tasks.saturating_add(1);
            self.stats.raw_bundle_files = self.stats.raw_bundle_files.saturating_add(1);
            self.stats.raw_bundle_bytes = self.stats.raw_bundle_bytes.saturating_add(size);
            self.emit_task(tx, TransferTask::RawBundle(vec![rel]))?;
            return Ok(());
        }

        const LARGE_THRESHOLD: u64 = 256 * 1024 * 1024;
        if size >= LARGE_THRESHOLD {
            self.chunk_bytes = 32 * 1024 * 1024;
            self.stats.large_tasks = self.stats.large_tasks.saturating_add(1);
            self.stats.large_bytes = self.stats.large_bytes.saturating_add(size);
            self.emit_task(tx, TransferTask::Large { path: rel })?;
            return Ok(());
        }

        if size < 1_048_576 {
            self.small_paths.push(rel);
            self.small_bytes += size;
            self.small_count = self.small_count.saturating_add(1);
            self.total_small_bytes = self.total_small_bytes.saturating_add(size);
            self.promote_small_strategy();
            self.update_small_profile();
            self.chunk_bytes = self.chunk_bytes.max(self.small_target as usize);

            let reached_bytes = self.small_bytes >= self.small_target;
            let reached_count = self.small_paths.len() >= self.small_count_target;

            if (reached_bytes || reached_count) && !self.small_paths.is_empty() {
                let shard_bytes = self.small_bytes;
                let paths = std::mem::take(&mut self.small_paths);
                self.small_bytes = 0;
                self.stats.tar_shard_tasks = self.stats.tar_shard_tasks.saturating_add(1);
                self.stats.tar_shard_files = self.stats.tar_shard_files.saturating_add(paths.len());
                self.stats.tar_shard_bytes = self.stats.tar_shard_bytes.saturating_add(shard_bytes);
                self.emit_task(tx, TransferTask::TarShard(paths))?;
            }
            return Ok(());
        }

        self.medium_paths.push(rel);
        self.medium_bytes += size;
        self.total_medium_bytes = self.total_medium_bytes.saturating_add(size);
        self.promote_medium_strategy();
        if (self.medium_bytes >= self.medium_target && !self.medium_paths.is_empty())
            || (self.medium_bytes > self.medium_max)
        {
            let bundle_bytes = self.medium_bytes;
            let bundle = std::mem::take(&mut self.medium_paths);
            self.medium_bytes = 0;
            self.stats.raw_bundle_tasks = self.stats.raw_bundle_tasks.saturating_add(1);
            self.stats.raw_bundle_files = self.stats.raw_bundle_files.saturating_add(bundle.len());
            self.stats.raw_bundle_bytes = self.stats.raw_bundle_bytes.saturating_add(bundle_bytes);
            self.emit_task(tx, TransferTask::RawBundle(bundle))?;
        }

        Ok(())
    }

    fn flush_remaining(&mut self, tx: &UnboundedSender<PlannerEvent>) -> Result<()> {
        if !self.small_paths.is_empty() {
            let leftover_bytes = self.small_bytes;
            let paths = std::mem::take(&mut self.small_paths);
            self.small_bytes = 0;
            let should_tar = self.options.force_tar
                || self.small_profile
                || paths.len() >= self.small_count_target
                || leftover_bytes >= self.small_target;
            if should_tar {
                self.chunk_bytes = self.chunk_bytes.max(self.small_target as usize);
                self.stats.tar_shard_tasks = self.stats.tar_shard_tasks.saturating_add(1);
                self.stats.tar_shard_files = self.stats.tar_shard_files.saturating_add(paths.len());
                self.stats.tar_shard_bytes =
                    self.stats.tar_shard_bytes.saturating_add(leftover_bytes);
                self.emit_task(tx, TransferTask::TarShard(paths))?;
            } else {
                self.stats.raw_bundle_tasks = self.stats.raw_bundle_tasks.saturating_add(1);
                self.stats.raw_bundle_files =
                    self.stats.raw_bundle_files.saturating_add(paths.len());
                self.stats.raw_bundle_bytes =
                    self.stats.raw_bundle_bytes.saturating_add(leftover_bytes);
                self.emit_task(tx, TransferTask::RawBundle(paths))?;
            }
        }
        if !self.medium_paths.is_empty() {
            let bundle_bytes = self.medium_bytes;
            let bundle = std::mem::take(&mut self.medium_paths);
            self.medium_bytes = 0;
            self.stats.raw_bundle_tasks = self.stats.raw_bundle_tasks.saturating_add(1);
            self.stats.raw_bundle_files = self.stats.raw_bundle_files.saturating_add(bundle.len());
            self.stats.raw_bundle_bytes = self.stats.raw_bundle_bytes.saturating_add(bundle_bytes);
            self.emit_task(tx, TransferTask::RawBundle(bundle))?;
        }
        Ok(())
    }

    fn emit_task(&self, tx: &UnboundedSender<PlannerEvent>, task: TransferTask) -> Result<()> {
        tx.send(PlannerEvent::Task(task))
            .map_err(|_| eyre!("planner consumer dropped"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn drain_tasks(
        rx: &mut tokio::sync::mpsc::UnboundedReceiver<PlannerEvent>,
    ) -> Vec<TransferTask> {
        let mut out = Vec::new();
        while let Ok(evt) = rx.try_recv() {
            if let PlannerEvent::Task(task) = evt {
                out.push(task);
            }
        }
        out
    }

    #[test]
    fn tiny_files_emit_tar_shards() {
        let options = PlanOptions::default();
        let mut aggregator = TaskAggregator::new(options, false);
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

        for idx in 0..2048 {
            let rel = PathBuf::from(format!("tiny-{idx}"));
            aggregator.push(rel, 4 * 1024, &tx).unwrap();
        }
        aggregator.flush_remaining(&tx).unwrap();

        let tasks = drain_tasks(&mut rx);
        assert!(
            tasks
                .iter()
                .any(|task| matches!(task, TransferTask::TarShard(list) if !list.is_empty())),
            "expected at least one tar shard task, got: {:?}",
            tasks
        );
    }

    #[test]
    fn handful_of_small_files_stay_raw() {
        let options = PlanOptions::default();
        let mut aggregator = TaskAggregator::new(options, false);
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

        for idx in 0..4 {
            let rel = PathBuf::from(format!("small-{idx}"));
            aggregator.push(rel, 512 * 1024, &tx).unwrap();
        }
        aggregator.flush_remaining(&tx).unwrap();

        let tasks = drain_tasks(&mut rx);
        assert_eq!(tasks.len(), 1);
        match &tasks[0] {
            TransferTask::RawBundle(paths) => assert_eq!(paths.len(), 4),
            other => panic!("expected raw bundle, got {other:?}"),
        }
    }

    #[test]
    fn block_clone_paths_emit_single_raw_tasks() {
        let options = PlanOptions::default();
        let mut aggregator = TaskAggregator::new(options, true);
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

        for idx in 0..8 {
            let rel = PathBuf::from(format!("clone-{idx}"));
            aggregator.push(rel, 128 * 1024, &tx).unwrap();
        }
        aggregator.flush_remaining(&tx).unwrap();

        let tasks = drain_tasks(&mut rx);
        assert_eq!(tasks.len(), 8);
        for task in tasks {
            match task {
                TransferTask::RawBundle(paths) => assert_eq!(paths.len(), 1),
                other => panic!("expected raw bundle task, got {other:?}"),
            }
        }
    }
}
