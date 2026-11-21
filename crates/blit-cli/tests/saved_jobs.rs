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

/// cr-ecm1-4: a job file holding BOTH case modes — which no command line
/// can write — is refused, not run under whichever of them the code reads.
/// Before, `jobs run` read `ignore_case`, and this mirror deleted
/// `ONLY.CFG`, which the job as made (`--match-case --include '*.cfg'`)
/// never named. A pair from before the case modes existed is refused too.
#[test]
fn a_job_file_holding_two_options_no_command_takes_together_is_refused() {
    let ctx = TestContext::new();
    let (src, dst) = (ctx.workspace.join("src"), ctx.workspace.join("dst"));
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst).unwrap();
    fs::write(src.join("keep.cfg"), b"kept").unwrap();
    fs::write(dst.join("ONLY.CFG"), b"only here").unwrap();
    let made = blit_in(
        &ctx,
        &ctx.workspace,
        &[
            "mirror",
            "--yes",
            "--match-case",
            "--include",
            "*.cfg",
            "--export",
            "job.json",
            "src/",
            "dst/",
        ],
    );
    assert!(made.status.success(), "{}", text(&made.stderr));
    assert_eq!(names(&dst), ["ONLY.CFG", "keep.cfg"], "as typed");
    let path = ctx.workspace.join("job.json");
    let exported = fs::read(&path).unwrap();

    for (one, other, typed) in [
        (
            "match_case",
            "ignore_case",
            ["--match-case", "--ignore-case"],
        ),
        ("checksum", "size_only", ["--checksum", "--size-only"]),
    ] {
        let mut job: serde_json::Value = serde_json::from_slice(&exported).unwrap();
        job["spec"]["options"][one] = true.into();
        job["spec"]["options"][other] = true.into();
        fs::write(&path, serde_json::to_vec(&job).unwrap()).unwrap();
        fs::remove_file(dst.join("keep.cfg")).unwrap();

        let refused = blit_in(&ctx, &ctx.workspace, &["jobs", "run", "./job.json"]);
        let stderr = text(&refused.stderr);
        assert!(!refused.status.success(), "{one} with {other}: {stderr}");
        assert!(
            typed.iter().all(|option| stderr.contains(option)),
            "{stderr}"
        );
        assert_eq!(names(&dst), ["ONLY.CFG"], "{one} with {other}: nothing ran");
        fs::write(dst.join("keep.cfg"), b"kept").unwrap();
    }

    // The job as it was exported still runs.
    fs::write(&path, &exported).unwrap();
    let ran = blit_in(&ctx, &ctx.workspace, &["jobs", "run", "./job.json"]);
    assert!(ran.status.success(), "{}", text(&ran.stderr));
    assert_eq!(names(&dst), ["ONLY.CFG", "keep.cfg"]);
}

/// cr-ecm3-2: a `move` that is refused for a filter saves no job. A job
/// keeps patterns, never a pattern file, so `move --exclude-from F --save
/// NAME` — refused, but only after the job was saved — left a job with no
/// filter in it, and `jobs run NAME` moved the whole tree, the file the
/// user meant to leave included. Every filter option is refused before
/// anything is kept: nothing saved, nothing exported, nothing moved.
#[test]
fn a_move_refused_for_a_filter_saves_no_job() {
    let ctx = TestContext::new();
    let (src, dst) = (ctx.workspace.join("src"), ctx.workspace.join("dst"));
    fs::create_dir_all(&src).unwrap();
    fs::write(src.join("keep.txt"), b"moved").unwrap();
    fs::write(src.join("stay.tmp"), b"left behind").unwrap();
    fs::write(ctx.workspace.join("skip.txt"), "*.tmp\n").unwrap();

    for (at, filter) in [
        &["--exclude-from", "skip.txt"][..],
        &["--include-from", "skip.txt"],
        &["--exclude", "*.tmp"],
        &["--files-from", "skip.txt"],
        &["--min-size", "1K"],
    ]
    .into_iter()
    .enumerate()
    {
        let name = format!("sample{at}");
        let export = format!("job{at}.json");
        let mut args = vec!["move", "-y"];
        args.extend(filter);
        args.extend(["--save", &name, "--export", &export, "src/", "dst/"]);
        let refused = blit_in(&ctx, &ctx.workspace, &args);
        let stderr = text(&refused.stderr);
        assert!(!refused.status.success(), "{filter:?}: {stderr}");
        assert!(
            stderr.contains("move does not support filters"),
            "{filter:?}: {stderr}"
        );
        assert!(
            !ctx.workspace.join(&export).exists(),
            "{filter:?}: no job is exported"
        );
        let again = blit_in(&ctx, &ctx.workspace, &["jobs", "run", &name]);
        assert!(
            !again.status.success(),
            "{filter:?}: no job was saved to run again\n{}{}",
            text(&again.stdout),
            text(&again.stderr)
        );
        assert_eq!(
            names(&src),
            ["keep.txt", "stay.tmp"],
            "{filter:?}: nothing moved"
        );
        assert!(!dst.exists(), "{filter:?}: nothing moved");
    }
    assert_eq!(listing(&ctx)["saved"].as_array().map(Vec::len), Some(0));
}

/// cr-ecm4-4: a command refused for what it says leaves a saved job of
/// that name as it was. A job is saved before its command runs, and the
/// refusals came after: `move --ignore-case --save nightly`, and a mirror
/// whose pattern file held `[aé].txt`, each REPLACED a working `nightly`
/// with a job that can never run. Every refusal a command's own words
/// decide — its options, its filter, its two endpoints — now comes first:
/// nothing saved, nothing exported, nothing transferred, and `jobs run
/// nightly` does what it did.
#[test]
fn a_command_refused_for_what_it_says_leaves_the_saved_job_as_it_was() {
    let ctx = TestContext::new();
    let (src, dst) = (ctx.workspace.join("src"), ctx.workspace.join("dst"));
    fs::create_dir_all(&src).unwrap();
    fs::write(src.join("keep.txt"), b"kept").unwrap();
    fs::write(ctx.workspace.join("bad.txt"), "[aé].txt\n").unwrap();

    let saved = blit_in(
        &ctx,
        &ctx.workspace,
        &["copy", "--save", "nightly", "src/", "dst/"],
    );
    assert!(saved.status.success(), "{}", text(&saved.stderr));
    assert_eq!(names(&dst), ["keep.txt"]);

    for (at, (command, words)) in [
        // The reviewer's two.
        (
            &["move", "-y", "--ignore-case", "src/", "other/"][..],
            "move does not support --ignore-case",
        ),
        (
            &[
                "mirror",
                "-y",
                "--exclude-from",
                "bad.txt",
                "src/",
                "other/",
            ],
            "a class like [aé] matches one byte",
        ),
        // Each kind of refusal there is: an option of `move`'s, a pattern
        // that is no glob, a size that cannot be read, an option the verb
        // or the route cannot honour, the two endpoints themselves.
        (
            &["move", "-y", "--dry-run", "src/", "other/"],
            "move does not support --dry-run",
        ),
        (
            &["move", "-y", "--force", "src/", "other/"],
            "move does not support --force",
        ),
        (
            &["mirror", "-y", "--exclude", "[", "src/", "other/"],
            "is not a valid glob",
        ),
        (
            &["copy", "--min-size", "tiny", "src/", "other/"],
            "--min-size tiny",
        ),
        (
            &["mirror", "-y", "--null", "src/", "other/"],
            "--null is not supported with `blit mirror`",
        ),
        (
            &["copy", "--detach", "src/", "other/"],
            "--detach is only supported for remote",
        ),
        (
            &["copy", "src/", "src/"],
            "source and destination resolve to the same path",
        ),
    ]
    .into_iter()
    .enumerate()
    {
        let before = listing(&ctx)["saved"].clone();
        let export = format!("job{at}.json");
        let mut args = command.to_vec();
        args.splice(1..1, ["--save", "nightly", "--export", export.as_str()]);
        let refused = blit_in(&ctx, &ctx.workspace, &args);
        let stderr = text(&refused.stderr);
        assert!(!refused.status.success(), "{command:?}: {stderr}");
        assert!(stderr.contains(words), "{command:?}: {stderr}");
        assert!(
            !stderr.contains("saved job nightly"),
            "{command:?}: nothing is saved\n{stderr}"
        );
        assert!(
            !ctx.workspace.join(&export).exists(),
            "{command:?}: no job is exported"
        );
        assert!(
            !ctx.workspace.join("other").exists(),
            "{command:?}: nothing is transferred"
        );
        assert_eq!(
            listing(&ctx)["saved"],
            before,
            "{command:?}: the saved job is as it was"
        );

        // And it runs as it did.
        fs::remove_file(dst.join("keep.txt")).unwrap();
        let again = blit_in(&ctx, &ctx.workspace, &["jobs", "run", "nightly"]);
        assert!(
            again.status.success(),
            "{command:?}: the job saved before still runs\n{}{}",
            text(&again.stdout),
            text(&again.stderr)
        );
        assert_eq!(names(&dst), ["keep.txt"], "{command:?}");
        assert_eq!(names(&src), ["keep.txt"], "{command:?}: nothing moved");
    }
}

/// cr-ecm5-2: and so does a command refused for its ROUTE — an option a
/// transfer through a daemon does not take, an endpoint that names no
/// module. Each route's arm asked these itself, after the job was saved:
/// a remote `mirror -y --workers 1 --save nightly` replaced a working
/// saved copy job and was then refused, and since a job does not keep
/// that diagnostic option, `jobs run nightly` ran the mirror and deleted a
/// destination-only file. Pushed to a daemon and pulled from one.
#[test]
fn a_command_refused_for_its_route_leaves_the_saved_job_as_it_was() {
    let ctx = TestContext::new();
    let (src, dst) = (ctx.workspace.join("src"), ctx.workspace.join("dst"));
    fs::create_dir_all(&src).unwrap();
    fs::write(src.join("keep.txt"), b"kept").unwrap();
    fs::write(ctx.module_dir.join("served.txt"), b"served").unwrap();
    let remote = format!("127.0.0.1:{}:/test/pushed/", ctx.daemon_port);
    let served = format!("127.0.0.1:{}:/test/", ctx.daemon_port);
    let no_module = format!("127.0.0.1:{}", ctx.daemon_port);

    let saved = blit_in(
        &ctx,
        &ctx.workspace,
        &["copy", "--save", "nightly", "src/", "dst/"],
    );
    assert!(saved.status.success(), "{}", text(&saved.stderr));
    // What only the destination holds: a copy leaves it, a mirror would not.
    fs::write(dst.join("only-here.txt"), b"kept by a copy").unwrap();

    for (at, (command, words)) in [
        // The reviewer's.
        (
            &["mirror", "-y", "--workers", "1", "src/", remote.as_str()][..],
            "--workers limiter is not supported for remote transfers",
        ),
        (
            &["copy", "--workers", "1", served.as_str(), "pulled/"],
            "--workers limiter is not supported for remote transfers",
        ),
        (
            &["move", "-y", "--workers", "1", "src/", remote.as_str()],
            "--workers limiter is not supported for remote transfers",
        ),
        (
            &["mirror", "--dry-run", "src/", remote.as_str()],
            "--dry-run is not supported for remote transfers",
        ),
        (
            &["copy", "src/", no_module.as_str()],
            "remote destination must include a module or root",
        ),
    ]
    .into_iter()
    .enumerate()
    {
        let before = listing(&ctx)["saved"].clone();
        let export = format!("job{at}.json");
        let mut args = command.to_vec();
        args.splice(1..1, ["--save", "nightly", "--export", export.as_str()]);
        let refused = blit_in(&ctx, &ctx.workspace, &args);
        let stderr = text(&refused.stderr);
        assert!(!refused.status.success(), "{command:?}: {stderr}");
        assert!(stderr.contains(words), "{command:?}: {stderr}");
        assert!(
            !stderr.contains("saved job nightly"),
            "{command:?}: nothing is saved\n{stderr}"
        );
        assert!(
            !ctx.workspace.join(&export).exists(),
            "{command:?}: no job is exported"
        );
        assert!(
            !ctx.module_dir.join("pushed").exists() && !ctx.workspace.join("pulled").exists(),
            "{command:?}: nothing is transferred"
        );
        assert_eq!(
            listing(&ctx)["saved"],
            before,
            "{command:?}: the saved job is as it was"
        );

        // The job is still the copy it was: it copies, and deletes nothing.
        fs::remove_file(dst.join("keep.txt")).unwrap();
        let again = blit_in(&ctx, &ctx.workspace, &["jobs", "run", "nightly"]);
        assert!(
            again.status.success(),
            "{command:?}: the job saved before still runs\n{}{}",
            text(&again.stdout),
            text(&again.stderr)
        );
        assert_eq!(
            names(&dst),
            ["keep.txt", "only-here.txt"],
            "{command:?}: a copy, as saved"
        );
        assert_eq!(names(&src), ["keep.txt"], "{command:?}: nothing moved");
    }
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
        // The usage line names the binary as invoked (`blit.exe` on
        // Windows), so look for the sub-verb, not the binary.
        let stdout = text(&help.stdout);
        assert!(
            stdout.contains("Usage: ") && stdout.contains(&format!("jobs {verb}")),
            "{verb}: {stdout}"
        );
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

/// Review cr-jl5-1: a run whose `--files-from` list lives in its own
/// destination, and is itself listed, overwrites that list as it copies;
/// its job still keeps the list it ran with. This passes with the fix but
/// cannot be made to fail without it on demand — the record's thread
/// usually reads the list before the copy writes it — so the guard is
/// `run_log::tests::the_spec_keeps_the_list_the_run_started_with`.
#[test]
fn a_job_keeps_the_list_its_run_overwrote() {
    let ctx = TestContext::new();
    let src = ctx.workspace.join("src");
    let dst = ctx.workspace.join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst).unwrap();
    fs::write(src.join("a.txt"), b"alpha").unwrap();
    fs::write(src.join("b.txt"), b"beta").unwrap();
    fs::write(src.join("list.txt"), "b.txt\n").unwrap();
    fs::write(dst.join("list.txt"), "a.txt\nlist.txt\n").unwrap();

    let copied = blit_in(
        &ctx,
        &ctx.workspace,
        &[
            "copy",
            "--export",
            "job.json",
            "--files-from",
            "dst/list.txt",
            "src/",
            "dst/",
        ],
    );
    assert!(copied.status.success(), "{}", text(&copied.stderr));
    // The copy ran the original list and overwrote it.
    assert_eq!(names(&dst), ["a.txt", "list.txt"]);
    assert_eq!(fs::read_to_string(dst.join("list.txt")).unwrap(), "b.txt\n");
    let job: serde_json::Value =
        serde_json::from_slice(&fs::read(ctx.workspace.join("job.json")).unwrap()).unwrap();
    assert_eq!(
        job["spec"]["files_from"],
        serde_json::json!(["a.txt", "list.txt"]),
        "{job}"
    );
}
