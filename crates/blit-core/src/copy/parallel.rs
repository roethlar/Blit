use super::file_copy::copy_file;
use super::stats::CopyStats;
use crate::buffer::BufferSizer;
use crate::fs_enum::FileEntry;
use crate::logger::Logger;
use parking_lot::Mutex;
use rayon::prelude::*;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

pub fn parallel_copy_files(
    pairs: Vec<(FileEntry, PathBuf)>,
    buffer_sizer: Arc<BufferSizer>,
    is_network: bool,
    logger: &dyn Logger,
) -> CopyStats {
    struct ConcurrentStats {
        files: AtomicU64,
        bytes: AtomicU64,
        errors: Mutex<Vec<String>>,
    }

    let stats = Arc::new(ConcurrentStats {
        files: AtomicU64::new(0),
        bytes: AtomicU64::new(0),
        errors: Mutex::new(Vec::new()),
    });

    pairs.par_iter().for_each(|(entry, dst)| {
        match copy_file(&entry.path, dst, &buffer_sizer, is_network, logger) {
            Ok(bytes) => {
                stats.files.fetch_add(1, Ordering::Relaxed);
                stats.bytes.fetch_add(bytes, Ordering::Relaxed);
            }
            Err(e) => {
                let mut errs = stats.errors.lock();
                errs.push(format!("Failed to copy {:?}: {}", entry.path, e));
            }
        }
    });

    let errors = stats.errors.lock().clone();
    CopyStats {
        files_copied: stats.files.load(Ordering::Relaxed),
        bytes_copied: stats.bytes.load(Ordering::Relaxed),
        errors,
    }
}
