mod chunked;
mod clone;
mod metadata;
mod mmap;

pub use chunked::chunked_copy_file;
pub use mmap::mmap_copy_file;

use crate::buffer::BufferSizer;
use crate::logger::Logger;
use eyre::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

#[cfg(windows)]
const FILE_FLAG_SEQUENTIAL_SCAN: u32 = 0x0800_0000;
#[cfg(windows)]
use crate::copy::windows;

/// Copy a single file with optimal buffer size
pub struct FileCopyOutcome {
    pub bytes_copied: u64,
    pub clone_succeeded: bool,
}


/// Suffix for temporary files during atomic copy operations.
const PARTIAL_FILE_SUFFIX: &str = ".blit.partial";

/// Guard that ensures temp files are cleaned up on failure.
/// Deletes the temp file on drop unless `commit()` is called.
struct TempFileGuard {
    temp_path: PathBuf,
    committed: bool,
}

impl TempFileGuard {
    fn new(temp_path: PathBuf) -> Self {
        Self {
            temp_path,
            committed: false,
        }
    }

    /// Atomically rename temp file to final destination.
    /// After commit succeeds, the guard won't delete the file on drop.
    fn commit(mut self, final_path: &Path) -> Result<()> {
        // On Windows, rename fails if destination exists, so remove first
        #[cfg(windows)]
        {
            let _ = fs::remove_file(final_path);
        }
        fs::rename(&self.temp_path, final_path)
            .with_context(|| format!("renaming {} to {}", self.temp_path.display(), final_path.display()))?;
        self.committed = true;
        Ok(())
    }

    fn path(&self) -> &Path {
        &self.temp_path
    }
}

impl Drop for TempFileGuard {
    fn drop(&mut self) {
        if !self.committed {
            let _ = fs::remove_file(&self.temp_path);
        }
    }
}

/// Generate temp path for atomic copy operations.
fn temp_path_for(dst: &Path) -> PathBuf {
    let mut temp = dst.as_os_str().to_owned();
    temp.push(PARTIAL_FILE_SUFFIX);
    PathBuf::from(temp)
}

pub fn copy_file(
    src: &Path,
    dst: &Path,
    buffer_sizer: &BufferSizer,
    is_network: bool,
    logger: &dyn Logger,
) -> Result<FileCopyOutcome> {
    logger.start(src, dst);

    // Ensure parent directory exists
    if let Some(parent) = dst.parent() {
        fs::create_dir_all(parent)?;
    }

    // Create temp path and guard for atomic write
    let temp_path = temp_path_for(dst);
    let guard = TempFileGuard::new(temp_path.clone());

    #[cfg(windows)]
    if !is_network {
        // Windows fast path: copy to temp file first
        match windows::windows_copyfile(src, guard.path()) {
            Ok(bytes) => {
                let clone_succeeded = windows::take_last_block_clone_success();
                if !clone_succeeded {
                    metadata::preserve_metadata(src, guard.path())?;
                } else {
                    log::debug!(
                        "block clone preserved metadata automatically for {}",
                        dst.display()
                    );
                }
                // Commit: rename temp to final destination
                guard.commit(dst)?;
                logger.copy_done(src, dst, bytes);
                return Ok(FileCopyOutcome {
                    bytes_copied: bytes,
                    clone_succeeded,
                });
            }
            Err(err) => {
                log::warn!(
                    "windows_copyfile fallback to streaming copy for {}: {}",
                    src.display(),
                    err
                );
                // Guard will clean up temp file on drop, continue to fallback
            }
        }
    }

    let result: Result<FileCopyOutcome> = (|| {
        let metadata = fs::metadata(src)?;
        let file_size = metadata.len();

        let buffer_size = buffer_sizer.calculate_buffer_size(file_size, is_network);

        #[cfg(windows)]
        use std::os::windows::fs::OpenOptionsExt;
        #[cfg(windows)]
        let src_file = {
            std::fs::OpenOptions::new()
                .read(true)
                .custom_flags(FILE_FLAG_SEQUENTIAL_SCAN)
                .open(src)?
        };
        #[cfg(not(windows))]
        let src_file = File::open(src)?;

        // Open temp file for writing
        #[cfg(windows)]
        let mut dst_file = {
            std::fs::OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .custom_flags(FILE_FLAG_SEQUENTIAL_SCAN)
                .open(guard.path())?
        };
        #[cfg(not(windows))]
        let dst_file = File::create(guard.path())?;

        let (total_bytes, clone_succeeded) = {
            #[cfg(windows)]
            {
                let mut clone_success = false;
                // Check block clone support using final destination path for volume check
                if crate::fs_capability::supports_block_clone_same_volume(src, dst)? {
                    match windows::try_block_clone_with_handles(&src_file, &dst_file, file_size)? {
                        windows::BlockCloneOutcome::Cloned => {
                            clone_success = true;
                            log::info!("block clone {} ({} bytes)", dst.display(), file_size);
                            if !log::log_enabled!(log::Level::Info) {
                                eprintln!("block clone {} ({} bytes)", dst.display(), file_size);
                            }
                            println!("block clone {} ({} bytes)", dst.display(), file_size);
                        }
                        windows::BlockCloneOutcome::Unsupported { code } => {
                            crate::fs_capability::mark_block_clone_unsupported(src, dst);
                            log::debug!(
                                "block clone unsupported for {} (error code {code}); falling back",
                                dst.display()
                            );
                        }
                        windows::BlockCloneOutcome::PrivilegeUnavailable => {
                            log::trace!(
                                "block clone privilege unavailable for {}; falling back",
                                dst.display()
                            );
                        }
                        windows::BlockCloneOutcome::Failed(err) => {
                            log::debug!(
                                "block clone streaming fallback for {} ({err})",
                                dst.display()
                            );
                        }
                    }
                }
                if clone_success {
                    (file_size, true)
                } else {
                    let copied = clone::sparse_copy_windows(
                        src_file,
                        &mut dst_file,
                        buffer_size,
                        file_size,
                    )?;
                    (copied, false)
                }
            }
            #[cfg(target_os = "macos")]
            {
                // macOS clonefile/fcopyfile use paths, pass temp path
                let cloned = clone::attempt_clonefile_macos(src, guard.path()).unwrap_or(false)
                    || clone::attempt_fcopyfile_macos(src, guard.path()).unwrap_or(false);
                if cloned {
                    // Close dst_file handle since clonefile created a new file
                    drop(dst_file);
                    (file_size, true)
                } else {
                    let mut reader = BufReader::with_capacity(buffer_size, src_file);
                    let mut writer = BufWriter::with_capacity(buffer_size, dst_file);
                    let n = io::copy(&mut reader, &mut writer)?;
                    writer.flush()?;
                    (n, false)
                }
            }
            #[cfg(all(unix, not(target_os = "macos")))]
            {
                let fast_linux =
                    clone::attempt_copy_file_range_linux(&src_file, &dst_file, file_size)
                        .unwrap_or(false)
                        || clone::attempt_sendfile_linux(&src_file, &dst_file, file_size)
                            .unwrap_or(false);
                if fast_linux {
                    (file_size, true)
                } else if let Some(n) =
                    clone::attempt_sparse_copy_unix(&src_file, &dst_file, file_size)?
                {
                    (n, false)
                } else {
                    let mut reader = BufReader::with_capacity(buffer_size, src_file);
                    let mut writer = BufWriter::with_capacity(buffer_size, dst_file);
                    let n = io::copy(&mut reader, &mut writer)?;
                    writer.flush()?;
                    (n, false)
                }
            }
        };
        
        // Preserve metadata on temp file before committing
        if !clone_succeeded {
            metadata::preserve_metadata(src, guard.path())?;
        }

        Ok(FileCopyOutcome {
            bytes_copied: total_bytes,
            clone_succeeded,
        })
    })();

    match result {
        Ok(outcome) => {
            // Commit: atomically rename temp to final destination
            guard.commit(dst)?;
            logger.copy_done(src, dst, outcome.bytes_copied);
            Ok(outcome)
        }
        Err(e) => {
            // Guard drops here and cleans up temp file
            logger.error("copy", src, &e.to_string());
            Err(e)
        }
    }
}
