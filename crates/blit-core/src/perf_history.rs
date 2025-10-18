//! Local performance history writer for adaptive planning.
//!
//! Records summarized run information to a capped JSONL file under the user's
//! config directory. The data stays on-device and can be disabled via
//! `BLIT_DISABLE_PERF_HISTORY=1`.

use std::collections::VecDeque;
use std::env;
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};

const DEFAULT_MAX_BYTES: u64 = 1_000_000; // ~1 MiB cap per design docs
const DISABLE_ENV: &str = "BLIT_DISABLE_PERF_HISTORY";

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
        }
    }
}

/// Append a record to the local performance history store.
///
/// Errors are bubbled up so callers can decide whether to log or ignore them.
pub fn append_local_record(record: &PerformanceRecord) -> Result<()> {
    if perf_history_disabled() {
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

fn perf_history_disabled() -> bool {
    env::var(DISABLE_ENV)
        .map(|val| val == "1" || val.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

pub fn config_dir() -> Result<PathBuf> {
    if let Some(proj) = ProjectDirs::from("com", "Blit", "Blit") {
        return Ok(proj.config_dir().to_path_buf());
    }
    let home = env::var_os("HOME").ok_or_else(|| {
        anyhow::anyhow!("cannot determine HOME directory for performance history")
    })?;
    Ok(Path::new(&home).join(".config").join("blit"))
}

fn history_path() -> Result<PathBuf> {
    Ok(config_dir()?.join("perf_local.jsonl"))
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
    let mut trimmed = false;

    while lines.len() > 1 && total_size > max_bytes as usize {
        if let Some(front) = lines.pop_front() {
            total_size -= front.len() + 1;
            trimmed = true;
        }
    }

    if !trimmed {
        // Either the file already fits under the cap or a single entry is larger than the cap.
        return Ok(());
    }

    // If another process appended between our stat and trimming pass, skip rotation to avoid
    // clobbering newer records. We'll enforce the cap on the next write.
    let current_len = fs::metadata(path)?.len();
    if current_len > observed_len {
        return Ok(());
    }

    let mut file = OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(path)
        .context("truncate performance history during rotation")?;

    for line in lines {
        writeln!(file, "{line}")?;
    }
    Ok(())
}
