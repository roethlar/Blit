//! JOB_LOGS jl-3b: saved and exported jobs — `--save`, `--export`,
//! `blit jobs save|export|run|delete` — and the acceptance criterion
//! "Saved jobs reproduce": `blit jobs run <name>` from another working
//! directory, or after the `--files-from` file changed, transfers exactly
//! what the original run did.

mod common;

use std::fs;
use std::path::Path;
use std::process::{Command, Output};
use std::time::Duration;

use common::{cli_bin, run_with_timeout, TestContext};

/// `blit <args>` in `cwd`, with the test's own per-user folder.
fn blit_in(ctx: &TestContext, cwd: &Path, args: &[&str]) -> Output {
    let mut cmd = Command::new(cli_bin());
    cmd.current_dir(cwd)
        .arg("--config-dir")
        .arg(&ctx.config_dir)
        .args(args);
    run_with_timeout(cmd, Duration::from_secs(60))
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

fn listing(ctx: &TestContext) -> serde_json::Value {
    let listed = blit_in(ctx, &ctx.workspace, &["jobs", "list", "--json"]);
    assert!(listed.status.success(), "{}", text(&listed.stderr));
    serde_json::from_slice(&listed.stdout).unwrap()
}

/// What `dir` holds, file names sorted.
fn names(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(dir)
        .map(|entries| {
            entries
                .flatten()
                .map(|entry| entry.file_name().to_string_lossy().into_owned())
                .collect()
        })
        .unwrap_or_default();
    names.sort();
    names
}

#[test]
fn a_saved_job_runs_again_exactly_from_anywhere() {
    let ctx = TestContext::new();
    let typed = ctx.workspace.join("typed");
    fs::create_dir_all(typed.join("src")).unwrap();
    fs::write(typed.join("src/a.txt"), b"alpha").unwrap();
    fs::write(typed.join("src/b.txt"), b"beta").unwrap();
    fs::write(typed.join("list.txt"), "a.txt\n").unwrap();

    // Relative paths, from the folder the command is typed in.
    let saved = blit_in(
        &ctx,
        &typed,
        &[
            "copy",
            "--save",
            "nightly",
            "--files-from",
            "list.txt",
            "src/",
            "dst/",
        ],
    );
    assert!(saved.status.success(), "{}", text(&saved.stderr));
    assert!(
        text(&saved.stderr).contains("saved job nightly"),
        "{}",
        text(&saved.stderr)
    );
    assert_eq!(names(&typed.join("dst")), ["a.txt"]);

    // The list changes, and the destination is emptied; the job runs from
    // another folder and still does exactly what it did.
    fs::write(typed.join("list.txt"), "b.txt\n").unwrap();
    fs::remove_dir_all(typed.join("dst")).unwrap();
    let elsewhere = ctx.workspace.join("elsewhere");
    fs::create_dir_all(&elsewhere).unwrap();
    let rerun = blit_in(&ctx, &elsewhere, &["jobs", "run", "nightly"]);
    assert!(rerun.status.success(), "{}", text(&rerun.stderr));
    assert_eq!(names(&typed.join("dst")), ["a.txt"]);
    assert!(!elsewhere.join("dst").exists());

    let listed = listing(&ctx);
    assert_eq!(listed["saved"][0]["name"], "nightly", "{listed}");
    assert_eq!(listed["saved"][0]["verb"], "copy", "{listed}");
    // Newest first: the rerun, recorded as running the saved job.
    assert_eq!(listed["jobs"][0]["saved_job"], "nightly", "{listed}");
    assert_eq!(listed["jobs"][0]["files_copied"], 1, "{listed}");
    // The list written out for the rerun is gone again.
    assert!(
        !names(&ctx.config_dir.join("jobs"))
            .iter()
            .any(|name| name.starts_with("files-from-")),
        "{:?}",
        names(&ctx.config_dir.join("jobs"))
    );

    let human = text(&blit_in(&ctx, &ctx.workspace, &["jobs", "list"]).stdout);
    assert!(human.contains("Saved jobs (1):"), "{human}");
    assert!(human.contains("  nightly  copy  "), "{human}");
}

#[test]
fn a_job_is_exported_saved_run_from_its_file_and_deleted() {
    let ctx = TestContext::new();
    let src = ctx.workspace.join("src");
    fs::create_dir_all(&src).unwrap();
    fs::write(src.join("a.txt"), b"alpha").unwrap();
    let dst = ctx.workspace.join("dst");
    let exported = ctx.workspace.join("job.json");

    let copied = blit_in(
        &ctx,
        &ctx.workspace,
        &["copy", "--export", "job.json", "src/", "dst/"],
    );
    assert!(copied.status.success(), "{}", text(&copied.stderr));
    let job: serde_json::Value = serde_json::from_slice(&fs::read(&exported).unwrap()).unwrap();
    assert_eq!(job["format"], "blit-job", "{job}");
    assert_eq!(job["spec"]["verb"], "copy", "{job}");
    assert_eq!(job["run"]["state"], "finished", "{job}");
    assert_eq!(job["run"]["files_copied"], 1, "{job}");
    let run_id = job["run"]["run_id"].as_str().unwrap().to_string();

    // The file runs again.
    fs::remove_dir_all(&dst).unwrap();
    let rerun = blit_in(&ctx, &ctx.workspace, &["jobs", "run", "./job.json"]);
    assert!(rerun.status.success(), "{}", text(&rerun.stderr));
    assert_eq!(fs::read(dst.join("a.txt")).unwrap(), b"alpha");

    // A past run kept by name, exported by name and by ID, then deleted.
    let saved = blit_in(&ctx, &ctx.workspace, &["jobs", "save", &run_id, "backup"]);
    assert!(saved.status.success(), "{}", text(&saved.stderr));
    for (job, file) in [("backup", "by-name.json"), (run_id.as_str(), "by-id.json")] {
        let out = blit_in(&ctx, &ctx.workspace, &["jobs", "export", job, file]);
        assert!(out.status.success(), "{}", text(&out.stderr));
        let written: serde_json::Value =
            serde_json::from_slice(&fs::read(ctx.workspace.join(file)).unwrap()).unwrap();
        assert_eq!(written["spec"]["verb"], "copy", "{written}");
    }
    let by_name: serde_json::Value =
        serde_json::from_slice(&fs::read(ctx.workspace.join("by-name.json")).unwrap()).unwrap();
    assert_eq!(by_name["name"], "backup", "{by_name}");
    assert!(by_name["run"].is_null(), "a saved job is no run: {by_name}");

    let deleted = blit_in(&ctx, &ctx.workspace, &["jobs", "delete", "backup"]);
    assert!(deleted.status.success(), "{}", text(&deleted.stderr));
    let gone = blit_in(&ctx, &ctx.workspace, &["jobs", "run", "backup"]);
    assert!(!gone.status.success());
    assert!(
        text(&gone.stderr).contains("no saved job named backup"),
        "{}",
        text(&gone.stderr)
    );
}

#[test]
fn a_job_from_another_machine_is_refused() {
    let ctx = TestContext::new();
    let src = ctx.workspace.join("src");
    fs::create_dir_all(&src).unwrap();
    fs::write(src.join("a.txt"), b"alpha").unwrap();
    let copied = blit_in(
        &ctx,
        &ctx.workspace,
        &["copy", "--export", "job.json", "src/", "dst/"],
    );
    assert!(copied.status.success(), "{}", text(&copied.stderr));
    let path = ctx.workspace.join("job.json");
    let mut job: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    job["spec"]["machine"] = "ffffffffffffffffffffffffffffffff".into();
    job["spec"]["host"] = "faraway".into();
    fs::write(&path, serde_json::to_vec(&job).unwrap()).unwrap();
    fs::remove_dir_all(ctx.workspace.join("dst")).unwrap();

    let refused = blit_in(&ctx, &ctx.workspace, &["jobs", "run", "./job.json"]);
    assert!(!refused.status.success());
    let stderr = text(&refused.stderr);
    assert!(
        stderr.contains("belongs to machine ffffffffffffffffffffffffffffffff (faraway)"),
        "{stderr}"
    );
    assert!(!ctx.workspace.join("dst").exists(), "nothing ran");
}

#[test]
fn a_job_name_must_be_a_plain_name_and_every_sub_verb_has_help() {
    let ctx = TestContext::new();
    let bad = blit_in(
        &ctx,
        &ctx.workspace,
        &["copy", "--save", "../escape", "a/", "b/"],
    );
    assert!(!bad.status.success());
    assert!(
        text(&bad.stderr).contains("a job name may not start with `.`"),
        "{}",
        text(&bad.stderr)
    );
    for verb in ["save", "export", "run", "delete"] {
        let help = blit_in(&ctx, &ctx.workspace, &["jobs", verb, "--help"]);
        assert!(help.status.success(), "{verb}: {}", text(&help.stderr));
        assert!(text(&help.stdout).contains("Usage: blit jobs"), "{verb}");
    }
}

/// Review cr-jl3b-1: a bare word is a saved job, whatever the current
/// folder holds; a job file is given as a path.
#[test]
fn a_saved_jobs_name_means_the_saved_job_wherever_it_is_typed() {
    let ctx = TestContext::new();
    let src = ctx.workspace.join("src");
    fs::create_dir_all(&src).unwrap();
    fs::write(src.join("a.txt"), b"alpha").unwrap();
    let saved = blit_in(
        &ctx,
        &ctx.workspace,
        &["copy", "--save", "nightly", "src/", "saved-dst/"],
    );
    assert!(saved.status.success(), "{}", text(&saved.stderr));
    // Another job, exported to a file named like the saved one.
    let other = blit_in(
        &ctx,
        &ctx.workspace,
        &["copy", "--export", "nightly", "src/", "file-dst/"],
    );
    assert!(other.status.success(), "{}", text(&other.stderr));
    fs::remove_dir_all(ctx.workspace.join("saved-dst")).unwrap();
    fs::remove_dir_all(ctx.workspace.join("file-dst")).unwrap();

    let by_name = blit_in(&ctx, &ctx.workspace, &["jobs", "run", "nightly"]);
    assert!(by_name.status.success(), "{}", text(&by_name.stderr));
    assert!(
        ctx.workspace.join("saved-dst/a.txt").is_file(),
        "the saved job ran"
    );
    assert!(!ctx.workspace.join("file-dst").exists(), "not the file");

    let by_path = blit_in(&ctx, &ctx.workspace, &["jobs", "run", "./nightly"]);
    assert!(by_path.status.success(), "{}", text(&by_path.stderr));
    assert!(
        ctx.workspace.join("file-dst/a.txt").is_file(),
        "the file's job ran"
    );

    // Nor does a file named like a run ID shadow that job's log.
    let listed = listing(&ctx);
    let run_id = listed["jobs"][0]["run_id"].as_str().unwrap().to_string();
    fs::write(ctx.workspace.join(&run_id), b"not a log").unwrap();
    let log = blit_in(&ctx, &ctx.workspace, &["jobs", "log", &run_id]);
    assert!(log.status.success(), "{}", text(&log.stderr));
    assert!(
        text(&log.stdout).starts_with("== initiator log from machine "),
        "{}",
        text(&log.stdout)
    );

    // A name shaped like a run ID is not a job name.
    let shaped = blit_in(
        &ctx,
        &ctx.workspace,
        &[
            "copy",
            "--save",
            "0123456789abcdef0123456789abcdef",
            "src/",
            "x/",
        ],
    );
    assert!(!shaped.status.success());
    // ...and the refusal says why (review cr-jl3bfix1-1).
    assert!(
        text(&shaped.stderr).contains("a run ID's shape, kept for run IDs"),
        "{}",
        text(&shaped.stderr)
    );
}
