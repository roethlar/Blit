mod clone;
mod metadata;
mod mmap;
pub mod resume;

pub use mmap::mmap_copy_file;
pub use resume::{resume_copy_file, resume_copy_from, ResumeCopyOutcome};

use crate::buffer::BufferSizer;
use eyre::{eyre, Result};
use std::fs;
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
) -> Result<FileCopyOutcome> {
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

    result
}

/// A reader whose failures name the SOURCE (ssc-4): the buffered tails
/// below copy through `io::copy`, whose error could otherwise be either
/// side's; wrapping the source read makes a mid-copy source failure
/// report as `source: read error: …` like every other carrier.
struct SourceRead<R>(R);

impl<R: std::io::Read> std::io::Read for SourceRead<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.0
            .read(buf)
            .map_err(|e| std::io::Error::new(e.kind(), format!("source: read error: {e}")))
    }
}

/// ssc-4 (SOURCE_SIDE_CONTAINMENT D-C, A12): the local copy cascade on
/// the OPENED source handle. The caller opened `src` once, validated its
/// length against the manifest, and re-stats the same handle afterwards;
/// nothing in here re-opens `src_path` for bytes, so the copy can never
/// land a replacement inode's content under the validated header. The
/// body is bounded to `expected_len` on every buffered path (growth
/// after the pre-check cannot spill), and the platform fast paths take
/// the descriptor: Linux `copy_file_range`/`sendfile`, macOS
/// `fclonefileat`/`fcopyfile` (clone first, into an ABSENT `dst`,
/// R58-F11), Windows block clone with handles (there is no handle-based
/// `CopyFileEx`, so the streaming path is the buffered one). `src_path`
/// is used only for the volume-capability probe and log lines.
pub fn copy_opened(
    src: &File,
    src_path: &Path,
    dst: &Path,
    expected_len: u64,
    buffer_sizer: &BufferSizer,
    is_network: bool,
) -> Result<FileCopyOutcome> {
    use std::io::{Read, Seek, SeekFrom};
    let _ = src_path;
    let buffer_size = buffer_sizer.calculate_buffer_size(expected_len, is_network);
    if let Some(parent) = dst.parent() {
        fs::create_dir_all(parent)?;
    }
    // Every primitive below reads from the descriptor's current offset.
    (&*src).seek(SeekFrom::Start(0))?;

    let (total_bytes, clone_succeeded) = {
        #[cfg(windows)]
        {
            use std::os::windows::fs::OpenOptionsExt;
            let mut dst_file = std::fs::OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .custom_flags(FILE_FLAG_SEQUENTIAL_SCAN)
                .open(dst)?;
            let mut clone_success = false;
            if !is_network && crate::fs_capability::supports_block_clone_same_volume(src_path, dst)?
            {
                match windows::try_block_clone_with_handles(src, &dst_file, expected_len)? {
                    windows::BlockCloneOutcome::Cloned => {
                        clone_success = true;
                        log::info!("block clone {} ({} bytes)", dst.display(), expected_len);
                    }
                    windows::BlockCloneOutcome::Unsupported { code } => {
                        crate::fs_capability::mark_block_clone_unsupported(src_path, dst);
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
                (expected_len, true)
            } else {
                (&*src).seek(SeekFrom::Start(0))?;
                let copied = clone::sparse_copy_windows(
                    SourceRead(src.take(expected_len)),
                    &mut dst_file,
                    buffer_size,
                    expected_len,
                )?;
                (copied, false)
            }
        }
        #[cfg(target_os = "macos")]
        {
            // Clone first: both primitives need `dst` absent / are
            // whole-file; a post-copy re-stat by the caller catches a
            // size that drifted past the manifest.
            let cloned = clone::attempt_fclonefileat_macos(src, dst).unwrap_or(false) || {
                (&*src).seek(SeekFrom::Start(0))?;
                clone::attempt_fcopyfile_macos_fd(src, dst).unwrap_or(false)
            };
            if cloned {
                (expected_len, true)
            } else {
                (&*src).seek(SeekFrom::Start(0))?;
                let dst_for_stream = File::create(dst)?;
                let mut reader =
                    BufReader::with_capacity(buffer_size, SourceRead(src.take(expected_len)));
                let mut writer = BufWriter::with_capacity(buffer_size, dst_for_stream);
                let n = io::copy(&mut reader, &mut writer)?;
                writer.flush()?;
                (n, false)
            }
        }
        #[cfg(all(unix, not(target_os = "macos")))]
        {
            let dst_file = File::create(dst)?;
            let fast_linux = clone::attempt_copy_file_range_linux(src, &dst_file, expected_len)
                .unwrap_or(false)
                || {
                    (&*src).seek(SeekFrom::Start(0))?;
                    dst_file.set_len(0)?;
                    clone::attempt_sendfile_linux(src, &dst_file, expected_len).unwrap_or(false)
                };
            if fast_linux {
                (expected_len, true)
            } else {
                (&*src).seek(SeekFrom::Start(0))?;
                dst_file.set_len(0)?;
                if let Some(n) = clone::attempt_sparse_copy_unix(src, &dst_file, expected_len)? {
                    (n, false)
                } else {
                    (&*src).seek(SeekFrom::Start(0))?;
                    dst_file.set_len(0)?;
                    let mut reader =
                        BufReader::with_capacity(buffer_size, SourceRead(src.take(expected_len)));
                    let mut writer = BufWriter::with_capacity(buffer_size, dst_file);
                    let n = io::copy(&mut reader, &mut writer)?;
                    writer.flush()?;
                    (n, false)
                }
            }
        }
    };
    if !clone_succeeded {
        metadata::preserve_metadata_from_handle(src, dst)?;
    }
    Ok(FileCopyOutcome {
        bytes_copied: total_bytes,
        clone_succeeded,
    })
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

    /// ssc-4 (D-C, A12): `copy_opened` copies the inode it was handed —
    /// after the path is atomically replaced, the destination holds the
    /// opened file's bytes — and is bounded to `expected_len` on the
    /// buffered path (a file that grew past the manifest cannot spill).
    #[test]
    fn copy_opened_copies_the_opened_inode_and_bounds_to_expected_len() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("src.bin");
        let other = dir.path().join("other.bin");
        let dst = dir.path().join("dst.bin");
        let data: Vec<u8> = (0u8..=255).cycle().take(120_000).collect();
        std::fs::write(&src, &data).unwrap();
        std::fs::write(&other, vec![0x77u8; 120_000]).unwrap();
        let opened = File::open(&src).unwrap();
        // The path now names a different inode.
        std::fs::rename(&other, &src).unwrap();

        let outcome = copy_opened(
            &opened,
            &src,
            &dst,
            data.len() as u64,
            &BufferSizer::default(),
            false,
        )
        .unwrap();
        assert_eq!(outcome.bytes_copied, data.len() as u64);
        assert_eq!(
            std::fs::read(&dst).unwrap(),
            data,
            "the opened inode's bytes, not the replacement's"
        );

        // Bounded: ask for fewer bytes than the file holds on the
        // buffered path (a pre-existing dst defeats the clone primitives
        // on macOS; on Linux the descriptor primitives take the bound).
        let dst2 = dir.path().join("dst2.bin");
        std::fs::write(&dst2, b"stale").unwrap();
        let outcome =
            copy_opened(&opened, &src, &dst2, 1_000, &BufferSizer::default(), false).unwrap();
        assert!(outcome.bytes_copied <= data.len() as u64);
        let got = std::fs::read(&dst2).unwrap();
        assert!(
            got.len() == 1_000 || outcome.clone_succeeded,
            "buffered copies are bounded to expected_len (got {} bytes, clone={})",
            got.len(),
            outcome.clone_succeeded
        );
        assert_eq!(&got[..1_000.min(got.len())], &data[..1_000.min(got.len())]);
    }

    /// Whatever fast path applies on this platform, the copy must be
    /// byte-identical and report the right size.
    #[test]
    fn copy_file_produces_byte_identical_copy() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("src.bin");
        let dst = dir.path().join("dst.bin");
        let data: Vec<u8> = (0u8..=255).cycle().take(100_000).collect();
        std::fs::write(&src, &data).unwrap();

        let outcome = copy_file(&src, &dst, &BufferSizer::default(), false).unwrap();
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

        let outcome = copy_file(&src, &dst, &BufferSizer::default(), false).unwrap();
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
