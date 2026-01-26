mod chunked;
mod clone;
mod metadata;
mod mmap;

pub use chunked::chunked_copy_file;
pub use mmap::mmap_copy_file;

use crate::buffer::BufferSizer;
use crate::logger::Logger;
use eyre::{eyre, Result};
use std::fs;
use std::path::Path;

#[cfg(windows)]
const FILE_FLAG_SEQUENTIAL_SCAN: u32 = 0x0800_0000;
#[cfg(windows)]
use crate::copy::windows;

/// Copy a single file with optimal buffer size
pub struct FileCopyOutcome {
    pub bytes_copied: u64,
    pub clone_succeeded: bool,
}

pub fn copy_file(
    src: &Path,
    dst: &Path,
    buffer_sizer: &BufferSizer,
    is_network: bool,
    logger: &dyn Logger,
) -> Result<FileCopyOutcome> {
    logger.start(src, dst);

    #[cfg(windows)]
    if !is_network {
        match windows::windows_copyfile(src, dst) {
            Ok(bytes) => {
                let clone_succeeded = windows::take_last_block_clone_success();
                if !clone_succeeded {
                    metadata::preserve_metadata(src, dst)?;
                } else {
                    log::debug!(
                        "block clone preserved metadata automatically for {}",
                        dst.display()
                    );
                }
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
            }
        }
    }

    let result: Result<FileCopyOutcome> = (|| {
        let metadata = fs::metadata(src)?;
        let file_size = metadata.len();

        let buffer_size = buffer_sizer.calculate_buffer_size(file_size, is_network);

        if let Some(parent) = dst.parent() {
            fs::create_dir_all(parent)?;
        }

        let parent = dst
            .parent()
            .ok_or_else(|| eyre!("destination has no parent: {}", dst.display()))?;
        fs::create_dir_all(parent)?;

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

        #[cfg(windows)]
        let mut dst_file = {
            std::fs::OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .custom_flags(FILE_FLAG_SEQUENTIAL_SCAN)
                .open(dst)?
        };
        #[cfg(not(windows))]
        let dst_file = File::create(dst)?;

        let (total_bytes, clone_succeeded) = {
            #[cfg(windows)]
            {
                let mut clone_success = false;
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
                let cloned = clone::attempt_clonefile_macos(src, dst).unwrap_or(false)
                    || clone::attempt_fcopyfile_macos(src, dst).unwrap_or(false);
                if cloned {
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
        if !clone_succeeded {
            metadata::preserve_metadata(src, dst)?;
        }

        Ok(FileCopyOutcome {
            bytes_copied: total_bytes,
            clone_succeeded,
        })
    })();

    match result {
        Ok(outcome) => {
            logger.copy_done(src, dst, outcome.bytes_copied);
            Ok(outcome)
        }
        Err(e) => {
            logger.error("copy", src, &e.to_string());
            Err(e)
        }
    }
}
