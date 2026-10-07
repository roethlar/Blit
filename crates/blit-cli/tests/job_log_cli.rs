//! JOB_LOGS jl-1b: `blit jobs log <host> <job-id>` reads the log a daemon
//! kept for a job — as text, as its JSON lines with `--json`, and only one
//! role with `--role`.

mod common;

use std::fs;
use std::process::Command;
use std::time::{Duration, Instant};

use common::{run_with_timeout, TestContext};

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
