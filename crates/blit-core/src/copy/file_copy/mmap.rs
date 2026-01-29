use eyre::{Context, Result};
#[cfg(unix)]
use std::fs;
#[cfg(unix)]
use std::fs::File;
use std::path::Path;

#[cfg(unix)]
pub fn mmap_copy_file(src: &Path, dst: &Path) -> Result<u64> {
    let src_file = File::open(src)?;
    let file_size = src_file.metadata()?.len();

    if let Some(parent) = dst.parent() {
        fs::create_dir_all(parent)?;
    }

    let dst_file = File::create(dst)?;
    dst_file.set_len(file_size)?;

    #[cfg(target_os = "linux")]
    {
        use std::os::unix::io::AsRawFd;
        let src_fd = src_file.as_raw_fd();
        let dst_fd = dst_file.as_raw_fd();

        let mut copied: u64 = 0;
        loop {
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
        if copied < file_size {
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
    }

    std::fs::copy(src, dst).context("Memory-mapped copy fallback failed")
}

#[cfg(not(unix))]
pub fn mmap_copy_file(src: &Path, dst: &Path) -> Result<u64> {
    std::fs::copy(src, dst).context("Copy failed")
}
