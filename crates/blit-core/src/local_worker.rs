//! Local filesystem copy helpers used by the orchestrator's fast-path.
//!
//! The main transfer pipeline now uses `FsTransferSink` (via `execute_sink_pipeline`).
//! These blocking helpers remain for the fast-path routing in `orchestrator::fast_path`
//! which handles tiny manifests and single huge files without the full pipeline overhead.

use std::path::{Path, PathBuf};

use eyre::{bail, Result};
use filetime::FileTime;

use crate::buffer::BufferSizer;
#[cfg(all(unix, not(target_os = "macos")))]
use crate::copy::mmap_copy_file;
use crate::copy::{copy_file, file_needs_copy_with_checksum_type, resume_copy_file};
use crate::logger::{Logger, NoopLogger};
use crate::CopyConfig;

pub(crate) fn copy_paths_blocking(
    src_root: &Path,
    dest_root: &Path,
    rels: &[PathBuf],
    config: &CopyConfig,
) -> Result<()> {
    if rels.is_empty() {
        return Ok(());
    }

    let sizer = BufferSizer::default();
    let logger = NoopLogger;
    for rel in rels {
        copy_path_maybe(src_root, dest_root, rel.as_path(), config, &sizer, &logger)?;
    }

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
