//! Zero-copy primitives for high-performance I/O.

#[cfg(all(unix, not(target_os = "macos")))]
use eyre::{Context, Result};

#[cfg(all(unix, not(target_os = "macos")))]
use std::os::unix::io::AsRawFd;

// Common interface for file-like objects that can expose a raw file descriptor.
#[cfg(all(unix, not(target_os = "macos")))]
pub trait AsRawFileDescriptor {
    fn as_raw_fd(&self) -> std::os::unix::io::RawFd;
}

#[cfg(all(unix, not(target_os = "macos")))]
impl AsRawFileDescriptor for std::fs::File {
    fn as_raw_fd(&self) -> std::os::unix::io::RawFd {
        AsRawFd::as_raw_fd(self)
    }
}

#[cfg(all(unix, not(target_os = "macos")))]
impl AsRawFileDescriptor for tokio::fs::File {
    fn as_raw_fd(&self) -> std::os::unix::io::RawFd {
        AsRawFd::as_raw_fd(self)
    }
}

/// Result of a zero-copy operation attempt.
#[derive(Debug)]
pub enum ZeroCopyResult {
    Copied {
        bytes: usize,
        new_offset: Option<u64>,
        disable: bool,
    },
    WouldBlock,
    Unsupported,
}

/// Attempts to use `splice` to move data from a socket to a file descriptor.
/// This is a complex operation that uses a kernel pipe to avoid userspace buffers.
#[cfg(all(unix, not(target_os = "macos")))]
pub async fn splice_from_socket_to_file(
    socket_fd: std::os::unix::io::RawFd,
    file_fd: std::os::unix::io::RawFd,
    offset: Option<u64>,
    len: usize,
) -> Result<ZeroCopyResult> {
    if len == 0 {
        return Ok(ZeroCopyResult::WouldBlock);
    }
    let res =
        tokio::task::spawn_blocking(move || splice_chunk_blocking(socket_fd, file_fd, offset, len))
            .await
            .context("zero-copy splice join failure")??;
    Ok(res)
}

#[cfg(all(unix, not(target_os = "macos")))]
fn splice_chunk_blocking(
    socket_fd: std::os::unix::io::RawFd,
    file_fd: std::os::unix::io::RawFd,
    offset: Option<u64>,
    len: usize,
) -> std::io::Result<ZeroCopyResult> {
    use std::io;

    const MAX_SPLICE: usize = 8 * 1024 * 1024;

    let pipe = Pipe::new()?;
    let mut remaining = len;
    let mut total_copied = 0usize;
    let mut disable = false;
    let mut out_off = offset.map(|o| o as libc::loff_t);

    while remaining > 0 {
        let chunk = remaining.min(MAX_SPLICE);
        let read_bytes = loop {
            let res = unsafe {
                libc::splice(
                    socket_fd,
                    std::ptr::null_mut(),
                    pipe.writer,
                    std::ptr::null_mut(),
                    chunk,
                    libc::SPLICE_F_MOVE | libc::SPLICE_F_MORE,
                )
            };
            if res >= 0 {
                break res as usize;
            }
            let err = io::Error::last_os_error();
            match err.raw_os_error() {
                Some(code) if code == libc::EINTR => continue,
                Some(code) if code == libc::EAGAIN => {
                    // This is a temporary condition, we can try again.
                    // In a truly async world we would yield here.
                    // For a blocking task, a short sleep is a pragmatic choice.
                    std::thread::sleep(std::time::Duration::from_millis(10));
                    continue;
                }
                Some(code) if code == libc::EINVAL || code == libc::ENOSYS => {
                    if total_copied == 0 {
                        return Ok(ZeroCopyResult::Unsupported);
                    }
                    disable = true;
                    return Ok(ZeroCopyResult::Copied {
                        bytes: total_copied,
                        new_offset: out_off.map(|o| o as u64),
                        disable,
                    });
                }
                _ => return Err(err),
            }
        };

        if read_bytes == 0 {
            break;
        }

        let mut to_write = read_bytes;
        while to_write > 0 {
            let res = unsafe {
                libc::splice(
                    pipe.reader,
                    std::ptr::null_mut(),
                    file_fd,
                    match out_off.as_mut() {
                        Some(off) => off as *mut libc::loff_t,
                        None => std::ptr::null_mut(),
                    },
                    to_write,
                    libc::SPLICE_F_MOVE | libc::SPLICE_F_MORE,
                )
            };
            if res > 0 {
                let written = res as usize;
                total_copied += written;
                remaining = remaining.saturating_sub(written);
                to_write = to_write.saturating_sub(written);
                continue;
            }
            let err = io::Error::last_os_error();
            match err.raw_os_error() {
                Some(code) if code == libc::EINTR => continue,
                Some(code) if code == libc::EAGAIN => continue, // Should not happen on file->file splice
                _ => return Err(err),
            }
        }
    }

    if total_copied == 0 && len > 0 {
        return Ok(ZeroCopyResult::WouldBlock);
    }

    Ok(ZeroCopyResult::Copied {
        bytes: total_copied,
        new_offset: out_off.map(|o| o as u64),
        disable,
    })
}

/// Attempts to use `sendfile` to move data from a file descriptor to a socket.
#[cfg(all(unix, not(target_os = "macos")))]
pub fn sendfile_chunk(
    socket_fd: std::os::unix::io::RawFd,
    file_fd: std::os::unix::io::RawFd,
    offset: u64,
    len: usize,
) -> std::io::Result<(usize, u64)> {
    use std::io;
    let mut off = offset as i64;
    loop {
        let res = unsafe { libc::sendfile(socket_fd, file_fd, &mut off, len) };
        if res >= 0 {
            return Ok((res as usize, off as u64));
        }
        let err = io::Error::last_os_error();
        match err.raw_os_error() {
            Some(code) if code == libc::EINTR => continue,
            Some(code) if code == libc::EAGAIN => return Ok((0, offset)),
            _ => return Err(err),
        }
    }
}

/// A helper struct for creating and managing a kernel pipe.
#[cfg(all(unix, not(target_os = "macos")))]
struct Pipe {
    reader: std::os::unix::io::RawFd,
    writer: std::os::unix::io::RawFd,
}

#[cfg(all(unix, not(target_os = "macos")))]
impl Pipe {
    fn new() -> std::io::Result<Self> {
        use std::io;
        let mut fds = [0; 2];
        // Use O_CLOEXEC to prevent descriptor leakage across exec calls.
        if unsafe { libc::pipe2(fds.as_mut_ptr(), libc::O_CLOEXEC) } < 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(Pipe {
            reader: fds[0],
            writer: fds[1],
        })
    }
}

#[cfg(all(unix, not(target_os = "macos")))]
impl Drop for Pipe {
    fn drop(&mut self) {
        unsafe {
            libc::close(self.reader);
            libc::close(self.writer);
        }
    }
}
