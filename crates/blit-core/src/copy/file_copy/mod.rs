mod clone;
mod metadata;
mod mmap;
pub mod resume;

pub use mmap::mmap_copy_file;
pub use resume::{resume_copy_file, ResumeCopyOutcome};

use crate::buffer::BufferSizer;
use crate::logger::Logger;
use eyre::{eyre, Result};
use std::fs;
#[cfg(not(windows))]
use std::fs::File;
#[cfg(unix)]
use std::io::{self, BufReader, BufWriter, Write};
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
        // R58-F11: on macOS, clonefile(2) requires the destination
        // to NOT exist (returns EEXIST otherwise). Pre-fix the
        // unconditional `File::create(dst)` above created an empty
        // file before the clone attempt, so clonefile ALWAYS failed
        // with EEXIST and APFS clones never succeeded. Defer the
        // create to the fallback streaming path; the clone branch
        // doesn't need a pre-existing destination handle.
        #[cfg(target_os = "macos")]
        let dst_file: Option<File> = None;
        #[cfg(all(unix, not(target_os = "macos")))]
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
                let _ = dst_file; // silence unused on this branch (always None)
                                  // R58-F11: try clone primitives FIRST (they need
                                  // dst to not exist), then fall back to streaming
                                  // copy if neither clone succeeded. The streaming
                                  // path creates the destination itself when it
                                  // opens its writer.
                let cloned = clone::attempt_clonefile_macos(src, dst).unwrap_or(false)
                    || clone::attempt_fcopyfile_macos(src, dst).unwrap_or(false);
                if cloned {
                    (file_size, true)
                } else {
                    let dst_for_stream = File::create(dst)?;
                    let mut reader = BufReader::with_capacity(buffer_size, src_file);
                    let mut writer = BufWriter::with_capacity(buffer_size, dst_for_stream);
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

#[cfg(test)]
mod fallback_tests {
    //! audit-6 item 7: copy_file's fast-path → fallback chain. A truly
    //! exhaustive "force every primitive down to the buffered streaming
    //! tail" test would need a production injection seam (the chain is
    //! inlined and cfg-gated per OS); see the note on the macOS test for
    //! why the buffered tail isn't deterministically reachable here
    //! without one. These cover end-to-end correctness plus a real
    //! fallback transition with no production change.
    use super::*;
    use crate::buffer::BufferSizer;
    use crate::logger::NoopLogger;

    /// Whatever fast path applies on this platform, the copy must be
    /// byte-identical and report the right size.
    #[test]
    fn copy_file_produces_byte_identical_copy() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("src.bin");
        let dst = dir.path().join("dst.bin");
        let data: Vec<u8> = (0u8..=255).cycle().take(100_000).collect();
        std::fs::write(&src, &data).unwrap();

        let outcome = copy_file(&src, &dst, &BufferSizer::default(), false, &NoopLogger).unwrap();
        assert_eq!(outcome.bytes_copied, data.len() as u64);
        assert_eq!(std::fs::read(&dst).unwrap(), data);
    }

    /// macOS: `clonefile(2)` returns `EEXIST` when the destination already
    /// exists, so a pre-existing dst deterministically forces the FIRST
    /// fast-path hop (clonefile) to fail. `fcopyfile` (opened with
    /// truncate, not COPYFILE_EXCL) then overwrites and the copy must
    /// still be byte-identical — exercising a genuine fallback transition
    /// in the chain with no production seam.
    ///
    /// Forcing all the way to the buffered streaming tail would require
    /// fcopyfile to ALSO fail, which has no benign deterministic trigger;
    /// that tail needs a production injection seam to test directly
    /// (flagged for a follow-up if full-chain coverage is wanted).
    #[cfg(target_os = "macos")]
    #[test]
    fn copy_file_falls_back_to_fcopyfile_when_clonefile_cannot_apply() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("src.bin");
        let dst = dir.path().join("dst.bin");
        let data: Vec<u8> = (0u8..=200).cycle().take(50_000).collect();
        std::fs::write(&src, &data).unwrap();
        // Pre-create dst so clonefile hits EEXIST and the chain advances.
        std::fs::write(&dst, b"stale pre-existing contents").unwrap();

        let outcome = copy_file(&src, &dst, &BufferSizer::default(), false, &NoopLogger).unwrap();
        assert_eq!(outcome.bytes_copied, data.len() as u64);
        assert_eq!(
            std::fs::read(&dst).unwrap(),
            data,
            "the fallback copy must overwrite the stale dst with src content"
        );
        // The load-bearing assertion: clonefile failed (EEXIST), so a
        // true clone_succeeded proves the NEXT fast path (fcopyfile)
        // handled the copy — not the buffered streaming tail (which sets
        // clone_succeeded = false). Without this the test would also pass
        // if fcopyfile were broken and the copy silently fell through to
        // buffered, leaving the intended hop unpinned.
        assert!(
            outcome.clone_succeeded,
            "after clonefile EEXIST, fcopyfile must handle the copy (clone_succeeded), \
             not the buffered tail"
        );
    }
}
