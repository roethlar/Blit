//! Integration tests for the `--checksum` capability refusal, both
//! directions.
//!
//! Spins up a daemon with `--no-server-checksums`, then runs
//! `blit copy … --checksum` against it. The transfer must bail with an
//! error naming checksum/disabled rather than silently degrading to
//! size+mtime — originally the pull-side F11 / R15-F1 ack flow; since
//! otp-10b-1/2 the unified session refuses at OPEN with
//! `CHECKSUM_DISABLED` for BOTH roles, and the push direction (whose
//! historical `--checksum` gate otp-10b-2 lifted) gets the same pair
//! of pins.
//!
//! The daemon build/spawn runs through `common` (w9-3); the
//! once-per-binary build keeps R16-F1's no-cross-test-ordering
//! property (see `common::ensure_daemon_built`).

use std::fs;
use std::process::Command;
use std::time::Duration;

mod common;
use common::{run_with_timeout, TestContext};

/// Daemon with caller-chosen daemon-side knobs
/// (e.g. `["--no-server-checksums"]`).
fn spawn_daemon_harness(extra_daemon_args: &[&str]) -> TestContext {
    TestContext::builder()
        .extra_daemon_args(extra_daemon_args.iter().copied())
        .build()
}

#[test]
fn pull_checksum_rejected_when_daemon_disables_checksums() {
    // R15-F1 regression. Daemon advertises checksums disabled
    // via `--no-server-checksums`; a pull with `--checksum` must
    // bail at the ack rather than silently using size+mtime.
    let h = spawn_daemon_harness(&["--no-server-checksums"]);
    fs::write(h.module_dir.join("payload.txt"), b"hello").expect("payload");

    let dest_dir = h.workspace.join("dest");
    fs::create_dir_all(&dest_dir).expect("dest dir");

    let remote_src = format!("127.0.0.1:{}:/test/payload.txt", h.daemon_port);
    let mut cli_cmd = Command::new(&h.cli_bin);
    cli_cmd
        .arg("--config-dir")
        .arg(&h.config_dir)
        .arg("copy")
        .arg("--yes")
        .arg("--checksum")
        .arg(&remote_src)
        .arg(&dest_dir);
    let output = run_with_timeout(cli_cmd, Duration::from_secs(60));
    drop(h.daemon);

    assert!(
        !output.status.success(),
        "client should have refused --checksum against a daemon with checksums disabled"
    );
    let stderr = String::from_utf8_lossy(&output.stderr).to_lowercase();
    assert!(
        stderr.contains("checksum") && stderr.contains("disabled"),
        "expected ack-mismatch error mentioning checksum/disabled, got stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Belt-and-suspenders: the file must NOT have been silently
    // copied via the size+mtime fallback. The handshake bailed
    // before any data flowed.
    assert!(
        !dest_dir.join("payload.txt").exists(),
        "no file should have been transferred when the handshake bailed"
    );
}

#[test]
fn pull_checksum_succeeds_when_daemon_enables_checksums() {
    // Companion: same setup minus `--no-server-checksums`. The
    // pull should succeed and copy the file. Proves the
    // capability check doesn't false-positive when the daemon
    // does support checksums.
    let h = spawn_daemon_harness(&[]);
    fs::write(h.module_dir.join("payload.txt"), b"hello").expect("payload");

    let dest_dir = h.workspace.join("dest");
    fs::create_dir_all(&dest_dir).expect("dest dir");

    let remote_src = format!("127.0.0.1:{}:/test/payload.txt", h.daemon_port);
    let mut cli_cmd = Command::new(&h.cli_bin);
    cli_cmd
        .arg("--config-dir")
        .arg(&h.config_dir)
        .arg("copy")
        .arg("--yes")
        .arg("--checksum")
        .arg(&remote_src)
        .arg(&dest_dir);
    let output = run_with_timeout(cli_cmd, Duration::from_secs(60));
    drop(h.daemon);

    if !output.status.success() {
        panic!(
            "checksum-enabled daemon pull should succeed, got:\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    assert!(dest_dir.join("payload.txt").exists());
    assert_eq!(fs::read(dest_dir.join("payload.txt")).unwrap(), b"hello");
}

/// otp-10b-2 gate lift, refusal half: `blit copy LOCAL server:… --checksum`
/// against a `--no-server-checksums` daemon is refused at session OPEN
/// (CHECKSUM_DISABLED) — the push twin of the pull refusal above. No
/// silent size+mtime degrade: nothing lands.
#[test]
fn push_checksum_rejected_when_daemon_disables_checksums() {
    let h = spawn_daemon_harness(&["--no-server-checksums"]);

    let src_dir = h.workspace.join("src");
    fs::create_dir_all(&src_dir).expect("src dir");
    fs::write(src_dir.join("payload.txt"), b"hello").expect("payload");

    let dest_remote = format!("127.0.0.1:{}:/test/", h.daemon_port);
    let src_arg = format!("{}/", src_dir.display());
    let mut cli_cmd = Command::new(&h.cli_bin);
    cli_cmd
        .arg("--config-dir")
        .arg(&h.config_dir)
        .arg("copy")
        .arg("--yes")
        .arg("--checksum")
        .arg(&src_arg)
        .arg(&dest_remote);
    let output = run_with_timeout(cli_cmd, Duration::from_secs(60));

    assert!(
        !output.status.success(),
        "client should have refused --checksum against a daemon with checksums disabled"
    );
    let stderr = String::from_utf8_lossy(&output.stderr).to_lowercase();
    assert!(
        stderr.contains("checksum") && stderr.contains("disabled"),
        "expected an OPEN refusal mentioning checksum/disabled, got stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !h.module_dir.join("payload.txt").exists(),
        "no file should have been transferred when the open was refused"
    );
    drop(h.daemon);
}

/// otp-10b-2 gate lift, success half: push `--checksum` against a
/// checksums-enabled daemon transfers (the historical CLI rejection
/// "--checksum is not supported for remote-destination transfers" is
/// gone) and skips content-equal files on a re-run.
#[test]
fn push_checksum_succeeds_when_daemon_enables_checksums() {
    let h = spawn_daemon_harness(&[]);

    let src_dir = h.workspace.join("src");
    fs::create_dir_all(&src_dir).expect("src dir");
    fs::write(src_dir.join("payload.txt"), b"hello").expect("payload");

    let dest_remote = format!("127.0.0.1:{}:/test/", h.daemon_port);
    let src_arg = format!("{}/", src_dir.display());
    let run = || {
        let mut cli_cmd = Command::new(&h.cli_bin);
        cli_cmd
            .arg("--config-dir")
            .arg(&h.config_dir)
            .arg("copy")
            .arg("--yes")
            .arg("--checksum")
            .arg(&src_arg)
            .arg(&dest_remote);
        run_with_timeout(cli_cmd, Duration::from_secs(60))
    };

    let output = run();
    if !output.status.success() {
        panic!(
            "checksum-enabled daemon push should succeed, got:\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    assert_eq!(
        fs::read(h.module_dir.join("payload.txt")).unwrap(),
        b"hello"
    );

    // Re-run: content-equal ⇒ the Checksum compare skips everything.
    let output = run();
    assert!(output.status.success(), "checksum re-push should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Remote already up to date")
            || stdout.contains("Transfer complete: 0 file(s)"),
        "content-equal re-push must skip, got stdout:\n{stdout}"
    );
    drop(h.daemon);
}
