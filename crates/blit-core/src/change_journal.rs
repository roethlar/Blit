use crate::perf_history::config_dir;
#[cfg(windows)]
use eyre::eyre;
use eyre::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
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
    pub marker: Option<PlatformMarker>,
    pub state: ChangeState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "platform", rename_all = "snake_case")]
pub enum PlatformMarker {
    Windows {
        volume: String,
        journal_id: u64,
        next_usn: i64,
        root_mtime_epoch_ms: Option<i128>,
    },
    MacOs {
        fsid: u64,
        event_id: u64,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct JournalRecord {
    marker: PlatformMarker,
    recorded_at_epoch_ms: u128,
}

pub struct ChangeTracker {
    path: PathBuf,
    records: HashMap<String, JournalRecord>,
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
        let records: HashMap<String, JournalRecord> = serde_json::from_reader(file)
            .with_context(|| format!("failed to parse journal cache {}", path.display()))?;

        Ok(Self { path, records })
    }

    pub fn probe(&self, root: &Path) -> Result<ProbeToken> {
        let canonical = canonicalize(root)?;
        let key = canonical_to_key(&canonical);
        let stored = self.records.get(&key);
        let marker = capture_marker(&canonical)?;

        let state = match (&marker, stored) {
            (None, _) => ChangeState::Unsupported,
            (Some(_), None) => ChangeState::Unknown,
            (Some(current), Some(record)) => match (current, &record.marker) {
                (PlatformMarker::Windows { .. }, PlatformMarker::Windows { .. }) => {
                    compare_windows(current, &record.marker)
                }
                (PlatformMarker::MacOs { .. }, PlatformMarker::MacOs { .. }) => {
                    compare_macos(current, &record.marker)
                }
                _ => ChangeState::Changes,
            },
        };

        Ok(ProbeToken {
            key,
            canonical_path: canonical,
            marker,
            state,
        })
    }

    pub fn refresh_and_persist(&mut self, tokens: &[ProbeToken]) -> Result<()> {
        if tokens.is_empty() {
            return Ok(());
        }

        let mut changed = false;
        for token in tokens {
            let marker = capture_marker(&token.canonical_path)?;
            match marker {
                Some(marker) => {
                    let record = JournalRecord {
                        marker,
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
}

fn journal_store_path() -> Result<PathBuf> {
    Ok(config_dir()?.join("journal_cache.json"))
}

fn canonicalize(path: &Path) -> Result<PathBuf> {
    #[cfg(windows)]
    {
        let norm = normpath::BasePath::new(std::env::current_dir()?)
            .map_err(|err| eyre!("failed to resolve base path for canonicalisation: {}", err))?;
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

#[cfg(windows)]
fn system_time_to_epoch_ms(st: SystemTime) -> Result<i128> {
    let duration = st
        .duration_since(UNIX_EPOCH)
        .map_err(|err| eyre!("system time before epoch: {err}"))?;
    Ok(duration.as_millis() as i128)
}

fn compare_windows(current: &PlatformMarker, previous: &PlatformMarker) -> ChangeState {
    match (current, previous) {
        (
            PlatformMarker::Windows {
                volume: cur_vol,
                journal_id: cur_journal,
                root_mtime_epoch_ms: cur_mtime,
                ..
            },
            PlatformMarker::Windows {
                volume: prev_vol,
                journal_id: prev_journal,
                root_mtime_epoch_ms: prev_mtime,
                ..
            },
        ) => {
            if cur_vol != prev_vol || cur_journal != prev_journal {
                ChangeState::Changes
            } else if cur_mtime.is_some() && prev_mtime.is_some() && cur_mtime == prev_mtime {
                ChangeState::NoChanges
            } else {
                ChangeState::Changes
            }
        }
        _ => ChangeState::Changes,
    }
}

fn compare_macos(current: &PlatformMarker, previous: &PlatformMarker) -> ChangeState {
    match (current, previous) {
        (
            PlatformMarker::MacOs {
                fsid: cur_fsid,
                event_id: cur_event,
            },
            PlatformMarker::MacOs {
                fsid: prev_fsid,
                event_id: prev_event,
            },
        ) => {
            if cur_fsid != prev_fsid {
                ChangeState::Changes
            } else if cur_event == prev_event {
                ChangeState::NoChanges
            } else {
                ChangeState::Changes
            }
        }
        _ => ChangeState::Changes,
    }
}

fn capture_marker(path: &Path) -> Result<Option<PlatformMarker>> {
    #[cfg(windows)]
    {
        return capture_windows_marker(path);
    }

    #[cfg(target_os = "macos")]
    {
        return capture_macos_marker(path);
    }

    #[cfg(not(any(windows, target_os = "macos")))]
    {
        let _ = path;
        Ok(None)
    }
}

#[cfg(windows)]
fn capture_windows_marker(path: &Path) -> Result<Option<PlatformMarker>> {
    use std::ffi::OsString;
    use std::mem::size_of;
    use std::os::windows::ffi::OsStrExt;
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::{CloseHandle, HANDLE};
    use windows::Win32::Foundation::{
        ERROR_ACCESS_DENIED, ERROR_INVALID_FUNCTION, ERROR_JOURNAL_DELETE_IN_PROGRESS,
        ERROR_JOURNAL_NOT_ACTIVE,
    };
    use windows::Win32::Storage::FileSystem::{
        CreateFileW, GetVolumePathNameW, FILE_ATTRIBUTE_NORMAL, FILE_GENERIC_READ,
        FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
    };
    use windows::Win32::System::Ioctl::{FSCTL_QUERY_USN_JOURNAL, USN_JOURNAL_DATA_V1};
    use windows::Win32::System::IO::DeviceIoControl;

    let wide_path: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();

    let mut buffer = vec![0u16; 512];
    if let Err(err) = unsafe { GetVolumePathNameW(PCWSTR(wide_path.as_ptr()), &mut buffer) } {
        let code = err.code();
        if is_usn_unsupported(code) {
            return Ok(None);
        }
        return Err(err).context("failed to resolve volume path for USN query");
    }

    let volume_len = buffer.iter().position(|&c| c == 0).unwrap_or(buffer.len());
    let volume_str =
        String::from_utf16(&buffer[..volume_len]).context("failed to decode volume path")?;

    let trimmed = volume_str.trim_end_matches(['\\', '/']).to_string();
    let (device_path, stored_volume) = if trimmed.starts_with(r"\\?\Volume") {
        (trimmed.clone(), trimmed.clone())
    } else {
        let drive = if trimmed.ends_with(':') {
            trimmed.clone()
        } else {
            format!("{}:", trimmed)
        };
        (format!(r"\\.\{}", drive), drive)
    };
    let device_wide: Vec<u16> = OsString::from(&device_path)
        .encode_wide()
        .chain(Some(0))
        .collect();

    let handle = unsafe {
        CreateFileW(
            PCWSTR(device_wide.as_ptr()),
            FILE_GENERIC_READ.0,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            None,
            OPEN_EXISTING,
            FILE_ATTRIBUTE_NORMAL,
            HANDLE::default(),
        )
    };

    let handle = match handle {
        Ok(handle) => handle,
        Err(err) => {
            let code = err.code();
            if is_usn_unsupported(code) {
                return Ok(None);
            }
            return Err(err).context("failed to open volume for USN query");
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

    match result {
        Ok(()) => {}
        Err(err) => {
            let code = err.code();
            if is_usn_unsupported(code) {
                return Ok(None);
            }
            return Err(err).context("failed to query USN journal");
        }
    }

    let root_mtime_epoch_ms = match std::fs::metadata(path) {
        Ok(md) => md
            .modified()
            .ok()
            .and_then(|st| system_time_to_epoch_ms(st).ok()),
        Err(_) => None,
    };

    Ok(Some(PlatformMarker::Windows {
        volume: stored_volume,
        journal_id: data.UsnJournalID,
        next_usn: data.NextUsn,
        root_mtime_epoch_ms,
    }))
}

#[cfg(windows)]
fn is_usn_unsupported(code: windows::core::HRESULT) -> bool {
    use windows::Win32::Foundation::{
        ERROR_ACCESS_DENIED, ERROR_INVALID_FUNCTION, ERROR_JOURNAL_DELETE_IN_PROGRESS,
        ERROR_JOURNAL_NOT_ACTIVE,
    };

    let raw = code.0 as u32;
    raw == ERROR_INVALID_FUNCTION.0
        || raw == ERROR_JOURNAL_NOT_ACTIVE.0
        || raw == ERROR_JOURNAL_DELETE_IN_PROGRESS.0
        || raw == ERROR_ACCESS_DENIED.0
}

#[cfg(target_os = "macos")]
fn capture_macos_marker(path: &Path) -> Result<Option<PlatformMarker>> {
    use fsevent_sys::FSEventsGetCurrentEventId;
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;

    let c_path =
        CString::new(path.as_os_str().as_bytes()).context("failed to convert path for statfs")?;
    let mut stat: libc::statfs = unsafe { std::mem::zeroed() };
    let rc = unsafe { libc::statfs(c_path.as_ptr(), &mut stat) };
    if rc != 0 {
        return Ok(None);
    }

    let fsid_parts: [i32; 2] = unsafe { std::mem::transmute(stat.f_fsid) };
    let fsid = (fsid_parts[0] as u32 as u64) | ((fsid_parts[1] as u32 as u64) << 32);
    let event_id = unsafe { FSEventsGetCurrentEventId() };
    Ok(Some(PlatformMarker::MacOs { fsid, event_id }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn compare_windows_detects_changes() {
        let previous = PlatformMarker::Windows {
            volume: "C:".into(),
            journal_id: 42,
            next_usn: 100,
            root_mtime_epoch_ms: Some(1234),
        };
        let same = PlatformMarker::Windows {
            volume: "C:".into(),
            journal_id: 42,
            next_usn: 100,
            root_mtime_epoch_ms: Some(1234),
        };
        let advanced = PlatformMarker::Windows {
            volume: "C:".into(),
            journal_id: 42,
            next_usn: 200,
            root_mtime_epoch_ms: Some(5678),
        };

        assert!(matches!(
            compare_windows(&same, &previous),
            ChangeState::NoChanges
        ));
        assert!(matches!(
            compare_windows(&advanced, &previous),
            ChangeState::Changes
        ));
    }

    #[test]
    fn compare_macos_detects_changes() {
        let previous = PlatformMarker::MacOs {
            fsid: 7,
            event_id: 10,
        };
        let same = PlatformMarker::MacOs {
            fsid: 7,
            event_id: 10,
        };
        let advanced = PlatformMarker::MacOs {
            fsid: 7,
            event_id: 11,
        };
        let different_fs = PlatformMarker::MacOs {
            fsid: 8,
            event_id: 10,
        };

        assert!(matches!(
            compare_macos(&same, &previous),
            ChangeState::NoChanges
        ));
        assert!(matches!(
            compare_macos(&advanced, &previous),
            ChangeState::Changes
        ));
        assert!(matches!(
            compare_macos(&different_fs, &previous),
            ChangeState::Changes
        ));
    }

    #[test]
    fn probe_without_marker_reports_unsupported() -> Result<()> {
        let temp_config = tempfile::tempdir()?;
        let original = env::var_os("BLIT_CONFIG_DIR");
        env::set_var("BLIT_CONFIG_DIR", temp_config.path());

        let tracker = ChangeTracker::load()?;
        let temp_root = tempfile::tempdir()?;
        let token = tracker.probe(temp_root.path())?;
        assert!(matches!(token.state, ChangeState::Unsupported));

        if let Some(orig) = original {
            env::set_var("BLIT_CONFIG_DIR", orig);
        } else {
            env::remove_var("BLIT_CONFIG_DIR");
        }

        Ok(())
    }
}
