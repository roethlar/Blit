use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

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
pub(super) struct StoredRecord {
    pub snapshot: StoredSnapshot,
    pub recorded_at_epoch_ms: u128,
}

#[derive(Debug, Default)]
pub struct ChangeTracker {
    pub(super) path: PathBuf,
    pub(super) records: HashMap<String, StoredRecord>,
}
