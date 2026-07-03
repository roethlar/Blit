//! Local performance history writer for adaptive planning.
//!
//! Records summarized run information to a capped JSONL file under the user's
//! config directory. The data stays on-device and can be toggled via the CLI
//! (`blit diagnostics perf --enable/--disable`).

use crate::config;
use std::collections::VecDeque;
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use eyre::{Context, Result};
use serde::{Deserialize, Serialize};

const DEFAULT_MAX_BYTES: u64 = 1_000_000; // ~1 MiB cap per design docs
const SETTINGS_FILE: &str = "settings.json";

/// Current schema version for PerformanceRecord.
///
/// Bump this when making changes to the record format. Old records without a
/// version field deserialize as version 0 thanks to `#[serde(default)]`.
///
/// Version history:
///   0 - implicit (records written before versioning was added)
///   1 - added schema_version field
pub const CURRENT_SCHEMA_VERSION: u32 = 1;

/// High-level category of a transfer run.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum TransferMode {
    Copy,
    Mirror,
}

/// Snapshot of the options that influence performance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptionSnapshot {
    pub dry_run: bool,
    pub preserve_symlinks: bool,
    pub include_symlinks: bool,
    pub skip_unchanged: bool,
    pub checksum: bool,
    pub workers: usize,
}

/// Telemetry-free performance record captured after each run.
///
/// The `schema_version` field tracks the format version for migration support.
/// See [`CURRENT_SCHEMA_VERSION`] for the version history.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceRecord {
    #[serde(default)]
    pub schema_version: u32,
    pub timestamp_epoch_ms: u128,
    pub mode: TransferMode,
    pub source_fs: Option<String>,
    pub dest_fs: Option<String>,
    pub file_count: usize,
    pub total_bytes: u64,
    pub options: OptionSnapshot,
    pub fast_path: Option<String>,
    pub planner_duration_ms: u128,
    pub transfer_duration_ms: u128,
    pub stall_events: u32,
    pub error_count: u32,
    #[serde(default)]
    pub tar_shard_tasks: u32,
    #[serde(default)]
    pub tar_shard_files: u32,
    #[serde(default)]
    pub tar_shard_bytes: u64,
    #[serde(default)]
    pub raw_bundle_tasks: u32,
    #[serde(default)]
    pub raw_bundle_files: u32,
    #[serde(default)]
    pub raw_bundle_bytes: u64,
    #[serde(default)]
    pub large_tasks: u32,
    #[serde(default)]
    pub large_bytes: u64,
}

impl PerformanceRecord {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        mode: TransferMode,
        source_fs: Option<String>,
        dest_fs: Option<String>,
        file_count: usize,
        total_bytes: u64,
        options: OptionSnapshot,
        fast_path: Option<String>,
        planner_duration_ms: u128,
        transfer_duration_ms: u128,
        stall_events: u32,
        error_count: u32,
    ) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default();
        Self {
            schema_version: CURRENT_SCHEMA_VERSION,
            timestamp_epoch_ms: now.as_millis(),
            mode,
            source_fs,
            dest_fs,
            file_count,
            total_bytes,
            options,
            fast_path,
            planner_duration_ms,
            transfer_duration_ms,
            stall_events,
            error_count,
            tar_shard_tasks: 0,
            tar_shard_files: 0,
            tar_shard_bytes: 0,
            raw_bundle_tasks: 0,
            raw_bundle_files: 0,
            raw_bundle_bytes: 0,
            large_tasks: 0,
            large_bytes: 0,
        }
    }
}

/// Append a record to the local performance history store.
///
/// Errors are bubbled up so callers can decide whether to log or ignore them.
/// The function honours the persisted enable/disable flag; callers do not need
/// to perform a separate check.
pub fn append_local_record(record: &PerformanceRecord) -> Result<()> {
    if !perf_history_enabled()? {
        return Ok(());
    }

    let path = history_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| {
            format!(
                "failed to create performance history directory {}",
                parent.display()
            )
        })?;
    }

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .with_context(|| format!("failed to open performance history file {}", path.display()))?;

    let line = serde_json::to_string(record).context("serialize performance record")?;
    writeln!(file, "{line}").context("write performance record")?;
    drop(file);

    enforce_size_cap(&path, DEFAULT_MAX_BYTES)?;
    Ok(())
}

/// Migrate a record from an older schema version to the current version.
///
/// Returns the record with `schema_version` set to `CURRENT_SCHEMA_VERSION`.
/// Future migrations (e.g., field renames, type changes) should be added here
/// as version-gated transformations.
pub fn migrate_record(mut record: PerformanceRecord) -> PerformanceRecord {
    // Version 0 → 1: no field changes needed, just stamp the version.
    // Future migrations would go here, e.g.:
    //   if record.schema_version < 2 { /* transform fields for v2 */ }
    record.schema_version = CURRENT_SCHEMA_VERSION;
    record
}

pub fn read_recent_records(limit: usize) -> Result<Vec<PerformanceRecord>> {
    let path = history_path()?;
    read_records_from_path(&path, limit)
}

fn read_records_from_path(path: &Path, limit: usize) -> Result<Vec<PerformanceRecord>> {
    if !path.exists() {
        return Ok(Vec::new());
    }

    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let mut records = Vec::new();

    for line in reader.lines() {
        let Ok(line) = line else { continue };
        if line.trim().is_empty() {
            continue;
        }
        if let Ok(record) = serde_json::from_str::<PerformanceRecord>(&line) {
            records.push(migrate_record(record));
        }
    }

    if limit == 0 || records.len() <= limit {
        return Ok(records);
    }

    let start = records.len().saturating_sub(limit);
    Ok(records[start..].to_vec())
}

/// Rewrite the history file, migrating all records to the current schema version.
///
/// This is safe to call at any time. Records that fail to parse are dropped.
/// Returns the number of records migrated, or `Ok(0)` if the file doesn't exist.
pub fn migrate_history_file() -> Result<usize> {
    let path = history_path()?;
    if !path.exists() {
        return Ok(0);
    }

    let records = read_records_from_path(&path, 0)?;
    let count = records.len();

    let mut file = File::create(&path)
        .with_context(|| format!("rewriting history file {}", path.display()))?;
    for record in &records {
        let line = serde_json::to_string(record).context("serialize migrated record")?;
        writeln!(file, "{line}")?;
    }

    Ok(count)
}

pub fn config_dir() -> Result<PathBuf> {
    config::config_dir()
}

fn history_path() -> Result<PathBuf> {
    Ok(config_dir()?.join("perf_local.jsonl"))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Settings {
    #[serde(default = "default_perf_history_enabled")]
    perf_history_enabled: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            perf_history_enabled: true,
        }
    }
}

fn default_perf_history_enabled() -> bool {
    true
}

fn settings_path() -> Result<PathBuf> {
    Ok(config_dir()?.join(SETTINGS_FILE))
}

fn load_settings() -> Result<Settings> {
    let path = settings_path()?;
    if !path.exists() {
        return Ok(Settings::default());
    }

    let bytes = fs::read(&path)
        .with_context(|| format!("failed to read perf history settings {}", path.display()))?;
    if bytes.is_empty() {
        return Ok(Settings::default());
    }

    let settings: Settings =
        serde_json::from_slice(&bytes).context("failed to parse perf history settings JSON")?;
    Ok(settings)
}

fn store_settings(settings: &Settings) -> Result<()> {
    let path = settings_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create settings directory {}", parent.display()))?;
    }

    let mut file = File::create(&path)
        .with_context(|| format!("failed to write perf history settings {}", path.display()))?;
    let json =
        serde_json::to_vec_pretty(settings).context("failed to serialize perf history settings")?;
    file.write_all(&json)
        .context("failed to persist perf history settings")?;
    file.write_all(b"\n")?;
    Ok(())
}

/// Returns whether performance history is currently enabled.
pub fn perf_history_enabled() -> Result<bool> {
    Ok(load_settings()?.perf_history_enabled)
}

/// Persist the performance history enablement flag.
pub fn set_perf_history_enabled(enabled: bool) -> Result<()> {
    let mut settings = load_settings().unwrap_or_default();
    settings.perf_history_enabled = enabled;
    store_settings(&settings)
}

/// Remove the stored performance history file. Returns `Ok(true)` if the file
/// was removed, `Ok(false)` if it did not exist.
pub fn clear_history() -> Result<bool> {
    let path = history_path()?;
    match fs::remove_file(&path) {
        Ok(_) => Ok(true),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(err) => Err(err.into()),
    }
}

/// Best-effort rotation that prefers keeping the newest records over enforcing the cap exactly.
/// If a concurrent writer appends while we're trimming, we skip rotation to avoid data loss.
fn enforce_size_cap(path: &Path, max_bytes: u64) -> Result<()> {
    let metadata = match fs::metadata(path) {
        Ok(meta) => meta,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(err) => return Err(err.into()),
    };

    if metadata.len() <= max_bytes {
        return Ok(());
    }

    // Capture the size we observed so we can detect concurrent appends.
    let observed_len = metadata.len();

    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let mut lines: VecDeque<String> = reader
        .lines()
        .collect::<std::result::Result<Vec<String>, _>>()
        .context("read performance history for rotation")?
        .into_iter()
        .filter(|line| !line.trim().is_empty())
        .collect();

    if lines.is_empty() {
        return Ok(());
    }

    let mut total_size: usize = lines.iter().map(|l| l.len() + 1).sum();
    while total_size as u64 > max_bytes {
        if lines.pop_front().is_none() {
            break;
        }
        total_size = lines.iter().map(|l| l.len() + 1).sum();
    }

    // Re-read metadata to ensure nothing appended during trimming.
    if fs::metadata(path).map(|m| m.len()).unwrap_or(observed_len) != observed_len {
        return Ok(());
    }

    let mut file = File::create(path)?;
    for line in lines {
        writeln!(file, "{line}")?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_v0_json() -> &'static str {
        // A record without schema_version (pre-versioning format)
        r#"{"timestamp_epoch_ms":1700000000000,"mode":"copy","source_fs":null,"dest_fs":null,"file_count":10,"total_bytes":1024,"options":{"dry_run":false,"preserve_symlinks":true,"include_symlinks":false,"skip_unchanged":true,"checksum":false,"workers":4},"fast_path":null,"planner_duration_ms":50,"transfer_duration_ms":200,"stall_events":0,"error_count":0}"#
    }

    fn sample_v1_json() -> &'static str {
        r#"{"schema_version":1,"timestamp_epoch_ms":1700000000000,"mode":"mirror","source_fs":"apfs","dest_fs":"apfs","file_count":5,"total_bytes":512,"options":{"dry_run":false,"preserve_symlinks":true,"include_symlinks":false,"skip_unchanged":false,"checksum":true,"workers":2},"fast_path":"tiny","planner_duration_ms":10,"transfer_duration_ms":100,"stall_events":0,"error_count":0,"tar_shard_tasks":1,"tar_shard_files":5,"tar_shard_bytes":512,"raw_bundle_tasks":0,"raw_bundle_files":0,"raw_bundle_bytes":0,"large_tasks":0,"large_bytes":0}"#
    }

    #[test]
    fn v0_record_deserializes_with_defaults() {
        let record: PerformanceRecord =
            serde_json::from_str(sample_v0_json()).expect("deserialize v0");
        assert_eq!(record.schema_version, 0);
        assert_eq!(record.tar_shard_tasks, 0);
        assert_eq!(record.file_count, 10);
    }

    #[test]
    fn v1_record_deserializes_fully() {
        let record: PerformanceRecord =
            serde_json::from_str(sample_v1_json()).expect("deserialize v1");
        assert_eq!(record.schema_version, 1);
        assert_eq!(record.tar_shard_files, 5);
        assert_eq!(record.mode, TransferMode::Mirror);
    }

    #[test]
    fn migrate_record_stamps_current_version() {
        let old: PerformanceRecord =
            serde_json::from_str(sample_v0_json()).expect("deserialize v0");
        assert_eq!(old.schema_version, 0);

        let migrated = migrate_record(old.clone());
        assert_eq!(migrated.schema_version, CURRENT_SCHEMA_VERSION);
        // Data preserved
        assert_eq!(migrated.file_count, old.file_count);
        assert_eq!(migrated.total_bytes, old.total_bytes);
    }

    #[test]
    fn read_records_migrates_on_load() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("test_history.jsonl");

        // Write a mix of v0 and v1 records
        let mut file = File::create(&path).expect("create");
        writeln!(file, "{}", sample_v0_json()).expect("write v0");
        writeln!(file, "{}", sample_v1_json()).expect("write v1");
        drop(file);

        let records = read_records_from_path(&path, 0).expect("read");
        assert_eq!(records.len(), 2);
        // Both should be migrated to current version
        assert_eq!(records[0].schema_version, CURRENT_SCHEMA_VERSION);
        assert_eq!(records[1].schema_version, CURRENT_SCHEMA_VERSION);
        // Original data intact
        assert_eq!(records[0].mode, TransferMode::Copy);
        assert_eq!(records[1].mode, TransferMode::Mirror);
    }

    #[test]
    fn read_records_skips_invalid_lines() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("test_history.jsonl");

        let mut file = File::create(&path).expect("create");
        writeln!(file, "{}", sample_v0_json()).expect("write v0");
        writeln!(file, "{{not valid json}}").expect("write garbage");
        writeln!(file, "").expect("write empty");
        writeln!(file, "{}", sample_v1_json()).expect("write v1");
        drop(file);

        let records = read_records_from_path(&path, 0).expect("read");
        assert_eq!(records.len(), 2, "should skip invalid/empty lines");
    }

    #[test]
    fn new_record_has_current_version() {
        let options = OptionSnapshot {
            dry_run: false,
            preserve_symlinks: true,
            include_symlinks: false,
            skip_unchanged: true,
            checksum: false,
            workers: 4,
        };
        let record = PerformanceRecord::new(
            TransferMode::Copy,
            None,
            None,
            1,
            100,
            options,
            None,
            10,
            20,
            0,
            0,
        );
        assert_eq!(record.schema_version, CURRENT_SCHEMA_VERSION);
    }

    #[test]
    fn read_records_respects_limit() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("test_history.jsonl");

        let mut file = File::create(&path).expect("create");
        for _ in 0..5 {
            writeln!(file, "{}", sample_v0_json()).expect("write");
        }
        drop(file);

        let records = read_records_from_path(&path, 2).expect("read");
        assert_eq!(records.len(), 2, "should return only the last 2 records");
    }
}
