//! Optimized copy operations for Windows
//! Focus on 10GbE saturation with minimal overhead

use crate::logger::Logger;
use anyhow::{anyhow, Context, Result};
use parking_lot::Mutex;
use rayon::prelude::*;
use std::fs::{self, File};
use std::io::{self, BufReader, BufWriter, Read, Write};
#[cfg(windows)]
use std::io::{Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;

use crate::buffer::BufferSizer;
use crate::checksum::{self, ChecksumType};
use crate::fs_enum::FileEntry;

/// Check if a file needs to be copied (for mirror mode)
pub fn file_needs_copy(src: &Path, dst: &Path, use_checksum: bool) -> Result<bool> {
    // If destination doesn't exist, definitely copy
    if !dst.exists() {
        return Ok(true);
    }

    let src_meta = src.metadata()?;
    let dst_meta = dst.metadata()?;

    // If sizes differ, copy
    if src_meta.len() != dst_meta.len() {
        return Ok(true);
    }

    if use_checksum {
        Ok(file_needs_copy_with_checksum_type(
            src,
            dst,
            Some(ChecksumType::Blake3),
        )?)
    } else {
        // Fast timestamp comparison (default)
        let src_time = src_meta.modified().unwrap_or(SystemTime::UNIX_EPOCH);
        let dst_time = dst_meta.modified().unwrap_or(SystemTime::UNIX_EPOCH);

        // Copy if source is newer (allow 2 second tolerance for filesystem precision)
        Ok(src_time
            .duration_since(dst_time)
            .is_ok_and(|diff| diff.as_secs() > 2))
    }
}

/// Like file_needs_copy, but with explicit checksum selection.
pub fn file_needs_copy_with_checksum_type(
    src: &Path,
    dst: &Path,
    checksum: Option<ChecksumType>,
) -> Result<bool> {
    if !dst.exists() {
        return Ok(true);
    }
    let src_meta = src.metadata()?;
    let dst_meta = dst.metadata()?;
    if src_meta.len() != dst_meta.len() {
        return Ok(true);
    }
    // Sizes equal: try partial-hash quick check
    let ph_bytes = 1024 * 1024; // 1 MiB
    let src_ph = checksum::partial_hash_first_last(src, ph_bytes)?;
    let dst_ph = checksum::partial_hash_first_last(dst, ph_bytes)?;
    if src_ph != dst_ph {
        return Ok(true);
    }
    match checksum {
        Some(ChecksumType::Blake3) => {
            // Full hash confirm
            let a = checksum::hash_file(src, ChecksumType::Blake3)?;
            let b = checksum::hash_file(dst, ChecksumType::Blake3)?;
            Ok(a != b)
        }
        Some(_) | None => {
            // Fallback to mtime heuristic
            let src_time = src_meta.modified().unwrap_or(SystemTime::UNIX_EPOCH);
            let dst_time = dst_meta.modified().unwrap_or(SystemTime::UNIX_EPOCH);
            Ok(src_time
                .duration_since(dst_time)
                .is_ok_and(|diff| diff.as_secs() > 2))
        }
    }
}

/// Compare file contents using fast hashing (for --checksum mode)
#[allow(dead_code)]
fn files_have_different_content(src: &Path, dst: &Path) -> Result<bool> {
    let src_hash = hash_file_content(src)?;
    let dst_hash = hash_file_content(dst)?;
    Ok(src_hash != dst_hash)
}

/// Fast file content hashing using BLAKE3
#[allow(dead_code)]
fn hash_file_content(path: &Path) -> Result<[u8; 32]> {
    let mut hasher = blake3::Hasher::new();
    let mut buffer = [0u8; 64 * 1024]; // 64KB chunks
    let mut file = File::open(path)?;

    loop {
        let bytes_read = file.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }

    Ok(hasher.finalize().into())
}

/// Statistics for copy operations
#[derive(Debug, Default, Clone)]
pub struct CopyStats {
    pub files_copied: u64,
    pub bytes_copied: u64,
    pub errors: Vec<String>,
}

impl CopyStats {
    pub fn add_file(&mut self, bytes: u64) {
        self.files_copied += 1;
        self.bytes_copied += bytes;
    }

    pub fn add_error(&mut self, error: String) {
        self.errors.push(error);
    }
}

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
        match windows_copyfile(src, dst) {
            Ok(bytes) => {
                preserve_metadata(src, dst)?;
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
        // Get file size for buffer calculation
        let metadata = fs::metadata(src)?;
        let file_size = metadata.len();

        // Calculate optimal buffer size
        let buffer_size = buffer_sizer.calculate_buffer_size(file_size, is_network);

        // Create parent directory if needed
        if let Some(parent) = dst.parent() {
            fs::create_dir_all(parent)?;
        }

        // Ensure destination directory exists
        let parent = dst
            .parent()
            .ok_or_else(|| anyhow!("destination has no parent: {}", dst.display()))?;
        fs::create_dir_all(parent)?;

        // Open files and stream directly to destination (no staging)
        #[cfg(windows)]
        use std::os::windows::fs::OpenOptionsExt;
        #[cfg(windows)]
        use windows::Win32::Storage::FileSystem::FILE_FLAG_SEQUENTIAL_SCAN;

        #[cfg(windows)]
        let src_file = {
            let o = std::fs::OpenOptions::new()
                .read(true)
                .custom_flags(FILE_FLAG_SEQUENTIAL_SCAN.0 as u32)
                .open(src)?;
            o
        };
        #[cfg(not(windows))]
        let src_file = File::open(src)?;

        #[cfg(windows)]
        let dst_file = {
            let o = std::fs::OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .custom_flags(FILE_FLAG_SEQUENTIAL_SCAN.0 as u32)
                .open(&dst)?;
            o
        };
        #[cfg(not(windows))]
        let dst_file = File::create(dst)?;

        // Try OS-specific fast paths (block clone/copy) before streaming
        let total_bytes = {
            #[cfg(windows)]
            {
                if attempt_block_clone_windows(&src_file, &dst_file, file_size).unwrap_or(false) {
                    file_size
                } else {
                    // Sparse-aware streaming copy on Windows
                    let n = sparse_copy_windows(src_file, &dst_file, buffer_size, file_size)?;
                    n
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
        // Preserve basic metadata best-effort on destination
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

#[cfg(windows)]
fn attempt_block_clone_windows(src: &File, dst: &File, file_size: u64) -> Result<bool> {
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
    const FILE_WRITE_ACCESS: DWORD = 0x0002;
    const fn ctl_code(device_type: DWORD, function: DWORD, method: DWORD, access: DWORD) -> DWORD {
        (device_type << 16) | (access << 14) | (function << 2) | method
    }
    const FSCTL_DUPLICATE_EXTENTS_TO_FILE: DWORD = ctl_code(
        FILE_DEVICE_FILE_SYSTEM,
        623,
        METHOD_BUFFERED,
        FILE_WRITE_ACCESS,
    );

    #[repr(C)]
    struct LARGE_INTEGER {
        QuadPart: i64,
    }
    #[repr(C)]
    struct DUPLICATE_EXTENTS_DATA {
        FileHandle: HANDLE,
        SourceFileOffset: LARGE_INTEGER,
        TargetFileOffset: LARGE_INTEGER,
        ByteCount: LARGE_INTEGER,
    }

    // Ensure destination length matches so clone can extend to size
    dst.set_len(file_size)?;

    let src_h = src.as_raw_handle() as isize;
    let dst_h = dst.as_raw_handle() as isize;
    let mut data = DUPLICATE_EXTENTS_DATA {
        FileHandle: src_h,
        SourceFileOffset: LARGE_INTEGER { QuadPart: 0 },
        TargetFileOffset: LARGE_INTEGER { QuadPart: 0 },
        ByteCount: LARGE_INTEGER {
            QuadPart: file_size as i64,
        },
    };
    let mut bytes: DWORD = 0;
    // SAFETY: `dst_h` and `data` come from live `File` handles and stack storage scoped to this
    // call; the buffer length passed matches the struct size and DeviceIoControl writes only
    // through the provided pointer when the FSCTL succeeds.
    let ok = unsafe {
        DeviceIoControl(
            dst_h,
            FSCTL_DUPLICATE_EXTENTS_TO_FILE,
            (&mut data as *mut DUPLICATE_EXTENTS_DATA).cast(),
            size_of::<DUPLICATE_EXTENTS_DATA>() as DWORD,
            core::ptr::null_mut(),
            0,
            &mut bytes,
            core::ptr::null_mut(),
        ) != 0
    };
    Ok(ok)
}

#[cfg(target_os = "macos")]
fn attempt_clonefile_macos(src: &Path, dst: &Path) -> Result<bool> {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;
    let c_src = CString::new(src.as_os_str().as_bytes())?;
    let c_dst = CString::new(dst.as_os_str().as_bytes())?;
    // SAFETY: `c_src` and `c_dst` are NUL-terminated by `CString`, and the paths stay alive for the
    // duration of the call, so passing their raw pointers to `clonefile` is valid.
    let rc = unsafe { libc::clonefile(c_src.as_ptr(), c_dst.as_ptr(), 0) };
    Ok(rc == 0)
}

#[cfg(target_os = "macos")]
fn attempt_fcopyfile_macos(src: &Path, dst: &Path) -> Result<bool> {
    use std::os::unix::io::AsRawFd;
    // OPEN and call fcopyfile for full copy with metadata
    let s = std::fs::OpenOptions::new().read(true).open(src)?;
    let d = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(dst)?;
    let flags: libc::copyfile_flags_t =
        libc::COPYFILE_ACL | libc::COPYFILE_STAT | libc::COPYFILE_XATTR | libc::COPYFILE_DATA;
    // SAFETY: `s` and `d` remain open for the call and expose stable raw FDs; the null output
    // buffer pointer is documented as allowed for `fcopyfile`, and we pass constant flags.
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
        // SAFETY: `sfd`/`dfd` are raw file descriptors borrowed from live `File`s; passing null
        // offsets lets the kernel advance them, and `to_copy` bounds the transfer size.
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
                // Cross-device or unsupported: fall back to userspace copy
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
        // SAFETY: `sfd`/`dfd` are raw file descriptors borrowed from live `File`s; passing null
        // offsets lets the kernel advance them, and `to_copy` bounds the transfer size.
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

    // Probe SEEK_HOLE/DATA support
    let mut pos: i64 = 0;
    let mut any = false;
    let mut copied: u64 = 0;
    // Pre-size destination to preserve file length and holes
    dst.set_len(file_size)
        .with_context(|| "pre-size destination for sparse copy")?;
    let buf_sz = 1 << 20; // 1 MiB chunks
    let mut buf = vec![0u8; buf_sz];
    while (pos as u64) < file_size {
        // Find next data extent
        // SAFETY: `sfd` stays valid for the entire copy, and `pos` advances monotonically, so
        // seeking with `SEEK_DATA` operates within the same file descriptor; errors are handled.
        let data_off = unsafe { libc::lseek(sfd, pos, libc::SEEK_DATA) };
        if data_off < 0 {
            let err = std::io::Error::last_os_error();
            if let Some(code) = err.raw_os_error() {
                if code == libc::ENXIO || code == libc::EINVAL {
                    // Not supported or no more data
                    if !any {
                        return Ok(None);
                    }
                    break;
                }
            }
            // Treat as unsupported
            if !any {
                return Ok(None);
            }
            break;
        }
        any = true;
        // SAFETY: `data_off` originated from a successful `SEEK_DATA` on the same descriptor, so
        // invoking `SEEK_HOLE` continues scanning the same file; any OS error is surfaced.
        let hole_off = unsafe { libc::lseek(sfd, data_off, libc::SEEK_HOLE) };
        if hole_off < 0 {
            // Fallback: copy to EOF
            let start = data_off as u64;
            let end = file_size;
            if (start as u64) >= end {
                break;
            }
            // Copy [start, end)
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
            // Copy [start, end)
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

    #[repr(C)]
    struct FILE_SET_SPARSE_BUFFER {
        SetSparse: u8, // BOOLEAN
    }

    let h = file.as_raw_handle() as isize;
    let mut inbuf = FILE_SET_SPARSE_BUFFER { SetSparse: 1 };
    let mut bytes: DWORD = 0;
    // SAFETY: `h` is derived from a valid `File`, and `inbuf` lives for the duration of the call;
    // the buffer length matches the struct size required by `DeviceIoControl`.
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
fn sparse_copy_windows(
    mut src: File,
    mut dst: &File,
    buffer_size: usize,
    file_size: u64,
) -> Result<u64> {
    // Enable sparse attribute (best-effort)
    let _ = mark_file_sparse(dst);

    let mut buf = vec![0u8; buffer_size];
    let mut zero_buf = vec![0u8; 64 * 1024];
    let zero_threshold: usize = 256 * 1024; // create holes only for >=256KiB zero runs
    let mut written: u64 = 0;
    let mut offset: u64 = 0;
    let mut zero_run: usize = 0;

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
        // Flush any pending zero run before writing non-zero data
        if zero_run > 0 {
            if zero_run >= zero_threshold {
                // Seek forward to create a hole
                dst.seek(SeekFrom::Current(zero_run as i64))?;
            } else {
                // Write zeros directly
                let mut remaining = zero_run;
                while remaining > 0 {
                    let to_write = remaining.min(zero_buf.len());
                    dst.write_all(&zero_buf[..to_write])?;
                    remaining -= to_write;
                    written += to_write as u64;
                }
            }
            offset += zero_run as u64;
            zero_run = 0;
        }
        // Write the non-zero chunk
        dst.write_all(chunk)?;
        offset += n as u64;
        written += n as u64;
    }
    // Flush trailing zeros
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
        offset += zero_run as u64;
    }
    // Ensure final logical length matches source
    dst.set_len(file_size)?;
    Ok(written)
}

fn preserve_metadata(src: &Path, dst: &Path) -> Result<()> {
    use crate::fs_capability::{get_platform_capability, FilesystemCapability};

    let fs_cap = get_platform_capability();
    let preserved = fs_cap.preserve_metadata(src, dst)?;

    // Log warnings for metadata that couldn't be preserved
    if !preserved.mtime {
        log::debug!("Could not preserve mtime for {}", dst.display());
    }
    if !preserved.permissions {
        log::debug!("Could not preserve permissions for {}", dst.display());
    }

    Ok(())
}

/// Parallel copy for medium-sized files (1-100MB)
pub fn parallel_copy_files(
    pairs: Vec<(FileEntry, PathBuf)>,
    buffer_sizer: Arc<BufferSizer>,
    is_network: bool,
    logger: &dyn Logger,
) -> CopyStats {
    use std::sync::atomic::{AtomicU64, Ordering};
    struct ConcurrentStats {
        files: AtomicU64,
        bytes: AtomicU64,
        errors: Mutex<Vec<String>>,
    }
    let stats = Arc::new(ConcurrentStats {
        files: AtomicU64::new(0),
        bytes: AtomicU64::new(0),
        errors: Mutex::new(Vec::new()),
    });

    pairs.par_iter().for_each(|(entry, dst)| {
        match copy_file(&entry.path, dst, &buffer_sizer, is_network, logger) {
            Ok(bytes) => {
                stats.files.fetch_add(1, Ordering::Relaxed);
                stats.bytes.fetch_add(bytes, Ordering::Relaxed);
            }
            Err(e) => {
                let mut errs = stats.errors.lock();
                errs.push(format!("Failed to copy {:?}: {}", entry.path, e));
            }
        }
    });

    let errors = stats.errors.lock().clone();
    CopyStats {
        files_copied: stats.files.load(Ordering::Relaxed),
        bytes_copied: stats.bytes.load(Ordering::Relaxed),
        errors,
    }
}

/// Memory-mapped copy for very large files (>100MB)
#[cfg(unix)]
pub fn mmap_copy_file(src: &Path, dst: &Path) -> Result<u64> {
    let src_file = File::open(src)?;
    let file_size = src_file.metadata()?.len();

    // Create parent directory
    if let Some(parent) = dst.parent() {
        fs::create_dir_all(parent)?;
    }

    let dst_file = File::create(dst)?;
    dst_file.set_len(file_size)?; // Pre-allocate space

    // For very large files, use copy_file_range or sendfile on Linux
    #[cfg(target_os = "linux")]
    {
        use std::os::unix::io::AsRawFd;
        let src_fd = src_file.as_raw_fd();
        let dst_fd = dst_file.as_raw_fd();

        let mut copied: u64 = 0;
        // Try copy_file_range in a loop
        loop {
            let to_copy = (file_size - copied) as usize;
            if to_copy == 0 {
                break;
            }
            // SAFETY: `src_fd`/`dst_fd` originate from `File` handles alive for the copy, offsets
            // are null to stream sequentially, and `to_copy` bounds the kernel transfer size.
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
                // SAFETY: `src_fd`/`dst_fd` originate from `File` handles alive for the copy, offsets
                // are null to stream sequentially, and `to_copy` bounds the kernel transfer size.
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

    // Fall back to regular copy if system calls fail
    std::fs::copy(src, dst).context("Memory-mapped copy fallback failed")
}

#[cfg(not(unix))]
pub fn mmap_copy_file(src: &Path, dst: &Path) -> Result<u64> {
    // Fall back to regular copy on non-Unix systems
    std::fs::copy(src, dst).context("Copy failed")
}

/// Chunked copy for large files (>10MB) with progress
pub fn chunked_copy_file(
    src: &Path,
    dst: &Path,
    buffer_sizer: &BufferSizer,
    is_network: bool,
    progress: Option<&indicatif::ProgressBar>,
    logger: &dyn Logger,
) -> Result<u64> {
    logger.start(src, dst);

    let result: Result<u64> = (|| {
        let metadata = fs::metadata(src)?;
        let file_size = metadata.len();

        // For very large files, use 16MB chunks
        let chunk_size = if file_size > 1_073_741_824 {
            // > 1GB
            16 * 1024 * 1024
        } else {
            buffer_sizer.calculate_buffer_size(file_size, is_network)
        };

        // Create parent directory
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

/// Direct system copy for local-to-local transfers on Windows
#[cfg(windows)]
pub fn windows_copyfile(src: &Path, dst: &Path) -> Result<u64> {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use windows::core::PCWSTR;
    use windows::Win32::Storage::FileSystem::CopyFileExW;

    // Ensure destination directory exists
    if let Some(parent) = dst.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("create parent dir {}", parent.display()))?;
    }

    let to_wide = |s: &OsStr| -> Vec<u16> { s.encode_wide().chain(std::iter::once(0)).collect() };
    let src_w = to_wide(src.as_os_str());
    let dst_w = to_wide(dst.as_os_str());
    // SAFETY: The wide strings are NUL-terminated and pinned in these vectors for the duration of
    // the call; we pass null progress/abort callbacks as allowed by `CopyFileExW` docs.
    let ok = unsafe {
        CopyFileExW(
            PCWSTR(src_w.as_ptr()),
            PCWSTR(dst_w.as_ptr()),
            None,
            None,
            None,
            0,
        )
        .is_ok()
    };
    if ok {
        let bytes = std::fs::metadata(dst)?.len();
        Ok(bytes)
    } else {
        // Fall back to Rust copy if API not available/failed
        std::fs::copy(src, dst).context("Failed to copy file via CopyFileExW (fallback)")
    }
}

#[cfg(not(windows))]
pub fn windows_copyfile(src: &Path, dst: &Path) -> Result<u64> {
    fs::copy(src, dst).context("Failed to copy file")
}
