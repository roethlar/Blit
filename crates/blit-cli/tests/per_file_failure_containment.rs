//! pfc-5 (D-2026-07-30-1): the end-of-operation per-file failure report,
//! exercised through the real CLI on a real local session.
//!
//! The session-level coverage owed by the pfc-3 landing notes. Everything
//! below the CLI is already pinned per-unit (sink classification, shard
//! folds, wire round trip); what only a process can prove is the operator
//! contract: a `copy` that keeps going, a block naming what did not land,
//! the distinct exit status, mirror still purging, and `move` refusing to
//! delete a source whose only copy of a file is still there.
//!
//! audit-17's closure shape is the first test: one filename the
//! destination filesystem rejects no longer aborts the whole transfer.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::Duration;

mod common;
use common::{cli_bin, run_with_timeout};

/// Exit status for "the operation completed, but some files did not land"
/// (`transfers::failures::EXIT_PARTIAL_FAILURE`). Duplicated as a literal
/// on purpose: an integration test pins the contract an operator's shell
/// sees, not the constant the binary happens to hold.
const EXIT_PARTIAL_FAILURE: i32 = 2;

/// A source tree with two files, one of which cannot be written at the
/// destination because a DIRECTORY already occupies its path — the
/// portable way to fail exactly one file's write (the same fixture the
/// sink's containment unit tests use).
fn one_blocked_file_fixture(root: &Path) -> (PathBuf, PathBuf) {
    let src = root.join("src");
    let dst = root.join("dst");
    fs::create_dir_all(&src).expect("mkdir src");
    fs::create_dir_all(&dst).expect("mkdir dst");
    fs::write(src.join("landed.txt"), b"alpha").expect("write landed");
    fs::write(src.join("blocked.txt"), b"never lands").expect("write blocked");
    fs::create_dir_all(dst.join("blocked.txt")).expect("block the destination path");
    (src, dst)
}

fn run_verb(verb: &str, extra: &[&str], src: &Path, dst: &Path) -> Output {
    let mut cmd = Command::new(cli_bin());
    cmd.arg(verb).arg("--yes");
    for arg in extra {
        cmd.arg(arg);
    }
    // Trailing slashes: rsync semantics — contents of src into dst.
    cmd.arg(format!("{}/", src.display()))
        .arg(format!("{}/", dst.display()));
    run_with_timeout(cmd, Duration::from_secs(60))
}

fn stdout_of(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr_of(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

/// audit-17's closure: a non-mirror `copy` survives one file the
/// destination cannot write. Everything else lands, the block names the
/// file and its reason, and the status is the partial-failure code — not
/// the abort that made the original report ("failed enumerating ~88k
/// entries in") and not a clean 0 either.
#[test]
fn non_mirror_copy_contains_the_failure_lands_the_rest_and_exits_two() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let (src, dst) = one_blocked_file_fixture(tmp.path());

    let output = run_verb("copy", &[], &src, &dst);
    let stdout = stdout_of(&output);

    assert_eq!(
        output.status.code(),
        Some(EXIT_PARTIAL_FAILURE),
        "a completed-with-failures copy exits {EXIT_PARTIAL_FAILURE}\nstdout:\n{stdout}\nstderr:\n{}",
        stderr_of(&output)
    );
    // ls-0 renamed the header ("Copy complete: N files, B in T" became
    // "Copy complete in T" plus a separate "• Copied:" line). The property
    // under guard is unchanged: the normal summary still prints BEFORE the
    // failure block, so a partial failure reads as "here is what landed,
    // and here is what did not" rather than as a bare error.
    assert!(
        stdout.contains("Copy complete in"),
        "the normal summary still prints first: {stdout}"
    );
    assert!(
        stdout.contains("• Copied: 1 file(s)"),
        "the landed files are still reported: {stdout}"
    );
    assert!(
        stdout.contains("1 file(s) could not be written"),
        "the block reports the count: {stdout}"
    );
    assert!(
        stdout.contains("blocked.txt"),
        "the block names the file: {stdout}"
    );
    assert!(
        stdout.contains("re-run the same command to converge"),
        "the block carries the re-run hint: {stdout}"
    );

    assert_eq!(
        fs::read(dst.join("landed.txt")).expect("the sibling landed"),
        b"alpha",
        "containment means the rest of the manifest transfers"
    );
    assert!(
        dst.join("blocked.txt").is_dir(),
        "no delete phase runs on a copy — the blocker is untouched"
    );
    assert!(
        src.join("blocked.txt").exists() && src.join("landed.txt").exists(),
        "a copy never removes anything from the source"
    );
}

/// The guard for the one summary shape that could swallow the report: a
/// run whose ONLY planned file fails copies zero files and classifies as
/// "up to date", whose summary line returns early. The block must still
/// print and the status must still be 2.
#[test]
fn a_wholly_failed_copy_still_reports_and_exits_two() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).expect("mkdir src");
    fs::create_dir_all(&dst).expect("mkdir dst");
    fs::write(src.join("blocked.txt"), b"never lands").expect("write blocked");
    fs::create_dir_all(dst.join("blocked.txt")).expect("block the destination path");

    let output = run_verb("copy", &[], &src, &dst);
    let stdout = stdout_of(&output);

    assert_eq!(
        output.status.code(),
        Some(EXIT_PARTIAL_FAILURE),
        "stdout:\n{stdout}\nstderr:\n{}",
        stderr_of(&output)
    );
    assert!(
        stdout.contains("1 file(s) could not be written") && stdout.contains("blocked.txt"),
        "a zero-copied run must not report only a clean summary: {stdout}"
    );
}

/// Q1(a): mirror's extraneous-delete phase still runs under a contained
/// per-file failure. The delete set is computed against a complete source
/// manifest and a write-failed file is IN that manifest, so copy failures
/// cannot corrupt it — the stale destination entry goes, the failed file
/// is reported, and the status is still 2.
#[test]
fn mirror_deletes_extraneous_entries_under_a_contained_failure() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let (src, dst) = one_blocked_file_fixture(tmp.path());
    fs::write(dst.join("stale.txt"), b"extraneous").expect("write stale");

    let output = run_verb("mirror", &[], &src, &dst);
    let stdout = stdout_of(&output);

    assert_eq!(
        output.status.code(),
        Some(EXIT_PARTIAL_FAILURE),
        "stdout:\n{stdout}\nstderr:\n{}",
        stderr_of(&output)
    );
    assert!(
        stdout.contains("1 file(s) could not be written") && stdout.contains("blocked.txt"),
        "the mirror reports its contained failure: {stdout}"
    );
    assert!(
        !dst.join("stale.txt").exists(),
        "the delete phase must still run under a contained failure (Q1(a))"
    );
    assert_eq!(
        fs::read(dst.join("landed.txt")).expect("the sibling landed"),
        b"alpha"
    );
}

/// Q1(b): a `move` whose destination could not write every file refuses
/// the source deletion entirely. The failed file's source copy is its only
/// copy — re-run to converge, then move. Both source files survive (the
/// refusal is whole-verb, not per-file), and the status is non-zero.
#[test]
fn move_refuses_source_deletion_while_a_file_failed_to_land() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let (src, dst) = one_blocked_file_fixture(tmp.path());

    let output = run_verb("move", &[], &src, &dst);
    let stdout = stdout_of(&output);
    let stderr = stderr_of(&output);

    assert!(
        !output.status.success(),
        "a refused move must not report success\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
    assert!(
        stderr.contains("refusing to remove source"),
        "the refusal says what it refused: {stderr}"
    );
    assert!(
        stderr.contains("blocked.txt"),
        "the refusal names the file that did not land: {stderr}"
    );
    assert!(
        stderr.contains("Re-run to converge, then move."),
        "the refusal says how to converge: {stderr}"
    );

    // The load-bearing assertion: the source is intact.
    assert_eq!(
        fs::read(src.join("blocked.txt")).expect("the failed file's only copy survives"),
        b"never lands"
    );
    assert_eq!(
        fs::read(src.join("landed.txt")).expect("the whole source survives the refusal"),
        b"alpha"
    );
    // R49-F3 posture: a refused verb leaves no success-looking summary.
    assert!(
        !stdout.contains("Copy complete:"),
        "a refused move must not print a transfer summary: {stdout}"
    );
    // What the destination could write is still there — containment, not
    // rollback.
    assert_eq!(
        fs::read(dst.join("landed.txt")).expect("the sibling landed"),
        b"alpha"
    );
}

/// `--json`: machine consumers get BOTH signals — the summary document
/// carries `files_failed` plus the named `failures`, and the process still
/// exits with the partial-failure status.
#[test]
fn json_mode_carries_the_failure_fields_and_still_exits_two() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let (src, dst) = one_blocked_file_fixture(tmp.path());

    let output = run_verb("copy", &["--json"], &src, &dst);
    let stdout = stdout_of(&output);

    assert_eq!(
        output.status.code(),
        Some(EXIT_PARTIAL_FAILURE),
        "stdout:\n{stdout}\nstderr:\n{}",
        stderr_of(&output)
    );
    let document: serde_json::Value = serde_json::from_str(stdout.trim())
        .unwrap_or_else(|err| panic!("stdout must be one JSON document ({err}):\n{stdout}"));
    assert_eq!(document["files_failed"], 1);
    assert_eq!(document["failures"][0]["relative_path"], "blocked.txt");
    assert!(
        document["failures"][0]["reason"]
            .as_str()
            .is_some_and(|reason| !reason.is_empty()),
        "a carried failure names its reason: {document}"
    );
    assert!(
        !stdout.contains("could not be written"),
        "the human block must not pollute the JSON document: {stdout}"
    );
}
