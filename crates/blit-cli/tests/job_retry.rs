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

/// `blit`, with no file allowed to grow past `kib` KiB: a write that would
/// fails, as on a volume with no room. (`ulimit -f` counts 512-byte
/// blocks; the signal a process is sent for it is ignored, so the write
/// returns its error.)
#[cfg(unix)]
fn blit_with_no_room_past(ctx: &TestContext, kib: u32, args: &[&str]) -> Output {
    let mut cmd = Command::new("sh");
    cmd.current_dir(&ctx.workspace)
        .arg("-c")
        .arg(format!(
            "trap '' XFSZ; ulimit -f {}; exec \"$0\" \"$@\"",
            kib * 2
        ))
        .arg(cli_bin())
        .arg("--config-dir")
        .arg(&ctx.config_dir)
        .args(args);
    run_with_timeout(cmd, Duration::from_secs(60))
}

/// ci-macos-clone-no-room: whether a run made under
/// [`blit_with_no_room_past`] met no limit at all — it exited 0 and `landed`
/// holds every byte of `whole`. On macOS a copy within one APFS volume is a
/// clone: no byte is written, so a limit on a file's size fails nothing, and
/// the guards that need a write to fail have nothing to check there. Nothing
/// else passes for it — a run that exits 0 without the whole file reported
/// success for a copy that did not land — and anywhere but macOS it is the
/// limit that failed, which the guards say.
#[cfg(unix)]
fn met_no_limit(ran: &Output, landed: &Path, whole: &[u8]) -> bool {
    let unlimited =
        ran.status.code() == Some(0) && fs::read(landed).is_ok_and(|bytes| bytes == whole);
    assert!(
        !unlimited || cfg!(target_os = "macos"),
        "the limit on a file's size did not hold here: {}",
        text(&ran.stderr)
    );
    unlimited
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

/// Review cr-jl4-1, as cr-ecm13-1 leaves it: a retry of an
/// `--ignore-existing` job writes over NOTHING that is there. A file
/// someone put where a file failed is kept, and that path is done. A file
/// standing where the run left its own incomplete copy is kept too — it
/// may be that copy, it may be somebody's — and that path is reported, so
/// the retry does not end well; once nothing stands there, a retry copies
/// the file. (The retry sent such a path with the option off, and replaced
/// whatever stood there.) Not knowing which paths are which, it refuses.
#[test]
fn an_ignore_existing_retry_writes_over_nothing_that_is_there() {
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
    let said = format!("{}{}", text(&retried.stdout), text(&retried.stderr));
    assert_eq!(retried.status.code(), Some(EXIT_PARTIAL_FAILURE), "{said}");
    assert!(
        said.contains("own.txt: an incomplete copy was left at the destination by this run")
            && said.contains("a file stands there: --ignore-existing keeps it"),
        "{said}"
    );
    assert_eq!(
        fs::read(dst.join("own.txt")).unwrap(),
        b"the whole",
        "nothing that is there is written over\n{said}"
    );
    assert_eq!(fs::read(dst.join("kept.txt")).unwrap(), b"mine");

    // Once nothing stands there, the file is copied.
    fs::remove_file(dst.join("own.txt")).unwrap();
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
/// retry knows the one by the other. One of the two sets had the file's
/// name put in place of the empty path and the other had not (cr-ecm10-3),
/// so they were split as two different files: the pass for the run's own
/// copies looked for a line naming nothing, and the retry exited 2 saying
/// the file was missing. As cr-ecm13-1 leaves it: a file standing where
/// the copy was left is kept and reported, and copied to once nothing
/// stands there; a file someone put where the file merely failed is kept,
/// and that is all.
#[test]
fn an_ignore_existing_retry_of_a_single_file_knows_where_its_copy_was_left() {
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
        assert!(
            !said.contains("missing at retry"),
            "own copy {own_copy}: {said}"
        );
        if !own_copy {
            assert!(retried.status.success(), "{said}");
            assert_eq!(fs::read(&dst).unwrap(), b"mine", "{said}");
            continue;
        }
        assert_eq!(retried.status.code(), Some(EXIT_PARTIAL_FAILURE), "{said}");
        assert!(
            said.contains("a file stands there: --ignore-existing keeps it"),
            "{said}"
        );
        assert_eq!(fs::read(&dst).unwrap(), b"the whole", "{said}");
        fs::remove_file(&dst).unwrap();
        let retried = blit(&ctx, &["jobs", "retry", &run_id]);
        let said = format!("{}{}", text(&retried.stdout), text(&retried.stderr));
        assert!(retried.status.success(), "{said}");
        assert_eq!(fs::read(&dst).unwrap(), b"the whole file", "{said}");
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

/// cr-ecm13-1: under `--ignore-existing` a run leaves no partial for a
/// retry to write over, and a retry writes over nothing. A write that ran
/// out of room left an empty `big.bin`; someone put another file there;
/// and `jobs retry` wrote 128 KiB over it and exited 0 — first because
/// such a path was retried with the option off, then (cr-ecm12-2) because
/// the copy was known by a time stamped on it, which a file put in its
/// place could carry. The failed write now removes what it made. With
/// nothing there the retry copies the file; with a file there, whoever
/// put it and whatever time it bears, the retry keeps it.
#[cfg(unix)]
#[test]
fn an_ignore_existing_run_leaves_no_partial_for_a_retry_to_write_over() {
    for put in ["nothing", "a file", "a file with a time of 1985"] {
        let ctx = TestContext::new();
        let src = ctx.workspace.join("big.bin");
        let dst = ctx.workspace.join("dst");
        let whole = vec![b'w'; 128 * 1024];
        fs::write(&src, &whole).unwrap();
        let first = blit_with_no_room_past(
            &ctx,
            64,
            &[
                "copy",
                "--resume",
                "--ignore-existing",
                "--retries",
                "0",
                &src.display().to_string(),
                &arg(&dst),
            ],
        );
        let said = format!("{}{}", text(&first.stdout), text(&first.stderr));
        let left = dst.join("big.bin");
        if met_no_limit(&first, &left, &whole) {
            eprintln!("the copy here is a clone, which no size limit fails: nothing to check");
            return;
        }
        assert_eq!(first.status.code(), Some(EXIT_PARTIAL_FAILURE), "{said}");
        assert!(
            fs::symlink_metadata(&left).is_err(),
            "{put}: the failed write removed what it had made\n{said}"
        );
        assert!(!said.contains("an incomplete copy was left"), "{said}");
        let run_id = jobs(&ctx)[0]["run_id"].as_str().unwrap().to_string();

        if put != "nothing" {
            fs::write(&left, b"someone's twenty byt").unwrap();
        }
        if put == "a file with a time of 1985" {
            // The time cr-ecm12-2 stamped an incomplete copy with.
            filetime::set_file_mtime(&left, filetime::FileTime::from_unix_time(487_368_674, 0))
                .unwrap();
        }
        let retried = blit(&ctx, &["jobs", "retry", &run_id]);
        let said = format!("{}{}", text(&retried.stdout), text(&retried.stderr));
        assert!(retried.status.success(), "{put}: {said}");
        if put == "nothing" {
            assert!(fs::read(&left).unwrap() == whole, "{put}: {said}");
        } else {
            assert_eq!(
                fs::read(&left).unwrap(),
                b"someone's twenty byt",
                "{put}: {said}"
            );
        }
    }
}

/// cr-ecm13-4: a retry is a run of its own, and its record carries what
/// the run before it left. A copy and its first retry both failed on
/// `big.bin`; the retry's record held no incomplete copy, so a retry of
/// THAT run sent the path as any other, `--ignore-existing` kept the file
/// standing there — the incomplete copy — and the command exited 0 with
/// the destination at zero bytes. Here a retry that finds a file where
/// the copy was left keeps it and says so in the words every record is
/// read for; a retry of that retry does the same, and goes on doing so
/// until nothing stands there — and then the file is copied.
#[test]
fn a_retry_of_a_retry_still_knows_where_an_incomplete_copy_was_left() {
    let ctx = TestContext::new();
    let src = ctx.workspace.join("src");
    let dst = ctx.workspace.join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::write(src.join("big.bin"), b"the whole file").unwrap();
    fs::create_dir_all(dst.join("big.bin")).unwrap();
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
    assert_eq!(first.status.code(), Some(EXIT_PARTIAL_FAILURE));
    let job = &jobs(&ctx)[0];
    let mut run_id = job["run_id"].as_str().unwrap().to_string();
    let record_file = PathBuf::from(job["record_file"].as_str().unwrap());
    let mut record: serde_json::Value =
        serde_json::from_slice(&fs::read(&record_file).unwrap()).unwrap();
    // As a run whose write left its own incomplete copy, empty.
    record["left_in_place"] = serde_json::json!(["big.bin"]);
    fs::write(&record_file, serde_json::to_vec(&record).unwrap()).unwrap();
    fs::remove_dir_all(dst.join("big.bin")).unwrap();
    fs::write(dst.join("big.bin"), b"").unwrap();

    // The retry, the retry of the retry, and the retry of that.
    for attempt in 1..=3 {
        let retried = blit(&ctx, &["jobs", "retry", &run_id]);
        let said = format!("{}{}", text(&retried.stdout), text(&retried.stderr));
        assert_eq!(
            retried.status.code(),
            Some(EXIT_PARTIAL_FAILURE),
            "retry {attempt} of {run_id}: {said}"
        );
        assert_eq!(
            fs::read(dst.join("big.bin")).unwrap(),
            b"",
            "retry {attempt}: {said}"
        );
        let newest = &jobs(&ctx)[0];
        assert_eq!(newest["parent"], run_id.as_str(), "{newest}");
        let its_record: serde_json::Value =
            serde_json::from_slice(&fs::read(newest["record_file"].as_str().unwrap()).unwrap())
                .unwrap();
        assert_eq!(
            its_record["left_in_place"],
            serde_json::json!(["big.bin"]),
            "retry {attempt} records where the copy was left: {its_record}"
        );
        run_id = newest["run_id"].as_str().unwrap().to_string();
    }

    fs::remove_file(dst.join("big.bin")).unwrap();
    let retried = blit(&ctx, &["jobs", "retry", &run_id]);
    assert!(retried.status.success(), "{}", text(&retried.stderr));
    assert_eq!(fs::read(dst.join("big.bin")).unwrap(), b"the whole file");
}

/// cr-ecm13-5: a file of the source's own length, standing where the run
/// left its incomplete copy, is not called copied. With `--size-only` the
/// session ordered such a copy repaired and the sink, comparing again by
/// size, wrote nothing: "one file copied", zero bytes, exit 0 — the source
/// held `good` and the destination still held `junk`. Nothing is ordered
/// repaired any more (cr-ecm13-1): the file is kept, the path is reported,
/// and nothing is said to have been copied until something is.
#[test]
fn a_size_only_retry_does_not_call_a_file_of_the_right_length_copied() {
    let ctx = TestContext::new();
    let src = ctx.workspace.join("src");
    let dst = ctx.workspace.join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::write(src.join("data.bin"), b"good").unwrap();
    fs::create_dir_all(dst.join("data.bin")).unwrap();
    let first = blit(
        &ctx,
        &[
            "copy",
            "--size-only",
            "--ignore-existing",
            "--retries",
            "0",
            &arg(&src),
            &arg(&dst),
        ],
    );
    assert_eq!(first.status.code(), Some(EXIT_PARTIAL_FAILURE));
    let job = &jobs(&ctx)[0];
    let run_id = job["run_id"].as_str().unwrap().to_string();
    let record_file = PathBuf::from(job["record_file"].as_str().unwrap());
    let mut record: serde_json::Value =
        serde_json::from_slice(&fs::read(&record_file).unwrap()).unwrap();
    // As a run whose write left its own incomplete copy — of the source's
    // length, as a write that failed at its very end leaves one.
    record["left_in_place"] = serde_json::json!(["data.bin"]);
    fs::write(&record_file, serde_json::to_vec(&record).unwrap()).unwrap();
    fs::remove_dir_all(dst.join("data.bin")).unwrap();
    fs::write(dst.join("data.bin"), b"junk").unwrap();

    let retried = blit(&ctx, &["jobs", "retry", &run_id]);
    let said = format!("{}{}", text(&retried.stdout), text(&retried.stderr));
    assert_eq!(retried.status.code(), Some(EXIT_PARTIAL_FAILURE), "{said}");
    assert!(
        said.contains("a file stands there: --ignore-existing keeps it"),
        "{said}"
    );
    assert_eq!(fs::read(dst.join("data.bin")).unwrap(), b"junk", "{said}");
    let newest = &jobs(&ctx)[0];
    assert_eq!(
        newest["files_copied"], 0,
        "nothing is called copied: {newest}"
    );
    assert_eq!(newest["files_failed"], 1, "{newest}");

    fs::remove_file(dst.join("data.bin")).unwrap();
    let retried = blit(&ctx, &["jobs", "retry", &run_id]);
    assert!(retried.status.success(), "{}", text(&retried.stderr));
    assert_eq!(fs::read(dst.join("data.bin")).unwrap(), b"good");
    assert_eq!(jobs(&ctx)[0]["files_copied"], 1);
}

/// cr-ecm13-2: a retry writes where the run wrote. `copy src backup`,
/// neither ending in `/`: with no `backup` there the run makes it and
/// writes `backup/…`; with a directory there it writes `backup/src/…`.
/// The retry read the same two words again — after the first run had made
/// `backup` — and sent the file that failed to `backup/src/`, exit 0, the
/// place it had failed at untouched. The run's record keeps what it
/// decided. And the other way about: a run that wrote under `backup/src/`
/// is retried there though `backup` has gone since.
#[test]
fn a_retry_writes_where_the_run_it_retries_wrote() {
    for there_at_first in [false, true] {
        let ctx = TestContext::new();
        let src = ctx.workspace.join("src");
        let backup = ctx.workspace.join("backup");
        fs::create_dir_all(&src).unwrap();
        fs::write(src.join("landed.txt"), b"alpha").unwrap();
        fs::write(src.join("blocked.txt"), b"lands on retry").unwrap();
        // Where the run will write: a folder in the way of one file.
        let root = if there_at_first {
            backup.join("src")
        } else {
            backup.clone()
        };
        if there_at_first {
            fs::create_dir_all(root.join("blocked.txt")).unwrap();
        }
        // No `/` after either: the second is a name, not "into here".
        let typed = [src.display().to_string(), backup.display().to_string()];
        let first = if there_at_first {
            blit(&ctx, &["copy", "--retries", "0", &typed[0], &typed[1]])
        } else {
            // Nothing can stand in a folder that is not there yet: the
            // file fails for want of room instead.
            #[cfg(not(unix))]
            continue;
            #[cfg(unix)]
            {
                fs::write(src.join("blocked.txt"), vec![b'w'; 128 * 1024]).unwrap();
                blit_with_no_room_past(&ctx, 64, &["copy", "--retries", "0", &typed[0], &typed[1]])
            }
        };
        let said = format!("{}{}", text(&first.stdout), text(&first.stderr));
        #[cfg(unix)]
        if !there_at_first
            && met_no_limit(&first, &root.join("blocked.txt"), &vec![b'w'; 128 * 1024])
        {
            eprintln!("the copy here is a clone, which no size limit fails: nothing to check");
            continue;
        }
        assert_eq!(
            first.status.code(),
            Some(EXIT_PARTIAL_FAILURE),
            "there at first {there_at_first}: {said}"
        );
        assert_eq!(
            fs::read(root.join("landed.txt")).unwrap(),
            b"alpha",
            "there at first {there_at_first}: {said}"
        );
        let job = &jobs(&ctx)[0];
        let run_id = job["run_id"].as_str().unwrap().to_string();
        let record: serde_json::Value =
            serde_json::from_slice(&fs::read(job["record_file"].as_str().unwrap()).unwrap())
                .unwrap();
        assert_eq!(record["destination_nested"], there_at_first, "{record}");

        if there_at_first {
            // All of it gone since: read again, the words would write
            // `backup/…`.
            fs::remove_dir_all(&backup).unwrap();
        }
        let retried = blit(&ctx, &["jobs", "retry", &run_id]);
        let said = format!("{}{}", text(&retried.stdout), text(&retried.stderr));
        assert!(
            retried.status.success(),
            "there at first {there_at_first}: {said}"
        );
        assert!(
            root.join("blocked.txt").is_file(),
            "there at first {there_at_first}: the file lands where it failed\n{said}"
        );
        let elsewhere = if there_at_first {
            backup.join("blocked.txt")
        } else {
            backup.join("src")
        };
        assert!(
            fs::symlink_metadata(&elsewhere).is_err(),
            "there at first {there_at_first}: and nowhere else\n{said}"
        );
    }
}

/// cr-ecm14-2: `--ignore-existing` keeps a LINK that stands at a path,
/// whatever it points to — and writes through none. The reviewer's case:
/// a recorded leftover retried with a link to nothing in its place; the
/// retry followed the link, made what it names, and exited 0. Now the
/// link is kept and the path reported, as a file there would be; a link
/// at a path that merely failed is kept and nothing is said; and a plain
/// `copy --ignore-existing` over a link to nothing makes nothing either.
#[cfg(unix)]
#[test]
fn an_ignore_existing_retry_follows_no_link_left_where_its_copy_was() {
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
    assert_eq!(first.status.code(), Some(EXIT_PARTIAL_FAILURE));
    let job = &jobs(&ctx)[0];
    let run_id = job["run_id"].as_str().unwrap().to_string();
    let record_file = PathBuf::from(job["record_file"].as_str().unwrap());
    let mut record: serde_json::Value =
        serde_json::from_slice(&fs::read(&record_file).unwrap()).unwrap();
    // As a run whose write of `own.txt` left its own incomplete copy.
    record["left_in_place"] = serde_json::json!(["own.txt"]);
    fs::write(&record_file, serde_json::to_vec(&record).unwrap()).unwrap();
    // And where each stood, a link to nothing.
    let elsewhere = ctx.workspace.join("elsewhere");
    fs::create_dir_all(&elsewhere).unwrap();
    for name in ["own.txt", "kept.txt"] {
        fs::remove_dir_all(dst.join(name)).unwrap();
        std::os::unix::fs::symlink(elsewhere.join(name), dst.join(name)).unwrap();
    }

    let retried = blit(&ctx, &["jobs", "retry", &run_id]);
    let said = format!("{}{}", text(&retried.stdout), text(&retried.stderr));
    assert_eq!(retried.status.code(), Some(EXIT_PARTIAL_FAILURE), "{said}");
    assert!(
        said.contains("own.txt: an incomplete copy was left at the destination by this run"),
        "{said}"
    );
    for name in ["own.txt", "kept.txt"] {
        assert!(
            dst.join(name).is_symlink(),
            "{name}: the link stays\n{said}"
        );
        assert!(
            !elsewhere.join(name).exists(),
            "{name}: what the link names is not made\n{said}"
        );
    }

    // A plain copy that keeps what exists, over a link to nothing.
    let plain = ctx.workspace.join("plain");
    fs::create_dir_all(&plain).unwrap();
    std::os::unix::fs::symlink(elsewhere.join("own.txt"), plain.join("own.txt")).unwrap();
    let copied = blit(
        &ctx,
        &["copy", "--ignore-existing", &arg(&src), &arg(&plain)],
    );
    assert!(copied.status.success(), "{}", text(&copied.stderr));
    assert!(plain.join("own.txt").is_symlink());
    assert!(!elsewhere.join("own.txt").exists());
    assert_eq!(
        fs::read(plain.join("kept.txt")).unwrap(),
        b"from the source"
    );
}

/// cr-ecm14-4: a record from before a run noted where it wrote
/// (`destination_nested`) is not replayed by reading its two words again
/// where their answer turns on what stood at the destination THEN. The
/// reviewer's case: `copy src backup`; the retry wrote `backup/src/…` for
/// a run that had written `backup/…`, and exited 0. Such a retry is
/// refused, saying why, with nothing written. Where the words alone
/// decide — a `/` after the source, or after the destination — an old
/// record is retried as ever.
#[test]
fn a_record_that_did_not_note_where_it_wrote_is_not_guessed_at() {
    // cr-ecm15-3: the last — a source with no name to put under the
    // destination — is written to one place under either reading, and is
    // retried; it was refused with the first.
    for (after_source, destination_slash, into_backup_itself, told_by_the_words) in [
        ("", false, false, false),
        ("/", true, true, true),
        ("", true, false, true),
        ("/", false, true, true),
        ("/child/..", false, true, true),
    ] {
        let what = format!("source{after_source}, destination/ {destination_slash}");
        let ctx = TestContext::new();
        let src = ctx.workspace.join("src");
        let backup = ctx.workspace.join("backup");
        fs::create_dir_all(src.join("child")).unwrap();
        fs::write(src.join("landed.txt"), b"alpha").unwrap();
        fs::write(src.join("blocked.txt"), b"lands on retry").unwrap();
        // `backup` is there, so a source that is not its contents goes
        // under it by its name; a folder is in the way of one file.
        let root = if into_backup_itself {
            backup.clone()
        } else {
            backup.join("src")
        };
        fs::create_dir_all(root.join("blocked.txt")).unwrap();
        let typed = [
            format!("{}{after_source}", src.display()),
            format!(
                "{}{}",
                backup.display(),
                if destination_slash { "/" } else { "" }
            ),
        ];
        let first = blit(&ctx, &["copy", "--retries", "0", &typed[0], &typed[1]]);
        assert_eq!(
            first.status.code(),
            Some(EXIT_PARTIAL_FAILURE),
            "{what}: {}",
            text(&first.stderr)
        );
        assert_eq!(
            fs::read(root.join("landed.txt")).unwrap(),
            b"alpha",
            "{what}"
        );
        let job = &jobs(&ctx)[0];
        let run_id = job["run_id"].as_str().unwrap().to_string();
        let record_file = PathBuf::from(job["record_file"].as_str().unwrap());
        let mut record: serde_json::Value =
            serde_json::from_slice(&fs::read(&record_file).unwrap()).unwrap();
        // As an earlier build wrote it: with no word on where it wrote.
        assert!(
            record
                .as_object_mut()
                .unwrap()
                .remove("destination_nested")
                .is_some(),
            "{what}: {record}"
        );
        fs::write(&record_file, serde_json::to_vec(&record).unwrap()).unwrap();
        fs::remove_dir_all(root.join("blocked.txt")).unwrap();
        let before = listed(&backup);

        let retried = blit(&ctx, &["jobs", "retry", &run_id]);
        let said = format!("{}{}", text(&retried.stdout), text(&retried.stderr));
        if told_by_the_words {
            assert!(retried.status.success(), "{what}: {said}");
            assert_eq!(
                fs::read(root.join("blocked.txt")).unwrap(),
                b"lands on retry",
                "{what}: {said}"
            );
        } else {
            assert!(!retried.status.success(), "{what}: {said}");
            assert!(
                said.contains("did not note where it wrote") && said.contains("nothing is retried"),
                "{what}: {said}"
            );
            assert_eq!(
                listed(&backup),
                before,
                "{what}: nothing was written\n{said}"
            );
        }
    }
}

/// Every entry below `root`, sorted.
fn listed(root: &Path) -> Vec<String> {
    fn walk(dir: &Path, root: &Path, out: &mut Vec<String>) {
        for entry in fs::read_dir(dir).into_iter().flatten().flatten() {
            let path = entry.path();
            out.push(path.strip_prefix(root).unwrap().display().to_string());
            if entry.file_type().is_ok_and(|kind| kind.is_dir()) {
                walk(&path, root, out);
            }
        }
    }
    let mut out = Vec::new();
    walk(root, root, &mut out);
    out.sort();
    out
}
