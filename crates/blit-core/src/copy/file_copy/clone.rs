use eyre::{Context, Result};
use std::fs::File;
#[cfg(target_os = "macos")]
use std::path::Path;

#[cfg(target_os = "macos")]
pub(crate) fn attempt_clonefile_macos(src: &Path, dst: &Path) -> Result<bool> {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;
    let c_src = CString::new(src.as_os_str().as_bytes())?;
    let c_dst = CString::new(dst.as_os_str().as_bytes())?;
    let rc = unsafe { libc::clonefile(c_src.as_ptr(), c_dst.as_ptr(), 0) };
    Ok(rc == 0)
}

#[cfg(target_os = "macos")]
pub(crate) fn attempt_fcopyfile_macos(src: &Path, dst: &Path) -> Result<bool> {
    use std::os::unix::io::AsRawFd;
    let s = std::fs::OpenOptions::new().read(true).open(src)?;
    let d = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(dst)?;
    let flags: libc::copyfile_flags_t =
        libc::COPYFILE_ACL | libc::COPYFILE_STAT | libc::COPYFILE_XATTR | libc::COPYFILE_DATA;
    let rc = unsafe { libc::fcopyfile(s.as_raw_fd(), d.as_raw_fd(), std::ptr::null_mut(), flags) };
    Ok(rc == 0)
}

#[cfg(all(unix, not(target_os = "macos")))]
pub(crate) fn attempt_copy_file_range_linux(
    src: &File,
    dst: &File,
    file_size: u64,
) -> Result<bool> {
    use std::os::unix::io::AsRawFd;
    let sfd = src.as_raw_fd();
    let dfd = dst.as_raw_fd();
    let mut copied: u64 = 0;
    while copied < file_size {
        let to_copy = (file_size - copied) as usize;
        let res = unsafe {
            libc::copy_file_range(
                sfd,
                std::ptr::null_mut(),
                dfd,
                std::ptr::null_mut(),
                to_copy,
                0,
            )
        };
        if res > 0 {
            copied += res as u64;
            continue;
        }
        if res == 0 {
            break;
        }
        let err = std::io::Error::last_os_error();
        if let Some(code) = err.raw_os_error() {
            if code == libc::EXDEV || code == libc::EINVAL {
                return Ok(false);
            }
            if code == libc::EINTR || code == libc::EAGAIN {
                continue;
            }
        }
        return Ok(false);
    }
    Ok(copied == file_size)
}

#[cfg(all(unix, not(target_os = "macos")))]
pub(crate) fn attempt_sendfile_linux(src: &File, dst: &File, file_size: u64) -> Result<bool> {
    use std::os::unix::io::AsRawFd;
    let sfd = src.as_raw_fd();
    let dfd = dst.as_raw_fd();
    let mut copied: u64 = 0;
    while copied < file_size {
        let to_copy = (file_size - copied) as usize;
        let res = unsafe { libc::sendfile(dfd, sfd, std::ptr::null_mut(), to_copy) };
        if res > 0 {
            copied += res as u64;
            continue;
        }
        if res == 0 {
            break;
        }
        let err = std::io::Error::last_os_error();
        if let Some(code) = err.raw_os_error() {
            if code == libc::EINTR || code == libc::EAGAIN {
                continue;
            }
            if code == libc::EINVAL || code == libc::ENOSYS {
                return Ok(false);
            }
        }
        return Ok(false);
    }
    Ok(copied == file_size)
}

#[cfg(all(unix, not(target_os = "macos")))]
pub(crate) fn attempt_sparse_copy_unix(
    src: &File,
    dst: &File,
    file_size: u64,
) -> Result<Option<u64>> {
    use std::io::{Read, Seek, SeekFrom};
    use std::os::unix::fs::FileExt as _;
    use std::os::unix::io::AsRawFd;

    let sfd = src.as_raw_fd();
    let dfd = dst;

    let mut pos: i64 = 0;
    let mut any = false;
    let mut copied: u64 = 0;
    dst.set_len(file_size)
        .with_context(|| "pre-size destination for sparse copy")?;
    let buf_sz = 1 << 20;
    let mut buf = vec![0u8; buf_sz];
    while (pos as u64) < file_size {
        let data_off = unsafe { libc::lseek(sfd, pos, libc::SEEK_DATA) };
        if data_off < 0 {
            let err = std::io::Error::last_os_error();
            if let Some(code) = err.raw_os_error() {
                if code == libc::ENXIO || code == libc::EINVAL {
                    if !any {
                        return Ok(None);
                    }
                    break;
                }
            }
            if !any {
                return Ok(None);
            }
            break;
        }
        any = true;
        let hole_off = unsafe { libc::lseek(sfd, data_off, libc::SEEK_HOLE) };
        if hole_off < 0 {
            let start = data_off as u64;
            let end = file_size;
            if start >= end {
                break;
            }
            let mut so = src.try_clone()?;
            so.seek(SeekFrom::Start(start))?;
            let mut remaining = end - start;
            let mut offset = start;
            while remaining > 0 {
                let to_read = remaining.min(buf.len() as u64) as usize;
                so.read_exact(&mut buf[..to_read])?;
                dfd.write_all_at(&buf[..to_read], offset)?;
                remaining -= to_read as u64;
                offset += to_read as u64;
            }
            copied = end;
            break;
        }
        let start = data_off as u64;
        let end = hole_off as u64;
        if start >= end {
            pos = hole_off;
            continue;
        }
        let mut so = src.try_clone()?;
        so.seek(SeekFrom::Start(start))?;
        let mut remaining = end - start;
        let mut offset = start;
        while remaining > 0 {
            let to_read = remaining.min(buf.len() as u64) as usize;
            so.read_exact(&mut buf[..to_read])?;
            dfd.write_all_at(&buf[..to_read], offset)?;
            remaining -= to_read as u64;
            offset += to_read as u64;
        }
        copied = end;
        pos = hole_off;
    }

    Ok(if any { Some(copied) } else { None })
}

#[cfg(windows)]
pub(crate) fn mark_file_sparse(file: &File) -> bool {
    use std::os::windows::io::AsRawHandle;
    use windows::Win32::Foundation::HANDLE;
    use windows::Win32::System::Ioctl::FSCTL_SET_SPARSE;
    use windows::Win32::System::IO::DeviceIoControl;

    #[repr(C)]
    struct FILE_SET_SPARSE_BUFFER {
        SetSparse: u8,
    }

    let handle = HANDLE(file.as_raw_handle() as isize);
    let mut inbuf = FILE_SET_SPARSE_BUFFER { SetSparse: 1 };
    let mut bytes: u32 = 0;
    unsafe {
        DeviceIoControl(
            handle,
            FSCTL_SET_SPARSE,
            Some((&mut inbuf as *mut FILE_SET_SPARSE_BUFFER).cast()),
            std::mem::size_of::<FILE_SET_SPARSE_BUFFER>() as u32,
            None,
            0,
            Some(&mut bytes),
            None,
        )
        .is_ok()
    }
}

#[cfg(windows)]
pub(crate) fn sparse_copy_windows(
    src: File,
    dst: &mut File,
    buffer_size: usize,
    file_size: u64,
) -> Result<u64> {
    use std::io::{Read, Seek, SeekFrom, Write};

    let _ = mark_file_sparse(dst);

    let mut buf = vec![0u8; buffer_size];
    let zero_buf = vec![0u8; 64 * 1024];
    let zero_threshold: usize = 256 * 1024;
    let mut written: u64 = 0;
    let mut zero_run: usize = 0;
    let mut src = src;

    loop {
        let n = src.read(&mut buf)?;
        if n == 0 {
            break;
        }
        let chunk = &buf[..n];
        let all_zero = chunk.iter().all(|&b| b == 0);
        if all_zero {
            zero_run += n;
            continue;
        }
        if zero_run > 0 {
            if zero_run >= zero_threshold {
                dst.seek(SeekFrom::Current(zero_run as i64))?;
            } else {
                let mut remaining = zero_run;
                while remaining > 0 {
                    let to_write = remaining.min(zero_buf.len());
                    dst.write_all(&zero_buf[..to_write])?;
                    remaining -= to_write;
                    written += to_write as u64;
                }
            }
            zero_run = 0;
        }
        dst.write_all(chunk)?;
        written += n as u64;
    }

    if zero_run > 0 {
        if zero_run >= zero_threshold {
            dst.seek(SeekFrom::Current(zero_run as i64))?;
        } else {
            let mut remaining = zero_run;
            while remaining > 0 {
                let to_write = remaining.min(zero_buf.len());
                dst.write_all(&zero_buf[..to_write])?;
                remaining -= to_write;
                written += to_write as u64;
            }
        }
    }
    dst.set_len(file_size)?;
    Ok(written)
}
