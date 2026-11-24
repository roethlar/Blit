use eyre::{Context, Result};
#[cfg(unix)]
use std::fs;
use std::path::Path;

#[cfg(unix)]
pub fn mmap_copy_file(src: &Path, dst: &Path) -> Result<u64> {
    // cr-ecm33-6: without waiting, and only a regular file — and the
    // destination made the same way.
    let src_file = crate::copy::open_regular(src)?;
    let file_size = src_file.metadata()?.len();

    if let Some(parent) = dst.parent() {
        fs::create_dir_all(parent)?;
    }

    let dst_file = crate::copy::create_regular(dst)?;

    // cr-ecm35-2: the copy, its source's mode given through the handle, and
    // any failure from the cut on settled: what this copy wrote is removed
    // while the name holds it, or marked unfinished — one byte longer than
    // its source — and said to be left. Left as it was, a short copy stood
    // at its source's size, zeroes after what was read, and the source's
    // mode was never given.
    #[cfg(target_os = "linux")]
    {
        use std::os::unix::fs::PermissionsExt as _;
        let mode = src_file.metadata()?.permissions().mode() & 0o7777;
        let done = copy_through(src, &src_file, &dst_file, file_size).and_then(|copied| {
            dst_file
                .set_permissions(fs::Permissions::from_mode(mode))
                .context("giving the copy its source's mode")?;
            Ok(copied)
        });
        done.map_err(|error| settle_failed_copy(dst, &dst_file, file_size, error))
    }

    #[cfg(not(target_os = "linux"))]
    {
        dst_file.set_len(file_size)?;
        std::fs::copy(src, dst).context("Memory-mapped copy fallback failed")
    }
}

/// The Linux copy of [`mmap_copy_file`]: the destination cut to the
/// source's size, the kernel's copies, and the rest through the handles.
#[cfg(target_os = "linux")]
fn copy_through(
    src: &Path,
    src_file: &fs::File,
    dst_file: &fs::File,
    file_size: u64,
) -> Result<u64> {
    dst_file.set_len(file_size)?;
    use std::os::unix::io::AsRawFd;
    let src_fd = src_file.as_raw_fd();
    let dst_fd = dst_file.as_raw_fd();

    let mut copied: u64 = 0;
    let fast = fast_paths_taken(src);
    loop {
        if !fast {
            break;
        }
        let to_copy = (file_size - copied) as usize;
        if to_copy == 0 {
            break;
        }
        let res = unsafe {
            libc::copy_file_range(
                src_fd,
                std::ptr::null_mut(),
                dst_fd,
                std::ptr::null_mut(),
                to_copy,
                0,
            )
        };
        if res > 0 {
            copied += res as u64;
            continue;
        }
        if res < 0 {
            let err = std::io::Error::last_os_error();
            if let Some(code) = err.raw_os_error() {
                if code == libc::EINTR || code == libc::EAGAIN {
                    continue;
                }
            }
        }
        break;
    }
    if fast && copied < file_size {
        loop {
            let to_copy = (file_size - copied) as usize;
            if to_copy == 0 {
                break;
            }
            let res = unsafe { libc::sendfile(dst_fd, src_fd, std::ptr::null_mut(), to_copy) };
            if res > 0 {
                copied += res as u64;
                continue;
            }
            if res < 0 {
                let err = std::io::Error::last_os_error();
                if let Some(code) = err.raw_os_error() {
                    if code == libc::EINTR || code == libc::EAGAIN {
                        continue;
                    }
                }
            }
            break;
        }
    }
    if copied == file_size {
        return Ok(copied);
    }
    // cr-ecm34-3: the rest through the handles opened and checked above,
    // from where the fast paths left both — never the names again: by
    // then either may name another file, or a FIFO that waits.
    let remaining = file_size - copied;
    let rest = {
        use std::io::Read as _;
        std::io::copy(&mut src_file.take(remaining), &mut { dst_file })
            .context("Memory-mapped copy fallback failed")?
    };
    if rest != remaining {
        eyre::bail!(
            "source: changed size during transfer (opened {file_size} bytes, read {})",
            copied + rest
        );
    }
    Ok(file_size)
}

/// cr-ecm35-2: a copy that failed after its destination was opened — what it
/// wrote is removed while the name still holds that file, and what cannot be
/// removed is marked unfinished through the handle and said to be left
/// (D-2026-11-24-1: by the [`LeftInPlace`](crate::remote::transfer::sink::LeftInPlace)
/// marker). A file put at the name since is left as it is.
#[cfg(target_os = "linux")]
fn settle_failed_copy(
    dst: &Path,
    written: &fs::File,
    size: u64,
    error: eyre::Report,
) -> eyre::Report {
    use std::os::unix::fs::MetadataExt as _;
    let identity = |found: &fs::Metadata| (found.dev(), found.ino());
    let Ok(mine) = written.metadata().map(|found| identity(&found)) else {
        return error;
    };
    let mark = |why: String| {
        let left = match written.set_len(size.saturating_add(1)) {
            Ok(()) => format!(
                "{}: {why}, so it was left one byte longer than the source",
                crate::remote::transfer::sink::INCOMPLETE_LEFT_IN_PLACE
            ),
            Err(unmarked) => format!(
                "{}: {why}, and it could not be marked unfinished ({unmarked})",
                crate::remote::transfer::sink::INCOMPLETE_LEFT_IN_PLACE
            ),
        };
        crate::remote::transfer::sink::LeftInPlace(left)
    };
    if fs::symlink_metadata(dst).map(|found| identity(&found)).ok() != Some(mine) {
        // cr-ecm36-2: a link at the name that leads to the file written — as
        // the sink's cleanup has it (cr-ecm17-2) — is not this copy's to
        // remove: the file it wrote through it is marked and said to be left.
        // Judged by the link, it was left at its source's size, unmarked.
        if fs::metadata(dst).map(|found| identity(&found)).ok() == Some(mine) {
            return error.wrap_err(mark(
                "its name is a link to it, which is not this copy's to remove".to_string(),
            ));
        }
        return error;
    }
    let unremoved = match fs::remove_file(dst) {
        Ok(()) => return error,
        Err(gone) if gone.kind() == std::io::ErrorKind::NotFound => return error,
        Err(unremoved) => unremoved,
    };
    error.wrap_err(mark(format!("it could not be removed ({unremoved})")))
}

/// Test seam (cr-ecm34-3): the fast paths refused for a source, and a hook
/// run before the fallback — where the source's name can be given to
/// another file. Production never sets it.
#[cfg(all(target_os = "linux", test))]
pub(crate) static MMAP_FAST_PATHS_REFUSED: std::sync::Mutex<Vec<(std::path::PathBuf, fn(&Path))>> =
    std::sync::Mutex::new(Vec::new());

#[cfg(target_os = "linux")]
fn fast_paths_taken(_src: &Path) -> bool {
    #[cfg(test)]
    {
        let hooks: Vec<fn(&Path)> = MMAP_FAST_PATHS_REFUSED
            .lock()
            .map(|hooks| {
                hooks
                    .iter()
                    .filter(|(at, _)| at == _src)
                    .map(|(_, hook)| *hook)
                    .collect()
            })
            .unwrap_or_default();
        if !hooks.is_empty() {
            for hook in hooks {
                hook(_src);
            }
            return false;
        }
    }
    true
}

#[cfg(all(target_os = "linux", test))]
mod tests {
    use super::*;

    /// cr-ecm34-3: with the fast paths refused, the copy goes on through the
    /// source it opened and checked: a file renamed over the source's name
    /// before the fallback is not what is copied. Copied again by name, the
    /// fallback copied whatever the name held by then.
    /// cr-ecm35-2: the copy is given its source's mode — through the kernel's
    /// copy or the fallback alike — over a destination of another mode.
    #[test]
    fn the_copy_has_its_sources_mode() {
        use std::os::unix::fs::PermissionsExt as _;
        let dir = tempfile::tempdir().unwrap();
        for (name, refused) in [("fast", false), ("fallback", true)] {
            let src = dir.path().join(format!("{name}.src"));
            let dst = dir.path().join(format!("{name}.dst"));
            std::fs::write(&src, b"private bytes").unwrap();
            std::fs::set_permissions(&src, std::fs::Permissions::from_mode(0o600)).unwrap();
            std::fs::write(&dst, b"old").unwrap();
            std::fs::set_permissions(&dst, std::fs::Permissions::from_mode(0o644)).unwrap();
            if refused {
                MMAP_FAST_PATHS_REFUSED
                    .lock()
                    .unwrap()
                    .push((src.clone(), |_| {}));
            }
            let copied = mmap_copy_file(&src, &dst);
            MMAP_FAST_PATHS_REFUSED
                .lock()
                .unwrap()
                .retain(|(at, _)| at != &src);
            assert_eq!(copied.unwrap(), 13, "{name}");
            let mode = std::fs::metadata(&dst).unwrap().permissions().mode() & 0o7777;
            assert_eq!(mode, 0o600, "{name}");
        }
    }

    /// cr-ecm36-2: a copy through a link at the destination that fails has
    /// the file it wrote through the link marked unfinished, and says it
    /// was left. Judged by the link, the file was left at its source's
    /// size, unmarked.
    #[test]
    fn a_failed_copy_through_a_link_marks_what_it_wrote() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("shrinks.bin");
        let target = dir.path().join("target.bin");
        let dst = dir.path().join("link.bin");
        std::fs::write(&src, b"8 bytes!").unwrap();
        std::fs::write(&target, b"old").unwrap();
        std::os::unix::fs::symlink(&target, &dst).unwrap();
        MMAP_FAST_PATHS_REFUSED
            .lock()
            .unwrap()
            .push((src.clone(), |at| {
                std::fs::OpenOptions::new()
                    .write(true)
                    .open(at)
                    .unwrap()
                    .set_len(3)
                    .unwrap();
            }));
        let copied = mmap_copy_file(&src, &dst);
        MMAP_FAST_PATHS_REFUSED
            .lock()
            .unwrap()
            .retain(|(at, _)| at != &src);
        let error = copied.expect_err("the source shrank");
        assert!(
            crate::remote::transfer::sink::leaves_in_place(&error),
            "{error:#}"
        );
        assert!(dst.is_symlink(), "the link is left as it is");
        assert_eq!(
            std::fs::metadata(&target).unwrap().len(),
            9,
            "marked unfinished"
        );
    }

    /// cr-ecm35-2: a source that shrinks while it is copied fails the copy,
    /// and leaves nothing that looks finished: what was written is removed.
    /// Left, it stood at the source's size, zeroes after the bytes read.
    #[test]
    fn a_short_copy_leaves_nothing_that_looks_finished() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("shrinks.bin");
        let dst = dir.path().join("dst.bin");
        std::fs::write(&src, b"8 bytes!").unwrap();
        MMAP_FAST_PATHS_REFUSED
            .lock()
            .unwrap()
            .push((src.clone(), |at| {
                std::fs::OpenOptions::new()
                    .write(true)
                    .open(at)
                    .unwrap()
                    .set_len(3)
                    .unwrap();
            }));
        let copied = mmap_copy_file(&src, &dst);
        MMAP_FAST_PATHS_REFUSED
            .lock()
            .unwrap()
            .retain(|(at, _)| at != &src);
        assert!(copied.is_err(), "{copied:?}");
        assert!(!dst.exists(), "nothing left: {:?}", std::fs::read(&dst));
    }

    #[test]
    fn the_fallback_copies_what_was_opened_not_what_the_name_holds() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("src.bin");
        let dst = dir.path().join("dst.bin");
        std::fs::write(&src, b"the opened source").unwrap();
        MMAP_FAST_PATHS_REFUSED
            .lock()
            .unwrap()
            .push((src.clone(), |at| {
                let other = at.with_extension("other");
                std::fs::write(&other, b"another file, put there").unwrap();
                std::fs::rename(&other, at).unwrap();
            }));
        let copied = mmap_copy_file(&src, &dst);
        MMAP_FAST_PATHS_REFUSED
            .lock()
            .unwrap()
            .retain(|(at, _)| at != &src);
        assert_eq!(copied.unwrap(), 17);
        assert_eq!(std::fs::read(&dst).unwrap(), b"the opened source");
    }
}

#[cfg(not(unix))]
pub fn mmap_copy_file(src: &Path, dst: &Path) -> Result<u64> {
    std::fs::copy(src, dst).context("Copy failed")
}
