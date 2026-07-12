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
///   2 - added `run_kind` to separate measurement lanes (real transfer
///       vs dry-run vs null-sink vs bench). Pre-v2 records carry their
///       lane implicitly in `options.dry_run` and
///       `fast_path == Some("null_sink")`; migration derives `run_kind`
///       from those without touching `mode`. R56-F1.
pub const CURRENT_SCHEMA_VERSION: u32 = 2;

/// High-level category of a transfer run (intent-side).
///
/// `mode` answers "what was the operator asking for?" — copy or mirror.
/// Orthogonal to `RunKind`, which answers "what kind of measurement is
/// this record?" — a real transfer, a dry-run, a null-sink benchmark,
/// etc. A `(mode=Mirror, run_kind=DryRun)` record means the user asked
/// for a mirror operation but routed it through the dry-run path; that
/// record should NOT teach the predictor anything about real-mirror
/// transfer cost (no writes happened).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum TransferMode {
    Copy,
    Mirror,
}

/// Measurement lane for a [`PerformanceRecord`]. Determines whether
/// the record is eligible to feed real-transfer aggregates. R56-F1
/// (historical, engine era): dry-run and null-sink records taught the
/// since-retired tuner that destination writes were free; filtering by
/// `run_kind == Real` is the single chokepoint that closes that class
/// of contamination for any consumer (`blit profile` today).
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum RunKind {
    /// Normal production transfer. Eligible for predictor training and
    /// auto-tune aggregates.
    #[default]
    Real,
    /// `--dry-run`: plan-and-stop, no writes happened. Useful for
    /// debugging but not representative of real transfer cost.
    DryRun,
    /// `--null` / null-sink benchmark: pipeline ran, destination
    /// writes discarded. Useful for diagnostics but writes were zero
    /// cost.
    NullSink,
    /// `blit bench transfer` (planned 0.2.0 verb): real source reads,
    /// null destination. Separate predictor lane.
    BenchTransfer,
    /// `blit bench wire` (planned 0.2.0 verb): synthetic source,
    /// null destination. Pure data-plane measurement.
    BenchWire,
}

impl RunKind {
    /// True iff the record is a "real transfer" — eligible to feed
    /// the predictor's real-transfer profile and the local auto-tune
    /// bucket aggregates. R56-F1: every consumer of historical
    /// records that drives production behavior MUST filter on this
    /// before consulting per-record fields.
    pub fn is_real_transfer(&self) -> bool {
        matches!(self, RunKind::Real)
    }
}

/// Comparison policy snapshot for performance history. Distinct
/// from `generated::ComparisonMode` (proto enum) because the perf
/// history file is JSONL and shouldn't depend on the generated
/// proto serialization surface. R59 finding #5: pre-fix the
/// tuning window keyed on `checksum: bool` alone, mixing
/// SizeMtime / SizeOnly / Force / IgnoreTimes records into the
/// same bucket.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum CompareModeSnapshot {
    #[default]
    SizeMtime,
    Checksum,
    SizeOnly,
    Force,
    IgnoreTimes,
}

/// Snapshot of the options that influence performance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptionSnapshot {
    pub dry_run: bool,
    pub preserve_symlinks: bool,
    pub include_symlinks: bool,
    pub skip_unchanged: bool,
    /// Legacy boolean — kept for back-compat with pre-R59
    /// history records. New records also set `compare_mode` to
    /// preserve the user's intent across the four non-default
    /// comparison policies. Tuning window selection should key
    /// on `compare_mode`; this bool stays as the legacy fallback.
    pub checksum: bool,
    /// R59 finding #5: full comparison policy. `serde(default)`
    /// so old records (which lack this field) deserialize as
    /// `SizeMtime`, which is the historical default behavior.
    #[serde(default)]
    pub compare_mode: CompareModeSnapshot,
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
    /// R56-F1: measurement lane. Pre-v2 records omit this; the
    /// migration derives it from `options.dry_run` and
    /// `fast_path == Some("null_sink")`. Filtering on
    /// `run_kind.is_real_transfer()` is the single chokepoint
    /// that keeps dry-run / null-sink / bench records out of
    /// production training data.
    #[serde(default)]
    pub run_kind: RunKind,
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
        // R56-F1: derive `run_kind` from the call-site inputs. The
        // callers that need a specific kind (bench verbs, future
        // synthetic source) should mutate `record.run_kind` after
        // construction; this default infers from existing fields so
        // we don't have to thread a new parameter through every
        // caller right now.
        let run_kind = if options.dry_run {
            RunKind::DryRun
        } else if fast_path.as_deref() == Some("null_sink") {
            RunKind::NullSink
        } else {
            RunKind::Real
        };
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default();
        Self {
            schema_version: CURRENT_SCHEMA_VERSION,
            timestamp_epoch_ms: now.as_millis(),
            mode,
            run_kind,
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
    // v0 → v1: no field changes; v1 just stamped the version field.
    //
    // v1 → v2: introduced `run_kind`. Older records didn't carry it
    // explicitly; the lane was implicit in `options.dry_run` and
    // `fast_path == Some("null_sink")`. R56-F1: derive the kind
    // without touching `mode` (which already correctly captures
    // copy vs mirror — old mirror records stay mirror, not
    // collapsed to Copy).
    //
    // We re-derive on every load below v2 — serde's #[serde(default)]
    // on the field gives us RunKind::Real for a missing-field
    // deserialize, which is the WRONG default for a dry-run record
    // whose run_kind we never wrote. The explicit migration here
    // is what makes loaded-from-v1 dry-run records actually carry
    // the DryRun lane.
    if record.schema_version < 2 {
        record.run_kind = if record.options.dry_run {
            RunKind::DryRun
        } else if record.fast_path.as_deref() == Some("null_sink") {
            RunKind::NullSink
        } else {
            RunKind::Real
        };
    }
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
        writeln!(file).expect("write empty");
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
            compare_mode: CompareModeSnapshot::SizeMtime,
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

    // ── R56-F1: run_kind lane + migration ──────────────────────────────

    /// Pre-v2 records carried lane in `options.dry_run` and
    /// `fast_path == Some("null_sink")`. Migration must derive the
    /// lane without collapsing `mode` — an old mirror record stays
    /// mirror.
    #[test]
    fn migration_v1_real_copy_record_lands_in_real_lane() {
        let record: PerformanceRecord =
            serde_json::from_str(sample_v0_json()).expect("deserialize v0");
        let migrated = migrate_record(record);
        assert_eq!(migrated.mode, TransferMode::Copy);
        assert_eq!(
            migrated.run_kind,
            RunKind::Real,
            "real copy record should land in Real lane"
        );
    }

    /// GPT explicit ask: "old mirror record migrates without
    /// becoming copy."
    #[test]
    fn migration_v1_mirror_record_preserves_mirror_mode_and_real_lane() {
        let record: PerformanceRecord =
            serde_json::from_str(sample_v1_json()).expect("deserialize v1");
        let migrated = migrate_record(record);
        assert_eq!(
            migrated.mode,
            TransferMode::Mirror,
            "mirror must NOT be collapsed to Copy by migration"
        );
        assert_eq!(
            migrated.run_kind,
            RunKind::Real,
            "non-dry-run mirror record should land in Real lane"
        );
    }

    #[test]
    fn migration_dry_run_record_lands_in_dryrun_lane() {
        // Old v1 record with options.dry_run = true and no
        // explicit run_kind field on the wire.
        let json = r#"{"schema_version":1,"timestamp_epoch_ms":1700000000000,"mode":"copy","source_fs":null,"dest_fs":null,"file_count":3,"total_bytes":100,"options":{"dry_run":true,"preserve_symlinks":true,"include_symlinks":false,"skip_unchanged":false,"checksum":false,"workers":1},"fast_path":null,"planner_duration_ms":5,"transfer_duration_ms":0,"stall_events":0,"error_count":0}"#;
        let record: PerformanceRecord = serde_json::from_str(json).expect("deserialize v1 dry-run");
        let migrated = migrate_record(record);
        assert_eq!(
            migrated.run_kind,
            RunKind::DryRun,
            "options.dry_run=true must migrate to DryRun lane"
        );
        assert_eq!(migrated.mode, TransferMode::Copy);
    }

    #[test]
    fn migration_null_sink_record_lands_in_nullsink_lane() {
        // Old v1 record with fast_path = "null_sink".
        let json = r#"{"schema_version":1,"timestamp_epoch_ms":1700000000000,"mode":"copy","source_fs":null,"dest_fs":null,"file_count":3,"total_bytes":100,"options":{"dry_run":false,"preserve_symlinks":true,"include_symlinks":false,"skip_unchanged":false,"checksum":false,"workers":1},"fast_path":"null_sink","planner_duration_ms":5,"transfer_duration_ms":2,"stall_events":0,"error_count":0}"#;
        let record: PerformanceRecord =
            serde_json::from_str(json).expect("deserialize v1 null-sink");
        let migrated = migrate_record(record);
        assert_eq!(
            migrated.run_kind,
            RunKind::NullSink,
            "fast_path=null_sink must migrate to NullSink lane"
        );
    }

    /// New records via the constructor pick up the lane from
    /// `options.dry_run` and `fast_path` so callers don't have to
    /// thread a new parameter through every existing path.
    #[test]
    fn new_record_with_dry_run_options_picks_dryrun_lane() {
        let options = OptionSnapshot {
            dry_run: true,
            preserve_symlinks: true,
            include_symlinks: false,
            skip_unchanged: true,
            checksum: false,
            compare_mode: CompareModeSnapshot::SizeMtime,
            workers: 4,
        };
        let record = PerformanceRecord::new(
            TransferMode::Mirror,
            None,
            None,
            10,
            1024,
            options,
            None,
            5,
            0,
            0,
            0,
        );
        assert_eq!(record.run_kind, RunKind::DryRun);
        assert_eq!(record.mode, TransferMode::Mirror);
    }

    #[test]
    fn new_record_with_null_sink_fast_path_picks_nullsink_lane() {
        let options = OptionSnapshot {
            dry_run: false,
            preserve_symlinks: true,
            include_symlinks: false,
            skip_unchanged: true,
            checksum: false,
            compare_mode: CompareModeSnapshot::SizeMtime,
            workers: 4,
        };
        let record = PerformanceRecord::new(
            TransferMode::Copy,
            None,
            None,
            10,
            1024,
            options,
            Some("null_sink".to_string()),
            5,
            2,
            0,
            0,
        );
        assert_eq!(record.run_kind, RunKind::NullSink);
    }

    #[test]
    fn new_record_default_is_real() {
        let options = OptionSnapshot {
            dry_run: false,
            preserve_symlinks: true,
            include_symlinks: false,
            skip_unchanged: true,
            checksum: false,
            compare_mode: CompareModeSnapshot::SizeMtime,
            workers: 4,
        };
        let record = PerformanceRecord::new(
            TransferMode::Copy,
            None,
            None,
            10,
            1024,
            options,
            None,
            5,
            10,
            0,
            0,
        );
        assert_eq!(record.run_kind, RunKind::Real);
        assert!(record.run_kind.is_real_transfer());
    }

    /// The eligibility helper is the actual chokepoint other modules
    /// gate on; pin it explicitly so changes to RunKind variants
    /// can't accidentally shift the contract.
    #[test]
    fn is_real_transfer_only_true_for_real() {
        assert!(RunKind::Real.is_real_transfer());
        assert!(!RunKind::DryRun.is_real_transfer());
        assert!(!RunKind::NullSink.is_real_transfer());
        assert!(!RunKind::BenchTransfer.is_real_transfer());
        assert!(!RunKind::BenchWire.is_real_transfer());
    }
}
