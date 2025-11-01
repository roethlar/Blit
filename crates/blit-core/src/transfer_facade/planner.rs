use super::aggregator::TaskAggregator;
use super::types::{
    LocalPlanFinal, LocalPlanStream, LocalTransferPlan, PlannerEvent, PullTransferPlan,
    TransferFacade,
};
use crate::enumeration::{EntryKind, FileEnumerator};
use crate::fs_enum::{CopyJob, FileEntry, FileFilter};
use crate::mirror_planner::{MirrorPlanner, RemoteEntryState};
use crate::transfer_plan::PlanOptions;
use eyre::{Context, Result};
use rayon::prelude::*;
use std::collections::HashMap;
use std::path::Path;
use std::thread;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver};

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

    pub fn subscribe(
        stream: LocalPlanStream,
    ) -> (
        UnboundedReceiver<PlannerEvent>,
        super::types::PlanJoinHandle,
    ) {
        stream.into_parts()
    }
}
