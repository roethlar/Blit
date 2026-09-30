//! ssc-6 (D-2026-09-28-1, D-2026-09-28-3): the end-of-run retry passes,
//! exercised through the real CLI on a real local session (A20).
//!
//! The owner's rule — "collect all errors, then … retry at the end of the
//! transfer that will rescan and retry" — with robocopy-shaped switches:
//! `--retries N` passes over the files that failed, `--retry-wait S`
//! seconds before each. What only a process can prove: a file that is
//! freed while the run waits lands on the retry and the run exits 0; a
//! file that keeps failing is reported once, marked `(retried)`, after
//! exactly N passes; a clean run retries nothing; `--retries 0` is today's
//! behaviour; the wait is honoured (recorded, and not slept when the
//! hidden diagnostics switch says so); mirror still deletes and still
//! shields; move deletes the source only once everything landed.
//!
//! Timing: the run records `retry_wait_seconds` to the diagnostics
//! counter file the moment a wait begins, so a test can free the blocked
//! file inside that window without racing the transfer.
//!
//! Why every fixture fails on the DESTINATION side: a source file a test
//! makes unreadable before the run is caught by the scan (an
//! `unreadable_paths` entry, not a per-file failure), and a source that
//! fails only at payload time needs a change between scan and read that
//! no outside process can time. The source-side skip/retract path is
//! pinned at the session level (`source_side_containment.rs`); the loop's
//! own unit tests drive it with source-shaped failures; here the CLI
//! proves the loop against the failure a process can stage.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::time::{Duration, Instant};

mod common;
use common::{cli_bin, run_with_timeout};

const EXIT_PARTIAL_FAILURE: i32 = 2;

/// Two source files; one cannot land because a DIRECTORY occupies its
/// destination path (the pfc fixture). Removing that directory is how a
/// test "frees" the file between passes.
fn one_blocked_file_fixture(root: &Path) -> (PathBuf, PathBuf) {
    let src = root.join("src");
    let dst = root.join("dst");
    fs::create_dir_all(&src).expect("mkdir src");
    fs::create_dir_all(&dst).expect("mkdir dst");
    fs::write(src.join("landed.txt"), b"alpha").expect("write landed");
    fs::write(src.join("blocked.txt"), b"lands on retry").expect("write blocked");
    fs::create_dir_all(dst.join("blocked.txt")).expect("block the destination path");
    (src, dst)
}

fn command(verb: &str, extra: &[&str], src: &Path, dst: &Path, counters: &Path) -> Command {
    let mut cmd = Command::new(cli_bin());
    cmd.arg("--diagnostics-counter-file").arg(counters);
    cmd.arg(verb).arg("--yes");
    for arg in extra {
        cmd.arg(arg);
    }
    cmd.arg(format!("{}/", src.display()))
        .arg(format!("{}/", dst.display()));
    cmd
}

fn run(verb: &str, extra: &[&str], src: &Path, dst: &Path, counters: &Path) -> Output {
    run_with_timeout(
        command(verb, extra, src, dst, counters),
        Duration::from_secs(90),
    )
}

fn spawn(verb: &str, extra: &[&str], src: &Path, dst: &Path, counters: &Path) -> Child {
    command(verb, extra, src, dst, counters)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn blit")
}

fn counter_lines(counters: &Path, event: &str) -> Vec<u64> {
    fs::read_to_string(counters)
        .unwrap_or_default()
        .lines()
        .filter_map(|line| {
            let (name, value) = line.split_once(' ')?;
            (name == event).then(|| value.parse::<u64>().ok()).flatten()
        })
        .collect()
}

/// Block until the counter file carries `event`, or fail loudly.
fn wait_for_counter(counters: &Path, event: &str, timeout: Duration) {
    let start = Instant::now();
    while start.elapsed() < timeout {
        if !counter_lines(counters, event).is_empty() {
            return;
        }
        std::thread::sleep(Duration::from_millis(25));
    }
    panic!("no `{event}` counter within {timeout:?}");
}

fn finish(child: Child) -> Output {
    let start = Instant::now();
    let output = child.wait_with_output().expect("wait for blit");
    assert!(
        start.elapsed() < Duration::from_secs(120),
        "blit did not finish in time"
    );
    output
}

fn stdout_of(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr_of(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

/// A20 (a): the file fails the main pass, is freed during the wait, and
/// lands on the retry — exit 0, no failure block, one retry pass recorded.
#[test]
fn a_file_freed_during_the_wait_lands_on_the_retry_pass() {
    let temp = tempfile::tempdir().expect("tempdir");
    let (src, dst) = one_blocked_file_fixture(temp.path());
    let counters = temp.path().join("counters.txt");
    let child = spawn(
        "copy",
        &["--retries", "1", "--retry-wait", "4"],
        &src,
        &dst,
        &counters,
    );
    wait_for_counter(&counters, "retry_wait_seconds", Duration::from_secs(60));
    fs::remove_dir_all(dst.join("blocked.txt")).expect("free the blocked path");
    let output = finish(child);
    let stdout = stdout_of(&output);
    let stderr = stderr_of(&output);
    assert_eq!(
        output.status.code(),
        Some(0),
        "the retry landed the file\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
    assert_eq!(
        fs::read(dst.join("blocked.txt")).expect("landed on retry"),
        b"lands on retry"
    );
    assert!(
        !stdout.contains("did not land") && !stdout.contains("blocked.txt"),
        "no failure block after a successful retry:\n{stdout}"
    );
    // cr-ssc6-2: the run transferred a file; it is not "up to date".
    assert!(
        !stdout.contains("Up to date"),
        "a file landed on retry, so the outcome is a transfer:\n{stdout}"
    );
    assert!(
        stderr.contains("retrying 1 file(s) (pass 1 of 1)"),
        "the pass is announced:\n{stderr}"
    );
    assert_eq!(counter_lines(&counters, "retry_pass"), vec![1]);
    assert_eq!(counter_lines(&counters, "retry_wait_seconds"), vec![4]);
}

/// A20 (b): a file that fails on every pass is reported once, marked
/// `(retried)`, after exactly `--retries` passes; exit 2.
#[test]
fn a_persistent_failure_is_reported_once_after_exactly_n_passes() {
    let temp = tempfile::tempdir().expect("tempdir");
    let (src, dst) = one_blocked_file_fixture(temp.path());
    let counters = temp.path().join("counters.txt");
    let output = run(
        "copy",
        &["--retries", "2", "--diagnostics-no-retry-wait"],
        &src,
        &dst,
        &counters,
    );
    let stdout = stdout_of(&output);
    assert_eq!(output.status.code(), Some(EXIT_PARTIAL_FAILURE), "{stdout}");
    assert_eq!(counter_lines(&counters, "retry_pass"), vec![1, 2]);
    assert_eq!(
        counter_lines(&counters, "retry_wait_seconds"),
        vec![30, 30],
        "the default wait is honoured (recorded) before each pass"
    );
    assert_eq!(
        stdout.matches("blocked.txt").count(),
        1,
        "reported exactly once:\n{stdout}"
    );
    assert!(stdout.contains("(retried)"), "marked as retried:\n{stdout}");
    assert_eq!(
        fs::read(dst.join("landed.txt")).expect("landed"),
        b"alpha",
        "the rest of the manifest landed on the main pass"
    );
    assert!(dst.join("blocked.txt").is_dir(), "the blocker is untouched");
}

/// A20 (b), JSON shape: the final state is what the document carries.
#[test]
fn json_carries_the_post_retry_state() {
    let temp = tempfile::tempdir().expect("tempdir");
    let (src, dst) = one_blocked_file_fixture(temp.path());
    let counters = temp.path().join("counters.txt");
    let output = run(
        "copy",
        &["--json", "--retries", "1", "--diagnostics-no-retry-wait"],
        &src,
        &dst,
        &counters,
    );
    assert_eq!(output.status.code(), Some(EXIT_PARTIAL_FAILURE));
    let stdout = stdout_of(&output);
    let doc: serde_json::Value = serde_json::from_str(stdout.trim()).expect("one JSON document");
    assert_eq!(doc["files_failed"], 1);
    let failures = doc["failures"].as_array().expect("failures array");
    assert_eq!(failures.len(), 1);
    assert_eq!(failures[0]["relative_path"], "blocked.txt");
    assert!(
        failures[0]["reason"]
            .as_str()
            .expect("reason")
            .ends_with("(retried)"),
        "{doc}"
    );
    assert!(
        !stderr_of(&output).contains("retrying"),
        "no human notice in JSON mode"
    );
}

/// A20 (c): a clean run performs no wait, no retry pass, no extra scan.
#[test]
fn a_clean_run_retries_nothing() {
    let temp = tempfile::tempdir().expect("tempdir");
    let src = temp.path().join("src");
    let dst = temp.path().join("dst");
    fs::create_dir_all(&src).expect("mkdir src");
    fs::write(src.join("a.txt"), b"a").expect("write");
    let counters = temp.path().join("counters.txt");
    let output = run("copy", &["--retries", "3"], &src, &dst, &counters);
    assert_eq!(output.status.code(), Some(0), "{}", stderr_of(&output));
    assert!(counter_lines(&counters, "retry_pass").is_empty());
    assert!(counter_lines(&counters, "retry_wait_seconds").is_empty());
    assert!(!stderr_of(&output).contains("retrying"));
}

/// A20 (d): `--retries 0` is today's behaviour — no pass, exit 2, the
/// reason unmarked.
#[test]
fn retries_zero_performs_no_pass() {
    let temp = tempfile::tempdir().expect("tempdir");
    let (src, dst) = one_blocked_file_fixture(temp.path());
    let counters = temp.path().join("counters.txt");
    let output = run("copy", &["--retries", "0"], &src, &dst, &counters);
    let stdout = stdout_of(&output);
    assert_eq!(output.status.code(), Some(EXIT_PARTIAL_FAILURE));
    assert!(counter_lines(&counters, "retry_pass").is_empty());
    assert!(counter_lines(&counters, "retry_wait_seconds").is_empty());
    assert!(stdout.contains("blocked.txt") && !stdout.contains("(retried)"));
}

/// A20 (e): the wait is the one asked for, recorded before each pass;
/// with the diagnostics switch it is recorded but not slept (the run
/// finishes far inside the wait it would otherwise take).
#[test]
fn the_requested_wait_is_honoured() {
    let temp = tempfile::tempdir().expect("tempdir");
    let (src, dst) = one_blocked_file_fixture(temp.path());
    let counters = temp.path().join("counters.txt");
    let start = Instant::now();
    let output = run(
        "copy",
        &[
            "--retries",
            "1",
            "--retry-wait",
            "600",
            "--diagnostics-no-retry-wait",
        ],
        &src,
        &dst,
        &counters,
    );
    assert_eq!(output.status.code(), Some(EXIT_PARTIAL_FAILURE));
    assert_eq!(counter_lines(&counters, "retry_wait_seconds"), vec![600]);
    assert!(
        start.elapsed() < Duration::from_secs(60),
        "the diagnostics switch replaced the sleep"
    );
}

/// A20 (f): mirror + retry — extraneous entries are still deleted, the
/// persistent failure's destination subtree is still shielded, and a
/// file freed during the wait lands.
#[test]
fn mirror_with_retries_still_deletes_and_still_shields() {
    let temp = tempfile::tempdir().expect("tempdir");
    let (src, dst) = one_blocked_file_fixture(temp.path());
    fs::write(dst.join("blocked.txt").join("inner.txt"), b"keep").expect("populate blocker");
    fs::write(dst.join("stale.txt"), b"extraneous").expect("stale");
    let counters = temp.path().join("counters.txt");
    let output = run(
        "mirror",
        &["--retries", "1", "--diagnostics-no-retry-wait"],
        &src,
        &dst,
        &counters,
    );
    assert_eq!(output.status.code(), Some(EXIT_PARTIAL_FAILURE));
    assert!(!dst.join("stale.txt").exists(), "extraneous entry deleted");
    assert_eq!(
        fs::read(dst.join("blocked.txt").join("inner.txt")).expect("shielded"),
        b"keep",
        "the failed path's destination subtree is shielded (A19)"
    );
    assert_eq!(counter_lines(&counters, "retry_pass"), vec![1]);

    // Freed during the wait: the file lands on the retry, exit 0.
    let temp = tempfile::tempdir().expect("tempdir");
    let (src, dst) = one_blocked_file_fixture(temp.path());
    fs::write(dst.join("stale.txt"), b"extraneous").expect("stale");
    let counters = temp.path().join("counters.txt");
    let child = spawn(
        "mirror",
        &["--retries", "1", "--retry-wait", "4"],
        &src,
        &dst,
        &counters,
    );
    wait_for_counter(&counters, "retry_wait_seconds", Duration::from_secs(60));
    fs::remove_dir_all(dst.join("blocked.txt")).expect("free the blocked path");
    let output = finish(child);
    assert_eq!(output.status.code(), Some(0), "{}", stderr_of(&output));
    assert!(!dst.join("stale.txt").exists(), "extraneous entry deleted");
    assert_eq!(
        fs::read(dst.join("blocked.txt")).expect("landed on retry"),
        b"lands on retry"
    );
}

/// A20 (g): move + retry — the source is deleted only once everything has
/// landed by the end: freed during the wait → moved; persistent → refused,
/// source intact.
#[test]
fn move_with_retries_deletes_the_source_only_once_everything_landed() {
    let temp = tempfile::tempdir().expect("tempdir");
    let (src, dst) = one_blocked_file_fixture(temp.path());
    let counters = temp.path().join("counters.txt");
    let child = spawn(
        "move",
        &["--retries", "1", "--retry-wait", "4"],
        &src,
        &dst,
        &counters,
    );
    wait_for_counter(&counters, "retry_wait_seconds", Duration::from_secs(60));
    fs::remove_dir_all(dst.join("blocked.txt")).expect("free the blocked path");
    let output = finish(child);
    assert_eq!(output.status.code(), Some(0), "{}", stderr_of(&output));
    assert!(!src.exists(), "source removed after everything landed");
    assert_eq!(
        fs::read(dst.join("blocked.txt")).expect("landed on retry"),
        b"lands on retry"
    );

    let temp = tempfile::tempdir().expect("tempdir");
    let (src, dst) = one_blocked_file_fixture(temp.path());
    let counters = temp.path().join("counters.txt");
    let output = run(
        "move",
        &["--retries", "1", "--diagnostics-no-retry-wait"],
        &src,
        &dst,
        &counters,
    );
    assert_ne!(output.status.code(), Some(0), "the move is refused");
    assert!(
        stderr_of(&output).contains("refusing to remove source"),
        "{}",
        stderr_of(&output)
    );
    assert!(src.join("blocked.txt").is_file(), "source intact");
    assert_eq!(counter_lines(&counters, "retry_pass"), vec![1]);
}

/// cr-ssc6-1: a failed file that is gone from the SOURCE by the time the
/// retry pass scans is still a failure — the pass names it as missing —
/// so the run exits 2 and says so instead of reporting a clean retry
/// while the destination lacks the file.
#[test]
fn a_failed_file_missing_at_retry_stays_reported() {
    let temp = tempfile::tempdir().expect("tempdir");
    let (src, dst) = one_blocked_file_fixture(temp.path());
    let counters = temp.path().join("counters.txt");
    let child = spawn(
        "copy",
        &["--retries", "1", "--retry-wait", "4"],
        &src,
        &dst,
        &counters,
    );
    wait_for_counter(&counters, "retry_wait_seconds", Duration::from_secs(60));
    // Free the destination so a retry COULD land it, then remove the
    // source: the retry scan has nothing to re-land.
    fs::remove_dir_all(dst.join("blocked.txt")).expect("free the blocked path");
    fs::remove_file(src.join("blocked.txt")).expect("remove the source file");
    let output = finish(child);
    let stdout = stdout_of(&output);
    let stderr = stderr_of(&output);
    assert_eq!(
        output.status.code(),
        Some(EXIT_PARTIAL_FAILURE),
        "a file the retry could not find is still a failure\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
    assert!(
        !dst.join("blocked.txt").exists(),
        "nothing landed for the missing file"
    );
    assert!(
        stdout.contains("blocked.txt") && stdout.contains("missing at retry"),
        "the failure block names the file and says it was missing at retry:\n{stdout}"
    );
    assert_eq!(counter_lines(&counters, "retry_pass"), vec![1]);
}

/// cr-ssc6-2: when the sole file lands only on the retry pass, the
/// final outcome is a transfer (not "up to date") and the duration spans
/// the whole operation, the retry wait included.
#[test]
fn a_retry_landed_file_reports_a_transfer_and_the_whole_duration() {
    let temp = tempfile::tempdir().expect("tempdir");
    let (src, dst) = one_blocked_file_fixture(temp.path());
    let counters = temp.path().join("counters.txt");
    let child = spawn(
        "copy",
        &["--json", "--retries", "1", "--retry-wait", "2"],
        &src,
        &dst,
        &counters,
    );
    wait_for_counter(&counters, "retry_wait_seconds", Duration::from_secs(60));
    fs::remove_dir_all(dst.join("blocked.txt")).expect("free the blocked path");
    let output = finish(child);
    let stdout = stdout_of(&output);
    assert_eq!(output.status.code(), Some(0), "{stdout}");
    let doc: serde_json::Value = serde_json::from_str(stdout.trim()).expect("one JSON document");
    assert_eq!(doc["outcome"], "transferred", "{doc}");
    assert_eq!(doc["files_failed"], 0, "{doc}");
    assert!(
        doc["files_transferred"].as_u64().unwrap_or(0) >= 1,
        "the retry-landed file is counted: {doc}"
    );
    assert!(
        doc["duration_ms"].as_u64().unwrap_or(0) >= 2000,
        "the duration spans the retry wait: {doc}"
    );
}
