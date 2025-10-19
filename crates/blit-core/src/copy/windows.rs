use eyre::{Context, Result};
use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::path::Path;

use windows::core::PCWSTR;
use windows::Win32::Storage::FileSystem::CopyFileExW;
use windows::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};

const WINDOWS_NO_BUFFERING_THRESHOLD: u64 = 1 * 1024 * 1024 * 1024; // 1 GiB
const WINDOWS_NO_BUFFERING_FLOOR: u64 = 2 * 1024 * 1024 * 1024; // 2 GiB baseline
const WINDOWS_NO_BUFFERING_HEADROOM: u64 = 512 * 1024 * 1024; // leave 512 MiB for cache
const WINDOWS_NO_BUFFERING_SMALL_FILE_MAX: u64 = 512 * 1024 * 1024; // always cache ≤512 MiB
const COPY_FILE_NO_BUFFERING_FLAG: u32 = 0x0000_1000; // per CopyFileExW docs

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

pub fn windows_copyfile(src: &Path, dst: &Path) -> Result<u64> {
    use std::fs;

    // Ensure destination directory exists
    if let Some(parent) = dst.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("create parent dir {}", parent.display()))?;
    }

    let file_size = fs::metadata(src)?.len();

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
}
