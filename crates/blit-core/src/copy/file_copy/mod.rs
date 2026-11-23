mod clone;
mod metadata;
mod mmap;
pub mod resume;

pub use mmap::mmap_copy_file;
#[cfg(unix)]
pub use resume::{fresh_partial, fresh_partial_ready, make_partial_private};
pub use resume::{
    open_resume_partial, open_resume_partial_held, resume_copy_file, resume_copy_from,
    resume_copy_into, resume_copy_through, ResumeCopyOutcome,
};

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
        // cr-ecm33-6: without waiting, and only a regular file.
        #[cfg(not(windows))]
        let src_file = open_regular(src)?;

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
        let dst_file = create_regular(dst)?;

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
                    let dst_for_stream = create_regular(dst)?;
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
        if !clone_succeeded || !A_CLONE_CARRIES_METADATA {
            metadata::preserve_metadata(src, dst)?;
        }

        Ok(FileCopyOutcome {
            bytes_copied: total_bytes,
            clone_succeeded,
        })
    })();

    result
}

/// Whether a copy that reports a clone carried the file's time and mode
/// with it (local-copy-mode). A clone does — macOS' `clonefile` and
/// `fcopyfile`, a block clone on Windows, where the mode is no part of
/// it. Linux's `copy_file_range` and `sendfile` are reported the same way
/// and carry BYTES AND NOTHING ELSE: a file copied by one was left with
/// the mode it was created with — 644 under the usual mask, whatever its
/// source's. A private file (600) became readable by all, and a program
/// (750) stopped being one. Small files, written from a shard, and every
/// file to or from a daemon kept theirs; `cp` and `rsync` keep them.
const A_CLONE_CARRIES_METADATA: bool = cfg!(not(all(unix, not(target_os = "macos"))));

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

/// cr-ecm32-3: `path` opened to be read without waiting — a FIFO there,
/// with no one writing it, opens at once instead of waiting for a writer.
/// What was opened is the caller's to check; see [`open_regular`].
pub fn open_to_read(path: &Path) -> std::io::Result<File> {
    let mut open = std::fs::OpenOptions::new();
    open.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt as _;
        open.custom_flags(libc::O_NONBLOCK);
    }
    open.open(path)
}

/// cr-ecm32-3: `path` opened to be read — without waiting ([`open_to_read`])
/// — and kept only if what was opened is a regular file: a FIFO, or a
/// device, is no file to send or hash. The one opener for a source a
/// transfer reads: the scan's probe, the local copy's, the tar packer's,
/// the checksum's. Opened as any file is, a FIFO waited for a writer, and
/// the transfer with it; a device was read as a file.
pub fn open_regular(path: &Path) -> std::io::Result<File> {
    let file = open_to_read(path)?;
    if !file.metadata()?.is_file() {
        return Err(std::io::Error::other(format!(
            "{} is not a regular file",
            path.display()
        )));
    }
    Ok(file)
}

/// cr-ecm31-4: `dst` made, or opened, to be written from its start —
/// without waiting: a FIFO at the name that no one reads is refused at once
/// — and kept only if what was opened is a regular file: a FIFO with a
/// reader, or a device, is no file to copy into. Opened to be written as
/// any file is, a FIFO waited for a reader, and the transfer with it.
///
/// cr-ecm32-2: not cut on opening — what was opened is checked first, and
/// cut only then, through the handle. Cut on opening, a file the check then
/// refused, or could not make, was left at no length.
pub fn create_regular(dst: &Path) -> std::io::Result<File> {
    let mut open = std::fs::OpenOptions::new();
    open.write(true).create(true).truncate(false);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt as _;
        open.custom_flags(libc::O_NONBLOCK);
    }
    let file = match open.open(dst) {
        Ok(file) => file,
        #[cfg(unix)]
        Err(error) if error.raw_os_error() == Some(libc::ENXIO) => {
            return Err(std::io::Error::other(format!(
                "{} is a FIFO no one reads, not a regular file",
                dst.display()
            )))
        }
        Err(error) => return Err(error),
    };
    if !create_look_fault(dst)
        .and_then(|()| file.metadata())?
        .is_file()
    {
        return Err(std::io::Error::other(format!(
            "{} is not a regular file",
            dst.display()
        )));
    }
    file.set_len(0)?;
    Ok(file)
}

/// Test seam (cr-ecm32-2): the look at a file opened to be written refused
/// (`EIO`), by path. Production never sets it.
#[cfg(test)]
pub(crate) static CREATE_LOOK_FAULTS: std::sync::Mutex<Vec<std::path::PathBuf>> =
    std::sync::Mutex::new(Vec::new());

fn create_look_fault(_dst: &Path) -> std::io::Result<()> {
    #[cfg(test)]
    if CREATE_LOOK_FAULTS
        .lock()
        .is_ok_and(|faults| faults.iter().any(|at| at == _dst))
    {
        return Err(std::io::Error::other("a look that failed"));
    }
    Ok(())
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
                let dst_for_stream = create_regular(dst)?;
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
            let dst_file = create_regular(dst)?;
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
    if !clone_succeeded || !A_CLONE_CARRIES_METADATA {
        metadata::preserve_metadata_from_handle(src, dst)?;
    }
    Ok(FileCopyOutcome {
        bytes_copied: total_bytes,
        clone_succeeded,
    })
}

/// cr-ecm15-1: [`copy_opened`] INTO a destination the caller made and
/// holds. Nothing here looks the destination up by its name: every byte,
/// and the time and mode `copy_opened` gives a destination by its path,
/// goes through `made`. A run that keeps what exists (`--ignore-existing`)
/// made the name by a creation that fails if it is taken, dropped what it
/// held, and opened the NAME again to write — truncating whatever had been
/// put there in between.
///
/// On Linux the same cascade as `copy_opened`, on the descriptors; on
/// every other system a buffered copy through what is held (their fast
/// paths go by name, or need the destination absent).
pub fn copy_opened_into(
    src: &File,
    made: &File,
    expected_len: u64,
    buffer_sizer: &BufferSizer,
) -> Result<FileCopyOutcome> {
    use std::io::{Read, Seek, SeekFrom, Write as _};
    let buffer_size = buffer_sizer.calculate_buffer_size(expected_len, false);
    let buffered = |src: &File, made: &File| -> Result<u64> {
        (&*src).seek(SeekFrom::Start(0))?;
        made.set_len(0)?;
        (&*made).seek(SeekFrom::Start(0))?;
        let mut reader =
            std::io::BufReader::with_capacity(buffer_size, SourceRead(src.take(expected_len)));
        let mut writer = std::io::BufWriter::with_capacity(buffer_size, made);
        let n = std::io::copy(&mut reader, &mut writer)?;
        writer.flush()?;
        Ok(n)
    };
    (&*src).seek(SeekFrom::Start(0))?;
    #[cfg(all(unix, not(target_os = "macos")))]
    let (total_bytes, clone_succeeded) = {
        let fast_linux =
            clone::attempt_copy_file_range_linux(src, made, expected_len).unwrap_or(false) || {
                (&*src).seek(SeekFrom::Start(0))?;
                made.set_len(0)?;
                clone::attempt_sendfile_linux(src, made, expected_len).unwrap_or(false)
            };
        if fast_linux {
            (expected_len, true)
        } else {
            (&*src).seek(SeekFrom::Start(0))?;
            made.set_len(0)?;
            match clone::attempt_sparse_copy_unix(src, made, expected_len)? {
                Some(n) => (n, false),
                None => (buffered(src, made)?, false),
            }
        }
    };
    #[cfg(not(all(unix, not(target_os = "macos"))))]
    let (total_bytes, clone_succeeded) = (buffered(src, made)?, false);
    if !clone_succeeded || !A_CLONE_CARRIES_METADATA {
        metadata::preserve_metadata_into(src, made)?;
    }
    Ok(FileCopyOutcome {
        bytes_copied: total_bytes,
        clone_succeeded,
    })
}

#[cfg(all(test, unix))]
mod mode_tests {
    //! local-copy-mode: every way a file is copied on this machine gives
    //! the destination its source's mode — the fast paths too, which
    //! carry bytes and nothing else.
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    /// cr-ecm33-6: the exported path-based copies take only a regular file as
    /// their source and do not wait on a FIFO: `/dev/null` is refused by
    /// each, and a FIFO no one writes by `copy_file` at once. Opened as any
    /// file is, `/dev/null` was copied as an empty file, and the FIFO waited
    /// for a writer.
    #[test]
    fn the_path_based_copies_take_only_a_regular_source() {
        let dir = tempfile::tempdir().unwrap();
        let dst = dir.path().join("out.bin");
        let sizer = BufferSizer::default();
        let null = Path::new("/dev/null");
        assert!(copy_file(null, &dst, &sizer, false).is_err(), "copy_file");
        assert!(
            crate::copy::resume_copy_file(null, &dst, 1024).is_err(),
            "resume_copy_file"
        );
        assert!(
            crate::copy::mmap_copy_file(null, &dst).is_err(),
            "mmap_copy_file"
        );
        let at = dir.path().join("pipe.bin");
        let name = std::ffi::CString::new(at.as_os_str().as_encoded_bytes()).unwrap();
        // SAFETY: `name` is a NUL-terminated path that outlives the call.
        assert_eq!(unsafe { libc::mkfifo(name.as_ptr(), 0o644) }, 0);
        let (done, waited) = std::sync::mpsc::channel();
        {
            let (at, dst) = (at.clone(), dir.path().join("from-pipe.bin"));
            std::thread::spawn(move || {
                let sizer = BufferSizer::default();
                let _ = done.send(copy_file(&at, &dst, &sizer, false).is_err());
            });
        }
        let Ok(refused) = waited.recv_timeout(std::time::Duration::from_secs(20)) else {
            let _ = std::fs::OpenOptions::new().write(true).open(&at);
            panic!("copy_file waited on the FIFO");
        };
        assert!(refused, "a FIFO is no file to copy");
    }

    /// cr-ecm32-3: a source opened to be read does not wait on a FIFO and is
    /// a regular file, or it is refused: a FIFO no one writes, and a device.
    /// Opened as any file is, the FIFO waited for a writer; the device was
    /// read as a file.
    #[test]
    fn a_source_opened_to_be_read_is_a_regular_file_had_without_waiting() {
        let fifo = |at: &std::path::Path| {
            let name = std::ffi::CString::new(at.as_os_str().as_encoded_bytes()).unwrap();
            // SAFETY: `name` is a NUL-terminated path that outlives the call.
            assert_eq!(unsafe { libc::mkfifo(name.as_ptr(), 0o644) }, 0);
        };
        let dir = tempfile::tempdir().unwrap();
        let at = dir.path().join("pipe.bin");
        fifo(&at);
        let (done, waited) = std::sync::mpsc::channel();
        {
            let at = at.clone();
            std::thread::spawn(move || {
                let _ = done.send(open_regular(&at).is_err());
            });
        }
        let Ok(refused) = waited.recv_timeout(std::time::Duration::from_secs(20)) else {
            // A writer lets the waiting open go, so the test can end.
            let _ = std::fs::OpenOptions::new().write(true).open(&at);
            panic!("the open waited on the FIFO");
        };
        assert!(refused, "a FIFO is no file to read");
        assert!(
            open_regular(std::path::Path::new("/dev/null")).is_err(),
            "a device is no file to read"
        );
    }

    /// cr-ecm32-2: a file opened to be written is checked before it is cut:
    /// a check that fails leaves what stood there whole; one that passes
    /// cuts it, through the handle. Cut on opening, an eleven-byte file was
    /// left at no length.
    #[test]
    fn a_file_opened_to_be_written_is_checked_before_it_is_cut() {
        let dir = tempfile::tempdir().unwrap();
        let at = dir.path().join("kept.bin");
        std::fs::write(&at, b"eleven byte").unwrap();
        CREATE_LOOK_FAULTS.lock().unwrap().push(at.clone());
        let made = create_regular(&at);
        CREATE_LOOK_FAULTS
            .lock()
            .unwrap()
            .retain(|path| path != &at);
        assert!(made.is_err(), "its look refused");
        assert_eq!(std::fs::read(&at).unwrap(), b"eleven byte", "left whole");
        let made = create_regular(&at).unwrap();
        assert_eq!(made.metadata().unwrap().len(), 0, "cut once checked");
    }

    #[test]
    fn a_copied_file_has_its_sources_mode_however_it_was_copied() {
        let dir = tempfile::tempdir().unwrap();
        // More than one block, so that the fast paths are the ones taken.
        let data = vec![0x5Au8; 256 * 1024];
        for mode in [0o600, 0o640, 0o750] {
            let src = dir.path().join(format!("src-{mode:o}"));
            std::fs::write(&src, &data).unwrap();
            std::fs::set_permissions(&src, std::fs::Permissions::from_mode(mode)).unwrap();
            let mode_of =
                |path: &Path| std::fs::metadata(path).unwrap().permissions().mode() & 0o7777;
            let sizer = BufferSizer::default();

            let by_path = dir.path().join(format!("by-path-{mode:o}"));
            copy_file(&src, &by_path, &sizer, false).unwrap();
            assert_eq!(mode_of(&by_path), mode, "copy_file, {mode:o}");

            let opened = File::open(&src).unwrap();
            let by_handle = dir.path().join(format!("by-handle-{mode:o}"));
            copy_opened(&opened, &src, &by_handle, data.len() as u64, &sizer, false).unwrap();
            assert_eq!(mode_of(&by_handle), mode, "copy_opened, {mode:o}");
            assert_eq!(std::fs::read(&by_handle).unwrap(), data);

            let into = dir.path().join(format!("into-{mode:o}"));
            let made = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&into)
                .unwrap();
            copy_opened_into(&opened, &made, data.len() as u64, &sizer).unwrap();
            assert_eq!(mode_of(&into), mode, "copy_opened_into, {mode:o}");
            assert_eq!(std::fs::read(&into).unwrap(), data);
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
