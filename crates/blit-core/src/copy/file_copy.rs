use crate::buffer::BufferSizer;
use crate::fs_capability::{get_platform_capability, FilesystemCapability};
use crate::logger::Logger;
use eyre::{eyre, Context, Result};
use std::fs::{self, File};
#[cfg(windows)]
use std::io::Read;
use std::io::{self, BufReader, BufWriter, Write};
#[cfg(windows)]
use std::io::{Seek, SeekFrom};
use std::path::Path;

#[cfg(windows)]
const FILE_FLAG_SEQUENTIAL_SCAN: u32 = 0x0800_0000;

#[cfg(windows)]
use std::env;

#[cfg(windows)]
use crate::copy::windows;

/// Copy a single file with optimal buffer size
pub fn copy_file(
    src: &Path,
    dst: &Path,
    buffer_sizer: &BufferSizer,
    is_network: bool,
    logger: &dyn Logger,
) -> Result<u64> {
    logger.start(src, dst);

    #[cfg(windows)]
    if !is_network {
        match windows::windows_copyfile(src, dst) {
            Ok(bytes) => {
                let clone_succeeded = windows::take_last_block_clone_success();
                let skip_clone_metadata =
                    clone_succeeded && env::var_os("BLIT_SKIP_METADATA_ON_CLONE").is_some();
                if skip_clone_metadata {
                    log::info!(
                        "prototype: skipping metadata preservation after block clone for {}",
                        dst.display()
                    );
                } else {
                    preserve_metadata(src, dst)?;
                }
                if clone_succeeded {
                    println!("block clone {} ({} bytes)", dst.display(), bytes);
                }
                logger.copy_done(src, dst, bytes);
                return Ok(bytes);
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

    let result: Result<u64> = (|| {
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
        let dst_file = {
            std::fs::OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .custom_flags(FILE_FLAG_SEQUENTIAL_SCAN)
                .open(dst)?
        };
        #[cfg(not(windows))]
        let dst_file = File::create(dst)?;

        let total_bytes = {
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
                    file_size
                } else {
                    sparse_copy_windows(src_file, &dst_file, buffer_size, file_size)?
                }
            }
            #[cfg(target_os = "macos")]
            {
                if attempt_clonefile_macos(src, dst).unwrap_or(false)
                    || attempt_fcopyfile_macos(src, dst).unwrap_or(false)
                {
                    file_size
                } else {
                    let mut reader = BufReader::with_capacity(buffer_size, src_file);
                    let mut writer = BufWriter::with_capacity(buffer_size, dst_file);
                    let n = io::copy(&mut reader, &mut writer)?;
                    writer.flush()?;
                    n
                }
            }
            #[cfg(all(unix, not(target_os = "macos")))]
            {
                let fast_linux = attempt_copy_file_range_linux(&src_file, &dst_file, file_size)
                    .unwrap_or(false)
                    || attempt_sendfile_linux(&src_file, &dst_file, file_size).unwrap_or(false);
                if fast_linux {
                    file_size
                } else if let Some(n) = attempt_sparse_copy_unix(&src_file, &dst_file, file_size)? {
                    n
                } else {
                    let mut reader = BufReader::with_capacity(buffer_size, src_file);
                    let mut writer = BufWriter::with_capacity(buffer_size, dst_file);
                    let n = io::copy(&mut reader, &mut writer)?;
                    writer.flush()?;
                    n
                }
            }
        };
        preserve_metadata(src, dst)?;

        Ok(total_bytes)
    })();

    match result {
        Ok(bytes) => {
            logger.copy_done(src, dst, bytes);
            Ok(bytes)
        }
        Err(e) => {
            logger.error("copy", src, &e.to_string());
            Err(e)
        }
    }
}

#[cfg(target_os = "macos")]
fn attempt_clonefile_macos(src: &Path, dst: &Path) -> Result<bool> {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;
    let c_src = CString::new(src.as_os_str().as_bytes())?;
    let c_dst = CString::new(dst.as_os_str().as_bytes())?;
    let rc = unsafe { libc::clonefile(c_src.as_ptr(), c_dst.as_ptr(), 0) };
    Ok(rc == 0)
}

#[cfg(target_os = "macos")]
fn attempt_fcopyfile_macos(src: &Path, dst: &Path) -> Result<bool> {
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
fn attempt_copy_file_range_linux(src: &File, dst: &File, file_size: u64) -> Result<bool> {
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
fn attempt_sendfile_linux(src: &File, dst: &File, file_size: u64) -> Result<bool> {
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
fn attempt_sparse_copy_unix(src: &File, dst: &File, file_size: u64) -> Result<Option<u64>> {
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
            let mut cursor = start;
            while cursor < end {
                let to = std::cmp::min(buf.len() as u64, end - cursor) as usize;
                let n = so.read(&mut buf[..to])?;
                if n == 0 {
                    break;
                }
                dfd.write_at(&buf[..n], cursor)?;
                cursor += n as u64;
                copied += n as u64;
            }
            break;
        } else {
            let start = data_off as u64;
            let mut end = hole_off as u64;
            if end > file_size {
                end = file_size;
            }
            let mut so = src.try_clone()?;
            so.seek(SeekFrom::Start(start))?;
            let mut cursor = start;
            while cursor < end {
                let to = std::cmp::min(buf.len() as u64, end - cursor) as usize;
                let n = so.read(&mut buf[..to])?;
                if n == 0 {
                    break;
                }
                dfd.write_at(&buf[..n], cursor)?;
                cursor += n as u64;
                copied += n as u64;
            }
            pos = hole_off;
        }
    }
    Ok(Some(copied))
}

#[cfg(windows)]
fn mark_file_sparse(file: &File) -> bool {
    use std::mem::size_of;
    use std::os::windows::io::AsRawHandle;
    type HANDLE = isize;
    type DWORD = u32;
    type BOOL = i32;
    type LPVOID = *mut core::ffi::c_void;
    type LPOVERLAPPED = *mut core::ffi::c_void;

    #[link(name = "Kernel32")]
    extern "system" {
        fn DeviceIoControl(
            hDevice: HANDLE,
            dwIoControlCode: DWORD,
            lpInBuffer: LPVOID,
            nInBufferSize: DWORD,
            lpOutBuffer: LPVOID,
            nOutBufferSize: DWORD,
            lpBytesReturned: *mut DWORD,
            lpOverlapped: LPOVERLAPPED,
        ) -> BOOL;
    }

    const FILE_DEVICE_FILE_SYSTEM: DWORD = 0x00000009;
    const METHOD_BUFFERED: DWORD = 0;
    const FILE_ANY_ACCESS: DWORD = 0;
    const FILE_SPECIAL_ACCESS: DWORD = FILE_ANY_ACCESS;
    const fn ctl_code(device_type: DWORD, function: DWORD, method: DWORD, access: DWORD) -> DWORD {
        (device_type << 16) | (access << 14) | (function << 2) | method
    }
    const FSCTL_SET_SPARSE: DWORD = ctl_code(
        FILE_DEVICE_FILE_SYSTEM,
        49,
        METHOD_BUFFERED,
        FILE_SPECIAL_ACCESS,
    );

    #[allow(non_camel_case_types, non_snake_case)]
    #[repr(C)]
    struct FILE_SET_SPARSE_BUFFER {
        SetSparse: u8,
    }

    let h = file.as_raw_handle() as isize;
    let mut inbuf = FILE_SET_SPARSE_BUFFER { SetSparse: 1 };
    let mut bytes: DWORD = 0;
    unsafe {
        DeviceIoControl(
            h,
            FSCTL_SET_SPARSE,
            (&mut inbuf as *mut FILE_SET_SPARSE_BUFFER).cast(),
            size_of::<FILE_SET_SPARSE_BUFFER>() as DWORD,
            core::ptr::null_mut(),
            0,
            &mut bytes,
            core::ptr::null_mut(),
        ) != 0
    }
}

#[cfg(windows)]
fn sparse_copy_windows(src: File, dst: &File, buffer_size: usize, file_size: u64) -> Result<u64> {
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

fn preserve_metadata(src: &Path, dst: &Path) -> Result<()> {
    let fs_cap = get_platform_capability();
    let preserved = fs_cap.preserve_metadata(src, dst)?;

    if !preserved.mtime {
        log::debug!("Could not preserve mtime for {}", dst.display());
    }
    if !preserved.permissions {
        log::debug!("Could not preserve permissions for {}", dst.display());
    }

    Ok(())
}

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

pub fn chunked_copy_file(
    src: &Path,
    dst: &Path,
    buffer_sizer: &BufferSizer,
    is_network: bool,
    progress: Option<&indicatif::ProgressBar>,
    logger: &dyn Logger,
) -> Result<u64> {
    logger.start(src, dst);

    #[cfg(windows)]
    if !is_network {
        if let Ok(bytes) = windows::windows_copyfile(src, dst) {
            preserve_metadata(src, dst)?;
            logger.copy_done(src, dst, bytes);
            return Ok(bytes);
        }
    }
    let result: Result<u64> = (|| {
        let metadata = fs::metadata(src)?;
        let file_size = metadata.len();

        let chunk_size = if file_size > 1_073_741_824 {
            16 * 1024 * 1024
        } else {
            buffer_sizer.calculate_buffer_size(file_size, is_network)
        };

        if let Some(parent) = dst.parent() {
            fs::create_dir_all(parent)?;
        }

        let mut reader = BufReader::with_capacity(chunk_size, File::open(src)?);
        let mut writer = BufWriter::with_capacity(chunk_size, File::create(dst)?);
        let total_bytes = io::copy(&mut reader, &mut writer)?;
        if let Some(pb) = progress {
            pb.set_position(total_bytes);
        }

        #[cfg(windows)]
        preserve_metadata(src, dst)?;

        Ok(total_bytes)
    })();

    match result {
        Ok(bytes) => {
            logger.copy_done(src, dst, bytes);
            Ok(bytes)
        }
        Err(e) => {
            logger.error("chunked_copy", src, &e.to_string());
            Err(e)
        }
    }
}
