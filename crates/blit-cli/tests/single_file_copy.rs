//! Regression tests for the "single-file source silent no-op" bug.
//!
//! See `docs/bugs/single-file-source-silent-noop.md`. Any local-to-local
//! copy with a file source must actually copy the file, not silently
//! report "0 files" with success.

use std::fs;
use std::process::Command;
use std::time::Duration;

mod common;
use common::{cli_bin, run_with_timeout};

fn run_copy(args: &[&str]) -> std::process::Output {
    let bin = cli_bin();
    let mut cmd = Command::new(&bin);
    cmd.arg("copy").arg("--yes");
    for a in args {
        cmd.arg(a);
    }
    run_with_timeout(cmd, Duration::from_secs(30))
}

#[test]
fn single_file_to_dir_with_trailing_slash() {
    let tmp = tempfile::tempdir().unwrap();
    let src = tmp.path().join("file.txt");
    let dst = tmp.path().join("dst");
    fs::write(&src, b"hello world").unwrap();
    fs::create_dir_all(&dst).unwrap();

    let dst_arg = format!("{}/", dst.display());
    let output = run_copy(&[&src.to_string_lossy(), &dst_arg]);
    assert!(
        output.status.success(),
        "blit copy failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(fs::read(dst.join("file.txt")).unwrap(), b"hello world");
}

#[test]
fn single_file_to_existing_dir_no_slash() {
    let tmp = tempfile::tempdir().unwrap();
    let src = tmp.path().join("file.txt");
    let dst = tmp.path().join("dst");
    fs::write(&src, b"hello world").unwrap();
    fs::create_dir_all(&dst).unwrap();

    let output = run_copy(&[&src.to_string_lossy(), &dst.to_string_lossy()]);
    assert!(output.status.success());
    assert_eq!(fs::read(dst.join("file.txt")).unwrap(), b"hello world");
}

#[test]
fn single_file_rename_to_exact_path() {
    let tmp = tempfile::tempdir().unwrap();
    let src = tmp.path().join("file.txt");
    let dst_dir = tmp.path().join("dst");
    let renamed = dst_dir.join("renamed.txt");
    fs::write(&src, b"hello world").unwrap();
    fs::create_dir_all(&dst_dir).unwrap();

    let output = run_copy(&[&src.to_string_lossy(), &renamed.to_string_lossy()]);
    assert!(output.status.success());
    assert_eq!(fs::read(&renamed).unwrap(), b"hello world");
}

#[test]
fn single_file_copy_reports_nonzero_files() {
    let tmp = tempfile::tempdir().unwrap();
    let src = tmp.path().join("file.txt");
    let dst = tmp.path().join("dst");
    fs::write(&src, b"hello world").unwrap();
    fs::create_dir_all(&dst).unwrap();

    let dst_arg = format!("{}/", dst.display());
    let output = run_copy(&[&src.to_string_lossy(), &dst_arg]);
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    // Must report "1 files" not "0 files" — the original bug silently
    // reported 0 while not copying anything.
    assert!(
        stdout.contains("1 files"),
        "expected '1 files' in summary, got:\n{}",
        stdout
    );
}

#[test]
fn single_file_copy_idempotent() {
    let tmp = tempfile::tempdir().unwrap();
    let src = tmp.path().join("file.txt");
    let dst = tmp.path().join("dst");
    fs::write(&src, b"hello world").unwrap();
    fs::create_dir_all(&dst).unwrap();

    let dst_arg = format!("{}/", dst.display());
    // First copy: transfers. Must report "1 files" (Transferred outcome).
    let out1 = run_copy(&[&src.to_string_lossy(), &dst_arg]);
    assert!(out1.status.success());
    assert_eq!(fs::read(dst.join("file.txt")).unwrap(), b"hello world");
    let stdout1 = String::from_utf8_lossy(&out1.stdout);
    assert!(
        stdout1.contains("Copy complete: 1 files"),
        "first run should report 'Copy complete: 1 files', got:\n{}",
        stdout1
    );

    // Second copy: skip_unchanged should detect the match and emit a
    // distinct "Up to date" summary — NOT "Copy complete: 0 files",
    // which is the regression this test guards against.
    let out2 = run_copy(&[&src.to_string_lossy(), &dst_arg]);
    assert!(out2.status.success());
    assert_eq!(fs::read(dst.join("file.txt")).unwrap(), b"hello world");
    let stdout2 = String::from_utf8_lossy(&out2.stdout);
    assert!(
        stdout2.contains("Up to date"),
        "second run should report 'Up to date', got:\n{}",
        stdout2
    );
    assert!(
        !stdout2.contains("Copy complete: 0 files"),
        "second run must not print misleading 'Copy complete: 0 files', got:\n{}",
        stdout2
    );
}

#[test]
fn single_file_missing_source_fails() {
    let tmp = tempfile::tempdir().unwrap();
    let missing = tmp.path().join("does-not-exist.txt");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&dst).unwrap();

    let output = run_copy(&[&missing.to_string_lossy(), &dst.to_string_lossy()]);
    assert!(
        !output.status.success(),
        "blit copy should fail when source is missing"
    );
}
