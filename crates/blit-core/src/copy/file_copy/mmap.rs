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
    dst_file.set_len(file_size)?;

    #[cfg(target_os = "linux")]
    {
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
            std::io::copy(&mut (&src_file).take(remaining), &mut &dst_file)
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

    #[cfg(not(target_os = "linux"))]
    std::fs::copy(src, dst).context("Memory-mapped copy fallback failed")
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
