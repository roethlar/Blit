use crate::config::config_dir;
#[cfg(windows)]
use eyre::eyre;
use eyre::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::{BufReader, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeState {
    Unsupported,
    Unknown,
    NoChanges,
    Changes,
}

#[derive(Debug, Clone)]
pub struct ProbeToken {
    pub key: String,
    pub canonical_path: PathBuf,
    pub snapshot: Option<StoredSnapshot>,
    pub state: ChangeState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "backend", rename_all = "snake_case")]
pub enum StoredSnapshot {
    Windows(WindowsSnapshot),
    MacOs(MacSnapshot),
    Linux(LinuxSnapshot),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowsSnapshot {
    pub volume: String,
    pub journal_id: u64,
    pub next_usn: i64,
    pub root_mtime_epoch_ms: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MacSnapshot {
    pub fsid: u64,
    pub event_id: u64,
    pub root_mtime_epoch_ms: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LinuxSnapshot {
    pub device: u64,
    pub inode: u64,
    pub ctime_sec: i64,
    pub ctime_nsec: i64,
    pub root_mtime_epoch_ms: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredRecord {
    snapshot: StoredSnapshot,
    recorded_at_epoch_ms: u128,
}

#[derive(Debug, Default)]
pub struct ChangeTracker {
    path: PathBuf,
    records: HashMap<String, StoredRecord>,
}

impl ChangeTracker {
    pub fn load() -> Result<Self> {
        let path = journal_store_path()?;
        if !path.exists() {
            return Ok(Self {
                path,
                records: HashMap::new(),
            });
        }

        let file = File::open(&path)
            .with_context(|| format!("failed to open journal cache {}", path.display()))?;
        let reader = BufReader::new(file);

        let records: HashMap<String, StoredRecord> = match serde_json::from_reader(reader) {
            Ok(records) => records,
            Err(err) => {
                eprintln!(
                    "change_journal: failed to parse journal cache {} ({err}); starting fresh",
                    path.display()
                );
                HashMap::new()
            }
        };

        Ok(Self { path, records })
    }

    pub fn probe(&self, root: &Path) -> Result<ProbeToken> {
        let canonical = canonicalize(root)?;
        let key = canonical_to_key(&canonical);
        let new_snapshot = capture_snapshot(&canonical)?;
        let state = match (&new_snapshot, self.records.get(&key)) {
            (None, _) => ChangeState::Unsupported,
            (Some(_), None) => ChangeState::Unknown,
            (Some(new), Some(stored)) => compare_snapshots(&stored.snapshot, new),
        };

        Ok(ProbeToken {
            key,
            canonical_path: canonical,
            snapshot: new_snapshot,
            state,
        })
    }

    pub fn refresh_and_persist(&mut self, tokens: &[ProbeToken]) -> Result<()> {
        let mut changed = false;

        for token in tokens {
            match &token.snapshot {
                Some(snapshot) => {
                    let record = StoredRecord {
                        snapshot: snapshot.clone(),
                        recorded_at_epoch_ms: now_ms(),
                    };
                    self.records.insert(token.key.clone(), record);
                    changed = true;
                }
                None => {
                    if self.records.remove(&token.key).is_some() {
                        changed = true;
                    }
                }
            }
        }

        if changed {
            self.persist()?;
        }

        Ok(())
    }

    fn persist(&self) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!(
                    "failed to create journal cache directory {}",
                    parent.display()
                )
            })?;
        }

        let mut file = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(&self.path)
            .with_context(|| {
                format!(
                    "failed to open journal cache {} for write",
                    self.path.display()
                )
            })?;

        serde_json::to_writer_pretty(&mut file, &self.records)
            .with_context(|| format!("failed to write journal cache {}", self.path.display()))?;
        file.flush()
            .with_context(|| format!("failed to flush journal cache {}", self.path.display()))?;
        Ok(())
    }

    pub fn reprobe_canonical(&self, canonical_path: &Path) -> Result<Option<StoredSnapshot>> {
        capture_snapshot(canonical_path)
    }
}

fn compare_snapshots(previous: &StoredSnapshot, current: &StoredSnapshot) -> ChangeState {
    match (previous, current) {
        (StoredSnapshot::Windows(prev), StoredSnapshot::Windows(cur)) => compare_windows(prev, cur),
        (StoredSnapshot::MacOs(prev), StoredSnapshot::MacOs(cur)) => compare_macos(prev, cur),
        (StoredSnapshot::Linux(prev), StoredSnapshot::Linux(cur)) => compare_linux(prev, cur),
        _ => ChangeState::Changes,
    }
}

fn compare_windows(previous: &WindowsSnapshot, current: &WindowsSnapshot) -> ChangeState {
    if previous.volume != current.volume || previous.journal_id != current.journal_id {
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

fn capture_snapshot(path: &Path) -> Result<Option<StoredSnapshot>> {
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

fn journal_store_path() -> Result<PathBuf> {
    Ok(config_dir()?.join("journal_cache.json"))
}

fn canonicalize(path: &Path) -> Result<PathBuf> {
    #[cfg(windows)]
    {
        let norm = normpath::BasePath::new(std::env::current_dir()?)
            .map_err(|err| eyre!("failed to resolve base path for canonicalisation: {err}"))?;
        let joined = norm.join(path);
        return Ok(joined.into_path_buf());
    }

    #[cfg(not(windows))]
    {
        std::fs::canonicalize(path)
            .with_context(|| format!("failed to canonicalize path {}", path.display()))
    }
}

fn canonical_to_key(path: &Path) -> String {
    path.to_string_lossy().to_string()
}

fn now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

fn system_time_to_epoch_ms(st: SystemTime) -> Result<i64> {
    let duration = st
        .duration_since(UNIX_EPOCH)
        .map_err(|err| eyre::eyre!("system time before epoch: {err}"))?;
    let millis = duration.as_millis();
    let millis_i64 = i64::try_from(millis)
        .map_err(|_| eyre::eyre!("system time milliseconds exceed i64 range"))?;
    Ok(millis_i64)
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
    use eyre::{eyre, Context, Result};
    use std::ffi::OsString;
    use std::mem::size_of;
    use std::os::windows::ffi::OsStrExt;
    use std::path::Path;
    use windows::core::{Error as WinError, PCWSTR, PWSTR};
    use windows::Win32::Foundation::{CloseHandle, HANDLE};
    use windows::Win32::Foundation::{
        ERROR_ACCESS_DENIED, ERROR_FILE_NOT_FOUND, ERROR_INVALID_FUNCTION,
        ERROR_JOURNAL_DELETE_IN_PROGRESS, ERROR_JOURNAL_NOT_ACTIVE,
    };
    use windows::Win32::Storage::FileSystem::{
        CreateFileW, GetVolumeNameForVolumeMountPointW, GetVolumePathNameW,
        FILE_FLAG_BACKUP_SEMANTICS, FILE_GENERIC_READ, FILE_SHARE_DELETE, FILE_SHARE_READ,
        FILE_SHARE_WRITE, OPEN_EXISTING,
    };
    use windows::Win32::System::Ioctl::{FSCTL_QUERY_USN_JOURNAL, USN_JOURNAL_DATA_V1};
    use windows::Win32::System::IO::DeviceIoControl;

    pub(super) fn capture_snapshot(path: &Path) -> Result<Option<WindowsSnapshot>> {
        let metadata = match std::fs::metadata(path) {
            Ok(md) => md,
            Err(err) => {
                return Err(err).with_context(|| format!("stat {}", path.display()));
            }
        };

        // Determine the volume path (e.g. C:\)
        let wide_path: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
        let mut volume_buffer = vec![0u16; 512];
        unsafe { GetVolumePathNameW(PCWSTR(wide_path.as_ptr()), &mut volume_buffer) }
            .map_err(|err| eyre!("GetVolumePathNameW failed: {err}"))?;
        let volume_len = volume_buffer
            .iter()
            .position(|&c| c == 0)
            .unwrap_or(volume_buffer.len());
        let volume = String::from_utf16(&volume_buffer[..volume_len])
            .context("failed to decode volume path")?;

        let stored_volume = volume.trim_end_matches(['\\', '/']).to_string();
        let mut candidates = Vec::new();

        // First try the drive letter paths (e.g. C:, C:\)
        if !stored_volume.is_empty() {
            candidates.push(format!("{stored_volume}:"));
            candidates.push(format!("{stored_volume}:\\"));
        }

        // Also gather the volume GUID path.
        let mut volume_guid = [0u16; 512];
        let volume_wide: Vec<u16> = OsString::from(&volume)
            .encode_wide()
            .chain(Some(0))
            .collect();
        if unsafe {
            GetVolumeNameForVolumeMountPointW(PCWSTR(volume_wide.as_ptr()), &mut volume_guid)
        }
        .is_ok()
        {
            if let Ok(full_guid) = String::from_utf16(
                &volume_guid[..volume_guid
                    .iter()
                    .position(|&c| c == 0)
                    .unwrap_or(volume_guid.len())],
            ) {
                let mut maybe_push = |path: String| {
                    if !path.is_empty() && !candidates.iter().any(|c| c.eq_ignore_ascii_case(&path))
                    {
                        candidates.push(path);
                    }
                };

                maybe_push(full_guid.clone()); // e.g. \\?\Volume{GUID}\
                maybe_push(full_guid.trim_end_matches('\\').to_string()); // e.g. \\?\Volume{GUID}
            }
        }

        let mut attempts = Vec::new();
        let mut last_err: Option<(String, WinError)> = None;
        let mut handle = None;
        for candidate in candidates {
            attempts.push(candidate.clone());
            let candidate_wide: Vec<u16> = OsString::from(&candidate)
                .encode_wide()
                .chain(Some(0))
                .collect();
            let attempt = unsafe {
                CreateFileW(
                    PCWSTR(candidate_wide.as_ptr()),
                    FILE_GENERIC_READ.0,
                    FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
                    None,
                    OPEN_EXISTING,
                    FILE_FLAG_BACKUP_SEMANTICS,
                    HANDLE::default(),
                )
            };
            match attempt {
                Ok(h) => {
                    handle = Some(h);
                    break;
                }
                Err(err) => {
                    if is_usn_unsupported(err.code().0 as u32) {
                        return Ok(None);
                    }
                    last_err = Some((candidate.clone(), err));
                }
            }
        }

        let handle = match handle {
            Some(handle) => handle,
            None => {
                if let Some((path, err)) = last_err {
                    eprintln!(
                        "change_journal: failed to open USN volume; attempts={:?}; last_path={path}; last_err={err}",
                        attempts
                    );
                    return Err(err).with_context(|| {
                        format!("failed to open volume for USN query (path tried: {path})")
                    });
                }
                if !attempts.is_empty() {
                    eprintln!(
                        "change_journal: failed to open USN volume; attempts={:?}; no last error recorded",
                        attempts
                    );
                }
                return Ok(None);
            }
        };

        let mut data = USN_JOURNAL_DATA_V1::default();
        let mut bytes = 0u32;
        let result = unsafe {
            DeviceIoControl(
                handle,
                FSCTL_QUERY_USN_JOURNAL,
                None,
                0,
                Some((&mut data as *mut USN_JOURNAL_DATA_V1).cast()),
                size_of::<USN_JOURNAL_DATA_V1>() as u32,
                Some(&mut bytes),
                None,
            )
        };

        unsafe {
            CloseHandle(handle);
        }

        result.map_err(|err| {
            if is_usn_unsupported(err.code().0 as u32) {
                eyre!("USN journal not active")
            } else {
                eyre!("DeviceIoControl(FSCTL_QUERY_USN_JOURNAL) failed: {err}")
            }
        })?;

        let root_mtime_epoch_ms = metadata
            .modified()
            .ok()
            .and_then(|st| system_time_to_epoch_ms(st).ok());

        Ok(Some(WindowsSnapshot {
            volume: stored_volume,
            journal_id: data.UsnJournalID,
            next_usn: data.NextUsn,
            root_mtime_epoch_ms,
        }))
    }

    fn is_usn_unsupported(code: u32) -> bool {
        code == ERROR_INVALID_FUNCTION.0
            || code == ERROR_JOURNAL_NOT_ACTIVE.0
            || code == ERROR_JOURNAL_DELETE_IN_PROGRESS.0
            || code == ERROR_ACCESS_DENIED.0
            || code == ERROR_FILE_NOT_FOUND.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compare_windows_no_changes_when_mtime_equal() {
        let prev = WindowsSnapshot {
            volume: "C:".into(),
            journal_id: 42,
            next_usn: 1_000,
            root_mtime_epoch_ms: Some(1_234),
        };
        let cur = WindowsSnapshot {
            volume: "C:".into(),
            journal_id: 42,
            next_usn: 2_000,
            root_mtime_epoch_ms: Some(1_234),
        };
        assert!(matches!(
            compare_windows(&prev, &cur),
            ChangeState::NoChanges
        ));
    }

    #[test]
    fn compare_windows_detects_changes_on_volume_swap() {
        let prev = WindowsSnapshot {
            volume: "C:".into(),
            journal_id: 42,
            next_usn: 1_000,
            root_mtime_epoch_ms: Some(1_234),
        };
        let cur = WindowsSnapshot {
            volume: "D:".into(),
            journal_id: 42,
            next_usn: 1_000,
            root_mtime_epoch_ms: Some(1_234),
        };
        assert!(matches!(compare_windows(&prev, &cur), ChangeState::Changes));
    }

    #[test]
    fn compare_macos_detects_changes() {
        let prev = MacSnapshot {
            fsid: 7,
            event_id: 10,
            root_mtime_epoch_ms: Some(1234),
        };
        let cur = MacSnapshot {
            fsid: 7,
            event_id: 11,
            root_mtime_epoch_ms: Some(5678),
        };
        assert!(matches!(compare_macos(&prev, &cur), ChangeState::Changes));
    }
}
