//! Unified local WorkerFactory leveraging the async transfer engine.
//!
//! Provides a WorkerFactory implementation that shares planning and
//! scheduling logic with network transfers while honoring local copy
//! options (dry-run, checksums, timestamp preservation) and streaming
//! TAR shards without buffering entire archives into memory.

use std::cmp::max;
use std::path::{Path, PathBuf};
use std::sync::{
    atomic::{AtomicUsize, Ordering::Relaxed},
    Arc,
};

use eyre::{bail, Context, Result};
use filetime::FileTime;
use tokio::task::JoinHandle;

use crate::buffer::BufferSizer;
#[cfg(all(unix, not(target_os = "macos")))]
use crate::copy::mmap_copy_file;
use crate::copy::{copy_file, file_needs_copy_with_checksum_type, resume_copy_file};
use crate::logger::{Logger, NoopLogger};
use crate::tar_stream::tar_stream_transfer_list;
use crate::tar_stream::TarConfig;
use crate::transfer_engine::{Sample, WorkerFactory, WorkerParams};
use crate::transfer_plan::TransferTask;
use crate::CopyConfig;
use rayon::prelude::*;
use rayon::ThreadPoolBuilder;

/// WorkerFactory implementation for local filesystem operations.
pub struct LocalWorkerFactory {
    pub src_root: PathBuf,
    pub dest_root: PathBuf,
    pub config: CopyConfig,
}

impl WorkerFactory for LocalWorkerFactory {
    fn spawn_worker(&self, params: WorkerParams) -> JoinHandle<Result<()>> {
        let src = self.src_root.clone();
        let dest = self.dest_root.clone();
        let config = self.config.clone();

        tokio::spawn(async move { local_worker_loop(src, dest, config, params).await })
    }
}

async fn local_worker_loop(
    src_root: PathBuf,
    dest_root: PathBuf,
    config: CopyConfig,
    params: WorkerParams,
) -> Result<()> {
    let WorkerParams {
        idx,
        chunk_bytes,
        progress,
        rx_shared,
        remaining,
        active,
        exit_tokens,
        stat_tx,
        max_retries,
    } = params;

    active.fetch_add(1, Relaxed);

    loop {
        if active.load(Relaxed) > 1 && try_consume_exit_token(&exit_tokens) {
            break;
        }

        let task_opt = {
            let mut rx = rx_shared.lock().await;
            rx.recv().await
        };
        let Some(task) = task_opt else { break };

        if progress {
            match &task {
                TransferTask::TarShard(list) => {
                    eprintln!("[w{idx}] tar {} files", list.len());
                }
                TransferTask::RawBundle(list) => {
                    eprintln!("[w{idx}] bundle {} files", list.len());
                }
                TransferTask::Large { path } => {
                    eprintln!("[w{idx}] large {}", path.display());
                }
            }
        }

        let started = std::time::Instant::now();

        // Execute task with retry logic
        let mut attempts = 0u8;
        #[allow(unused_assignments)]
        let mut last_error: Option<eyre::Report> = None;

        loop {
            let result = match &task {
                TransferTask::TarShard(files) => {
                    handle_tar_shard(&src_root, &dest_root, files, chunk_bytes, &config).await
                }
                TransferTask::RawBundle(files) => {
                    handle_copy_list(&src_root, &dest_root, files, &config).await
                }
                TransferTask::Large { path } => {
                    handle_large_file(&src_root, &dest_root, path, &config).await
                }
            };

            match result {
                Ok(()) => {
                    last_error = None;
                    break;
                }
                Err(err) => {
                    attempts += 1;
                    let is_retryable = is_retryable_error(&err);

                    if is_retryable && attempts <= max_retries {
                        if progress {
                            eprintln!("[w{idx}] retrying ({}/{}): {err}", attempts, max_retries);
                        }
                        // Brief delay before retry to avoid hammering on transient errors
                        tokio::time::sleep(std::time::Duration::from_millis(100 * attempts as u64)).await;
                        continue;
                    }

                    last_error = Some(err);
                    break;
                }
            }
        }

        let task_bytes = if config.dry_run {
            0
        } else {
            estimate_task_bytes(&src_root, &task).unwrap_or(0)
        };
        let elapsed = started.elapsed();
        let _ = stat_tx.send(Sample {
            bytes: task_bytes,
            ms: elapsed.as_millis(),
        });

        if let Some(err) = last_error {
            if attempts > 1 {
                eprintln!("[w{idx}] error after {} attempts: {err}", attempts);
            } else {
                eprintln!("[w{idx}] error: {err}");
            }
        }

        remaining.fetch_sub(1, Relaxed);
    }

    active.fetch_sub(1, Relaxed);
    Ok(())
}

/// Check if an error is retryable (transient I/O errors, not fatal ones).
fn is_retryable_error(err: &eyre::Report) -> bool {
    let msg = err.to_string().to_lowercase();
    // Retryable: temporary conditions that may resolve
    if msg.contains("resource temporarily unavailable")
        || msg.contains("interrupted")
        || msg.contains("would block")
        || msg.contains("connection reset")
        || msg.contains("broken pipe")
        || msg.contains("timed out")
    {
        return true;
    }
    // Not retryable: permanent conditions
    if msg.contains("permission denied")
        || msg.contains("no such file")
        || msg.contains("not a directory")
        || msg.contains("is a directory")
        || msg.contains("no space left")
        || msg.contains("disk quota")
        || msg.contains("read-only file system")
    {
        return false;
    }
    // Default: retry once for unknown errors
    true
}

async fn handle_tar_shard(
    src_root: &Path,
    dest_root: &Path,
    files: &[PathBuf],
    chunk_bytes: usize,
    config: &CopyConfig,
) -> Result<()> {
    if files.is_empty() {
        return Ok(());
    }

    if config.dry_run {
        for rel in files {
            if let Some(parent) = dest_root.join(rel).parent() {
                std::fs::create_dir_all(parent).ok();
            }
        }
        return Ok(());
    }

    let mut entries = Vec::with_capacity(files.len());
    for rel in files {
        let abs = src_root.join(rel);
        if abs.is_file() {
            entries.push((abs, rel.clone()));
        }
    }

    if entries.is_empty() {
        return Ok(());
    }

    let dest = dest_root.to_path_buf();
    let shard_chunk = max(chunk_bytes, 1 << 20);

    tokio::task::block_in_place(move || -> Result<()> {
        let cfg = TarConfig {
            chunk_size: shard_chunk,
            ..TarConfig::default()
        };
        tar_stream_transfer_list(&entries, &dest, &cfg, false)?;
        Ok(())
    })
    .context("tar shard task execution failed")?;

    Ok(())
}

async fn handle_copy_list(
    src_root: &Path,
    dest_root: &Path,
    rels: &[PathBuf],
    config: &CopyConfig,
) -> Result<()> {
    if rels.is_empty() {
        return Ok(());
    }

    let src = src_root.to_path_buf();
    let dest = dest_root.to_path_buf();
    let list = rels.to_vec();
    let cfg = config.clone();

    tokio::task::block_in_place(move || copy_paths_blocking(&src, &dest, &list, &cfg))
        .context("local copy task execution failed")?;
    Ok(())
}

async fn handle_large_file(
    src_root: &Path,
    dest_root: &Path,
    rel: &Path,
    config: &CopyConfig,
) -> Result<()> {
    let src = src_root.to_path_buf();
    let dest = dest_root.to_path_buf();
    let rel_clone = rel.to_path_buf();
    let cfg = config.clone();

    tokio::task::block_in_place(move || copy_large_blocking(&src, &dest, &rel_clone, &cfg))
        .context("large file task execution failed")?;
    Ok(())
}

pub(crate) fn copy_paths_blocking(
    src_root: &Path,
    dest_root: &Path,
    rels: &[PathBuf],
    config: &CopyConfig,
) -> Result<()> {
    if rels.is_empty() {
        return Ok(());
    }

    let workers = config.workers.max(1);
    if workers == 1 || rels.len() < 32 {
        let sizer = BufferSizer::default();
        let logger = NoopLogger;
        for rel in rels {
            copy_path_maybe(src_root, dest_root, rel.as_path(), config, &sizer, &logger)?;
        }
        return Ok(());
    }

    let sizer = BufferSizer::default();
    let logger = NoopLogger;

    let pool = ThreadPoolBuilder::new()
        .num_threads(workers)
        .build()
        .context("build parallel copy pool")?;

    pool.install(|| {
        rels.par_iter().try_for_each(|rel| {
            copy_path_maybe(src_root, dest_root, rel.as_path(), config, &sizer, &logger)
        })
    })?;

    Ok(())
}

pub(crate) fn copy_large_blocking(
    src_root: &Path,
    dest_root: &Path,
    rel: &Path,
    config: &CopyConfig,
) -> Result<()> {
    let dest = dest_root.join(rel);

    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)?;
    }

    if config.dry_run {
        return Ok(());
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let src = src_root.join(rel);
        let _ = mmap_copy_file(&src, &dest)?;
        if config.preserve_times {
            if let Ok(md) = std::fs::metadata(&src) {
                if let Ok(modified) = md.modified() {
                    let ft = FileTime::from_system_time(modified);
                    let _ = filetime::set_file_mtime(&dest, ft);
                }
            }
        }
        Ok(())
    }

    #[cfg(any(not(unix), target_os = "macos"))]
    {
        let rel_buf = rel.to_path_buf();
        copy_paths_blocking(src_root, dest_root, std::slice::from_ref(&rel_buf), config)
    }
}

fn copy_path_maybe(
    src_root: &Path,
    dest_root: &Path,
    rel: &Path,
    config: &CopyConfig,
    sizer: &BufferSizer,
    logger: &dyn Logger,
) -> Result<()> {
    if rel.is_absolute() {
        bail!("refusing absolute relative path: {}", rel.display());
    }
    for comp in rel.components() {
        if matches!(comp, std::path::Component::ParentDir) {
            bail!(
                "refusing path containing parent components: {}",
                rel.display()
            );
        }
    }

    let src = src_root.join(rel);
    let dst = dest_root.join(rel);

    if config.dry_run {
        if file_needs_copy_with_checksum_type(&src, &dst, config.checksum)? {
            if let Some(parent) = dst.parent() {
                std::fs::create_dir_all(parent).ok();
            }
        }
        return Ok(());
    }

    let mut did_copy = false;
    let mut clone_succeeded = false;

    if config.resume {
        // Resume mode: use block-level comparison to transfer only differences
        let outcome = resume_copy_file(&src, &dst, 0)?;
        did_copy = outcome.bytes_transferred > 0;
        logger.copy_done(&src, &dst, outcome.bytes_transferred);
    } else if file_needs_copy_with_checksum_type(&src, &dst, config.checksum)? {
        let outcome = copy_file(&src, &dst, sizer, false, logger)?;
        did_copy = true;
        clone_succeeded = outcome.clone_succeeded;
    }

    if config.preserve_times && did_copy && !clone_succeeded {
        if let Ok(meta) = std::fs::metadata(&src) {
            if let Ok(modified) = meta.modified() {
                let ft = FileTime::from_system_time(modified);
                let _ = filetime::set_file_mtime(&dst, ft);
            }
        }
    }

    Ok(())
}

fn estimate_task_bytes(src_root: &Path, task: &TransferTask) -> Result<u64> {
    match task {
        TransferTask::TarShard(files) | TransferTask::RawBundle(files) => {
            let mut total = 0u64;
            for file in files {
                if let Ok(meta) = std::fs::metadata(src_root.join(file)) {
                    total += meta.len();
                }
            }
            Ok(total)
        }
        TransferTask::Large { path } => {
            let meta = std::fs::metadata(src_root.join(path))?;
            Ok(meta.len())
        }
    }
}

fn try_consume_exit_token(tok: &Arc<AtomicUsize>) -> bool {
    loop {
        let cur = tok.load(Relaxed);
        if cur == 0 {
            return false;
        }
        if tok
            .compare_exchange_weak(cur, cur - 1, Relaxed, Relaxed)
            .is_ok()
        {
            return true;
        }
    }
}
