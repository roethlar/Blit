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

/// cr-ecm11-1: a retry asks for what failed AS IT FAILED. A source that
/// is one file fails under no name — it is the source itself — and its
/// retry was given the file's name instead (cr-ecm10-3). With the file
/// replaced since by a DIRECTORY holding another `Report.txt`, the retry
/// took the inner file for the one that failed, wrote it to
/// `out/Report.txt/Report.txt` and exited 0. The source itself is no
/// longer a file: there is nothing to retry, and the retry says so.
#[test]
fn a_retry_of_a_single_file_does_not_take_a_directory_of_its_name() {
    // From this machine, and from a daemon — where the mark for the
    // source itself crosses the wire.
    for from_a_daemon in [false, true] {
        let ctx = TestContext::new();
        let (src, typed) = if from_a_daemon {
            (
                ctx.module_dir.join("Report.txt"),
                format!("127.0.0.1:{}:/test/Report.txt", ctx.daemon_port),
            )
        } else {
            let src = ctx.workspace.join("Report.txt");
            (src.clone(), src.display().to_string())
        };
        let out = ctx.workspace.join("out");
        let dst = out.join("Report.txt");
        fs::write(&src, b"the file that failed").unwrap();
        // A folder stands where the file is to land, and holds a file of
        // the source's name: what a retry by name would overwrite.
        fs::create_dir_all(&dst).unwrap();
        fs::write(dst.join("Report.txt"), b"not the run's to touch").unwrap();
        let first = blit(&ctx, &["copy", "--retries", "0", &typed, &arg(&out)]);
        assert_eq!(
            first.status.code(),
            Some(EXIT_PARTIAL_FAILURE),
            "daemon {from_a_daemon}: {}",
            text(&first.stderr)
        );
        let run_id = jobs(&ctx)[0]["run_id"].as_str().unwrap().to_string();

        // The source becomes a directory holding another file of its name.
        fs::remove_file(&src).unwrap();
        fs::create_dir_all(&src).unwrap();
        fs::write(src.join("Report.txt"), b"another file altogether").unwrap();
        let retried = blit(&ctx, &["jobs", "retry", &run_id]);
        let said = format!("{}{}", text(&retried.stdout), text(&retried.stderr));
        assert_eq!(
            retried.status.code(),
            Some(EXIT_PARTIAL_FAILURE),
            "daemon {from_a_daemon}: {said}"
        );
        assert!(
            said.contains("missing at retry"),
            "daemon {from_a_daemon}: {said}"
        );
        assert_eq!(
            fs::read(dst.join("Report.txt")).unwrap(),
            b"not the run's to touch",
            "daemon {from_a_daemon}: nothing of the directory was sent\n{said}"
        );

        // And put back as the file it was, with its place freed, it lands.
        fs::remove_dir_all(&src).unwrap();
        fs::write(&src, b"the file that failed").unwrap();
        fs::remove_dir_all(&dst).unwrap();
        let again = jobs(&ctx)[0]["run_id"].as_str().unwrap().to_string();
        let retried = blit(&ctx, &["jobs", "retry", &again]);
        assert!(
            retried.status.success(),
            "daemon {from_a_daemon}: {}{}",
            text(&retried.stdout),
            text(&retried.stderr)
        );
        assert_eq!(
            fs::read(&dst).unwrap(),
            b"the file that failed",
            "daemon {from_a_daemon}"
        );
    }
}

/// Review cr-jl4-3: when a record's failure list was cut short, its log
/// completes it with the run's terminal state — a file a later retry pass
/// landed is not retried again.
#[test]
fn a_retry_skips_what_a_later_pass_already_landed() {
    use std::process::Stdio;
    use std::time::Instant;
    let ctx = TestContext::new();
    let src = ctx.workspace.join("src");
    let dst = ctx.workspace.join("dst");
    fs::create_dir_all(&src).unwrap();
    for name in ["freed.txt", "stuck.txt"] {
        fs::write(src.join(name), name.as_bytes()).unwrap();
        fs::create_dir_all(dst.join(name)).unwrap();
    }
    let counters = ctx.workspace.join("counters.txt");
    // One retry pass: `freed.txt` is freed during its wait and lands on it;
    // `stuck.txt` stays failed.
    let child = Command::new(cli_bin())
        .current_dir(&ctx.workspace)
        .arg("--config-dir")
        .arg(&ctx.config_dir)
        .arg("--diagnostics-counter-file")
        .arg(&counters)
        .args(["copy", "--retries", "1", "--retry-wait", "4"])
        .arg(arg(&src))
        .arg(arg(&dst))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let start = Instant::now();
    while !fs::read_to_string(&counters)
        .unwrap_or_default()
        .contains("retry_wait_seconds")
    {
        assert!(start.elapsed() < Duration::from_secs(60), "no retry wait");
        std::thread::sleep(Duration::from_millis(25));
    }
    fs::remove_dir_all(dst.join("freed.txt")).unwrap();
    let first = child.wait_with_output().unwrap();
    assert_eq!(
        first.status.code(),
        Some(EXIT_PARTIAL_FAILURE),
        "{}",
        text(&first.stderr)
    );
    assert_eq!(fs::read(dst.join("freed.txt")).unwrap(), b"freed.txt");

    // As a record whose list was cut short: the log must complete it.
    let job = &jobs(&ctx)[0];
    let run_id = job["run_id"].as_str().unwrap().to_string();
    let record_file = PathBuf::from(job["record_file"].as_str().unwrap());
    let mut record: serde_json::Value =
        serde_json::from_slice(&fs::read(&record_file).unwrap()).unwrap();
    assert_eq!(record["files_failed"], 1, "{record}");
    record["failures"] = serde_json::json!([]);
    record["failures_truncated"] = true.into();
    fs::write(&record_file, serde_json::to_vec(&record).unwrap()).unwrap();

    // `freed.txt` changes at the source; a retry must not send it again.
    fs::write(src.join("freed.txt"), b"changed since").unwrap();
    fs::remove_dir_all(dst.join("stuck.txt")).unwrap();
    let retried = blit(&ctx, &["jobs", "retry", &run_id]);
    assert!(retried.status.success(), "{}", text(&retried.stderr));
    assert_eq!(fs::read(dst.join("stuck.txt")).unwrap(), b"stuck.txt");
    assert_eq!(
        fs::read(dst.join("freed.txt")).unwrap(),
        b"freed.txt",
        "the file a pass landed was not retried"
    );
    assert_eq!(jobs(&ctx)[0]["files_copied"], 1);
}

/// Review cr-jl4-1: a retry of an `--ignore-existing` job sends a file
/// whose failed write left the run's own incomplete copy with the flag off
/// — the copy is replaced — and another failed file with it on — what the
/// person put there since is kept. Not knowing which is which, it refuses.
#[test]
fn an_ignore_existing_retry_replaces_only_its_own_incomplete_copies() {
    let ctx = TestContext::new();
    let src = ctx.workspace.join("src");
    let dst = ctx.workspace.join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::write(src.join("own.txt"), b"the whole file").unwrap();
    fs::write(src.join("kept.txt"), b"from the source").unwrap();
    for name in ["own.txt", "kept.txt"] {
        fs::create_dir_all(dst.join(name)).unwrap();
    }
    let first = blit(
        &ctx,
        &[
            "copy",
            "--ignore-existing",
            "--retries",
            "0",
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
    let job = &jobs(&ctx)[0];
    let run_id = job["run_id"].as_str().unwrap().to_string();
    let record_file = PathBuf::from(job["record_file"].as_str().unwrap());
    let mut record: serde_json::Value =
        serde_json::from_slice(&fs::read(&record_file).unwrap()).unwrap();
    assert_eq!(record["files_failed"], 2, "{record}");
    // As a run whose write of `own.txt` left its own incomplete copy.
    record["left_in_place"] = serde_json::json!(["own.txt"]);
    fs::write(&record_file, serde_json::to_vec(&record).unwrap()).unwrap();
    fs::remove_dir_all(dst.join("own.txt")).unwrap();
    fs::write(dst.join("own.txt"), b"the whole").unwrap();
    fs::remove_dir_all(dst.join("kept.txt")).unwrap();
    fs::write(dst.join("kept.txt"), b"mine").unwrap();

    let retried = blit(&ctx, &["jobs", "retry", &run_id]);
    assert!(retried.status.success(), "{}", text(&retried.stderr));
    assert_eq!(fs::read(dst.join("own.txt")).unwrap(), b"the whole file");
    assert_eq!(fs::read(dst.join("kept.txt")).unwrap(), b"mine");
    let retry = &jobs(&ctx)[0];
    assert_eq!(retry["parent"], run_id.as_str(), "{retry}");
    assert_eq!(retry["files_copied"], 1, "the two parts add up: {retry}");

    // Which failures left their own copy is not all known: refused.
    record["left_in_place_truncated"] = true.into();
    fs::write(&record_file, serde_json::to_vec(&record).unwrap()).unwrap();
    let refused = blit(&ctx, &["jobs", "retry", &run_id]);
    assert!(!refused.status.success());
    assert!(
        text(&refused.stderr).contains("is not all known"),
        "{}",
        text(&refused.stderr)
    );
}

/// cr-ecm11-2: the same for a source that is ONE FILE. Its failure is
/// recorded under no name, and so is the copy its failed write left; the
/// retry sends that copy's file with `--ignore-existing` off and replaces
/// it. One of the two sets had the file's name put in place of the empty
/// path and the other had not (cr-ecm10-3), so they were split as two
/// different files: the pass that repairs looked for a line naming
/// nothing, the other skipped `Report.txt` as a file to keep, and the
/// retry exited 2 with the partial bytes in place.
#[test]
fn an_ignore_existing_retry_of_a_single_file_replaces_its_own_incomplete_copy() {
    for own_copy in [true, false] {
        let ctx = TestContext::new();
        let src = ctx.workspace.join("Report.txt");
        let out = ctx.workspace.join("out");
        let dst = out.join("Report.txt");
        fs::write(&src, b"the whole file").unwrap();
        fs::create_dir_all(&dst).unwrap();
        let first = blit(
            &ctx,
            &[
                "copy",
                "--ignore-existing",
                "--retries",
                "0",
                &src.display().to_string(),
                &arg(&out),
            ],
        );
        assert_eq!(
            first.status.code(),
            Some(EXIT_PARTIAL_FAILURE),
            "{}",
            text(&first.stderr)
        );
        let job = &jobs(&ctx)[0];
        let run_id = job["run_id"].as_str().unwrap().to_string();
        let record_file = PathBuf::from(job["record_file"].as_str().unwrap());
        let mut record: serde_json::Value =
            serde_json::from_slice(&fs::read(&record_file).unwrap()).unwrap();
        assert_eq!(record["files_failed"], 1, "{record}");
        assert_eq!(
            record["failures"][0]["path"], "",
            "the source itself has no name: {record}"
        );
        fs::remove_dir_all(&dst).unwrap();
        if own_copy {
            // As a run whose write left its own incomplete copy.
            record["left_in_place"] = serde_json::json!([""]);
            fs::write(&record_file, serde_json::to_vec(&record).unwrap()).unwrap();
            fs::write(&dst, b"the whole").unwrap();
        } else {
            // A file someone put there since: theirs to keep.
            fs::write(&dst, b"mine").unwrap();
        }

        let retried = blit(&ctx, &["jobs", "retry", &run_id]);
        let said = format!("{}{}", text(&retried.stdout), text(&retried.stderr));
        assert!(retried.status.success(), "own copy {own_copy}: {said}");
        assert_eq!(
            fs::read(&dst).unwrap(),
            if own_copy {
                &b"the whole file"[..]
            } else {
                &b"mine"[..]
            },
            "own copy {own_copy}: {said}"
        );
        assert!(
            !said.contains("missing at retry"),
            "own copy {own_copy}: {said}"
        );
    }
}

/// cr-ecm12-1: the converse. A CHILD that failed is retried as that child
/// and nothing else. A copy of the directory `Report.txt` failed on its
/// child `Report.txt`; with the directory replaced since by a FILE of
/// that name, the retry's list — `Report.txt` — was read as a list of
/// names, which a source that is one file answers to by its own: the file
/// was sent as the source itself and written over `out/Report.txt`, an
/// unrelated file by then. A retry's list holds exact paths; there is no
/// such child any more, and the retry says so.
#[test]
fn a_retry_of_a_child_does_not_take_a_file_of_its_name_for_it() {
    for from_a_daemon in [false, true] {
        let ctx = TestContext::new();
        let (src, typed) = if from_a_daemon {
            (
                ctx.module_dir.join("Report.txt"),
                format!("127.0.0.1:{}:/test/Report.txt", ctx.daemon_port),
            )
        } else {
            let src = ctx.workspace.join("Report.txt");
            (src.clone(), src.display().to_string())
        };
        let out = ctx.workspace.join("out");
        fs::create_dir_all(&src).unwrap();
        fs::write(src.join("Report.txt"), b"the child that failed").unwrap();
        fs::write(src.join("landed.txt"), b"alpha").unwrap();
        // The child's place is taken by a folder, with something in it.
        fs::create_dir_all(out.join("Report.txt/Report.txt")).unwrap();
        fs::write(out.join("Report.txt/Report.txt/kept"), b"x").unwrap();
        let first = blit(&ctx, &["copy", "--retries", "0", &typed, &arg(&out)]);
        assert_eq!(
            first.status.code(),
            Some(EXIT_PARTIAL_FAILURE),
            "daemon {from_a_daemon}: {}{}",
            text(&first.stdout),
            text(&first.stderr)
        );
        let run_id = jobs(&ctx)[0]["run_id"].as_str().unwrap().to_string();

        // The source becomes one file of that name; and where the copy's
        // directory stood there is now a file nobody asked to replace.
        fs::remove_dir_all(&src).unwrap();
        fs::write(&src, b"another file altogether").unwrap();
        fs::remove_dir_all(out.join("Report.txt")).unwrap();
        fs::write(out.join("Report.txt"), b"not the run's to touch").unwrap();
        let retried = blit(&ctx, &["jobs", "retry", &run_id]);
        let said = format!("{}{}", text(&retried.stdout), text(&retried.stderr));
        assert_eq!(
            retried.status.code(),
            Some(EXIT_PARTIAL_FAILURE),
            "daemon {from_a_daemon}: {said}"
        );
        assert!(
            said.contains("missing at retry"),
            "daemon {from_a_daemon}: {said}"
        );
        assert_eq!(
            fs::read(out.join("Report.txt")).unwrap(),
            b"not the run's to touch",
            "daemon {from_a_daemon}: the file there is not the child\n{said}"
        );
    }
}
