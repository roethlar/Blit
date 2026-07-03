use blit_core::config;
use blit_core::orchestrator::{LocalMirrorOptions, TransferOrchestrator};
use blit_core::perf_history;
use eyre::Result;
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use tempfile::tempdir;

/// Both tests mutate process-global state (the config-dir override
/// and the perf-history file inside it); they must not interleave.
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

#[test]
fn tiny_manifest_records_fast_path() -> Result<()> {
    let _serial = SERIAL.lock().unwrap_or_else(|poison| poison.into_inner());
    let _guard = ConfigDirGuard::new()?;
    perf_history::set_perf_history_enabled(true)?;
    let _ = perf_history::clear_history()?;

    let tmp = tempdir()?;
    let src = tmp.path().join("src");
    let dest = tmp.path().join("dest");
    fs::create_dir_all(&src)?;
    fs::create_dir_all(&dest)?;
    fs::write(src.join("a.txt"), b"one")?;
    fs::write(src.join("b.txt"), b"two")?;
    fs::write(src.join("c.txt"), b"three")?;

    let options = LocalMirrorOptions {
        progress: false,
        perf_history: true,
        ..Default::default()
    };

    let orchestrator = TransferOrchestrator::new();
    let summary = orchestrator.execute_local_mirror(&src, &dest, options)?;
    assert_eq!(summary.copied_files, 3);

    let records = perf_history::read_recent_records(0)?;
    let last = records.last().expect("expected perf history record");
    assert_eq!(last.fast_path.as_deref(), Some("tiny_manifest"));
    Ok(())
}

/// ue-r2-1c behavior pin (added before the engine move): a second run
/// over an already-synced directory routes through
/// `FastPathDecision::NoWork{examined > 0}`, reports
/// `TransferOutcome::UpToDate`, and records the `no_work` perf-history
/// tag. Previously this strategy had no test at all.
#[test]
fn up_to_date_second_run_records_no_work() -> Result<()> {
    use blit_core::orchestrator::TransferOutcome;

    let _serial = SERIAL.lock().unwrap_or_else(|poison| poison.into_inner());
    let _guard = ConfigDirGuard::new()?;
    perf_history::set_perf_history_enabled(true)?;
    let _ = perf_history::clear_history()?;

    let tmp = tempdir()?;
    let src = tmp.path().join("src");
    let dest = tmp.path().join("dest");
    fs::create_dir_all(&src)?;
    fs::write(src.join("a.txt"), b"one")?;
    fs::write(src.join("b.txt"), b"two")?;

    let options = || LocalMirrorOptions {
        progress: false,
        perf_history: true,
        // preserve_times keeps mtimes matching so the second run's
        // size+mtime comparison sees both files as unchanged.
        preserve_times: true,
        ..Default::default()
    };

    let orchestrator = TransferOrchestrator::new();
    let first = orchestrator.execute_local_mirror(&src, &dest, options())?;
    assert_eq!(first.copied_files, 2);

    let second = orchestrator.execute_local_mirror(&src, &dest, options())?;
    assert_eq!(second.copied_files, 0);
    assert_eq!(second.outcome, TransferOutcome::UpToDate);
    assert!(
        second.scanned_files >= 2,
        "NoWork must report examined files"
    );

    let records = perf_history::read_recent_records(0)?;
    let last = records.last().expect("expected perf history record");
    assert_eq!(last.fast_path.as_deref(), Some("no_work"));
    Ok(())
}

/// ue-r2-1c behavior pin (added before the engine move): an empty
/// source directory routes through `NoWork{examined: 0}` and reports
/// `TransferOutcome::SourceEmpty`. Previously untested.
#[test]
fn empty_source_dir_reports_source_empty() -> Result<()> {
    use blit_core::orchestrator::TransferOutcome;

    let _serial = SERIAL.lock().unwrap_or_else(|poison| poison.into_inner());
    let _guard = ConfigDirGuard::new()?;
    perf_history::set_perf_history_enabled(true)?;
    let _ = perf_history::clear_history()?;

    let tmp = tempdir()?;
    let src = tmp.path().join("src");
    let dest = tmp.path().join("dest");
    fs::create_dir_all(&src)?;

    let options = LocalMirrorOptions {
        progress: false,
        perf_history: true,
        ..Default::default()
    };

    let orchestrator = TransferOrchestrator::new();
    let summary = orchestrator.execute_local_mirror(&src, &dest, options)?;
    assert_eq!(summary.copied_files, 0);
    assert_eq!(summary.outcome, TransferOutcome::SourceEmpty);

    let records = perf_history::read_recent_records(0)?;
    let last = records.last().expect("expected perf history record");
    assert_eq!(last.fast_path.as_deref(), Some("no_work"));
    Ok(())
}

/// ue-r2-1c: the single-file shortcut historically bypassed
/// perf-history recording entirely — the only strategy that did. It
/// now records with the `single_file` tag and scanned-feature
/// accounting (REV4 Design §2: strategies share common accounting).
#[test]
fn single_file_copy_records_history() -> Result<()> {
    let _serial = SERIAL.lock().unwrap_or_else(|poison| poison.into_inner());
    let _guard = ConfigDirGuard::new()?;
    perf_history::set_perf_history_enabled(true)?;
    let _ = perf_history::clear_history()?;

    let tmp = tempdir()?;
    let src = tmp.path().join("one.bin");
    let dest = tmp.path().join("dest.bin");
    fs::write(&src, b"payload-bytes")?;

    let options = LocalMirrorOptions {
        progress: false,
        perf_history: true,
        ..Default::default()
    };

    let orchestrator = TransferOrchestrator::new();
    let summary = orchestrator.execute_local_mirror(&src, &dest, options)?;
    assert_eq!(summary.copied_files, 1);

    let records = perf_history::read_recent_records(0)?;
    let last = records.last().expect("expected perf history record");
    assert_eq!(last.fast_path.as_deref(), Some("single_file"));
    assert_eq!(last.file_count, 1);
    assert_eq!(last.total_bytes, b"payload-bytes".len() as u64);
    Ok(())
}

#[test]
fn larger_manifest_records_streaming_path() -> Result<()> {
    let _serial = SERIAL.lock().unwrap_or_else(|poison| poison.into_inner());
    let _guard = ConfigDirGuard::new()?;
    perf_history::set_perf_history_enabled(true)?;
    let _ = perf_history::clear_history()?;

    let tmp = tempdir()?;
    let src = tmp.path().join("src");
    let dest = tmp.path().join("dest");
    fs::create_dir_all(&src)?;
    fs::create_dir_all(&dest)?;
    // Must exceed the fast-path tiny budget (TINY_FILE_LIMIT = 256
    // in engine/strategy.rs) so the streaming planner runs.
    // The original 32-file version predates that threshold.
    for idx in 0..300 {
        let file = src.join(format!("file-{idx}.txt"));
        fs::write(file, format!("payload-{idx}"))?;
    }

    let options = LocalMirrorOptions {
        progress: false,
        perf_history: true,
        ..Default::default()
    };

    let orchestrator = TransferOrchestrator::new();
    let summary = orchestrator.execute_local_mirror(&src, &dest, options)?;
    assert_eq!(summary.copied_files, 300);

    let records = perf_history::read_recent_records(0)?;
    let last = records.last().expect("expected perf history record");
    assert!(
        last.fast_path.is_none(),
        "streaming path should not record a fast-path tag"
    );
    Ok(())
}
