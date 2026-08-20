//! Local performance history writer for adaptive planning.
//!
//! Records summarized run information to a capped JSONL file under the user's
//! config directory. The data stays on-device and can be toggled via the CLI
//! (`blit diagnostics perf --enable/--disable`).

use crate::config;
use std::collections::VecDeque;
use std::ffi::OsString;
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
///   3 - route identity (PERF_HISTORY_PLANNING ph-1): `topology`,
///       `local_role`, `initiator`, `peer_key`. Every field is
///       serde-defaulted; pre-v3 records were only ever written by the
///       local CLI session path (the sole writer that existed), so the
///       defaults (local / source / cli / no peer key) are historically
///       accurate — migration stamps the version without deriving
///       anything.
pub const CURRENT_SCHEMA_VERSION: u32 = 3;

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

/// Where the two ends of the recorded session lived (v3, ph-1).
///
/// Orthogonal to [`TransferMode`] (operator intent) and [`RunKind`]
/// (measurement lane): topology answers "which machines were
/// involved?" so per-route aggregates and seeds never mix a local
/// disk-to-disk run with a wire transfer.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum Topology {
    /// Both ends on this machine (the classic local session).
    #[default]
    Local,
    /// One end here, one end on a daemon across the wire.
    Remote,
    /// Delegated remote→remote: neither data end is this process's
    /// filesystem (coordinator), or this end is a delegated daemon
    /// participant.
    RemoteToRemote,
}

/// Which part this process played in the recorded session (v3, ph-1).
///
/// For `Topology::Local` the process owns both ends; those records use
/// `Source` by convention (the run is one machine — the distinction
/// only exists once a session splits across machines).
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum LocalRole {
    #[default]
    Source,
    Destination,
    /// Delegated remote→remote initiator that moved no bytes itself
    /// (`blit src:// dst://` CLI coordinating two daemons).
    Coordinator,
}

/// What kind of process initiated the recorded session (v3, ph-1).
///
/// A daemon-served push and a daemon-served pull are distinct rows a
/// single `daemon_served` bucket would have collapsed; `initiator`
/// keeps the CLI-initiated and daemon-initiated (delegated) variants
/// separate. Responder-side records tag the initiator by best wire
/// knowledge: the session protocol does not identify the peer's
/// process kind, so served sessions record `Cli` (the delegated
/// dst-daemon→src-daemon serve is indistinguishable on the wire today;
/// wire changes are out of ph-1's scope).
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum Initiator {
    #[default]
    Cli,
    Daemon,
}

/// Route identity for one recorded session end, as that end
/// experienced it (v3, ph-1). Callers that run a session build one of
/// these and attach it via [`PerformanceRecord::with_route`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteTag {
    pub topology: Topology,
    pub local_role: LocalRole,
    pub initiator: Initiator,
    /// Stable destination identity for seed/aggregate keying:
    /// endpoint host + destination root for remote routes, the
    /// destination root for local ones. Plain text — the file is
    /// on-device and debuggable.
    pub peer_key: Option<String>,
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
    /// v3 (ph-1): where the session's ends lived. Serde-defaulted so
    /// v0–v2 records (all written by the local CLI path) stay loadable
    /// with historically-accurate values.
    #[serde(default)]
    pub topology: Topology,
    /// v3 (ph-1): the part this process played.
    #[serde(default)]
    pub local_role: LocalRole,
    /// v3 (ph-1): what kind of process initiated the session.
    #[serde(default)]
    pub initiator: Initiator,
    /// v3 (ph-1): stable destination identity for keying; see
    /// [`RouteTag::peer_key`].
    #[serde(default)]
    pub peer_key: Option<String>,
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
            topology: Topology::Local,
            local_role: LocalRole::Source,
            initiator: Initiator::Cli,
            peer_key: None,
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

    /// Attach a [`RouteTag`] (v3, ph-1). Builder-shaped so the
    /// too-many-arguments `new` does not grow four more parameters.
    pub fn with_route(mut self, route: RouteTag) -> Self {
        self.topology = route.topology;
        self.local_role = route.local_role;
        self.initiator = route.initiator;
        self.peer_key = route.peer_key;
        self
    }
}

/// Process-wide writer serialization (ph-1 writer safety). Every
/// append and every compaction in this process runs under this lock,
/// so concurrent session completions (the daemon spawns served
/// transfers independently) cannot interleave an append with a
/// rotation and lose a record. Cross-process writers are covered by
/// the append being a single O_APPEND write plus compaction being an
/// atomic rename that is skipped when a concurrent append is detected.
static WRITER_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// One performance-history store rooted at a directory (ph-1).
///
/// The CLI store lives under the user's config dir ([`Self::user`]);
/// the daemon owns a separate store in its writable service data
/// directory ([`Self::daemon`], ruling R5) because the documented
/// service unit runs `ProtectHome=read-only` — and even a writable
/// config dir would be the service account's, invisible to the
/// operator. Enable/disable settings live inside each store's
/// directory, so the toggle controls whichever store the recording
/// process owns.
#[derive(Debug, Clone)]
pub struct HistoryStore {
    dir: PathBuf,
}

impl HistoryStore {
    /// The operator's store: the user config directory.
    pub fn user() -> Result<Self> {
        Ok(Self {
            dir: config::config_dir()?,
        })
    }

    /// The daemon's store: systemd's `$STATE_DIRECTORY` when the unit
    /// sets `StateDirectory=` (the explicitly writable service data
    /// directory under `ProtectSystem=strict`), else the process
    /// config dir (foreground / dev runs, where it is writable).
    pub fn daemon() -> Result<Self> {
        if let Some(raw) = std::env::var_os("STATE_DIRECTORY") {
            // systemd passes a colon-separated list when multiple
            // directories are configured; the first is ours.
            if let Some(first) = std::env::split_paths(&raw).next() {
                if !first.as_os_str().is_empty() {
                    return Ok(Self { dir: first });
                }
            }
        }
        Self::user()
    }

    /// A store rooted at an explicit directory (tests, tools).
    pub fn at_dir(dir: PathBuf) -> Self {
        Self { dir }
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    pub fn history_path(&self) -> PathBuf {
        self.dir.join("perf_local.jsonl")
    }

    fn settings_path(&self) -> PathBuf {
        self.dir.join(SETTINGS_FILE)
    }

    fn load_settings(&self) -> Result<Settings> {
        load_settings_at(&self.settings_path())
    }

    /// Whether recording to this store is enabled (default: yes).
    pub fn enabled(&self) -> Result<bool> {
        Ok(self.load_settings()?.perf_history_enabled)
    }

    /// Persist the enablement flag for this store.
    pub fn set_enabled(&self, enabled: bool) -> Result<()> {
        let mut settings = self.load_settings().unwrap_or_default();
        settings.perf_history_enabled = enabled;
        store_settings_at(&self.settings_path(), &settings)
    }

    /// Append a record, honouring this store's enable flag, under the
    /// process writer lock, with capped atomic rotation.
    ///
    /// Errors are bubbled up so callers can decide whether to log or
    /// ignore them.
    pub fn append(&self, record: &PerformanceRecord) -> Result<()> {
        if !self.enabled()? {
            return Ok(());
        }

        let path = self.history_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!(
                    "failed to create performance history directory {}",
                    parent.display()
                )
            })?;
        }

        let _writer = WRITER_LOCK.lock().unwrap_or_else(|e| e.into_inner());

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .with_context(|| {
                format!("failed to open performance history file {}", path.display())
            })?;

        let line = serde_json::to_string(record).context("serialize performance record")?;
        writeln!(file, "{line}").context("write performance record")?;
        drop(file);

        enforce_size_cap(&path, DEFAULT_MAX_BYTES)?;
        Ok(())
    }

    /// Remove this store's history file. `Ok(true)` if it existed.
    pub fn clear(&self) -> Result<bool> {
        let _writer = WRITER_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        match fs::remove_file(self.history_path()) {
            Ok(_) => Ok(true),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(false),
            Err(err) => Err(err.into()),
        }
    }

    /// Load up to `limit` most-recent records (0 = all), migrated.
    pub fn read_recent_records(&self, limit: usize) -> Result<Vec<PerformanceRecord>> {
        read_records_from_path(&self.history_path(), limit)
    }
}

/// Append a record to the user (CLI) performance history store.
///
/// Back-compat wrapper over [`HistoryStore::user`]; see
/// [`HistoryStore::append`] for the gating and safety contract.
pub fn append_local_record(record: &PerformanceRecord) -> Result<()> {
    HistoryStore::user()?.append(record)
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
    // v2 → v3: route identity fields. Nothing to derive — pre-v3
    // records were only ever written by the local CLI session path, so
    // the serde defaults (local / source / cli / no peer key) already
    // hold; the version stamp is the whole migration.
    record.schema_version = CURRENT_SCHEMA_VERSION;
    record
}

/// Load up to `limit` most-recent records (`0` = all) from the user
/// (CLI) store, each migrated to [`CURRENT_SCHEMA_VERSION`].
///
/// Back-compat wrapper over [`HistoryStore::read_recent_records`].
pub fn read_recent_records(limit: usize) -> Result<Vec<PerformanceRecord>> {
    HistoryStore::user()?.read_recent_records(limit)
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

/// Rewrite the user (CLI) history file, migrating all records to the current
/// schema version.
///
/// This is safe to call at any time. Records that fail to parse are dropped.
/// Returns the number of records migrated, or `Ok(0)` if the file doesn't exist.
///
/// Writer safety (ph-1): runs under [`WRITER_LOCK`] and stages the rewritten
/// history in a sibling temp file that is `sync_all`'d and then renamed over
/// the live file. The live file is never truncated in place, so a crash or a
/// concurrent reader can only ever observe the complete old file or the
/// complete new one.
pub fn migrate_history_file() -> Result<usize> {
    let path = HistoryStore::user()?.history_path();

    let _writer = WRITER_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    if !path.exists() {
        return Ok(0);
    }

    let records = read_records_from_path(&path, 0)?;
    let count = records.len();

    let lines = records
        .iter()
        .map(|record| serde_json::to_string(record).context("serialize migrated record"))
        .collect::<Result<Vec<String>>>()?;

    write_lines_atomically(&path, &lines)?;

    Ok(count)
}

pub fn config_dir() -> Result<PathBuf> {
    config::config_dir()
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

/// Read the settings file at `path`, tolerantly: a missing or empty file is
/// the default settings, a malformed one is an error (silently resetting an
/// operator's explicit opt-out would be worse than failing loudly).
fn load_settings_at(path: &Path) -> Result<Settings> {
    if !path.exists() {
        return Ok(Settings::default());
    }

    let bytes = fs::read(path)
        .with_context(|| format!("failed to read perf history settings {}", path.display()))?;
    if bytes.is_empty() {
        return Ok(Settings::default());
    }

    let settings: Settings =
        serde_json::from_slice(&bytes).context("failed to parse perf history settings JSON")?;
    Ok(settings)
}

/// Persist `settings` to `path`, creating the containing directory.
fn store_settings_at(path: &Path, settings: &Settings) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create settings directory {}", parent.display()))?;
    }

    let mut file = File::create(path)
        .with_context(|| format!("failed to write perf history settings {}", path.display()))?;
    let json =
        serde_json::to_vec_pretty(settings).context("failed to serialize perf history settings")?;
    file.write_all(&json)
        .context("failed to persist perf history settings")?;
    file.write_all(b"\n")?;
    Ok(())
}

/// Returns whether performance history is currently enabled for the user
/// (CLI) store. Back-compat wrapper over [`HistoryStore::enabled`].
pub fn perf_history_enabled() -> Result<bool> {
    HistoryStore::user()?.enabled()
}

/// Persist the performance history enablement flag for the user (CLI) store.
/// Back-compat wrapper over [`HistoryStore::set_enabled`].
pub fn set_perf_history_enabled(enabled: bool) -> Result<()> {
    HistoryStore::user()?.set_enabled(enabled)
}

/// Remove the user (CLI) store's performance history file. Returns `Ok(true)`
/// if the file was removed, `Ok(false)` if it did not exist. Back-compat
/// wrapper over [`HistoryStore::clear`].
pub fn clear_history() -> Result<bool> {
    HistoryStore::user()?.clear()
}

/// Sibling temp path used to stage an atomic rewrite of `path`
/// (`perf_local.jsonl` → `perf_local.jsonl.tmp`).
fn temp_path_for(path: &Path) -> PathBuf {
    let mut name = path.file_name().map(OsString::from).unwrap_or_default();
    name.push(".tmp");
    path.with_file_name(name)
}

/// Write `lines` to `tmp` and `sync_all` the result, removing the partial
/// file if anything fails. On success `tmp` is a complete history file on
/// disk, ready to be renamed over the live one.
fn stage_lines(tmp: &Path, lines: &[String]) -> Result<()> {
    let staged = (|| -> Result<()> {
        let mut file = File::create(tmp)
            .with_context(|| format!("failed to stage history rewrite at {}", tmp.display()))?;
        for line in lines {
            writeln!(file, "{line}")?;
        }
        file.sync_all()
            .with_context(|| format!("failed to flush staged history {}", tmp.display()))?;
        Ok(())
    })();
    if staged.is_err() {
        let _ = fs::remove_file(tmp);
    }
    staged
}

/// Rename a staged file over the live history file.
fn swap_into_place(tmp: &Path, path: &Path) -> Result<()> {
    fs::rename(tmp, path).with_context(|| {
        format!(
            "failed to swap staged history {} over {}",
            tmp.display(),
            path.display()
        )
    })
}

/// Replace `path`'s contents with `lines` atomically: stage to a sibling temp
/// file, flush it, then rename. The live file is never truncated in place, so
/// readers and crash recovery see either the whole old file or the whole new
/// one.
///
/// Callers hold [`WRITER_LOCK`], which is what keeps two in-process rewrites
/// from racing over the single temp path.
fn write_lines_atomically(path: &Path, lines: &[String]) -> Result<()> {
    let tmp = temp_path_for(path);
    stage_lines(&tmp, lines)?;
    swap_into_place(&tmp, path)
}

/// True iff `path` is still exactly `observed_len` bytes — the cross-process
/// concurrent-append guard for [`enforce_size_cap`]. A different length means
/// another process appended while we were trimming, and swapping our trimmed
/// copy in would silently drop that record.
fn history_len_unchanged(path: &Path, observed_len: u64) -> bool {
    fs::metadata(path).map(|m| m.len()).unwrap_or(observed_len) == observed_len
}

/// Best-effort rotation that prefers keeping the newest records over enforcing the cap exactly.
/// If a concurrent writer appends while we're trimming, we skip rotation to avoid data loss.
///
/// Callers hold [`WRITER_LOCK`] — both call sites ([`HistoryStore::append`]
/// and this module's tests) take it before calling — so the lock covers
/// in-process races; [`history_len_unchanged`] covers cross-process ones.
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

    // Stage the trimmed history in a sibling temp file first: truncating the
    // live file in place would expose a window where a crash or a concurrent
    // reader sees a half-written history.
    let tmp = temp_path_for(path);
    let survivors: Vec<String> = lines.into_iter().collect();
    stage_lines(&tmp, &survivors)?;

    // Re-read metadata to ensure nothing appended while we trimmed and staged.
    // A cross-process append would be discarded by the rename below, so drop
    // the staged copy and leave the live file alone instead.
    if !history_len_unchanged(path, observed_len) {
        let _ = fs::remove_file(&tmp);
        return Ok(());
    }

    swap_into_place(&tmp, path)
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

    /// review explicit ask: "old mirror record migrates without
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

    // ── ph-1: route identity, per-store settings, writer safety ────────

    fn sample_options() -> OptionSnapshot {
        OptionSnapshot {
            dry_run: false,
            preserve_symlinks: true,
            include_symlinks: false,
            skip_unchanged: true,
            checksum: false,
            compare_mode: CompareModeSnapshot::SizeMtime,
            workers: 4,
        }
    }

    /// A small real-lane record; `file_count` doubles as the identity
    /// the writer-safety and rotation tests match on.
    fn sample_record(file_count: usize) -> PerformanceRecord {
        PerformanceRecord::new(
            TransferMode::Copy,
            None,
            None,
            file_count,
            1024,
            sample_options(),
            None,
            1,
            2,
            0,
            0,
        )
    }

    /// schema_version 2: `run_kind` is explicit, but the v3 route
    /// fields (topology / local_role / initiator / peer_key) do not
    /// exist on the wire yet.
    fn sample_v2_json() -> &'static str {
        r#"{"schema_version":2,"timestamp_epoch_ms":1700000000000,"mode":"copy","run_kind":"real","source_fs":"apfs","dest_fs":"apfs","file_count":7,"total_bytes":2048,"options":{"dry_run":false,"preserve_symlinks":true,"include_symlinks":false,"skip_unchanged":true,"checksum":false,"compare_mode":"size_mtime","workers":4},"fast_path":null,"planner_duration_ms":11,"transfer_duration_ms":22,"stall_events":0,"error_count":0}"#
    }

    /// ph-1 migration: a v2 line has no route fields at all. Pre-v3
    /// records were only ever written by the local CLI session path, so
    /// the serde defaults are historically accurate and the load must
    /// land on local / source / cli / no peer key at v3.
    #[test]
    fn v2_record_migrates_to_local_cli_route_at_v3() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = HistoryStore::at_dir(dir.path().to_path_buf());

        let mut file = File::create(store.history_path()).expect("create");
        writeln!(file, "{}", sample_v2_json()).expect("write v2");
        drop(file);

        let records = store.read_recent_records(0).expect("read");
        assert_eq!(records.len(), 1);
        let record = &records[0];
        assert_eq!(record.schema_version, CURRENT_SCHEMA_VERSION);
        assert_eq!(record.schema_version, 3, "v3 is the current stamp");
        assert_eq!(record.topology, Topology::Local);
        assert_eq!(record.local_role, LocalRole::Source);
        assert_eq!(record.initiator, Initiator::Cli);
        assert_eq!(record.peer_key, None);
        // Pre-existing fields survive the migration untouched.
        assert_eq!(record.file_count, 7);
        assert_eq!(record.run_kind, RunKind::Real);
    }

    /// The v1→v2 lane derivation still bites now that migration stamps
    /// v3: an old dry-run line has no `run_kind` on the wire, and
    /// serde's default (`Real`) is the wrong answer for it.
    #[test]
    fn v0_dry_run_record_still_derives_dryrun_lane_at_v3() {
        let json = r#"{"timestamp_epoch_ms":1700000000000,"mode":"copy","source_fs":null,"dest_fs":null,"file_count":3,"total_bytes":100,"options":{"dry_run":true,"preserve_symlinks":true,"include_symlinks":false,"skip_unchanged":false,"checksum":false,"workers":1},"fast_path":null,"planner_duration_ms":5,"transfer_duration_ms":0,"stall_events":0,"error_count":0}"#;
        let record: PerformanceRecord = serde_json::from_str(json).expect("deserialize v0 dry-run");
        assert_eq!(record.schema_version, 0);

        let migrated = migrate_record(record);
        assert_eq!(
            migrated.run_kind,
            RunKind::DryRun,
            "v0 dry-run lane derivation must survive the v3 stamp"
        );
        assert_eq!(migrated.schema_version, CURRENT_SCHEMA_VERSION);
        assert_eq!(migrated.topology, Topology::Local);
    }

    /// A remote, daemon-initiated destination-side record must survive
    /// serialize → append → read → migrate with all four route fields
    /// intact (the round-trip that per-route aggregates depend on).
    #[test]
    fn route_tag_round_trips_through_store() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = HistoryStore::at_dir(dir.path().to_path_buf());

        let record = sample_record(42).with_route(RouteTag {
            topology: Topology::Remote,
            local_role: LocalRole::Destination,
            initiator: Initiator::Daemon,
            peer_key: Some("host:/root".into()),
        });
        store.append(&record).expect("append");

        let records = store.read_recent_records(0).expect("read");
        assert_eq!(records.len(), 1);
        let loaded = &records[0];
        assert_eq!(loaded.topology, Topology::Remote);
        assert_eq!(loaded.local_role, LocalRole::Destination);
        assert_eq!(loaded.initiator, Initiator::Daemon);
        assert_eq!(loaded.peer_key.as_deref(), Some("host:/root"));
        assert_eq!(loaded.file_count, 42);
        assert_eq!(loaded.schema_version, CURRENT_SCHEMA_VERSION);
    }

    /// Writer-safety guard (ph-1): the daemon completes served
    /// transfers concurrently, so appends from several threads must all
    /// land. Records are small enough that the size cap never fires.
    #[test]
    fn concurrent_appends_lose_no_records() {
        const THREADS: usize = 8;
        const PER_THREAD: usize = 8;

        let dir = tempfile::tempdir().expect("tempdir");
        let store = HistoryStore::at_dir(dir.path().to_path_buf());

        std::thread::scope(|scope| {
            for thread in 0..THREADS {
                let store = store.clone();
                scope.spawn(move || {
                    for index in 0..PER_THREAD {
                        let record = sample_record(thread * PER_THREAD + index);
                        store.append(&record).expect("append");
                    }
                });
            }
        });

        let records = store.read_recent_records(0).expect("read");
        assert_eq!(
            records.len(),
            THREADS * PER_THREAD,
            "concurrent appends must not lose records"
        );
        let mut seen: Vec<usize> = records.iter().map(|r| r.file_count).collect();
        seen.sort_unstable();
        let expected: Vec<usize> = (0..THREADS * PER_THREAD).collect();
        assert_eq!(seen, expected, "every distinct record must be present once");
    }

    /// Rotation compacts through a sibling temp file and an atomic
    /// rename, so the live file is always a whole set of parseable
    /// lines and no staging file is left behind.
    ///
    /// Branch coverage note: this exercises the ROTATION branch
    /// end-to-end. The skip branch needs an append to land between
    /// trimming and the rename, which is not reachable from outside
    /// `enforce_size_cap` without adding a test hook inside it; the
    /// guard predicate itself is covered by
    /// `history_len_guard_detects_concurrent_append` below.
    #[test]
    fn enforce_size_cap_rotates_atomically_keeping_newest() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("perf_local.jsonl");

        let lines: Vec<String> = (0..10usize)
            .map(|i| serde_json::to_string(&sample_record(i)).expect("serialize"))
            .collect();
        // Every record serializes to the same width here, so a cap of
        // three lines keeps exactly the newest three.
        let cap = (lines.last().expect("lines").len() as u64 + 1) * 3;

        // Documented contract: callers of enforce_size_cap hold the
        // writer lock. This test is a caller.
        let _writer = WRITER_LOCK.lock().unwrap_or_else(|e| e.into_inner());

        let mut file = File::create(&path).expect("create");
        for line in &lines {
            writeln!(file, "{line}").expect("write");
        }
        drop(file);
        assert!(fs::metadata(&path).expect("metadata").len() > cap);

        enforce_size_cap(&path, cap).expect("rotate");

        assert!(
            fs::metadata(&path).expect("metadata").len() <= cap,
            "rotation must bring the file under the cap"
        );
        assert!(
            !temp_path_for(&path).exists(),
            "staging file must not survive rotation"
        );

        let records = read_records_from_path(&path, 0).expect("read");
        assert!(!records.is_empty(), "rotation must keep the newest records");
        let text = fs::read_to_string(&path).expect("read back");
        let non_empty_lines = text.lines().filter(|l| !l.trim().is_empty()).count();
        assert_eq!(
            non_empty_lines,
            records.len(),
            "every surviving line must be a whole parseable record"
        );

        let survivors: Vec<usize> = records.iter().map(|r| r.file_count).collect();
        let first = *survivors.first().expect("survivor");
        assert_eq!(
            survivors,
            (first..=9).collect::<Vec<usize>>(),
            "survivors must be the newest contiguous tail, ending at the newest record"
        );
    }

    /// The cross-process guard: if the file grew while we were
    /// trimming, rotation must be abandoned rather than renaming a
    /// trimmed copy over the appended record.
    #[test]
    fn history_len_guard_detects_concurrent_append() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("perf_local.jsonl");

        let mut file = File::create(&path).expect("create");
        writeln!(file, "{}", sample_v1_json()).expect("write");
        drop(file);

        let observed = fs::metadata(&path).expect("metadata").len();
        assert!(
            history_len_unchanged(&path, observed),
            "an untouched file must stay eligible for rotation"
        );

        let mut file = OpenOptions::new().append(true).open(&path).expect("append");
        writeln!(file, "{}", sample_v1_json()).expect("write");
        drop(file);

        assert!(
            !history_len_unchanged(&path, observed),
            "an append while trimming must abort rotation"
        );
    }

    /// Settings live inside each store's directory, so the toggle
    /// controls whichever store the recording process owns — a disabled
    /// CLI store must not silence the daemon's store, or vice versa.
    #[test]
    fn disable_is_per_store() {
        let off_dir = tempfile::tempdir().expect("tempdir");
        let on_dir = tempfile::tempdir().expect("tempdir");
        let off = HistoryStore::at_dir(off_dir.path().to_path_buf());
        let on = HistoryStore::at_dir(on_dir.path().to_path_buf());

        off.set_enabled(false).expect("disable");
        assert!(!off.enabled().expect("read disabled store"));
        assert!(
            on.enabled().expect("read sibling store"),
            "a sibling store keeps its own (default-on) setting"
        );

        let record = sample_record(1);
        off.append(&record).expect("append to disabled store");
        on.append(&record).expect("append to enabled store");

        assert!(
            !off.history_path().exists(),
            "a disabled store must not create a history file"
        );
        assert_eq!(on.read_recent_records(0).expect("read").len(), 1);
    }

    /// Tolerant settings load: absent and empty files are the default,
    /// malformed JSON is an error (never a silent re-enable).
    #[test]
    fn load_settings_at_is_tolerant_of_missing_and_empty() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join(SETTINGS_FILE);

        assert!(
            load_settings_at(&path)
                .expect("missing")
                .perf_history_enabled
        );

        File::create(&path).expect("create empty");
        assert!(load_settings_at(&path).expect("empty").perf_history_enabled);

        store_settings_at(
            &path,
            &Settings {
                perf_history_enabled: false,
            },
        )
        .expect("store");
        assert!(
            !load_settings_at(&path)
                .expect("stored")
                .perf_history_enabled
        );

        fs::write(&path, b"{not json}").expect("write garbage");
        assert!(
            load_settings_at(&path).is_err(),
            "malformed settings must surface as an error"
        );
    }
}
