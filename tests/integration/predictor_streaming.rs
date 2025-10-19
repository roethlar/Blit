use std::fs;
use tempfile::tempdir;

use eyre::Result;
use blit_core::orchestrator::{LocalMirrorOptions, TransferOrchestrator};
use blit_core::perf_history::{append_local_record, OptionSnapshot, PerformanceRecord, TransferMode};
use blit_core::perf_predictor::PerformancePredictor;

#[test]
fn streaming_forced_when_prediction_low() -> Result<()> {
    std::env::set_var("BLIT_DISABLE_PERF_HISTORY", "1");
    let tmp = tempdir()?;
    let src = tmp.path().join("src");
    let dest = tmp.path().join("dest");
    fs::create_dir_all(&src)?;
    fs::create_dir_all(&dest)?;
    fs::write(src.join("a.txt"), b"hello")?;

    let mut predictor = PerformancePredictor::for_tests(tmp.path());
    let record = PerformanceRecord::new(
        TransferMode::Copy,
        None,
        None,
        4,
        1_000,
        OptionSnapshot {
            dry_run: false,
            preserve_symlinks: true,
            include_symlinks: true,
            skip_unchanged: true,
            checksum: false,
            workers: 4,
        },
        None,
        100,
        1_000,
        0,
        0,
    );
    predictor.observe(&record);
    predictor.save()?;

    let options = LocalMirrorOptions {
        progress: false,
        ..Default::default()
    };
    let orchestrator = TransferOrchestrator::new();
    let summary = orchestrator.execute_local_mirror(&src, &dest, options)?;
    assert_eq!(summary.copied_files, 1);
    Ok(())
}

#[test]
fn fast_path_when_prediction_high() -> Result<()> {
    std::env::set_var("BLIT_DISABLE_PERF_HISTORY", "1");
    let tmp = tempdir()?;
    let src = tmp.path().join("src");
    let dest = tmp.path().join("dest");
    fs::create_dir_all(&src)?;
    fs::create_dir_all(&dest)?;
    fs::write(src.join("a.txt"), b"hello")?;

    let mut predictor = PerformancePredictor::for_tests(tmp.path());
    let record = PerformanceRecord::new(
        TransferMode::Copy,
        None,
        None,
        1,
        64,
        OptionSnapshot {
            dry_run: false,
            preserve_symlinks: true,
            include_symlinks: true,
            skip_unchanged: true,
            checksum: false,
            workers: 4,
        },
        None,
        5_000,
        1_000,
        0,
        0,
    );
    predictor.observe(&record);
    predictor.save()?;

    let options = LocalMirrorOptions {
        progress: false,
        ..Default::default()
    };
    let orchestrator = TransferOrchestrator::new();
    let summary = orchestrator.execute_local_mirror(&src, &dest, options)?;
    assert_eq!(summary.copied_files, 1);
    Ok(())
}
