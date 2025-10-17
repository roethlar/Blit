//! Local performance history writer for adaptive planning.
//!
//! Records summarized run information to a capped JSONL file under the user's
//! config directory. The data stays on-device and can be disabled via
//! `BLIT_DISABLE_PERF_HISTORY=1`.

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

fn enforce_size_cap(path: &Path, max_bytes: u64) -> Result<()> {
    let metadata = match fs::metadata(path) {
        Ok(meta) => meta,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(err) => return Err(err.into()),
    };

    if metadata.len() <= max_bytes {
        return Ok(());
    }

    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let mut lines: Vec<String> = reader
        .lines()
        .collect::<std::result::Result<_, _>>()
        .context("read performance history for rotation")?;

    if lines.is_empty() {
        return Ok(());
    }

    // Keep trimming from the front until we're under the cap.
    while lines.len() > 1 && estimated_size(&lines) > max_bytes as usize {
        lines.remove(0);
    }

    let mut file = File::create(path).context("rewrite performance history during rotation")?;
    for line in lines {
        writeln!(file, "{line}")?;
    }
    Ok(())
}

fn estimated_size(lines: &[String]) -> usize {
    // Rough estimate including newline per line.
    lines.iter().map(|l| l.len() + 1).sum()
}
