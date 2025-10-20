use blit_core::orchestrator::{LocalMirrorOptions, TransferOrchestrator};
use blit_core::perf_history;
use eyre::Result;
use std::ffi::OsString;
use std::fs;
use tempfile::tempdir;

struct ConfigDirGuard {
    temp: tempfile::TempDir,
    prev: Option<OsString>,
}

impl ConfigDirGuard {
    fn new() -> Result<Self> {
        let temp = tempdir()?;
        let prev = std::env::var_os("BLIT_CONFIG_DIR");
        std::env::set_var("BLIT_CONFIG_DIR", temp.path());
        Ok(Self { temp, prev })
    }
}

impl Drop for ConfigDirGuard {
    fn drop(&mut self) {
        if let Some(prev) = &self.prev {
            std::env::set_var("BLIT_CONFIG_DIR", prev);
        } else {
            std::env::remove_var("BLIT_CONFIG_DIR");
        }
    }
}

#[test]
fn tiny_manifest_records_fast_path() -> Result<()> {
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

    let mut options = LocalMirrorOptions::default();
    options.progress = false;
    options.perf_history = true;

    let orchestrator = TransferOrchestrator::new();
    let summary = orchestrator.execute_local_mirror(&src, &dest, options)?;
    assert_eq!(summary.copied_files, 3);

    let records = perf_history::read_recent_records(0)?;
    let last = records.last().expect("expected perf history record");
    assert_eq!(last.fast_path.as_deref(), Some("tiny_manifest"));
    Ok(())
}

#[test]
fn larger_manifest_records_streaming_path() -> Result<()> {
    let _guard = ConfigDirGuard::new()?;
    perf_history::set_perf_history_enabled(true)?;
    let _ = perf_history::clear_history()?;

    let tmp = tempdir()?;
    let src = tmp.path().join("src");
    let dest = tmp.path().join("dest");
    fs::create_dir_all(&src)?;
    fs::create_dir_all(&dest)?;
    for idx in 0..32 {
        let file = src.join(format!("file-{idx}.txt"));
        fs::write(file, format!("payload-{idx}"))?;
    }

    let mut options = LocalMirrorOptions::default();
    options.progress = false;
    options.perf_history = true;

    let orchestrator = TransferOrchestrator::new();
    let summary = orchestrator.execute_local_mirror(&src, &dest, options)?;
    assert_eq!(summary.copied_files, 32);

    let records = perf_history::read_recent_records(0)?;
    let last = records.last().expect("expected perf history record");
    assert!(
        last.fast_path.is_none(),
        "streaming path should not record a fast-path tag"
    );
    Ok(())
}
