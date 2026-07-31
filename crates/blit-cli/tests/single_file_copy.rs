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
    // Must report one copied file, not zero — the original bug silently
    // reported 0 while not copying anything. ls-0 moved the count onto its
    // own labelled line; the property under guard is unchanged.
    assert!(
        stdout.contains("• Copied: 1 file(s)"),
        "expected one copied file in summary, got:\n{}",
        stdout
    );
    assert!(
        !stdout.contains("• Copied: 0 file(s)"),
        "the zero-file regression must not return, got:\n{}",
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
    // First copy: transfers. ls-0 moved the count off the header onto its
    // own "• Copied:" line, so the Transferred outcome now shows as a
    // header plus a copied line rather than "Copy complete: 1 files".
    let out1 = run_copy(&[&src.to_string_lossy(), &dst_arg]);
    assert!(out1.status.success());
    assert_eq!(fs::read(dst.join("file.txt")).unwrap(), b"hello world");
    let stdout1 = String::from_utf8_lossy(&out1.stdout);
    assert!(
        stdout1.contains("Copy complete in"),
        "first run should report the Transferred header, got:\n{}",
        stdout1
    );
    assert!(
        stdout1.contains("• Copied: 1 file(s)"),
        "first run should report one copied file, got:\n{}",
        stdout1
    );

    // Second copy: skip_unchanged should detect the match and emit a
    // distinct "Up to date" summary — NOT a zero-file copy report, which
    // is the regression this test guards against.
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
        !stdout2.contains("• Copied: 0 file(s)"),
        "second run must not print a misleading zero-file copy line, got:\n{}",
        stdout2
    );
}

/// ls-0: the end-of-run summary must not read as self-contradictory.
///
/// The owner's field run printed `Mirror complete: 9578 files, 393.01 MiB in
/// 355.23s` immediately above `Repaired metadata on 5445 file(s) — no bytes
/// re-sent` and read it as incoherent: a byte total next to "no bytes
/// re-sent". The two counts describe DISJOINT sets and always did, but
/// nothing on screen said so. This pins the shape that fixes it, against the
/// real binary's real stdout.
#[test]
fn the_summary_separates_copied_files_from_the_byte_rate() {
    let tmp = tempfile::tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst).unwrap();
    // Two files so the throughput/workers block is not suppressed as
    // small-transfer noise.
    fs::write(src.join("a.bin"), vec![b'a'; 4096]).unwrap();
    fs::write(src.join("b.bin"), vec![b'b'; 4096]).unwrap();

    let out = run_copy(&[
        &format!("{}/", src.display()),
        &format!("{}/", dst.display()),
    ]);
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);

    // The header commits to no count, so it can never sit in tension with a
    // separate repaired-file line beneath it.
    assert!(
        stdout.contains("Copy complete in"),
        "header should carry the duration only, got:\n{stdout}"
    );
    assert!(
        !stdout.contains("complete: 2 files"),
        "the header must not carry a file count again, got:\n{stdout}"
    );
    // Copied files are their own labelled line.
    assert!(
        stdout.contains("• Copied: 2 file(s)"),
        "copied files need their own line, got:\n{stdout}"
    );
    // The rate says what it averages over. Unlabelled, it reads as a copy
    // rate, which is what made 1.11 MiB/s look like a transfer speed when
    // most of the wall clock was scanning.
    assert!(
        stdout.contains("• Average:") && stdout.contains("includes scan and compare"),
        "the rate must state its interval, got:\n{stdout}"
    );
    assert!(
        !stdout.contains("• Throughput:"),
        "the unqualified 'Throughput' label is what misled, got:\n{stdout}"
    );
    // The worker count stays visible: it is the defect, not noise to hide.
    assert!(
        stdout.contains("• Workers used: 1"),
        "the effective worker count must stay on screen, got:\n{stdout}"
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
