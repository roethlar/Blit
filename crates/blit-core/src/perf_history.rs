//! Local performance history writer for adaptive planning.
//!
//! Records summarized run information to a capped JSONL file under the user's
//! config directory. The data stays on-device and can be toggled via the CLI
//! (`blit diagnostics perf --enable/--disable`). Environment variables no longer
//! control the behaviour; configuration is persisted alongside the history file.

use std::collections::VecDeque;
use std::env;
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use directories::ProjectDirs;
use eyre::{eyre, Context, Result};
use serde::{Deserialize, Serialize};

const DEFAULT_MAX_BYTES: u64 = 1_000_000; // ~1 MiB cap per design docs
const SETTINGS_FILE: &str = "settings.json";

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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceRecord {
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

pub fn read_recent_records(limit: usize) -> Result<Vec<PerformanceRecord>> {
    let path = history_path()?;
    if !path.exists() {
        return Ok(Vec::new());
    }

    let file = File::open(&path)?;
    let reader = BufReader::new(file);
    let mut records = Vec::new();

    for line in reader.lines() {
        let Ok(line) = line else { continue };
        if line.trim().is_empty() {
            continue;
        }
        if let Ok(record) = serde_json::from_str::<PerformanceRecord>(&line) {
            records.push(record);
        }
    }

    if limit == 0 || records.len() <= limit {
        return Ok(records);
    }

    let start = records.len().saturating_sub(limit);
    Ok(records[start..].to_vec())
}

pub fn config_dir() -> Result<PathBuf> {
    if let Some(path) = env::var_os("BLIT_CONFIG_DIR") {
        return Ok(PathBuf::from(path));
    }
    if let Some(proj) = ProjectDirs::from("com", "Blit", "Blit") {
        return Ok(proj.config_dir().to_path_buf());
    }
    let home = env::var_os("HOME")
        .ok_or_else(|| eyre!("cannot determine HOME directory for performance history"))?;
    Ok(Path::new(&home).join(".config").join("blit"))
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
