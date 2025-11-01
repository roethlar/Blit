use super::types::{ChangeState, LinuxSnapshot, MacSnapshot, StoredSnapshot, WindowsSnapshot};
use super::util::system_time_to_epoch_ms;
use eyre::Result;
use std::path::Path;

pub fn capture_snapshot(path: &Path) -> Result<Option<StoredSnapshot>> {
    #[cfg(windows)]
    {
        return windows::capture_snapshot(path).map(|opt| opt.map(StoredSnapshot::Windows));
    }

    #[cfg(target_os = "macos")]
    {
        return macos::capture_snapshot(path).map(|opt| opt.map(StoredSnapshot::MacOs));
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    {
        return linux::capture_snapshot(path).map(|opt| opt.map(StoredSnapshot::Linux));
    }

    #[allow(unreachable_code)]
    Ok(None)
}

pub fn compare_snapshots(previous: &StoredSnapshot, current: &StoredSnapshot) -> ChangeState {
    match (previous, current) {
        (StoredSnapshot::Windows(prev), StoredSnapshot::Windows(cur)) => compare_windows(prev, cur),
        (StoredSnapshot::MacOs(prev), StoredSnapshot::MacOs(cur)) => compare_macos(prev, cur),
        (StoredSnapshot::Linux(prev), StoredSnapshot::Linux(cur)) => compare_linux(prev, cur),
        _ => ChangeState::Changes,
    }
}

fn compare_macos(previous: &MacSnapshot, current: &MacSnapshot) -> ChangeState {
    if previous.fsid != current.fsid {
        return ChangeState::Changes;
    }

    if previous.event_id == current.event_id {
        return ChangeState::NoChanges;
    }

    match (previous.root_mtime_epoch_ms, current.root_mtime_epoch_ms) {
        (Some(prev), Some(cur)) if prev == cur => ChangeState::NoChanges,
        _ => ChangeState::Changes,
    }
}

fn compare_linux(previous: &LinuxSnapshot, current: &LinuxSnapshot) -> ChangeState {
    if previous.device != current.device || previous.inode != current.inode {
        return ChangeState::Changes;
    }

    if previous.ctime_sec == current.ctime_sec && previous.ctime_nsec == current.ctime_nsec {
        return ChangeState::NoChanges;
    }

    match (previous.root_mtime_epoch_ms, current.root_mtime_epoch_ms) {
        (Some(prev), Some(cur)) if prev == cur => ChangeState::NoChanges,
        _ => ChangeState::Changes,
    }
}

fn compare_windows(previous: &WindowsSnapshot, current: &WindowsSnapshot) -> ChangeState {
    if previous.volume != current.volume {
        return ChangeState::Changes;
    }
    if previous.journal_id != current.journal_id {
        return ChangeState::Changes;
    }
    if previous.next_usn == current.next_usn {
        return ChangeState::NoChanges;
    }
    match (previous.root_mtime_epoch_ms, current.root_mtime_epoch_ms) {
        (Some(prev), Some(cur)) if prev == cur => ChangeState::NoChanges,
        _ => ChangeState::Changes,
    }
}

#[cfg(target_os = "macos")]
mod macos {
    use super::{system_time_to_epoch_ms, MacSnapshot};
    use eyre::{Context, Result};
    use fsevent_sys::FSEventsGetCurrentEventId;
    use std::ffi::CString;
    use std::mem::MaybeUninit;
    use std::os::unix::ffi::OsStrExt;
    use std::path::Path;

    pub(super) fn capture_snapshot(path: &Path) -> Result<Option<MacSnapshot>> {
        let metadata = std::fs::metadata(path)
            .with_context(|| format!("failed to stat {}", path.display()))?;

        let path_bytes = path.as_os_str().as_bytes();
        let c_path = CString::new(path_bytes)
            .map_err(|_| eyre::eyre!("path contains interior null bytes: {}", path.display()))?;

        let mut statfs_info = MaybeUninit::<libc::statfs>::uninit();
        let rc = unsafe { libc::statfs(c_path.as_ptr(), statfs_info.as_mut_ptr()) };
        if rc != 0 {
            return Err(std::io::Error::last_os_error())
                .with_context(|| format!("statfs {}", path.display()));
        }
        let statfs_info = unsafe { statfs_info.assume_init() };
        let fsid_parts: [i32; 2] = unsafe { std::mem::transmute(statfs_info.f_fsid) };
        let fsid_high = fsid_parts[0] as u64;
        let fsid_low = fsid_parts[1] as u64;
        let fsid = (fsid_high << 32) | (fsid_low & 0xffff_ffff);

        let event_id = unsafe { FSEventsGetCurrentEventId() };

        let root_mtime_epoch_ms = metadata
            .modified()
            .ok()
            .and_then(|st| system_time_to_epoch_ms(st).ok());

        Ok(Some(MacSnapshot {
            fsid,
            event_id,
            root_mtime_epoch_ms,
        }))
    }
}

#[cfg(all(unix, not(target_os = "macos")))]
mod linux {
    use super::{system_time_to_epoch_ms, LinuxSnapshot};
    use eyre::{Context, Result};
    use std::os::unix::fs::MetadataExt;
    use std::path::Path;

    pub(super) fn capture_snapshot(path: &Path) -> Result<Option<LinuxSnapshot>> {
        let metadata = std::fs::metadata(path)
            .with_context(|| format!("failed to stat {}", path.display()))?;

        let root_mtime_epoch_ms = metadata
            .modified()
            .ok()
            .and_then(|st| system_time_to_epoch_ms(st).ok());

        Ok(Some(LinuxSnapshot {
            device: metadata.dev(),
            inode: metadata.ino(),
            ctime_sec: metadata.ctime(),
            ctime_nsec: metadata.ctime_nsec(),
            root_mtime_epoch_ms,
        }))
    }
}

#[cfg(windows)]
mod windows {
    use super::{system_time_to_epoch_ms, WindowsSnapshot};
    use eyre::{Context, Result};
    use std::mem::MaybeUninit;
    use std::path::Path;
    use std::ptr::null_mut;
    use windows::Win32::Foundation::{CloseHandle, HANDLE};
    use windows::Win32::Storage::FileSystem::{
        CreateFileW, GetFileInformationByHandle, FILE_ACCESS_FLAGS, FILE_FLAG_BACKUP_SEMANTICS,
        FILE_FLAG_OPEN_REPARSE_POINT, FILE_SHARE_MODE, OPEN_EXISTING,
    };
    use windows::Win32::System::Ioctl::{
        DeviceIoControl, FSCTL_QUERY_USN_JOURNAL, USN_JOURNAL_DATA_V1,
    };

    pub(super) fn capture_snapshot(path: &Path) -> Result<Option<WindowsSnapshot>> {
        let metadata = std::fs::metadata(path)
            .with_context(|| format!("failed to stat {}", path.display()))?;
        let root_mtime_epoch_ms = metadata
            .modified()
            .ok()
            .and_then(|st| system_time_to_epoch_ms(st).ok());

        let wide_path = widestring::U16CString::from_os_str(path.as_os_str())
            .map_err(|_| eyre::eyre!("path contains interior null bytes: {}", path.display()))?;

        let handle = unsafe {
            CreateFileW(
                wide_path.as_ptr(),
                FILE_ACCESS_FLAGS::default(),
                FILE_SHARE_MODE::FILE_SHARE_READ
                    | FILE_SHARE_MODE::FILE_SHARE_WRITE
                    | FILE_SHARE_MODE::FILE_SHARE_DELETE,
                None,
                OPEN_EXISTING,
                FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT,
                None,
            )
        };
        if handle == HANDLE::default() {
            return Err(std::io::Error::last_os_error())
                .with_context(|| format!("CreateFileW {}", path.display()));
        }

        let mut volume_info = MaybeUninit::uninit();
        let info_rc = unsafe { GetFileInformationByHandle(handle, volume_info.as_mut_ptr()) };
        let volume_info = unsafe { volume_info.assume_init() };
        if info_rc.as_bool() == false {
            unsafe {
                CloseHandle(handle);
            }
            return Err(std::io::Error::last_os_error())
                .with_context(|| format!("GetFileInformationByHandle {}", path.display()));
        }

        let mut journal_info = MaybeUninit::<USN_JOURNAL_DATA_V1>::uninit();
        let mut bytes_returned = 0u32;
        let io_rc = unsafe {
            DeviceIoControl(
                handle,
                FSCTL_QUERY_USN_JOURNAL,
                None,
                0,
                journal_info.as_mut_ptr().cast(),
                std::mem::size_of::<USN_JOURNAL_DATA_V1>() as u32,
                &mut bytes_returned,
                None,
            )
        };
        unsafe {
            CloseHandle(handle);
        }
        if io_rc.as_bool() == false {
            return Ok(None);
        }
        let journal_info = unsafe { journal_info.assume_init() };

        let volume_id = format!(
            "{}:{}",
            volume_info.dwVolumeSerialNumber, volume_info.nFileIndexHigh
        );

        Ok(Some(WindowsSnapshot {
            volume: volume_id,
            journal_id: journal_info.UsnJournalID,
            next_usn: journal_info.NextUsn,
            root_mtime_epoch_ms,
        }))
    }
}
