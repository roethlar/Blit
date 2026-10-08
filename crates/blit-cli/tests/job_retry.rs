//! JOB_LOGS jl-4: `blit jobs retry <job-id|file>` sends again only the
//! files a run failed to send, with the run's own options, as a new run
//! (the next attempt, its parent recorded); it refuses a job of another
//! machine, a run still going, and a run whose failures are not all
//! known. A mirror's retry deletes nothing; a move's retry finishes the
//! move.

mod common;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::Duration;

use common::{cli_bin, run_with_timeout, TestContext};

const EXIT_PARTIAL_FAILURE: i32 = 2;

fn blit(ctx: &TestContext, args: &[&str]) -> Output {
    let mut cmd = Command::new(cli_bin());
    cmd.current_dir(&ctx.workspace)
        .arg("--config-dir")
        .arg(&ctx.config_dir)
        .args(args);
    run_with_timeout(cmd, Duration::from_secs(60))
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

/// Two source files; one cannot land because a folder occupies its
/// destination path. Removing the folder frees it.
fn blocked_fixture(ctx: &TestContext) -> (PathBuf, PathBuf) {
    let src = ctx.workspace.join("src");
    let dst = ctx.workspace.join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(dst.join("blocked.txt")).unwrap();
    fs::write(src.join("landed.txt"), b"alpha").unwrap();
    fs::write(src.join("blocked.txt"), b"lands on retry").unwrap();
    (src, dst)
}

fn arg(path: &Path) -> String {
    format!("{}/", path.display())
}

/// The jobs this machine ran, newest first.
fn jobs(ctx: &TestContext) -> Vec<serde_json::Value> {
    let listed = blit(ctx, &["jobs", "list", "--json"]);
    assert!(listed.status.success(), "{}", text(&listed.stderr));
    let listing: serde_json::Value = serde_json::from_slice(&listed.stdout).unwrap();
    listing["jobs"].as_array().unwrap().clone()
}

#[test]
fn a_copys_retry_sends_only_the_files_that_failed() {
    let ctx = TestContext::new();
    let (src, dst) = blocked_fixture(&ctx);
    let first = blit(&ctx, &["copy", "--retries", "0", &arg(&src), &arg(&dst)]);
    assert_eq!(
        first.status.code(),
        Some(EXIT_PARTIAL_FAILURE),
        "{}",
        text(&first.stderr)
    );
    let run_id = jobs(&ctx)[0]["run_id"].as_str().unwrap().to_string();

    // The landed file changes at the source: a whole run would send it
    // again, a retry must not.
    fs::write(src.join("landed.txt"), b"changed since").unwrap();
    fs::remove_dir_all(dst.join("blocked.txt")).unwrap();
    let retried = blit(&ctx, &["jobs", "retry", &run_id]);
    assert!(retried.status.success(), "{}", text(&retried.stderr));
    assert_eq!(
        fs::read(dst.join("blocked.txt")).unwrap(),
        b"lands on retry"
    );
    assert_eq!(fs::read(dst.join("landed.txt")).unwrap(), b"alpha");

    let retry = &jobs(&ctx)[0];
    assert_eq!(retry["parent"], run_id.as_str(), "{retry}");
    assert_eq!(retry["attempt"], 2, "{retry}");
    assert_eq!(retry["state"], "finished", "{retry}");
    assert_eq!(retry["outcome"], "ok", "{retry}");
    assert_eq!(retry["files_copied"], 1, "{retry}");

    // Nothing failed this time: nothing to retry.
    let again = blit(&ctx, &["jobs", "retry", retry["run_id"].as_str().unwrap()]);
    assert!(again.status.success(), "{}", text(&again.stderr));
    assert!(
        text(&again.stdout).contains("failed no files; nothing to retry"),
        "{}",
        text(&again.stdout)
    );
    assert_eq!(jobs(&ctx).len(), 2, "no run was made");
}

#[test]
fn a_mirrors_retry_deletes_nothing() {
    let ctx = TestContext::new();
    let (src, dst) = blocked_fixture(&ctx);
    let first = blit(
        &ctx,
        &[
            "mirror",
            "--yes",
            "--retries",
            "0",
            "--delete-scope",
            "all",
            &arg(&src),
            &arg(&dst),
        ],
    );
    assert_eq!(
        first.status.code(),
        Some(EXIT_PARTIAL_FAILURE),
        "{}",
        text(&first.stderr)
    );
    let run_id = jobs(&ctx)[0]["run_id"].as_str().unwrap().to_string();

    // A file only the destination holds, put there after the mirror ran.
    fs::write(dst.join("late.txt"), b"keep me").unwrap();
    fs::remove_dir_all(dst.join("blocked.txt")).unwrap();
    let retried = blit(&ctx, &["jobs", "retry", &run_id]);
    assert!(retried.status.success(), "{}", text(&retried.stderr));
    assert_eq!(
        fs::read(dst.join("blocked.txt")).unwrap(),
        b"lands on retry"
    );
    assert_eq!(fs::read(dst.join("late.txt")).unwrap(), b"keep me");
}

#[test]
fn a_moves_retry_finishes_the_move() {
    let ctx = TestContext::new();
    let (src, dst) = blocked_fixture(&ctx);
    let first = blit(
        &ctx,
        &["move", "--yes", "--retries", "0", &arg(&src), &arg(&dst)],
    );
    assert!(!first.status.success(), "the move cannot finish");
    assert!(src.join("blocked.txt").is_file(), "its source stays");
    let run_id = jobs(&ctx)[0]["run_id"].as_str().unwrap().to_string();

    fs::remove_dir_all(dst.join("blocked.txt")).unwrap();
    let retried = blit(&ctx, &["jobs", "retry", &run_id]);
    assert!(retried.status.success(), "{}", text(&retried.stderr));
    assert_eq!(
        fs::read(dst.join("blocked.txt")).unwrap(),
        b"lands on retry"
    );
    assert_eq!(fs::read(dst.join("landed.txt")).unwrap(), b"alpha");
    assert!(
        !src.exists(),
        "the move removed its source once everything landed"
    );
    let retry = &jobs(&ctx)[0];
    assert_eq!(retry["verb"], "move", "{retry}");
    assert_eq!(retry["parent"], run_id.as_str(), "{retry}");
}

#[test]
fn a_job_file_retries_on_its_own_machine_only() {
    let ctx = TestContext::new();
    let (src, dst) = blocked_fixture(&ctx);
    let first = blit(
        &ctx,
        &[
            "copy",
            "--retries",
            "0",
            "--export",
            "job.json",
            &arg(&src),
            &arg(&dst),
        ],
    );
    assert_eq!(
        first.status.code(),
        Some(EXIT_PARTIAL_FAILURE),
        "{}",
        text(&first.stderr)
    );
    let path = ctx.workspace.join("job.json");

    // Another machine's copy of the file is refused, naming the machine.
    let mut foreign: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    foreign["spec"]["machine"] = "ffffffffffffffffffffffffffffffff".into();
    foreign["spec"]["host"] = "faraway".into();
    fs::write(
        ctx.workspace.join("foreign.json"),
        serde_json::to_vec(&foreign).unwrap(),
    )
    .unwrap();
    let refused = blit(&ctx, &["jobs", "retry", "./foreign.json"]);
    assert!(!refused.status.success());
    assert!(
        text(&refused.stderr)
            .contains("belongs to machine ffffffffffffffffffffffffffffffff (faraway)"),
        "{}",
        text(&refused.stderr)
    );

    // Its own runs from the file.
    fs::remove_dir_all(dst.join("blocked.txt")).unwrap();
    let retried = blit(&ctx, &["jobs", "retry", "./job.json"]);
    assert!(retried.status.success(), "{}", text(&retried.stderr));
    assert_eq!(
        fs::read(dst.join("blocked.txt")).unwrap(),
        b"lands on retry"
    );
}

#[test]
fn a_run_still_waiting_on_its_daemon_is_not_retried() {
    let ctx = TestContext::new();
    let (src, dst) = blocked_fixture(&ctx);
    let first = blit(&ctx, &["copy", "--retries", "0", &arg(&src), &arg(&dst)]);
    assert_eq!(first.status.code(), Some(EXIT_PARTIAL_FAILURE));
    let job = &jobs(&ctx)[0];
    let run_id = job["run_id"].as_str().unwrap().to_string();
    // As a `--detach` run leaves it: waiting on a daemon, here one that
    // cannot be reached.
    let port = std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let record_file = PathBuf::from(job["record_file"].as_str().unwrap());
    let mut record: serde_json::Value =
        serde_json::from_slice(&fs::read(&record_file).unwrap()).unwrap();
    record["state"] = serde_json::json!({
        "waiting": {"daemon": format!("127.0.0.1:{port}"), "job_id": "t1-0"}
    });
    fs::write(&record_file, serde_json::to_vec(&record).unwrap()).unwrap();

    let refused = blit(&ctx, &["jobs", "retry", &run_id]);
    assert!(!refused.status.success());
    assert!(
        text(&refused.stderr).contains("how it ended is not known yet"),
        "{}",
        text(&refused.stderr)
    );
}

#[test]
fn a_run_whose_failures_are_not_all_known_is_not_retried() {
    let ctx = TestContext::new();
    let (src, dst) = blocked_fixture(&ctx);
    let first = blit(&ctx, &["copy", "--retries", "0", &arg(&src), &arg(&dst)]);
    assert_eq!(first.status.code(), Some(EXIT_PARTIAL_FAILURE));
    let job = &jobs(&ctx)[0];
    let run_id = job["run_id"].as_str().unwrap().to_string();
    // As a run whose report counted more failures than it could name, and
    // whose log here does not name them either.
    let record_file = PathBuf::from(job["record_file"].as_str().unwrap());
    let mut record: serde_json::Value =
        serde_json::from_slice(&fs::read(&record_file).unwrap()).unwrap();
    record["files_failed"] = 5.into();
    record["failures_truncated"] = true.into();
    fs::write(&record_file, serde_json::to_vec(&record).unwrap()).unwrap();

    let refused = blit(&ctx, &["jobs", "retry", &run_id]);
    assert!(!refused.status.success());
    assert!(
        text(&refused.stderr).contains("names only 1 of the 5 files it failed"),
        "{}",
        text(&refused.stderr)
    );
}
