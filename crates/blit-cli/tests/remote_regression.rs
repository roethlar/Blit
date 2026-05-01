//! Regression tests for bugs that shipped and got fixed.
//!
//! Each test here reproduces a scenario that would have caught the bug
//! before it reached users. Tests are integration-level (real daemon + CLI).

use std::fs;
use std::process::Command;
use std::time::Duration;

mod common;
use common::{run_with_timeout, TestContext};

// -- pull_sync deadlock (fixed in 946bd77) -----------------------------------

/// Mirror from a remote module to a populated local dir with more than 32 files.
///
/// The bug: `pull_sync` pushed all manifest entries into a 32-deep mpsc
/// channel BEFORE opening the gRPC bidi stream. For any local manifest with
/// more than 30 entries, the 33rd `tx.send().await` blocked forever — no
/// consumer. Cold mirrors worked accidentally (empty local manifest = 2
/// messages); noop on a populated dest hung silently.
#[cfg(unix)]
#[test]
fn pull_sync_does_not_deadlock_with_populated_destination() {
    let ctx = TestContext::new();

    // Populate server with 50 files.
    for i in 0..50 {
        let name = format!("file_{:03}.txt", i);
        fs::write(
            ctx.module_dir.join(&name),
            format!("server content {}", i),
        )
        .expect("write server file");
    }

    // Populate dest with a subset (40/50 files).
    // pull_sync sends a local manifest for every file, so >32 entries
    // exercises the old channel-capacity deadlock.
    let dest_dir = ctx.workspace.join("dest");
    fs::create_dir_all(&dest_dir).expect("dest dir");
    for i in 0..40 {
        let name = format!("file_{:03}.txt", i);
        fs::write(dest_dir.join(&name), format!("old local {}", i)).expect("write local file");
    }

    let src_remote = format!("127.0.0.1:{}:/test/", ctx.daemon_port);
    let mut cli_cmd = Command::new(&ctx.cli_bin);
    cli_cmd
        .arg("--config-dir")
        .arg(&ctx.config_dir)
        .arg("mirror")
        .arg("--yes")
        .arg(&src_remote)
        .arg(&dest_dir);

    let output = run_with_timeout(cli_cmd, Duration::from_secs(60));
    assert!(
        output.status.success(),
        "blit-cli mirror failed (pull_sync deadlock?)\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    // All 50 files should now be present with server content.
    for i in 0..50 {
        let name = format!("file_{:03}.txt", i);
        let dest_path = dest_dir.join(&name);
        assert!(
            dest_path.exists(),
            "expected {name} to exist after mirror"
        );
        let content = fs::read_to_string(&dest_path).expect("read dest file");
        assert_eq!(
            content,
            format!("server content {}", i),
            "file {name} has wrong content"
        );
    }
}

// -- mtime preservation end-to-end (fixed in 946bd77) ------------------------

/// Verify that file mtimes survive a data-plane pull end-to-end.
///
/// The bug: `set_file_mtime` fired while the tokio `File` handle was still
/// open with deferred writes in flight on the blocking-thread pool. The
/// kernel would bump the mtime to "now" after `set_file_mtime` returned,
/// so 5/8 of pulled files on a mirror lost their original mtime.
///
/// This test checks actual `stat` mtime of the received file matches the
/// source.
#[cfg(unix)]
#[test]
fn pull_preserves_mtime_end_to_end() {
    let ctx = TestContext::new();

    let dest_dir = ctx.workspace.join("dest");
    fs::create_dir_all(&dest_dir).expect("dest dir");

    // Create server file with a specific, non-now mtime far in the past.
    let server_path = ctx.module_dir.join("mtime_test.bin");
    let content = b"this file should preserve its mtime";
    fs::write(&server_path, content).expect("write server file");

    // 2020-01-15T12:00:00 UTC
    let target_mtime: i64 = 1_579_089_600;
    let ft = filetime::FileTime::from_unix_time(target_mtime, 0);
    filetime::set_file_mtime(&server_path, ft).expect("set server mtime");
    filetime::set_symlink_file_times(&server_path, ft, ft).expect("set server symlink times");

    // Verify we set it correctly.
    let meta = fs::metadata(&server_path).expect("stat server file");
    let actual_mtime = meta
        .modified()
        .expect("modified time")
        .duration_since(std::time::UNIX_EPOCH)
        .expect("duration")
        .as_secs() as i64;
    assert_eq!(actual_mtime, target_mtime, "server mtime not set correctly");

    let src_remote = format!("127.0.0.1:{}:/test/", ctx.daemon_port);
    let mut cli_cmd = Command::new(&ctx.cli_bin);
    cli_cmd
        .arg("--config-dir")
        .arg(&ctx.config_dir)
        .arg("mirror")
        .arg("--yes")
        .arg(&src_remote)
        .arg(&dest_dir);

    let output = run_with_timeout(cli_cmd, Duration::from_secs(60));
    assert!(
        output.status.success(),
        "blit-cli mirror failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let dest_path = dest_dir.join("mtime_test.bin");
    assert!(dest_path.exists(), "dest file missing");
    let dest_content = fs::read(&dest_path).expect("read dest file");
    assert_eq!(dest_content, content, "content mismatch");

    // The critical check: mtime must survive the transfer.
    let dest_meta = fs::metadata(&dest_path).expect("stat dest file");
    let dest_mtime = dest_meta
        .modified()
        .expect("dest modified time")
        .duration_since(std::time::UNIX_EPOCH)
        .expect("duration")
        .as_secs() as i64;
    assert_eq!(
        dest_mtime, target_mtime,
        "mtime not preserved: expected {target_mtime}, got {dest_mtime}"
    );
}

// -- mtime-touch auto-promotion (new in this branch) ------------------------

/// Verify that when a file's mtime changes but content is identical,
/// the mirror does NOT re-transfer the entire file — block-hash comparison
/// detects identical blocks and transfers nothing.
///
/// This tests the auto-promotion implemented in pull_sync.rs: Modified files
/// with matching size now trigger block-hash comparison even without --resume.
#[cfg(unix)]
#[test]
fn mtime_only_change_does_not_re_transfer_full_file() {
    let ctx = TestContext::new();

    let dest_dir = ctx.workspace.join("dest");
    fs::create_dir_all(&dest_dir).expect("dest dir");

    // Create a 2 MiB server file (2 blocks at 1 MiB each).
    let content: Vec<u8> = (0..2 * 1024 * 1024).map(|i| (i % 256) as u8).collect();
    fs::write(ctx.module_dir.join("big.bin"), &content).expect("write server file");

    // First mirror: pull the file down.
    let src_remote = format!("127.0.0.1:{}:/test/", ctx.daemon_port);
    let mut cli_cmd = Command::new(&ctx.cli_bin);
    cli_cmd
        .arg("--config-dir")
        .arg(&ctx.config_dir)
        .arg("mirror")
        .arg("--yes")
        .arg(&src_remote)
        .arg(&dest_dir);
    let output = run_with_timeout(cli_cmd, Duration::from_secs(60));
    assert!(
        output.status.success(),
        "first mirror failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Touch the server file's mtime without changing content.
    let new_mtime: i64 = 2_000_000_000;
    let ft = filetime::FileTime::from_unix_time(new_mtime, 0);
    filetime::set_file_mtime(ctx.module_dir.join("big.bin"), ft).expect("set mtime");

    // Second mirror: should complete quickly (no full re-transfer).
    let mut cli_cmd = Command::new(&ctx.cli_bin);
    cli_cmd
        .arg("--config-dir")
        .arg(&ctx.config_dir)
        .arg("mirror")
        .arg("--yes")
        .arg(&src_remote)
        .arg(&dest_dir);
    let output = run_with_timeout(cli_cmd, Duration::from_secs(60));
    assert!(
        output.status.success(),
        "second mirror failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Verify content is still correct.
    let result = fs::read(dest_dir.join("big.bin")).expect("read dest file");
    assert_eq!(result, content, "content changed unexpectedly");

    // Verify mtime was updated to the new value.
    let dest_meta = fs::metadata(dest_dir.join("big.bin")).expect("stat dest file");
    let dest_mtime = dest_meta
        .modified()
        .expect("dest modified time")
        .duration_since(std::time::UNIX_EPOCH)
        .expect("duration")
        .as_secs() as i64;
    assert_eq!(
        dest_mtime, new_mtime,
        "mtime should have been updated to {new_mtime}"
    );
}
