//! CLI-level safety-gate regression tests.
//!
//! Pins the "we accept this flag in clap but reject it before any
//! work happens" contracts for data-loss / silent-bug
//! combinations. Each test invokes the real binary against
//! tempdir paths and asserts (a) non-zero exit, (b) the
//! specific rejection message, (c) no side effects on the
//! source tree.
//!
//! Covers:
//!   - R54-F1: `--null` gated to local copy only
//!   - R54-F2: `--force` / `--ignore-times` rejected on move

use std::fs;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::Duration;

use tempfile::tempdir;
use wait_timeout::ChildExt;

fn run_with_timeout(mut cmd: Command, timeout: Duration) -> std::process::Output {
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = cmd.spawn().expect("spawn command");
    match child.wait_timeout(timeout).expect("wait for process") {
        Some(_) => child.wait_with_output().expect("collect output"),
        None => {
            let _ = child.kill();
            let output = child.wait_with_output().expect("output after kill");
            panic!(
                "command timed out after {:?}\nstdout:\n{}\nstderr:\n{}",
                timeout,
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
        }
    }
}

fn cli_bin() -> PathBuf {
    let exe_path = std::env::current_exe().expect("current_exe");
    let deps_dir = exe_path.parent().expect("test binary directory");
    let bin_dir = deps_dir
        .parent()
        .expect("deps parent directory")
        .to_path_buf();
    let name = if cfg!(windows) { "blit.exe" } else { "blit" };
    bin_dir.join(name)
}

fn assert_rejected(args: &[&str], expect_in_stderr: &str) {
    let mut cmd = Command::new(cli_bin());
    for a in args {
        cmd.arg(a);
    }
    let output = run_with_timeout(cmd, Duration::from_secs(15));
    assert!(
        !output.status.success(),
        "command must fail: {:?}\nstdout: {}\nstderr: {}",
        args,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains(expect_in_stderr),
        "expected stderr to contain {:?}, got:\n{}",
        expect_in_stderr,
        stderr
    );
}

// ── R54-F1: --null is local copy only ───────────────────────────────

/// `blit mirror --null` would still run apply_mirror_deletions
/// and physically delete destination-only files even though the
/// null sink discards writes. Refuse it.
#[test]
fn mirror_rejects_null_sink() {
    let tmp = tempdir().expect("tempdir");
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst).unwrap();
    fs::write(src.join("keep.txt"), b"src").unwrap();
    // Pre-existing destination-only file mirror would normally delete.
    fs::write(dst.join("stale.txt"), b"would have been purged").unwrap();

    assert_rejected(
        &[
            "mirror",
            "--null",
            "--yes",
            &format!("{}/", src.display()),
            &format!("{}/", dst.display()),
        ],
        "--null is not supported with `blit mirror`",
    );

    // The destination-only file must still be on disk — the
    // rejection fires before any work.
    assert!(
        dst.join("stale.txt").exists(),
        "dst/stale.txt must survive; mirror --null was rejected"
    );
}

/// `blit copy --null SRC remote:/dst/` silently ignored the
/// null flag on the remote-push path. Refuse it.
#[test]
fn copy_rejects_null_with_remote_destination() {
    let tmp = tempdir().expect("tempdir");
    let src = tmp.path().join("src");
    fs::create_dir_all(&src).unwrap();
    fs::write(src.join("file.txt"), b"payload").unwrap();

    assert_rejected(
        &[
            "copy",
            "--null",
            &format!("{}/", src.display()),
            "127.0.0.1:12349:/mod/",
        ],
        "--null is not supported with remote endpoints",
    );
}

/// `blit copy --null remote:/src/ /tmp/dst/` — same on pull.
#[test]
fn copy_rejects_null_with_remote_source() {
    let tmp = tempdir().expect("tempdir");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&dst).unwrap();

    assert_rejected(
        &[
            "copy",
            "--null",
            "127.0.0.1:12349:/mod/",
            &format!("{}/", dst.display()),
        ],
        "--null is not supported with remote endpoints",
    );
}

/// Sanity: local copy --null still works (this is the documented
/// supported case). Source file remains, destination unchanged
/// because writes are discarded.
#[test]
fn local_copy_null_still_accepted() {
    let tmp = tempdir().expect("tempdir");
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::write(src.join("file.txt"), b"payload").unwrap();

    let mut cmd = Command::new(cli_bin());
    cmd.arg("copy")
        .arg("--null")
        .arg(format!("{}/", src.display()))
        .arg(format!("{}/", dst.display()));
    let output = run_with_timeout(cmd, Duration::from_secs(15));
    assert!(
        output.status.success(),
        "local copy --null must succeed; stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    // Source intact.
    assert_eq!(fs::read(src.join("file.txt")).unwrap(), b"payload");
    // Destination has nothing (null sink discarded the write).
    assert!(!dst.join("file.txt").exists());
}

// ── R54-F2: --force / --ignore-times rejected on move ──────────────

#[test]
fn local_move_rejects_force_flag() {
    let tmp = tempdir().expect("tempdir");
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst).unwrap();
    fs::write(src.join("file.txt"), b"src content").unwrap();

    let mut cmd = Command::new(cli_bin());
    cmd.arg("move")
        .arg("--yes")
        .arg("--force")
        .arg(format!("{}/", src.display()))
        .arg(format!("{}/", dst.display()));
    let output = run_with_timeout(cmd, Duration::from_secs(15));
    assert!(
        !output.status.success(),
        "move --force must fail; stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("move does not support --force"),
        "expected R54-F2 --force rejection, got stderr: {}",
        stderr
    );
    // R55: the remediation must NOT recommend `blit copy --force`
    // or `blit copy --ignore-times` — those flags aren't plumbed
    // through the local or push paths either, so the recommended
    // workaround would have the same data-loss class as the move
    // we just rejected.
    assert!(
        !stderr.contains("blit copy --force") && !stderr.contains("blit copy --ignore-times"),
        "R55: error must not recommend `blit copy --force` / \
         `--ignore-times` — those copy flags aren't plumbed end-to-end \
         and reusing them would have the same skip-then-delete bug. \
         Got stderr:\n{}",
        stderr
    );
    assert!(
        src.join("file.txt").exists(),
        "src/file.txt must survive — move --force rejected before any work"
    );
}

#[test]
fn local_move_rejects_ignore_times_flag() {
    let tmp = tempdir().expect("tempdir");
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst).unwrap();
    fs::write(src.join("file.txt"), b"src content").unwrap();

    let mut cmd = Command::new(cli_bin());
    cmd.arg("move")
        .arg("--yes")
        .arg("--ignore-times")
        .arg(format!("{}/", src.display()))
        .arg(format!("{}/", dst.display()));
    let output = run_with_timeout(cmd, Duration::from_secs(15));
    assert!(
        !output.status.success(),
        "move --ignore-times must fail; stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("move does not support --ignore-times"),
        "expected R54-F2 --ignore-times rejection, got stderr: {}",
        stderr
    );
    assert!(
        !stderr.contains("blit copy --force") && !stderr.contains("blit copy --ignore-times"),
        "R55: error must not recommend `blit copy --force` / \
         `--ignore-times` as the workaround. Got stderr:\n{}",
        stderr
    );
    assert!(
        src.join("file.txt").exists(),
        "src/file.txt must survive — move --ignore-times rejected before any work"
    );
}
