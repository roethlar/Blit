use crate::fs_capability::{mark_block_clone_unsupported, supports_block_clone_same_volume};
use crate::win_fs::enable_manage_volume_privilege;
use eyre::{Context, Result};
use once_cell::sync::OnceCell;
use std::cell::Cell;
use std::ffi::OsStr;
use std::fs::{self, File};
use std::io;
use std::os::windows::ffi::OsStrExt;
use std::os::windows::io::AsRawHandle;
use std::path::Path;

use windows::core::PCWSTR;
use windows::Win32::Foundation::{
    ERROR_INVALID_FUNCTION, ERROR_NOT_SUPPORTED, ERROR_PRIVILEGE_NOT_HELD,
};
use windows::Win32::Storage::FileSystem::CopyFileExW;
use windows::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};

const WINDOWS_NO_BUFFERING_THRESHOLD: u64 = 1 * 1024 * 1024 * 1024; // 1 GiB
const WINDOWS_NO_BUFFERING_FLOOR: u64 = 2 * 1024 * 1024 * 1024; // 2 GiB baseline
const WINDOWS_NO_BUFFERING_HEADROOM: u64 = 512 * 1024 * 1024; // leave 512 MiB for cache
const WINDOWS_NO_BUFFERING_SMALL_FILE_MAX: u64 = 512 * 1024 * 1024; // always cache ≤512 MiB
const COPY_FILE_NO_BUFFERING_FLAG: u32 = 0x0000_1000; // per CopyFileExW docs

static MANAGE_VOLUME_PRIVILEGE: OnceCell<bool> = OnceCell::new();

thread_local! {
    static LAST_BLOCK_CLONE_SUCCESS: Cell<bool> = Cell::new(false);
}

fn set_last_block_clone_success(value: bool) {
    LAST_BLOCK_CLONE_SUCCESS.with(|flag| flag.set(value));
}

pub(crate) fn take_last_block_clone_success() -> bool {
    LAST_BLOCK_CLONE_SUCCESS.with(|flag| {
        let value = flag.get();
        flag.set(false);
        value
    })
}

#[derive(Debug)]
pub(crate) enum BlockCloneOutcome {
    Cloned,
    PrivilegeUnavailable,
    Unsupported { code: i32 },
    Failed(io::Error),
}

#[derive(Clone, Copy, Debug)]
struct MemorySnapshot {
    total_phys: u64,
    avail_phys: u64,
}

fn should_use_copyfile_no_buffering(file_size: u64) -> bool {
    let snapshot = unsafe {
        let mut status = MEMORYSTATUSEX::default();
        status.dwLength = std::mem::size_of::<MEMORYSTATUSEX>() as u32;

        if GlobalMemoryStatusEx(&mut status).is_ok() {
            Some(MemorySnapshot {
                total_phys: status.ullTotalPhys,
                avail_phys: status.ullAvailPhys,
            })
        } else {
            None
        }
    };

    should_use_copyfile_no_buffering_inner(file_size, snapshot)
}

fn should_use_copyfile_no_buffering_inner(
    file_size: u64,
    snapshot: Option<MemorySnapshot>,
) -> bool {
    if file_size <= WINDOWS_NO_BUFFERING_SMALL_FILE_MAX {
        log::trace!(
            "windows_copyfile: small file ({} bytes) – keeping cached path",
            file_size
        );
        return false;
    }

    if file_size < WINDOWS_NO_BUFFERING_THRESHOLD {
        log::trace!(
            "windows_copyfile: below threshold ({} < {}) – keeping cached path",
            file_size,
            WINDOWS_NO_BUFFERING_THRESHOLD
        );
        return false;
    }

    if let Some(status) = snapshot {
        if file_size.saturating_add(WINDOWS_NO_BUFFERING_HEADROOM) > status.avail_phys {
            log::debug!(
                "windows_copyfile: enabling NO_BUFFERING — insufficient available RAM (file={} bytes, avail_phys={} bytes)",
                file_size,
                status.avail_phys
            );
            return true;
        }

        let mut threshold = WINDOWS_NO_BUFFERING_FLOOR;
        if status.total_phys > 0 {
            let half_total = status.total_phys / 2;
            if half_total > 0 {
                threshold = threshold.min(half_total);
            }
        }

        log::trace!(
            "windows_copyfile: evaluating NO_BUFFERING — file={} bytes, total_phys={} bytes, avail_phys={} bytes, threshold={} bytes",
            file_size,
            status.total_phys,
            status.avail_phys,
            threshold
        );

        if file_size >= threshold {
            log::debug!(
                "windows_copyfile: enabling NO_BUFFERING — file {} exceeds threshold {}",
                file_size,
                threshold
            );
            return true;
        }

        log::trace!(
            "windows_copyfile: keeping cached path — file {} below threshold {}",
            file_size,
            threshold
        );
        false
    } else {
        if file_size >= WINDOWS_NO_BUFFERING_FLOOR {
            log::debug!(
                "windows_copyfile: enabling NO_BUFFERING — no memory snapshot; file {} exceeds static floor {}",
                file_size,
                WINDOWS_NO_BUFFERING_FLOOR
            );
            true
        } else {
            log::trace!(
                "windows_copyfile: GlobalMemoryStatusEx unavailable and file {} below static floor {}; keeping cached path",
                file_size,
                WINDOWS_NO_BUFFERING_FLOOR
            );
            false
        }
    }
}

fn ensure_manage_volume_privilege_once() -> bool {
    *MANAGE_VOLUME_PRIVILEGE.get_or_init(|| enable_manage_volume_privilege())
}

fn duplicate_extents(src: &File, dst: &File, file_size: u64) -> Result<BlockCloneOutcome> {
    let manage_privilege = ensure_manage_volume_privilege_once();
    if !manage_privilege {
        log::trace!("block clone: SeManageVolumePrivilege unavailable (attempting anyway)");
    }

    use core::ffi::c_void;
    type HANDLE = isize;
    type DWORD = u32;
    type BOOL = i32;
    type LPVOID = *mut c_void;
    type LPOVERLAPPED = *mut c_void;

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

    const FILE_DEVICE_FILE_SYSTEM: DWORD = 0x0000_0009;
    const METHOD_BUFFERED: DWORD = 0;
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

    dst.set_len(file_size)?;

    let src_h = src.as_raw_handle() as HANDLE;
    let dst_h = dst.as_raw_handle() as HANDLE;
    let mut data = DUPLICATE_EXTENTS_DATA {
        FileHandle: src_h,
        SourceFileOffset: LARGE_INTEGER { QuadPart: 0 },
        TargetFileOffset: LARGE_INTEGER { QuadPart: 0 },
        ByteCount: LARGE_INTEGER {
            QuadPart: file_size as i64,
        },
    };
    let mut bytes_returned: DWORD = 0;
    let ok = unsafe {
        DeviceIoControl(
            dst_h,
            FSCTL_DUPLICATE_EXTENTS_TO_FILE,
            (&mut data as *mut DUPLICATE_EXTENTS_DATA).cast(),
            std::mem::size_of::<DUPLICATE_EXTENTS_DATA>() as DWORD,
            core::ptr::null_mut(),
            0,
            &mut bytes_returned,
            core::ptr::null_mut(),
        ) != 0
    };

    if ok {
        Ok(BlockCloneOutcome::Cloned)
    } else {
        let err = std::io::Error::last_os_error();
        if let Some(code) = err.raw_os_error() {
            if code == ERROR_PRIVILEGE_NOT_HELD.0 as i32 {
                return Ok(BlockCloneOutcome::PrivilegeUnavailable);
            }
        }
        if let Some(code) = err.raw_os_error() {
            if code == ERROR_INVALID_FUNCTION.0 as i32 || code == ERROR_NOT_SUPPORTED.0 as i32 {
                return Ok(BlockCloneOutcome::Unsupported { code });
            }
        }
        Ok(BlockCloneOutcome::Failed(err))
    }
}

pub(crate) fn try_block_clone_with_handles(
    src: &File,
    dst: &File,
    file_size: u64,
) -> Result<BlockCloneOutcome> {
    let outcome = duplicate_extents(src, dst, file_size)?;
    if matches!(outcome, BlockCloneOutcome::Cloned) {
        set_last_block_clone_success(true);
    } else {
        set_last_block_clone_success(false);
    }
    Ok(outcome)
}

pub(crate) fn try_block_clone_same_volume(
    src: &Path,
    dst: &Path,
    file_size: u64,
) -> Result<Option<u64>> {
    set_last_block_clone_success(false);
    if !supports_block_clone_same_volume(src, dst)? {
        return Ok(None);
    }

    if file_size == 0 {
        log::debug!(
            "block clone: zero-length file {} treated as cloned",
            dst.display()
        );
        return Ok(Some(0));
    }

    let src_file =
        File::open(src).with_context(|| format!("open {} for block clone", src.display()))?;
    let dst_file = fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(dst)
        .with_context(|| format!("open {} for block clone", dst.display()))?;

    match duplicate_extents(&src_file, &dst_file, file_size)? {
        BlockCloneOutcome::Cloned => {
            set_last_block_clone_success(true);
            log::info!("block clone {} ({} bytes)", dst.display(), file_size);
            if !log::log_enabled!(log::Level::Info) {
                eprintln!("block clone {} ({} bytes)", dst.display(), file_size);
            }
            println!("block clone {} ({} bytes)", dst.display(), file_size);
            Ok(Some(file_size))
        }
        BlockCloneOutcome::PrivilegeUnavailable => {
            set_last_block_clone_success(false);
            log::trace!(
                "block clone: missing SeManageVolumePrivilege for {}; falling back",
                dst.display()
            );
            Ok(None)
        }
        BlockCloneOutcome::Unsupported { code } => {
            set_last_block_clone_success(false);
            log::debug!(
                "block clone unsupported on volume for {} (error code {code}); caching fallback",
                dst.display()
            );
            mark_block_clone_unsupported(src, dst);
            Ok(None)
        }
        BlockCloneOutcome::Failed(err) => {
            set_last_block_clone_success(false);
            log::debug!("block clone failed for {} ({err})", dst.display());
            Ok(None)
        }
    }
}

pub fn windows_copyfile(src: &Path, dst: &Path) -> Result<u64> {
    // Ensure destination directory exists
    if let Some(parent) = dst.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("create parent dir {}", parent.display()))?;
    }

    let file_size = fs::metadata(src)?.len();

    if let Some(bytes) = try_block_clone_same_volume(src, dst, file_size)? {
        return Ok(bytes);
    }

    let to_wide = |s: &OsStr| -> Vec<u16> { s.encode_wide().chain(std::iter::once(0)).collect() };
    let src_w = to_wide(src.as_os_str());
    let dst_w = to_wide(dst.as_os_str());

    let mut flags: u32 = 0;
    if should_use_copyfile_no_buffering(file_size) {
        flags |= COPY_FILE_NO_BUFFERING_FLAG;
    }

    let ok = unsafe {
        CopyFileExW(
            PCWSTR(src_w.as_ptr()),
            PCWSTR(dst_w.as_ptr()),
            None,
            None,
            None,
            flags,
        )
        .is_ok()
    };
    if ok {
        Ok(file_size)
    } else {
        fs::copy(src, dst).context("Failed to copy file via CopyFileExW (fallback)")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MB: u64 = 1024 * 1024;
    const GB: u64 = 1024 * MB;

    fn snapshot(total_gb: u64, avail_gb: u64) -> MemorySnapshot {
        MemorySnapshot {
            total_phys: total_gb * GB,
            avail_phys: avail_gb * GB,
        }
    }

    #[test]
    fn small_files_prefer_cached_path() {
        assert!(!should_use_copyfile_no_buffering_inner(
            256 * MB,
            Some(snapshot(32, 28))
        ));
    }

    #[test]
    fn small_file_threshold_is_inclusive() {
        assert!(!should_use_copyfile_no_buffering_inner(
            WINDOWS_NO_BUFFERING_SMALL_FILE_MAX,
            Some(snapshot(32, 28))
        ));
    }

    #[test]
    fn low_available_memory_triggers_no_buffering() {
        assert!(should_use_copyfile_no_buffering_inner(
            2 * GB,
            Some(snapshot(8, 1))
        ));
    }

    #[test]
    fn large_files_hit_threshold_even_with_healthy_cache() {
        assert!(should_use_copyfile_no_buffering_inner(
            3 * GB,
            Some(snapshot(8, 6))
        ));
    }

    #[test]
    fn generous_memory_keeps_cached_path() {
        assert!(!should_use_copyfile_no_buffering_inner(
            1_500 * MB,
            Some(snapshot(64, 60))
        ));
    }

    #[test]
    fn floor_threshold_triggers_no_buffering() {
        assert!(should_use_copyfile_no_buffering_inner(
            WINDOWS_NO_BUFFERING_FLOOR,
            Some(snapshot(32, 28))
        ));
    }
}
