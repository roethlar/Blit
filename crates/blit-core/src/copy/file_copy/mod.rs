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
///
/// cr-ecm33-5: and what it MADE — where nothing stood — is its own: made, a
/// file whose check then fails is removed, not left behind; one that stood
/// there is left as it was.
///
/// See [`create_regular_within`]: this is it with no destination root.
pub fn create_regular(dst: &Path) -> std::io::Result<File> {
    create_regular_within(dst, None)
}

/// [`create_regular`] for a destination whose root is `within` (canonical).
///
/// cr-ecm35-1 (Unix): a file is made only by a creation that fails if the
/// name is taken — at the name itself, never through a link — and a taken
/// name is opened without making anything: a file gone between the two
/// opens is not made by the second, unrecorded. A link at the name that
/// leads to nothing has the file it names made where it leads, found by
/// the same resolution a resume's partial is (the chain followed as the
/// system follows it, its end within `within`, the directory held): a link
/// out of the destination makes nothing outside it. A refused file it made
/// is removed only while that place still holds it, its handle held.
#[cfg(unix)]
pub fn create_regular_within(dst: &Path, within: Option<&Path>) -> std::io::Result<File> {
    let (file, made) = acquire_to_write(dst, within)?;
    // Which file was made, taken before anything else is asked of it: the
    // look below may be what fails.
    let mine = made
        .as_ref()
        .and_then(|_| file.metadata().ok())
        .map(|found| identity_of(&found));
    let checked = create_look_fault(dst)
        .and_then(|()| file.metadata())
        .and_then(|found| {
            if found.is_file() {
                Ok(())
            } else {
                Err(std::io::Error::other(format!(
                    "{} is not a regular file",
                    dst.display()
                )))
            }
        })
        .and_then(|()| file.set_len(0));
    if let Err(error) = checked {
        if let Some(at) = made {
            let removed = at.remove_if_mine(mine);
            drop(file);
            if let Err(unremoved) = removed {
                return Err(std::io::Error::other(
                    crate::remote::transfer::sink::LeftInPlace(format!(
                        "{}: {error}; the empty file made at {} could not be removed: {unremoved}",
                        crate::remote::transfer::sink::INCOMPLETE_LEFT_IN_PLACE,
                        dst.display()
                    )),
                ));
            }
        }
        return Err(error);
    }
    Ok(file)
}

/// Off Unix: as before — made where nothing stands, else opened, a link
/// written through and what it names made — and a refused file made is
/// removed by name.
#[cfg(not(unix))]
pub fn create_regular_within(dst: &Path, _within: Option<&Path>) -> std::io::Result<File> {
    let options = |make: bool| {
        let mut open = std::fs::OpenOptions::new();
        open.write(true);
        if make {
            open.create_new(true);
        } else {
            open.create(true).truncate(false);
        }
        open
    };
    let (file, made) = match options(true).open(dst) {
        Ok(file) => (file, true),
        Err(taken) if taken.kind() == std::io::ErrorKind::AlreadyExists => {
            (options(false).open(dst)?, false)
        }
        Err(error) => return Err(error),
    };
    let checked = create_look_fault(dst)
        .and_then(|()| file.metadata())
        .and_then(|found| {
            if found.is_file() {
                Ok(())
            } else {
                Err(std::io::Error::other(format!(
                    "{} is not a regular file",
                    dst.display()
                )))
            }
        })
        .and_then(|()| file.set_len(0));
    if let Err(error) = checked {
        if made {
            drop(file);
            if let Err(unremoved) = std::fs::remove_file(dst) {
                return Err(std::io::Error::other(
                    crate::remote::transfer::sink::LeftInPlace(format!(
                        "{}: {error}; the empty file made at {} could not be removed: {unremoved}",
                        crate::remote::transfer::sink::INCOMPLETE_LEFT_IN_PLACE,
                        dst.display()
                    )),
                ));
            }
        }
        return Err(error);
    }
    Ok(file)
}

/// cr-ecm35-1: where a file the opener made stands — at the name itself, or
/// where a link at the name led, in the directory held.
#[cfg(unix)]
enum MadeAt {
    Name(std::path::PathBuf),
    Held(crate::copy::Resolved),
}

#[cfg(unix)]
impl MadeAt {
    fn remove_if_mine(&self, mine: Option<(u64, u64)>) -> std::io::Result<()> {
        match self {
            MadeAt::Name(dst) => remove_if_mine(dst, mine),
            MadeAt::Held(resolved) => resolved.remove_if(mine),
        }
    }
}

/// The file at `dst` to be written, and where it was made if it was. See
/// [`create_regular_within`].
#[cfg(unix)]
fn acquire_to_write(dst: &Path, within: Option<&Path>) -> std::io::Result<(File, Option<MadeAt>)> {
    let refused = |error: std::io::Error| {
        if error.raw_os_error() == Some(libc::ENXIO) {
            std::io::Error::other(format!(
                "{} is a FIFO no one reads, not a regular file",
                dst.display()
            ))
        } else {
            error
        }
    };
    // A name that keeps changing between the opens is given up on.
    for _ in 0..4 {
        match open_to_write(dst, within, true) {
            Ok(file) => return Ok((file, Some(MadeAt::Name(dst.to_path_buf())))),
            Err(taken) if taken.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(refused(error)),
        }
        between_the_two_opens(dst);
        match open_to_write(dst, within, false) {
            Ok(file) => return Ok((file, None)),
            Err(gone) if gone.kind() == std::io::ErrorKind::NotFound => {
                // Taken a moment ago, leading to nothing now: a link to
                // nothing, whose file is made where it leads — or a file gone
                // between the opens, and the name is tried again.
                let a_link = std::fs::symlink_metadata(dst)
                    .is_ok_and(|there| there.file_type().is_symlink());
                if !a_link {
                    continue;
                }
                let resolved = crate::copy::resolve_destination(dst, within)
                    .map_err(|error| std::io::Error::other(format!("{error:#}")))?;
                match resolved
                    .open(libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL | libc::O_NONBLOCK)
                {
                    Ok(file) => return Ok((file, Some(MadeAt::Held(resolved)))),
                    Err(taken) if taken.kind() == std::io::ErrorKind::AlreadyExists => continue,
                    Err(error) => return Err(refused(error)),
                }
            }
            Err(error) => return Err(refused(error)),
        }
    }
    Err(std::io::Error::other(format!(
        "{} kept changing while it was opened",
        dst.display()
    )))
}

/// One of the opener's two opens of `dst` to write it, without waiting:
/// `make`, by a creation that fails if the name is taken; else what has the
/// name. cr-ecm36-1: with a root, on Linux, made beneath it — the place
/// found from the root down, and the kernel refusing any way out of it
/// (`openat2`, `RESOLVE_BENEATH`): a directory on the way replaced by a
/// link out since the place was judged cannot lead the open out. Without
/// `openat2` (an older kernel), and elsewhere, by the name, as before.
#[cfg(unix)]
pub fn open_to_write(dst: &Path, within: Option<&Path>, make: bool) -> std::io::Result<File> {
    use std::os::unix::fs::OpenOptionsExt as _;
    #[cfg(target_os = "linux")]
    if let Some(root) = within {
        if let Some(opened) = open_beneath(dst, root, make) {
            return opened;
        }
    }
    #[cfg(not(target_os = "linux"))]
    let _ = within;
    let mut open = std::fs::OpenOptions::new();
    open.write(true).custom_flags(libc::O_NONBLOCK);
    if make {
        open.create_new(true);
    }
    open.open(dst)
}

/// [`open_to_write`] beneath `root` (canonical), or `None` where it cannot
/// be: no `openat2` here, or a root that is the destination file itself.
#[cfg(target_os = "linux")]
fn open_beneath(dst: &Path, root: &Path, make: bool) -> Option<std::io::Result<File>> {
    use std::os::unix::io::{FromRawFd as _, OwnedFd};
    let Some(name) = dst.file_name() else {
        return Some(Err(std::io::Error::other(format!(
            "{} names no file",
            dst.display()
        ))));
    };
    let parent = match dst.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent,
        _ => Path::new("."),
    };
    // Written beneath the root already, by its own name: as it is — the
    // kernel refuses any `..` or link in it that leads out. Else where its
    // directory lies beneath the root, found once for the directory.
    let lexical = dst
        .strip_prefix(root)
        .ok()
        .filter(|beneath| !beneath.as_os_str().is_empty())
        .map(|beneath| {
            use std::os::unix::ffi::OsStrExt as _;
            let mut bytes = Vec::with_capacity(beneath.as_os_str().len() + 1);
            bytes.extend_from_slice(beneath.as_os_str().as_bytes());
            Ok(Ok(bytes))
        });
    let beneath = match lexical.unwrap_or_else(|| place_beneath(parent, root, name)) {
        Ok(Ok(beneath)) => beneath,
        // A destination that is one file: its root is the file itself.
        Ok(Err(directory)) if directory.join(name) == root => return None,
        Ok(Err(_)) => {
            return Some(Err(std::io::Error::other(format!(
                "{} leads out of the destination",
                dst.display()
            ))))
        }
        Err(error) => return Some(Err(error)),
    };
    let Ok(beneath) = std::ffi::CString::new(beneath) else {
        return Some(Err(std::io::Error::from(std::io::ErrorKind::InvalidInput)));
    };
    let held = held_root(root)?;
    let mut flags = libc::O_WRONLY | libc::O_NONBLOCK | libc::O_CLOEXEC;
    if make {
        flags |= libc::O_CREAT | libc::O_EXCL;
    }
    // SAFETY: `open_how` is plain integers; zero is each field's empty
    // value, and the three the call reads are set below.
    let mut how: libc::open_how = unsafe { std::mem::zeroed() };
    how.flags = flags as u64;
    how.mode = if make { 0o666 } else { 0 };
    how.resolve = libc::RESOLVE_BENEATH | libc::RESOLVE_NO_MAGICLINKS;
    // SAFETY: the held root is open for the call; `beneath` is a
    // NUL-terminated path and `how` a properly sized `open_how`, both
    // outliving it.
    let opened = unsafe {
        libc::syscall(
            libc::SYS_openat2,
            held,
            beneath.as_ptr(),
            &how as *const libc::open_how,
            std::mem::size_of::<libc::open_how>(),
        )
    };
    if opened < 0 {
        let error = std::io::Error::last_os_error();
        return match error.raw_os_error() {
            // No `openat2` here (a kernel before 5.6, or a filter that
            // refuses it): by the name, as before.
            Some(libc::ENOSYS) | Some(libc::EPERM) => None,
            // cr-ecm37-4: a way out, as the kernel sees every absolute link —
            // one that leads back into the destination among them. The place
            // is resolved against the destination instead — the chain of
            // links followed, its directory held — and opened there only
            // when it lies within; else it leads out. Refused outright, an
            // absolute link to a file within the destination failed a copy
            // it had always taken.
            Some(libc::EXDEV) => Some(
                crate::copy::resolve_destination(dst, Some(root))
                    .map_err(|error| std::io::Error::other(format!("{error:#}")))
                    .and_then(|resolved| {
                        let mode = if make { 0o666 } else { 0 };
                        resolved.open_as(flags & !libc::O_CLOEXEC, mode)
                    }),
            ),
            _ => Some(Err(error)),
        };
    }
    // SAFETY: `opened` is an open descriptor nothing else owns.
    Some(Ok(File::from(unsafe {
        OwnedFd::from_raw_fd(opened as i32)
    })))
}

#[cfg(target_os = "linux")]
thread_local! {
    /// Where the last directory this thread wrote into lies beneath the last
    /// root — or its canonical form, where it does not — and the root held:
    /// a copy writes many files of one directory in a row. A place gone
    /// stale only names, beneath the root, where the directory really is —
    /// the open there is still made beneath the root.
    #[allow(clippy::type_complexity)]
    static LAST_DIRECTORY: std::cell::RefCell<
        Vec<(std::path::PathBuf, std::path::PathBuf, Result<Vec<u8>, std::path::PathBuf>)>,
    > = const { std::cell::RefCell::new(Vec::new()) };
    static HELD_ROOT: std::cell::RefCell<Option<(std::path::PathBuf, std::os::unix::io::OwnedFd)>> =
        const { std::cell::RefCell::new(None) };
}

/// `name` in `parent`, as the bytes of its path beneath `root` (canonical);
/// or, where `parent` does not lie beneath it, `parent`'s canonical form.
/// The place of `parent` is remembered for the next file of it.
#[cfg(target_os = "linux")]
fn place_beneath(
    parent: &Path,
    root: &Path,
    name: &std::ffi::OsStr,
) -> std::io::Result<Result<Vec<u8>, std::path::PathBuf>> {
    use std::os::unix::ffi::OsStrExt as _;
    // The few directories a thread's writes go between, the latest first.
    const KEPT: usize = 16;
    LAST_DIRECTORY.with(|last| {
        let mut last = last.borrow_mut();
        let found = last
            .iter()
            .position(|(of, under, _)| of == parent && under == root);
        match found {
            Some(0) => {}
            Some(at) => {
                let entry = last.remove(at);
                last.insert(0, entry);
            }
            None => {
                let canonical = std::fs::canonicalize(parent)?;
                let place = match canonical.strip_prefix(root) {
                    Ok(beneath) => Ok(beneath.as_os_str().as_bytes().to_vec()),
                    Err(_) => Err(canonical),
                };
                last.insert(0, (parent.to_path_buf(), root.to_path_buf(), place));
                last.truncate(KEPT);
            }
        }
        let Some((_, _, place)) = last.first() else {
            unreachable!("the place was just remembered");
        };
        Ok(match place {
            Ok(directory) => {
                let mut bytes = Vec::with_capacity(directory.len() + name.len() + 2);
                if !directory.is_empty() {
                    bytes.extend_from_slice(directory);
                    bytes.push(b'/');
                }
                bytes.extend_from_slice(name.as_bytes());
                Ok(bytes)
            }
            Err(canonical) => Err(canonical.clone()),
        })
    })
}

/// The destination's root held (`O_PATH`) — opened once per thread and
/// root — as the descriptor an open beneath it starts from; `None` when it
/// cannot be held (a root that is one file).
#[cfg(target_os = "linux")]
fn held_root(root: &Path) -> Option<std::os::unix::io::RawFd> {
    use std::os::unix::ffi::OsStrExt as _;
    use std::os::unix::io::{AsRawFd as _, FromRawFd as _, OwnedFd};
    HELD_ROOT.with(|held| {
        let mut held = held.borrow_mut();
        if let Some((of, fd)) = held.as_ref() {
            if of == root {
                return Some(fd.as_raw_fd());
            }
        }
        let name = std::ffi::CString::new(root.as_os_str().as_bytes()).ok()?;
        // SAFETY: `name` is a NUL-terminated path that outlives the call.
        let fd = unsafe {
            libc::open(
                name.as_ptr(),
                libc::O_PATH | libc::O_DIRECTORY | libc::O_CLOEXEC,
            )
        };
        if fd < 0 {
            return None;
        }
        // SAFETY: `fd` is an open descriptor nothing else owns.
        let fd = unsafe { OwnedFd::from_raw_fd(fd) };
        let raw = fd.as_raw_fd();
        *held = Some((root.to_path_buf(), fd));
        Some(raw)
    })
}

/// Test seam (cr-ecm35-1): runs between the creation that found the name
/// taken and the open of what has it. Production never sets it.
#[cfg(all(unix, test))]
pub(crate) static BETWEEN_THE_TWO_OPENS: std::sync::Mutex<Vec<(std::path::PathBuf, fn(&Path))>> =
    std::sync::Mutex::new(Vec::new());

#[cfg(unix)]
fn between_the_two_opens(_dst: &Path) {
    #[cfg(test)]
    {
        let hooks: Vec<fn(&Path)> = BETWEEN_THE_TWO_OPENS
            .lock()
            .map(|hooks| {
                hooks
                    .iter()
                    .filter(|(at, _)| at == _dst)
                    .map(|(_, hook)| *hook)
                    .collect()
            })
            .unwrap_or_default();
        for hook in hooks {
            hook(_dst);
        }
    }
}

#[cfg(unix)]
fn identity_of(found: &std::fs::Metadata) -> (u64, u64) {
    use std::os::unix::fs::MetadataExt as _;
    (found.dev(), found.ino())
}

/// cr-ecm34-1: remove the file made at `dst` only while the name holds it —
/// `mine`, the file made. One put there since is somebody's, and the name
/// no longer holds this one: nothing of it is there to remove. Made and not
/// known which file it is, it is not removed: what is there cannot be told
/// to be its own.
#[cfg(unix)]
fn remove_if_mine(dst: &Path, mine: Option<(u64, u64)>) -> std::io::Result<()> {
    let Some(mine) = mine else {
        return Err(std::io::Error::other(
            "which file was made could not be told, so what is at the name is not removed",
        ));
    };
    before_a_made_file_is_removed(dst);
    let there = match std::fs::symlink_metadata(dst) {
        Ok(found) => identity_of(&found),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error),
    };
    if there != mine {
        return Ok(());
    }
    match std::fs::remove_file(dst) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        other => other,
    }
}

/// Test seam (cr-ecm34-1): runs after a made file was refused and before
/// the name is looked at to remove it. Production never sets it.
#[cfg(all(unix, test))]
pub(crate) static BEFORE_A_MADE_FILE_IS_REMOVED: std::sync::Mutex<
    Vec<(std::path::PathBuf, fn(&Path))>,
> = std::sync::Mutex::new(Vec::new());

#[cfg(unix)]
fn before_a_made_file_is_removed(_dst: &Path) {
    #[cfg(test)]
    {
        let hooks: Vec<fn(&Path)> = BEFORE_A_MADE_FILE_IS_REMOVED
            .lock()
            .map(|hooks| {
                hooks
                    .iter()
                    .filter(|(at, _)| at == _dst)
                    .map(|(_, hook)| *hook)
                    .collect()
            })
            .unwrap_or_default();
        for hook in hooks {
            hook(_dst);
        }
    }
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

    /// cr-ecm33-5: a file made where nothing stood, whose check then fails,
    /// is removed: nothing is left at the name. Left, it stood there an
    /// empty file, reported nowhere.
    #[test]
    fn a_file_made_and_then_refused_is_not_left() {
        let dir = tempfile::tempdir().unwrap();
        let at = dir.path().join("absent.bin");
        CREATE_LOOK_FAULTS.lock().unwrap().push(at.clone());
        let made = create_regular(&at);
        CREATE_LOOK_FAULTS
            .lock()
            .unwrap()
            .retain(|path| path != &at);
        assert!(made.is_err(), "its look refused");
        assert!(!at.exists(), "nothing left at the name");
    }

    /// cr-ecm34-1: through a dangling link at the name, what is made is the
    /// file the link leads to — known to be made, and removed when its look
    /// refuses it; the link stays. Opened by the link's name, the file was
    /// made through it unrecorded, and left there, empty, reported nowhere.
    #[test]
    fn a_file_made_through_a_dangling_link_and_refused_is_not_left() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("target.bin");
        let link = dir.path().join("link.bin");
        std::os::unix::fs::symlink(&target, &link).unwrap();
        CREATE_LOOK_FAULTS
            .lock()
            .unwrap()
            .extend([link.clone(), target.clone()]);
        let made = create_regular(&link);
        CREATE_LOOK_FAULTS
            .lock()
            .unwrap()
            .retain(|path| path != &link && path != &target);
        assert!(made.is_err(), "its look refused");
        assert!(!target.exists(), "nothing made through the link is left");
        assert!(link.is_symlink(), "the link is left as it was");
    }

    /// cr-ecm35-1: a link at the destination that leads out of it, to
    /// nothing, has nothing made outside: the file it names is made where
    /// it leads only within the destination's root. Judged by its lexical
    /// parent, the link passed, and the file was made outside.
    #[test]
    fn a_dangling_link_out_of_the_destination_makes_nothing_outside() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("dst");
        let outside = dir.path().join("outside");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::create_dir_all(&outside).unwrap();
        let link = root.join("link.bin");
        std::os::unix::fs::symlink("../outside/new.bin", &link).unwrap();
        let root = std::fs::canonicalize(&root).unwrap();
        let made = create_regular_within(&link, Some(&root));
        assert!(made.is_err(), "refused: {made:?}");
        assert!(!outside.join("new.bin").exists(), "nothing made outside");
        assert!(link.is_symlink(), "the link is left as it was");
    }

    /// cr-ecm36-1 (Linux): the opener's own opens are made beneath the
    /// destination's root: a directory on the way that is a link out of it
    /// — however it came to be one — leads nothing out. By the name, the
    /// creation followed it and made the file outside.
    #[cfg(target_os = "linux")]
    #[test]
    fn a_directory_on_the_way_that_leads_out_makes_nothing_outside() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("dst");
        let outside = dir.path().join("outside");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::create_dir_all(&outside).unwrap();
        std::os::unix::fs::symlink("../outside", root.join("sub")).unwrap();
        let root = std::fs::canonicalize(&root).unwrap();
        let made = create_regular_within(&root.join("sub").join("f.bin"), Some(&root));
        assert!(made.is_err(), "refused: {made:?}");
        assert!(!outside.join("f.bin").exists(), "nothing made outside");
        let made = open_to_write(&root.join("sub").join("g.bin"), Some(&root), true);
        assert!(made.is_err(), "refused: {made:?}");
        assert!(!outside.join("g.bin").exists(), "nothing made outside");
    }

    /// cr-ecm37-4 (Linux): an absolute link that leads within the
    /// destination is followed, as it always was — to a file, and as a
    /// directory on the way — and one that leads out is still refused.
    /// Opened beneath the root, the kernel took every absolute link for a
    /// way out, and the copy failed.
    #[cfg(target_os = "linux")]
    #[test]
    fn an_absolute_link_within_the_destination_is_followed() {
        let dir = tempfile::tempdir().unwrap();
        let outside = dir.path().join("outside");
        std::fs::create_dir_all(dir.path().join("dst").join("real")).unwrap();
        std::fs::create_dir_all(&outside).unwrap();
        let root = std::fs::canonicalize(dir.path().join("dst")).unwrap();
        let target = root.join("real").join("target.bin");
        std::fs::write(&target, b"old").unwrap();
        std::os::unix::fs::symlink(&target, root.join("link.bin")).unwrap();
        std::os::unix::fs::symlink(root.join("real"), root.join("into")).unwrap();
        std::os::unix::fs::symlink(&outside, root.join("out")).unwrap();

        let mut through = open_to_write(&root.join("link.bin"), Some(&root), false).unwrap();
        std::io::Write::write_all(&mut through, b"new").unwrap();
        drop(through);
        assert_eq!(std::fs::read(&target).unwrap(), b"new");
        assert!(
            root.join("link.bin").is_symlink(),
            "the link is left as it was"
        );

        create_regular_within(&root.join("into").join("made.bin"), Some(&root)).unwrap();
        assert!(root.join("real").join("made.bin").is_file());

        let made = create_regular_within(&root.join("out").join("f.bin"), Some(&root));
        assert!(made.is_err(), "refused: {made:?}");
        assert!(!outside.join("f.bin").exists(), "nothing made outside");
    }

    /// cr-ecm35-1: a chain of links to nothing as long as the system follows
    /// — 40 on Linux, 32 on macOS (cr-ecm36-8) — has its file made at the
    /// end. The opener's own resolution refused it.
    #[test]
    fn a_chain_of_forty_links_to_nothing_has_its_file_made() {
        let longest = super::resume::MAX_FOLLOWED_LINKS;
        let dir = tempfile::tempdir().unwrap();
        for n in 1..=longest {
            let next = if n == longest {
                "target.bin".to_string()
            } else {
                format!("l{}", n + 1)
            };
            std::os::unix::fs::symlink(next, dir.path().join(format!("l{n}"))).unwrap();
        }
        let made = create_regular(&dir.path().join("l1"));
        assert!(made.is_ok(), "{made:?}");
        assert!(dir.path().join("target.bin").is_file());
    }

    /// cr-ecm35-1: a taken name is opened without making anything: a file
    /// gone between the two opens is made by a creation that fails if the
    /// name is taken, and so known to be made — removed when its look
    /// refuses it. Made by the second open, it was recorded as one that
    /// stood there, and left.
    #[test]
    fn a_file_gone_between_the_opens_is_made_only_exclusively() {
        let dir = tempfile::tempdir().unwrap();
        let at = dir.path().join("was.bin");
        std::fs::write(&at, b"there a moment ago").unwrap();
        BETWEEN_THE_TWO_OPENS
            .lock()
            .unwrap()
            .push((at.clone(), |at| {
                let _ = std::fs::remove_file(at);
            }));
        CREATE_LOOK_FAULTS.lock().unwrap().push(at.clone());
        let made = create_regular(&at);
        BETWEEN_THE_TWO_OPENS
            .lock()
            .unwrap()
            .retain(|(path, _)| path != &at);
        CREATE_LOOK_FAULTS
            .lock()
            .unwrap()
            .retain(|path| path != &at);
        assert!(made.is_err(), "its look refused");
        assert!(!at.exists(), "what it made is not left");
    }

    /// cr-ecm34-1: a file made and refused is removed only while the name
    /// holds it. One put there since — before the name was looked at to
    /// remove it — is somebody's, and is kept.
    #[test]
    fn a_file_put_where_a_refused_one_was_made_is_kept() {
        let dir = tempfile::tempdir().unwrap();
        let at = dir.path().join("absent.bin");
        CREATE_LOOK_FAULTS.lock().unwrap().push(at.clone());
        BEFORE_A_MADE_FILE_IS_REMOVED
            .lock()
            .unwrap()
            .push((at.clone(), |at| {
                let theirs = at.with_extension("theirs");
                std::fs::write(&theirs, b"theirs").unwrap();
                std::fs::rename(&theirs, at).unwrap();
            }));
        let made = create_regular(&at);
        CREATE_LOOK_FAULTS
            .lock()
            .unwrap()
            .retain(|path| path != &at);
        BEFORE_A_MADE_FILE_IS_REMOVED
            .lock()
            .unwrap()
            .retain(|(path, _)| path != &at);
        assert!(made.is_err(), "its look refused");
        assert_eq!(std::fs::read(&at).unwrap(), b"theirs", "somebody's, kept");
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
