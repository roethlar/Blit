use anyhow::{anyhow, Context, Result};
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
            Err(err) => Err(anyhow!("planner thread panicked: {:?}", err)),
        }
    }
}

impl TransferFacade {
    const PARALLEL_FILE_THRESHOLD: usize = 4096;
    const PARALLEL_CHECKSUM_THRESHOLD: usize = 1024;
    const PARALLEL_BYTE_THRESHOLD: u64 = 8 * 1024 * 1024 * 1024; // 8 GiB

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
            let mut entries = Vec::new();
            let mut copy_jobs = Vec::new();
            let mut total_bytes = 0u64;
            let mut enumerated_files = 0usize;

            let mut aggregator = TaskAggregator::new(plan_options);

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
                copy_jobs = copy_jobs
                    .into_iter()
                    .filter(|job| planner.should_copy_entry(job, &src_root, &dest_root))
                    .collect();
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
    small_target: u64,
    medium_paths: Vec<PathBuf>,
    medium_bytes: u64,
    medium_target: u64,
    medium_max: u64,
    chunk_bytes: usize,
    options: PlanOptions,
}

impl TaskAggregator {
    fn new(options: PlanOptions) -> Self {
        let small_target = if options.ludicrous {
            768 * 1024 * 1024
        } else {
            512 * 1024 * 1024
        };
        let medium_target = if options.ludicrous {
            384 * 1024 * 1024
        } else {
            128 * 1024 * 1024
        };
        let medium_max = (medium_target as f64 * 1.25) as u64;
        let chunk_bytes = if options.ludicrous {
            32 * 1024 * 1024
        } else {
            16 * 1024 * 1024
        };

        Self {
            small_paths: Vec::new(),
            small_bytes: 0,
            small_target,
            medium_paths: Vec::new(),
            medium_bytes: 0,
            medium_target,
            medium_max,
            chunk_bytes,
            options,
        }
    }

    fn push(&mut self, rel: PathBuf, size: u64, tx: &UnboundedSender<PlannerEvent>) -> Result<()> {
        const LARGE_THRESHOLD: u64 = 256 * 1024 * 1024;
        if size >= LARGE_THRESHOLD {
            self.chunk_bytes = 32 * 1024 * 1024;
            self.emit_task(tx, TransferTask::Large { path: rel })?;
            return Ok(());
        }

        if size < 1_048_576 {
            self.small_paths.push(rel);
            self.small_bytes += size;
            if self.small_bytes >= self.small_target && !self.small_paths.is_empty() {
                let paths = std::mem::take(&mut self.small_paths);
                self.small_bytes = 0;
                self.emit_task(tx, TransferTask::TarShard(paths))?;
            }
            return Ok(());
        }

        self.medium_paths.push(rel);
        self.medium_bytes += size;
        if (self.medium_bytes >= self.medium_target && !self.medium_paths.is_empty())
            || (self.medium_bytes > self.medium_max)
        {
            let bundle = std::mem::take(&mut self.medium_paths);
            self.medium_bytes = 0;
            self.emit_task(tx, TransferTask::RawBundle(bundle))?;
        }

        Ok(())
    }

    fn flush_remaining(&mut self, tx: &UnboundedSender<PlannerEvent>) -> Result<()> {
        if !self.small_paths.is_empty() {
            let paths = std::mem::take(&mut self.small_paths);
            self.small_bytes = 0;
            if self.options.force_tar {
                self.emit_task(tx, TransferTask::TarShard(paths))?;
            } else {
                self.emit_task(tx, TransferTask::RawBundle(paths))?;
            }
        }
        if !self.medium_paths.is_empty() {
            let bundle = std::mem::take(&mut self.medium_paths);
            self.medium_bytes = 0;
            self.emit_task(tx, TransferTask::RawBundle(bundle))?;
        }
        Ok(())
    }

    fn emit_task(&self, tx: &UnboundedSender<PlannerEvent>, task: TransferTask) -> Result<()> {
        tx.send(PlannerEvent::Task(task))
            .map_err(|_| anyhow!("planner consumer dropped"))
    }
}
