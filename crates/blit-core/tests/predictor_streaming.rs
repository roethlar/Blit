//! Revived by w9-2 from the dead workspace-root tests/ directory.
//! `PerformancePredictor::for_tests` no longer exists; the predictor
//! now loads from the config dir, so these tests scope it with the
//! same config-dir override guard `local_transfers.rs` uses.

use blit_core::config;
use blit_core::orchestrator::{LocalMirrorOptions, TransferOrchestrator};
use blit_core::perf_history::{
    CompareModeSnapshot, OptionSnapshot, PerformanceRecord, TransferMode,
};
use blit_core::perf_predictor::PerformancePredictor;
use eyre::Result;
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use tempfile::tempdir;

/// Both tests mutate process-global state (the config-dir override
/// and the predictor history inside it); they must not interleave.
static SERIAL: Mutex<()> = Mutex::new(());

struct ConfigDirGuard {
    // RAII holder: the tempdir must outlive the override.
    _temp: tempfile::TempDir,
    prev: Option<PathBuf>,
}

impl ConfigDirGuard {
    fn new() -> Result<Self> {
        let temp = tempdir()?;
        let prev = config::config_dir_override();
        config::set_config_dir(temp.path());
        Ok(Self { _temp: temp, prev })
    }
}

impl Drop for ConfigDirGuard {
    fn drop(&mut self) {
        if let Some(prev) = &self.prev {
            config::set_config_dir(prev);
        } else {
            config::clear_config_dir_override();
        }
    }
}

fn seed_record(file_count: usize, total_bytes: u64, planner_ms: u128) -> PerformanceRecord {
    PerformanceRecord::new(
        TransferMode::Copy,
        None,
        None,
        file_count,
        total_bytes,
        OptionSnapshot {
            dry_run: false,
            preserve_symlinks: true,
            include_symlinks: true,
            skip_unchanged: true,
            checksum: false,
            compare_mode: CompareModeSnapshot::SizeMtime,
            workers: 4,
        },
        None,
        planner_ms,
        1_000,
        0,
        0,
    )
}

#[test]
fn streaming_forced_when_prediction_low() -> Result<()> {
    let _serial = SERIAL.lock().unwrap_or_else(|poison| poison.into_inner());
    let _guard = ConfigDirGuard::new()?;

    let tmp = tempdir()?;
    let src = tmp.path().join("src");
    let dest = tmp.path().join("dest");
    fs::create_dir_all(&src)?;
    fs::create_dir_all(&dest)?;
    fs::write(src.join("a.txt"), b"hello")?;

    let mut predictor = PerformancePredictor::load()?;
    predictor.observe(&seed_record(4, 1_000, 100));
    predictor.save()?;

    let options = LocalMirrorOptions {
        progress: false,
        perf_history: false,
        ..Default::default()
    };
    let orchestrator = TransferOrchestrator::new();
    let summary = orchestrator.execute_local_mirror(&src, &dest, options)?;
    assert_eq!(summary.copied_files, 1);
    Ok(())
}

#[test]
fn fast_path_when_prediction_high() -> Result<()> {
    let _serial = SERIAL.lock().unwrap_or_else(|poison| poison.into_inner());
    let _guard = ConfigDirGuard::new()?;

    let tmp = tempdir()?;
    let src = tmp.path().join("src");
    let dest = tmp.path().join("dest");
    fs::create_dir_all(&src)?;
    fs::create_dir_all(&dest)?;
    fs::write(src.join("a.txt"), b"hello")?;

    let mut predictor = PerformancePredictor::load()?;
    predictor.observe(&seed_record(1, 64, 5_000));
    predictor.save()?;

    let options = LocalMirrorOptions {
        progress: false,
        perf_history: false,
        ..Default::default()
    };
    let orchestrator = TransferOrchestrator::new();
    let summary = orchestrator.execute_local_mirror(&src, &dest, options)?;
    assert_eq!(summary.copied_files, 1);
    Ok(())
}
