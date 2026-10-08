//! JOB_LOGS jl-1b: `blit jobs log <host> <job-id>` reads the log a daemon
//! kept for a job — as text, as its JSON lines with `--json`, and only one
//! role with `--role`. jl-2: every command keeps its own log on this
//! machine, under the run ID it sends the daemons, read by `blit jobs list`
//! and `blit jobs log <job-id|file>` without a host.

mod common;

use std::fs;
use std::process::Command;
use std::time::{Duration, Instant};

use common::{run_with_timeout, DaemonOptions, TestContext};

fn run(ctx: &TestContext, args: &[&str]) -> std::process::Output {
    let mut cmd = Command::new(&ctx.cli_bin);
    cmd.arg("--config-dir").arg(&ctx.config_dir).args(args);
    run_with_timeout(cmd, Duration::from_secs(60))
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

#[test]
fn jobs_log_reads_a_pushed_jobs_log_as_text_and_json() {
    let ctx = TestContext::new();
    let src = ctx.workspace.join("src");
    fs::create_dir_all(src.join("sub")).unwrap();
    fs::write(src.join("a.txt"), b"alpha").unwrap();
    fs::write(src.join("sub/b.txt"), b"beta").unwrap();
    let remote = format!("127.0.0.1:{}", ctx.daemon_port);

    let pushed = run(
        &ctx,
        &[
            "copy",
            &format!("{}/", src.display()),
            &format!("{remote}:/test/"),
        ],
    );
    assert!(
        pushed.status.success(),
        "push failed: {}",
        text(&pushed.stderr)
    );

    let listed = run(&ctx, &["jobs", "list", &remote, "--json"]);
    let state: serde_json::Value = serde_json::from_slice(&listed.stdout).unwrap();
    // The newest job: a test daemon's recent list can also hold jobs that
    // earlier daemons persisted in the shared config folder.
    let id = state["recent"]
        .as_array()
        .and_then(|recent| {
            recent
                .iter()
                .max_by_key(|job| job["start_unix_ms"].as_u64().unwrap_or(0))
        })
        .and_then(|job| job["transfer_id"].as_str())
        .unwrap_or_else(|| panic!("no recent job: {state}"))
        .to_string();

    // The daemon closes the log just after the job's record; until then it
    // reads as not finished.
    let deadline = Instant::now() + Duration::from_secs(30);
    let shown = loop {
        let shown = run(&ctx, &["jobs", "log", &remote, &id]);
        assert!(
            shown.status.success(),
            "jobs log failed: {}",
            text(&shown.stderr)
        );
        let out = text(&shown.stdout);
        if !out.contains("not finished") {
            break out;
        }
        assert!(Instant::now() < deadline, "the log never finished:\n{out}");
        std::thread::sleep(Duration::from_millis(100));
    };
    assert!(
        shown.starts_with("== destination log from machine "),
        "{shown}"
    );
    for expected in [
        "start    push 127.0.0.1:",
        "copied   a.txt",
        "copied   sub/b.txt",
        "summary  2 copied (9 B), 0 deleted, 0 failed",
        "end      ok",
    ] {
        assert!(
            shown.contains(expected),
            "missing {expected:?} in:\n{shown}"
        );
    }

    let raw = run(&ctx, &["jobs", "log", &remote, &id, "--json"]);
    assert!(raw.status.success(), "{}", text(&raw.stderr));
    let kinds: Vec<String> = text(&raw.stdout)
        .lines()
        .map(|line| {
            let event: serde_json::Value =
                serde_json::from_str(line).unwrap_or_else(|err| panic!("not JSON ({err}): {line}"));
            event["kind"].as_str().unwrap().to_string()
        })
        .collect();
    assert_eq!(kinds.first().map(String::as_str), Some("run-start"));
    assert_eq!(kinds.last().map(String::as_str), Some("run-end"));
    assert_eq!(
        kinds.iter().filter(|kind| *kind == "file-copied").count(),
        2
    );

    let other_role = run(&ctx, &["jobs", "log", &remote, &id, "--role", "source"]);
    assert!(!other_role.status.success());
    assert!(
        text(&other_role.stderr).contains(&format!("no source log for job {id}")),
        "{}",
        text(&other_role.stderr)
    );

    let unknown = run(&ctx, &["jobs", "log", &remote, "t1-0"]);
    assert!(!unknown.status.success());
    assert!(
        text(&unknown.stderr).contains("no log for job t1-0"),
        "{}",
        text(&unknown.stderr)
    );
}

/// The run ID a command printed under `-v`.
fn job_id_in(stderr: &str) -> String {
    stderr
        .lines()
        .find_map(|line| line.strip_prefix("blit: job "))
        .and_then(|rest| rest.split_whitespace().next())
        .unwrap_or_else(|| panic!("no job ID printed:\n{stderr}"))
        .to_string()
}

fn event_kinds(json_lines: &[u8]) -> Vec<String> {
    text(json_lines)
        .lines()
        .map(|line| {
            let event: serde_json::Value =
                serde_json::from_str(line).unwrap_or_else(|err| panic!("not JSON ({err}): {line}"));
            event["kind"].as_str().unwrap().to_string()
        })
        .collect()
}

#[test]
fn a_local_copy_keeps_its_log_on_this_machine() {
    let ctx = TestContext::new();
    let src = ctx.workspace.join("src");
    fs::create_dir_all(src.join("sub")).unwrap();
    fs::write(src.join("a.txt"), b"alpha").unwrap();
    fs::write(src.join("sub/b.txt"), b"beta").unwrap();
    let dst = ctx.workspace.join("dst");

    let copied = run(
        &ctx,
        &[
            "copy",
            "-v",
            &format!("{}/", src.display()),
            &format!("{}/", dst.display()),
        ],
    );
    assert!(copied.status.success(), "{}", text(&copied.stderr));
    let id = job_id_in(&text(&copied.stderr));

    let listed = run(&ctx, &["jobs", "list", "--json"]);
    assert!(listed.status.success(), "{}", text(&listed.stderr));
    let listing: serde_json::Value = serde_json::from_slice(&listed.stdout).unwrap();
    let jobs = listing["jobs"].as_array().expect("jobs");
    assert_eq!(jobs.len(), 1, "{listing}");
    let job = &jobs[0];
    assert_eq!(job["run_id"], id.as_str(), "{job}");
    assert_eq!(job["verb"], "copy", "{job}");
    assert_eq!(job["state"], "finished", "{job}");
    assert_eq!(job["outcome"], "ok", "{job}");
    assert_eq!(job["files_copied"], 2, "{job}");
    // JOB_LOGS jl-3: the job, kept here — its spec names the source as an
    // absolute path, contents-of (trailing separator kept).
    let spec: serde_json::Value =
        serde_json::from_slice(&fs::read(job["spec_file"].as_str().expect("spec file")).unwrap())
            .unwrap();
    assert_eq!(spec["format"], "blit-job-spec", "{spec}");
    assert_eq!(spec["verb"], "copy", "{spec}");
    assert_eq!(spec["source"]["kind"], "local", "{spec}");
    let source = spec["source"]["path"].as_str().unwrap();
    assert!(
        std::path::Path::new(source).is_absolute() && source.ends_with(['/', '\\']),
        "{spec}"
    );
    let log_file = job["log"].as_str().expect("log path").to_string();

    let human = text(&run(&ctx, &["jobs", "list"]).stdout);
    let row = human
        .lines()
        .find(|line| line.trim_start().starts_with(&id))
        .unwrap_or_else(|| panic!("no row for {id}:\n{human}"));
    assert!(row.contains("  copy  "), "{row}");
    assert!(row.ends_with("ok (2 copied, 0 deleted, 0 failed)"), "{row}");

    let shown = run(&ctx, &["jobs", "log", &id]);
    assert!(shown.status.success(), "{}", text(&shown.stderr));
    let out = text(&shown.stdout);
    assert!(out.starts_with("== initiator log from machine "), "{out}");
    assert!(!out.contains("not finished"), "{out}");
    for expected in [
        "start    copy ",
        "copied   a.txt",
        "copied   sub/b.txt",
        "summary  2 copied (9 B), 0 deleted, 0 failed",
        "end      ok",
    ] {
        assert!(out.contains(expected), "missing {expected:?} in:\n{out}");
    }

    // The log file itself reads the same; a copied log needs no folder.
    let by_file = run(&ctx, &["jobs", "log", &log_file]);
    assert!(by_file.status.success(), "{}", text(&by_file.stderr));
    assert_eq!(text(&by_file.stdout), out);

    let raw = run(&ctx, &["jobs", "log", &id, "--json"]);
    assert!(raw.status.success(), "{}", text(&raw.stderr));
    let kinds = event_kinds(&raw.stdout);
    assert_eq!(kinds.first().map(String::as_str), Some("run-start"));
    assert_eq!(kinds.last().map(String::as_str), Some("run-end"));
    assert_eq!(
        kinds.iter().filter(|kind| *kind == "file-copied").count(),
        2
    );

    // A second run lists first; `--recent-limit` keeps the newest.
    let again = run(
        &ctx,
        &[
            "copy",
            "-v",
            &format!("{}/", src.display()),
            &format!("{}/", ctx.workspace.join("dst2").display()),
        ],
    );
    assert!(again.status.success(), "{}", text(&again.stderr));
    let newest = job_id_in(&text(&again.stderr));
    let listed = run(&ctx, &["jobs", "list", "--json"]);
    let listing: serde_json::Value = serde_json::from_slice(&listed.stdout).unwrap();
    let ids: Vec<&str> = listing["jobs"]
        .as_array()
        .unwrap()
        .iter()
        .map(|job| job["run_id"].as_str().unwrap())
        .collect();
    assert_eq!(ids, [newest.as_str(), id.as_str()]);
    let limited = run(&ctx, &["jobs", "list", "--json", "--recent-limit", "1"]);
    let listing: serde_json::Value = serde_json::from_slice(&limited.stdout).unwrap();
    assert_eq!(listing["jobs"].as_array().unwrap().len(), 1, "{listing}");
    assert_eq!(listing["jobs"][0]["run_id"], newest.as_str());

    let unknown = run(&ctx, &["jobs", "log", "0123abcd"]);
    assert!(!unknown.status.success());
    assert!(
        text(&unknown.stderr).contains("no log for job 0123abcd on this machine"),
        "{}",
        text(&unknown.stderr)
    );
}

#[test]
fn a_push_is_logged_here_and_on_the_daemon_under_one_run_id() {
    let ctx = TestContext::new();
    let src = ctx.workspace.join("src");
    fs::create_dir_all(&src).unwrap();
    fs::write(src.join("a.txt"), b"alpha").unwrap();
    let remote = format!("127.0.0.1:{}", ctx.daemon_port);

    let pushed = run(
        &ctx,
        &[
            "copy",
            "-v",
            &format!("{}/", src.display()),
            &format!("{remote}:/test/"),
        ],
    );
    assert!(pushed.status.success(), "{}", text(&pushed.stderr));
    let id = job_id_in(&text(&pushed.stderr));

    let here = text(&run(&ctx, &["jobs", "log", &id]).stdout);
    assert!(here.starts_with("== initiator log from machine "), "{here}");
    assert!(here.contains("sent     a.txt"), "{here}");
    assert!(here.contains("end      ok"), "{here}");

    // The daemon keeps its log under the same run ID.
    let deadline = Instant::now() + Duration::from_secs(30);
    let there = loop {
        let shown = run(&ctx, &["jobs", "log", &remote, &id]);
        assert!(shown.status.success(), "{}", text(&shown.stderr));
        let out = text(&shown.stdout);
        if !out.contains("not finished") {
            break out;
        }
        assert!(Instant::now() < deadline, "the log never finished:\n{out}");
        std::thread::sleep(Duration::from_millis(100));
    };
    assert!(
        there.starts_with("== destination log from machine "),
        "{there}"
    );
    assert!(there.contains("copied   a.txt"), "{there}");
    // ...as its first session (attempt 1).
    assert!(there.contains("(attempt 1)"), "{there}");

    // Its record names the run; its own job ID finds the same log.
    let listed = run(&ctx, &["jobs", "list", &remote, "--json"]);
    let state: serde_json::Value = serde_json::from_slice(&listed.stdout).unwrap();
    let record = state["recent"]
        .as_array()
        .and_then(|recent| recent.iter().find(|job| job["run_id"] == id.as_str()))
        .unwrap_or_else(|| panic!("no record of run {id}: {state}"));
    let job_id = record["transfer_id"].as_str().unwrap();
    let by_job_id = text(&run(&ctx, &["jobs", "log", &remote, job_id]).stdout);
    assert_eq!(by_job_id, there);
}

#[test]
fn a_pull_logs_each_file_copied_here() {
    let ctx = TestContext::new();
    fs::write(ctx.module_dir.join("pulled.txt"), b"gamma").unwrap();
    let dst = ctx.workspace.join("pulled");
    let remote = format!("127.0.0.1:{}", ctx.daemon_port);

    let pulled = run(
        &ctx,
        &[
            "copy",
            "-v",
            &format!("{remote}:/test/"),
            &format!("{}/", dst.display()),
        ],
    );
    assert!(pulled.status.success(), "{}", text(&pulled.stderr));
    let id = job_id_in(&text(&pulled.stderr));

    let here = text(&run(&ctx, &["jobs", "log", &id]).stdout);
    assert!(here.starts_with("== initiator log from machine "), "{here}");
    // This end wrote the file: copied, not merely sent.
    assert!(here.contains("copied   pulled.txt"), "{here}");
    assert!(here.contains("end      ok"), "{here}");
}

/// Waits out a daemon's closing of the run's logs; the text of every log it
/// kept for `id`.
fn daemon_logs(ctx: &TestContext, remote: &str, id: &str) -> String {
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        let shown = run(ctx, &["jobs", "log", remote, id]);
        assert!(shown.status.success(), "{}", text(&shown.stderr));
        let out = text(&shown.stdout);
        if !out.contains("not finished") {
            return out;
        }
        assert!(Instant::now() < deadline, "the log never finished:\n{out}");
        std::thread::sleep(Duration::from_millis(100));
    }
}

#[test]
fn a_remote_to_remote_run_is_logged_everywhere_under_one_run_id() {
    let ctx = TestContext::new();
    let other = ctx.spawn_second_daemon(
        "daemon_b",
        &DaemonOptions {
            delegation: true,
            ..Default::default()
        },
    );
    fs::write(ctx.module_dir.join("a.txt"), b"alpha").unwrap();
    let a = format!("127.0.0.1:{}", ctx.daemon_port);
    let b = format!("127.0.0.1:{}", other.port);

    let copied = run(
        &ctx,
        &["copy", "-v", &format!("{a}:/test/"), &format!("{b}:/test/")],
    );
    assert!(copied.status.success(), "{}", text(&copied.stderr));
    let id = job_id_in(&text(&copied.stderr));

    let here = text(&run(&ctx, &["jobs", "log", &id]).stdout);
    assert!(here.starts_with("== initiator log from machine "), "{here}");
    assert!(here.contains("end      ok"), "{here}");
    let source = daemon_logs(&ctx, &a, &id);
    assert!(
        source.starts_with("== source log from machine "),
        "{source}"
    );
    let destination = daemon_logs(&ctx, &b, &id);
    assert!(
        destination.starts_with("== destination log from machine "),
        "{destination}"
    );
    assert!(destination.contains("copied   a.txt"), "{destination}");

    // A daemon at both ends of one run keeps two logs, one per role.
    fs::create_dir_all(other.module_dir.join("in")).unwrap();
    fs::write(other.module_dir.join("in/c.txt"), b"gamma").unwrap();
    let copied = run(
        &ctx,
        &[
            "copy",
            "-v",
            &format!("{b}:/test/in/"),
            &format!("{b}:/test/out/"),
        ],
    );
    assert!(copied.status.success(), "{}", text(&copied.stderr));
    let id = job_id_in(&text(&copied.stderr));
    let both = daemon_logs(&ctx, &b, &id);
    let headings: Vec<&str> = both
        .lines()
        .filter(|line| line.starts_with("== "))
        .collect();
    assert_eq!(headings.len(), 2, "{both}");
    assert!(
        headings.iter().any(|h| h.starts_with("== source log")),
        "{both}"
    );
    assert!(
        headings.iter().any(|h| h.starts_with("== destination log")),
        "{both}"
    );
}

#[test]
fn this_machines_config_says_how_many_logs_to_keep() {
    let ctx = TestContext::new();
    fs::write(ctx.config_dir.join("config.toml"), "[jobs]\nkeep = 1\n").unwrap();
    let src = ctx.workspace.join("src");
    fs::create_dir_all(&src).unwrap();
    fs::write(src.join("a.txt"), b"alpha").unwrap();
    let mut newest = String::new();
    for dst in ["one", "two"] {
        let copied = run(
            &ctx,
            &[
                "copy",
                "-v",
                &format!("{}/", src.display()),
                &format!("{}/", ctx.workspace.join(dst).display()),
            ],
        );
        assert!(copied.status.success(), "{}", text(&copied.stderr));
        newest = job_id_in(&text(&copied.stderr));
    }
    let listed = run(&ctx, &["jobs", "list", "--json"]);
    let listing: serde_json::Value = serde_json::from_slice(&listed.stdout).unwrap();
    let jobs = listing["jobs"].as_array().unwrap();
    assert_eq!(jobs.len(), 1, "{listing}");
    assert_eq!(jobs[0]["run_id"], newest.as_str());
}

#[test]
fn a_move_logs_that_it_removed_its_source() {
    let ctx = TestContext::new();
    let src = ctx.workspace.join("src");
    fs::create_dir_all(&src).unwrap();
    fs::write(src.join("a.txt"), b"alpha").unwrap();
    let moved = run(
        &ctx,
        &[
            "move",
            "--yes",
            "-v",
            &format!("{}/", src.display()),
            &format!("{}/", ctx.workspace.join("dst").display()),
        ],
    );
    assert!(moved.status.success(), "{}", text(&moved.stderr));
    assert!(!src.exists());
    let id = job_id_in(&text(&moved.stderr));
    let shown = text(&run(&ctx, &["jobs", "log", &id]).stdout);
    assert!(shown.contains("start    move "), "{shown}");
    // How the path is spelled is the platform's; that it is named is ours.
    assert!(
        shown
            .lines()
            .any(|line| line.contains("info     move: removed the source ")
                && line.contains("src")),
        "{shown}"
    );
    assert!(shown.contains("end      ok"), "{shown}");
}

/// Review cr-jl2-1: a run that writes nothing — `--dry-run`, `--null` —
/// names no file copied or deleted and counts none; its log says nothing
/// was written, and what the run would have done.
#[test]
fn a_run_that_writes_nothing_logs_no_copies() {
    let ctx = TestContext::new();
    let src = ctx.workspace.join("src");
    fs::create_dir_all(&src).unwrap();
    fs::write(src.join("planned.txt"), b"alpha").unwrap();
    let log_of = |args: &[&str]| -> String {
        let ran = run(&ctx, args);
        assert!(ran.status.success(), "{}", text(&ran.stderr));
        let id = job_id_in(&text(&ran.stderr));
        text(&run(&ctx, &["jobs", "log", &id]).stdout)
    };

    let dst = ctx.workspace.join("dry");
    let dry = log_of(&[
        "copy",
        "-v",
        "--dry-run",
        &format!("{}/", src.display()),
        &format!("{}/", dst.display()),
    ]);
    assert!(!dst.join("planned.txt").exists());
    assert!(dry.contains("options: dry-run"), "{dry}");
    assert!(!dry.contains("copied   "), "{dry}");
    assert!(
        dry.contains(
            "info     dry run: nothing was written; it would have copied 1 file(s), 5 B, \
             and deleted 0"
        ),
        "{dry}"
    );
    assert!(
        dry.contains("summary  0 copied (0 B), 0 deleted, 0 failed"),
        "{dry}"
    );
    assert!(
        dry.contains("end      ok: dry run: nothing was written"),
        "{dry}"
    );

    let discarded = log_of(&[
        "copy",
        "-v",
        "--null",
        &format!("{}/", src.display()),
        &format!("{}/", ctx.workspace.join("null").display()),
    ]);
    assert!(discarded.contains("options: null"), "{discarded}");
    assert!(!discarded.contains("copied   "), "{discarded}");
    assert!(
        discarded
            .contains("info     --null: nothing was written; 1 file(s), 5 B read and discarded"),
        "{discarded}"
    );
    assert!(
        discarded.contains("summary  0 copied (0 B), 0 deleted, 0 failed"),
        "{discarded}"
    );

    // A dry-run mirror deletes nothing, and says so.
    let mirror_dst = ctx.workspace.join("mirror");
    fs::create_dir_all(&mirror_dst).unwrap();
    fs::write(mirror_dst.join("extra.txt"), b"stays").unwrap();
    let mirrored = log_of(&[
        "mirror",
        "-v",
        "--yes",
        "--dry-run",
        &format!("{}/", src.display()),
        &format!("{}/", mirror_dst.display()),
    ]);
    assert!(mirror_dst.join("extra.txt").exists());
    assert!(!mirrored.contains("deleted  "), "{mirrored}");
    assert!(mirrored.contains("and deleted 1"), "{mirrored}");
    assert!(!mirrored.contains("copied   "), "{mirrored}");
    assert!(
        mirrored.contains("summary  0 copied (0 B), 0 deleted, 0 failed"),
        "{mirrored}"
    );
}

/// JOB_LOGS jl-3, "Detached jobs": a run waiting on a daemon that cannot
/// be reached stays waiting, and the listing says why.
#[test]
fn a_detached_run_whose_daemon_is_unreachable_stays_waiting() {
    let ctx = TestContext::new();
    // A port nothing listens on: bind one, then let it go.
    let port = std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let runs = ctx.config_dir.join("jobs").join("runs");
    fs::create_dir_all(&runs).unwrap();
    let id = "0123456789abcdef0123456789abcdef";
    let record = serde_json::json!({
        "format": "blit-run-record", "version": 1, "run_id": id, "machine": "m1",
        "attempt": 1, "verb": "copy", "source": "a:/m/", "destination": "b:/m/",
        "started_ms": 1, "ended_ms": 2,
        "state": {"waiting": {"daemon": format!("127.0.0.1:{port}"), "job_id": "t1-0"}},
        "detail": "runs elsewhere",
    });
    fs::write(
        runs.join(format!("{id}.run.json")),
        serde_json::to_vec(&record).unwrap(),
    )
    .unwrap();

    let listed = run(&ctx, &["jobs", "list"]);
    assert!(listed.status.success(), "{}", text(&listed.stderr));
    let out = text(&listed.stdout);
    assert!(
        out.contains(&format!(
            "waiting on 127.0.0.1:{port} (job t1-0) — could not ask"
        )),
        "{out}"
    );
    let on_disk: serde_json::Value =
        serde_json::from_slice(&fs::read(runs.join(format!("{id}.run.json"))).unwrap()).unwrap();
    assert!(on_disk["state"]["waiting"].is_object(), "{on_disk}");
}

/// JOB_LOGS jl-3 with review cr-jl2-1: a dry run's record, like its log,
/// counts nothing copied and says nothing was written.
#[test]
fn a_dry_runs_record_counts_nothing_copied() {
    let ctx = TestContext::new();
    let src = ctx.workspace.join("src");
    fs::create_dir_all(&src).unwrap();
    fs::write(src.join("planned.txt"), b"alpha").unwrap();
    let ran = run(
        &ctx,
        &[
            "copy",
            "--dry-run",
            &format!("{}/", src.display()),
            &format!("{}/", ctx.workspace.join("dst").display()),
        ],
    );
    assert!(ran.status.success(), "{}", text(&ran.stderr));
    let listed = run(&ctx, &["jobs", "list", "--json"]);
    let listing: serde_json::Value = serde_json::from_slice(&listed.stdout).unwrap();
    let job = &listing["jobs"][0];
    assert_eq!(job["state"], "finished", "{job}");
    assert_eq!(job["outcome"], "ok", "{job}");
    assert_eq!(job["files_copied"], 0, "{job}");
    assert_eq!(job["bytes_copied"], 0, "{job}");
    assert_eq!(job["detail"], "dry run: nothing was written", "{job}");
}
