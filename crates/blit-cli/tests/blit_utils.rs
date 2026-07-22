use std::fs;
use std::path::Path;
use std::process::Command;
use std::time::Duration;

mod common;
use common::{run_with_timeout, TestContext};

#[test]
fn daemon_readiness_waits_for_the_owned_module_identity() {
    let expected = Path::new("owned-module-root");
    let probes = ["foreign-module-root", "owned-module-root"];
    let mut probes = probes.into_iter();
    let mut probe_count = 0;

    common::wait_for_owned_readiness(
        2,
        Duration::ZERO,
        || Ok(None),
        || {
            probe_count += 1;
            let path = probes.next().expect("one identity per probe");
            common::exported_modules_include_path([path], expected)
        },
    )
    .expect("the second probe belongs to the spawned daemon");

    assert_eq!(
        probe_count, 2,
        "foreign listeners must not satisfy readiness"
    );
}

#[test]
fn daemon_startup_failure_includes_captured_stderr() {
    const INVALID_FLAG: &str = "--blit-test-invalid-startup-option";
    let panic = std::panic::catch_unwind(|| {
        let _ = TestContext::builder()
            .extra_daemon_args([INVALID_FLAG])
            .build();
    })
    .expect_err("invalid daemon option must fail startup");
    let message = panic
        .downcast_ref::<String>()
        .map(String::as_str)
        .or_else(|| panic.downcast_ref::<&str>().copied())
        .expect("startup panic carries text");
    assert!(
        message.contains(INVALID_FLAG),
        "startup panic must include the daemon's captured stderr:\n{message}"
    );
    assert!(
        message.contains("after 3 attempt(s)"),
        "an early daemon exit must exhaust the bounded startup retry:\n{message}"
    );
}

// ── scan ──────────────────────────────────────────────────────────────

#[test]
fn test_utils_scan() {
    let ctx = TestContext::new();

    // Run with a very short wait so the test completes quickly.
    // Test daemon has no_mdns: true, so we expect no results — just a clean exit.
    let mut cmd = Command::new(&ctx.cli_bin);
    cmd.arg("scan").arg("--wait").arg("1");

    let output = run_with_timeout(cmd, Duration::from_secs(10));
    assert!(
        output.status.success(),
        "blit scan failed:\nstderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    // Either "No blit daemons discovered" or a list of daemons
    assert!(
        stdout.contains("daemon") || stdout.contains("Discovered"),
        "unexpected scan output:\n{}",
        stdout
    );
}

// ── list-modules ──────────────────────────────────────────────────────

#[test]
fn test_utils_list_modules() {
    let ctx = TestContext::new();

    let remote = format!("127.0.0.1:{}", ctx.daemon_port);
    let mut cmd = Command::new(&ctx.cli_bin);
    cmd.arg("list-modules").arg(&remote);

    let output = run_with_timeout(cmd, Duration::from_secs(10));
    assert!(
        output.status.success(),
        "blit list-modules failed:\nstderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("test"),
        "expected module 'test' in output, got:\n{}",
        stdout
    );
}

#[test]
fn test_utils_list_modules_json() {
    let ctx = TestContext::new();

    let remote = format!("127.0.0.1:{}", ctx.daemon_port);
    let mut cmd = Command::new(&ctx.cli_bin);
    cmd.arg("list-modules").arg(&remote).arg("--json");

    let output = run_with_timeout(cmd, Duration::from_secs(10));
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("invalid JSON: {}\noutput: {}", e, stdout));
    let modules = parsed.as_array().expect("expected JSON array");
    assert!(
        modules.iter().any(|m| m["name"] == "test"),
        "expected module 'test' in JSON output: {}",
        stdout
    );
}

// ── ls ────────────────────────────────────────────────────────────────

#[test]
fn test_utils_ls_remote() {
    let ctx = TestContext::new();

    fs::write(ctx.module_dir.join("hello.txt"), "world").expect("write file");
    fs::create_dir(ctx.module_dir.join("subdir")).expect("create subdir");

    let remote = format!("127.0.0.1:{}:/test/", ctx.daemon_port);
    let mut cmd = Command::new(&ctx.cli_bin);
    cmd.arg("ls").arg(&remote);

    let output = run_with_timeout(cmd, Duration::from_secs(10));
    assert!(
        output.status.success(),
        "blit ls failed:\nstderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("hello.txt"),
        "missing hello.txt in ls output"
    );
    assert!(stdout.contains("subdir"), "missing subdir in ls output");
}

#[test]
fn test_utils_ls_local() {
    let ctx = TestContext::new();
    let tmp = tempfile::tempdir().expect("tempdir");
    fs::write(tmp.path().join("local.txt"), "data").expect("write");

    let mut cmd = Command::new(&ctx.cli_bin);
    cmd.arg("ls").arg(tmp.path());

    let output = run_with_timeout(cmd, Duration::from_secs(10));
    assert!(
        output.status.success(),
        "blit ls (local) failed:\nstderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("local.txt"),
        "missing local.txt in ls output:\n{}",
        stdout
    );
}

#[test]
fn test_utils_ls_json() {
    let ctx = TestContext::new();

    fs::write(ctx.module_dir.join("j.txt"), "json").expect("write");

    let remote = format!("127.0.0.1:{}:/test/", ctx.daemon_port);
    let mut cmd = Command::new(&ctx.cli_bin);
    cmd.arg("ls").arg(&remote).arg("--json");

    let output = run_with_timeout(cmd, Duration::from_secs(10));
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("invalid JSON: {}\noutput: {}", e, stdout));
    let entries = parsed.as_array().expect("expected JSON array");
    assert!(
        entries.iter().any(|e| e["name"] == "j.txt"),
        "expected j.txt in JSON ls output: {}",
        stdout
    );
}

// ── find ──────────────────────────────────────────────────────────────

#[test]
fn test_utils_find() {
    let ctx = TestContext::new();

    fs::write(ctx.module_dir.join("readme.md"), "# hi").expect("write md");
    fs::write(ctx.module_dir.join("data.csv"), "a,b").expect("write csv");
    fs::create_dir(ctx.module_dir.join("deep")).expect("mkdir");
    fs::write(ctx.module_dir.join("deep/nested.md"), "# deep").expect("write nested");

    let remote = format!("127.0.0.1:{}:/test/", ctx.daemon_port);
    let mut cmd = Command::new(&ctx.cli_bin);
    cmd.arg("find").arg(&remote).arg("--pattern").arg("*.md");

    let output = run_with_timeout(cmd, Duration::from_secs(10));
    assert!(
        output.status.success(),
        "blit find failed:\nstderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("readme.md"), "missing readme.md");
    assert!(stdout.contains("nested.md"), "missing nested.md");
    assert!(
        !stdout.contains("data.csv"),
        "data.csv should not match .md"
    );
}

#[test]
fn test_utils_find_json() {
    let ctx = TestContext::new();

    fs::write(ctx.module_dir.join("target.log"), "log").expect("write");

    let remote = format!("127.0.0.1:{}:/test/", ctx.daemon_port);
    let mut cmd = Command::new(&ctx.cli_bin);
    cmd.arg("find")
        .arg(&remote)
        .arg("--pattern")
        .arg("*.log")
        .arg("--json");

    let output = run_with_timeout(cmd, Duration::from_secs(10));
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("invalid JSON: {}\noutput: {}", e, stdout));
    let rows = parsed.as_array().expect("expected JSON array");
    assert!(
        rows.iter()
            .any(|r| r["path"].as_str().unwrap_or("").contains("target.log")),
        "expected target.log in JSON find output: {}",
        stdout
    );
}

#[test]
fn test_utils_find_dirs_only() {
    let ctx = TestContext::new();

    fs::create_dir(ctx.module_dir.join("mydir")).expect("mkdir");
    fs::write(ctx.module_dir.join("myfile.txt"), "x").expect("write");

    let remote = format!("127.0.0.1:{}:/test/", ctx.daemon_port);
    let mut cmd = Command::new(&ctx.cli_bin);
    cmd.arg("find").arg(&remote).arg("--dirs");

    let output = run_with_timeout(cmd, Duration::from_secs(10));
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("mydir"), "missing mydir in dirs-only find");
    // files should be excluded
    assert!(
        !stdout.contains("myfile.txt"),
        "myfile.txt should not appear in --dirs output"
    );
}

#[test]
fn test_utils_find_limit() {
    let ctx = TestContext::new();

    for i in 0..10 {
        fs::write(ctx.module_dir.join(format!("item_{}.txt", i)), "x").expect("write");
    }

    let remote = format!("127.0.0.1:{}:/test/", ctx.daemon_port);
    let mut cmd = Command::new(&ctx.cli_bin);
    cmd.arg("find").arg(&remote).arg("--limit").arg("3");

    let output = run_with_timeout(cmd, Duration::from_secs(10));
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    // Count non-header lines containing "item_"
    let matches: Vec<&str> = stdout.lines().filter(|l| l.contains("item_")).collect();
    assert!(
        matches.len() <= 3,
        "expected at most 3 results with --limit 3, got {}:\n{}",
        matches.len(),
        stdout
    );
}

// ── du ────────────────────────────────────────────────────────────────

#[test]
fn test_utils_du() {
    let ctx = TestContext::new();

    fs::write(ctx.module_dir.join("sized.bin"), vec![0u8; 1024]).expect("write");
    fs::create_dir(ctx.module_dir.join("sub")).expect("mkdir");
    fs::write(ctx.module_dir.join("sub/inner.bin"), vec![0u8; 512]).expect("write");

    let remote = format!("127.0.0.1:{}:/test/", ctx.daemon_port);
    let mut cmd = Command::new(&ctx.cli_bin);
    cmd.arg("du").arg(&remote);

    let output = run_with_timeout(cmd, Duration::from_secs(10));
    assert!(
        output.status.success(),
        "blit du failed:\nstderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("BYTES"), "expected BYTES header");
    assert!(stdout.contains("FILES"), "expected FILES header");
}

#[test]
fn test_utils_du_json() {
    let ctx = TestContext::new();

    fs::write(ctx.module_dir.join("f.txt"), "hello").expect("write");

    let remote = format!("127.0.0.1:{}:/test/", ctx.daemon_port);
    let mut cmd = Command::new(&ctx.cli_bin);
    cmd.arg("du").arg(&remote).arg("--json");

    let output = run_with_timeout(cmd, Duration::from_secs(10));
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("invalid JSON: {}\noutput: {}", e, stdout));
    let rows = parsed.as_array().expect("expected JSON array");
    assert!(!rows.is_empty(), "du JSON should have at least one entry");
    assert!(
        rows[0].get("bytes").is_some(),
        "du JSON entries should have 'bytes' field"
    );
}

// ── df ────────────────────────────────────────────────────────────────

#[test]
fn test_utils_df() {
    let ctx = TestContext::new();

    let remote = format!("127.0.0.1:{}:/test/", ctx.daemon_port);
    let mut cmd = Command::new(&ctx.cli_bin);
    cmd.arg("df").arg(&remote);

    let output = run_with_timeout(cmd, Duration::from_secs(10));
    assert!(
        output.status.success(),
        "blit df failed:\nstderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Module:"),
        "expected 'Module:' in df output"
    );
    assert!(stdout.contains("Total:"), "expected 'Total:' in df output");
    assert!(stdout.contains("Free :"), "expected 'Free :' in df output");
    // Verify human-readable formatting is present (e.g. "GiB" or "MiB")
    assert!(
        stdout.contains("iB"),
        "expected human-readable byte units (KiB/MiB/GiB) in df output, got:\n{}",
        stdout
    );
}

#[test]
fn test_utils_df_json() {
    let ctx = TestContext::new();

    let remote = format!("127.0.0.1:{}:/test/", ctx.daemon_port);
    let mut cmd = Command::new(&ctx.cli_bin);
    cmd.arg("df").arg(&remote).arg("--json");

    let output = run_with_timeout(cmd, Duration::from_secs(10));
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("invalid JSON: {}\noutput: {}", e, stdout));
    assert!(
        parsed["total_bytes"].is_u64(),
        "expected total_bytes as u64"
    );
    assert!(parsed["free_bytes"].is_u64(), "expected free_bytes as u64");
    assert!(parsed["module"].is_string(), "expected module as string");
}

// ── rm ────────────────────────────────────────────────────────────────

#[test]
fn test_utils_rm_file() {
    let ctx = TestContext::new();

    let file = ctx.module_dir.join("doomed.txt");
    fs::write(&file, "bye").expect("write");

    let remote = format!("127.0.0.1:{}:/test/doomed.txt", ctx.daemon_port);
    let mut cmd = Command::new(&ctx.cli_bin);
    cmd.arg("rm").arg("--yes").arg(&remote);

    let output = run_with_timeout(cmd, Duration::from_secs(10));
    assert!(
        output.status.success(),
        "blit rm failed:\nstderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    assert!(!file.exists(), "file should have been deleted");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Deleted"),
        "expected 'Deleted' in rm output, got:\n{}",
        stdout
    );
}

#[test]
fn test_utils_rm_directory() {
    let ctx = TestContext::new();

    fs::create_dir_all(ctx.module_dir.join("rmdir/child")).expect("mkdir");
    fs::write(ctx.module_dir.join("rmdir/child/f.txt"), "x").expect("write");

    let remote = format!("127.0.0.1:{}:/test/rmdir", ctx.daemon_port);
    let mut cmd = Command::new(&ctx.cli_bin);
    cmd.arg("rm").arg("--yes").arg(&remote);

    let output = run_with_timeout(cmd, Duration::from_secs(10));
    assert!(output.status.success());

    assert!(
        !ctx.module_dir.join("rmdir").exists(),
        "directory should have been deleted recursively"
    );
}

#[test]
fn test_utils_rm_refuses_module_root() {
    let ctx = TestContext::new();

    // Attempt to delete the module root — should be refused
    let remote = format!("127.0.0.1:{}:/test/", ctx.daemon_port);
    let mut cmd = Command::new(&ctx.cli_bin);
    cmd.arg("rm").arg("--yes").arg(&remote);

    let output = run_with_timeout(cmd, Duration::from_secs(10));
    // Should fail with a non-zero exit code
    assert!(
        !output.status.success(),
        "rm of module root should be refused"
    );
}

// ── completions ───────────────────────────────────────────────────────

#[test]
fn test_utils_completions() {
    let ctx = TestContext::new();

    fs::write(ctx.module_dir.join("foo.txt"), "f").expect("write");
    fs::write(ctx.module_dir.join("foobar.txt"), "fb").expect("write");
    fs::write(ctx.module_dir.join("baz.txt"), "b").expect("write");

    let remote = format!("127.0.0.1:{}:/test/", ctx.daemon_port);
    let mut cmd = Command::new(&ctx.cli_bin);
    cmd.arg("completions")
        .arg("remote")
        .arg(&remote)
        .arg("--prefix")
        .arg("foo");

    let output = run_with_timeout(cmd, Duration::from_secs(10));
    assert!(
        output.status.success(),
        "blit completions failed:\nstderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("foo.txt"), "missing foo.txt in completions");
    assert!(
        stdout.contains("foobar.txt"),
        "missing foobar.txt in completions"
    );
    assert!(
        !stdout.contains("baz.txt"),
        "baz.txt should not match 'foo' prefix"
    );
}

#[test]
fn test_utils_completions_dirs_only() {
    let ctx = TestContext::new();

    fs::write(ctx.module_dir.join("file.txt"), "f").expect("write");
    fs::create_dir(ctx.module_dir.join("dirname")).expect("mkdir");

    let remote = format!("127.0.0.1:{}:/test/", ctx.daemon_port);
    let mut cmd = Command::new(&ctx.cli_bin);
    cmd.arg("completions")
        .arg("remote")
        .arg(&remote)
        .arg("--dirs");

    let output = run_with_timeout(cmd, Duration::from_secs(10));
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("dirname"),
        "missing dirname in --dirs completions"
    );
    assert!(
        !stdout.contains("file.txt"),
        "file.txt should not appear in --dirs completions"
    );
}

#[test]
fn test_utils_completions_shell_bash_emits_script() {
    // P0 §2.5 integration test. Daemon is unused here; we just need
    // the binary on disk. `blit completions shell bash` must exit
    // cleanly and emit a non-empty bash-completion script that
    // defines `_blit` and references known subcommands.
    let ctx = TestContext::new();

    let mut cmd = Command::new(&ctx.cli_bin);
    cmd.arg("completions").arg("shell").arg("bash");

    let output = run_with_timeout(cmd, Duration::from_secs(10));
    assert!(
        output.status.success(),
        "blit completions shell bash failed:\nstderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("_blit()"), "missing _blit() definition");
    assert!(stdout.contains("copy"), "missing copy subcommand");
    assert!(stdout.contains("mirror"), "missing mirror subcommand");
    assert!(
        stdout.contains("completions"),
        "missing completions subcommand"
    );
}

// ── profile ───────────────────────────────────────────────────────────

#[test]
fn test_utils_profile() {
    let ctx = TestContext::new();

    let mut cmd = Command::new(&ctx.cli_bin);
    cmd.arg("profile");

    let output = run_with_timeout(cmd, Duration::from_secs(10));
    assert!(
        output.status.success(),
        "blit profile failed:\nstderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Performance history"),
        "expected 'Performance history' header, got:\n{}",
        stdout
    );
    assert!(
        stdout.contains("record(s)"),
        "expected record count in profile output"
    );
}

#[test]
fn test_utils_profile_json() {
    let ctx = TestContext::new();

    let mut cmd = Command::new(&ctx.cli_bin);
    cmd.arg("profile").arg("--json");

    let output = run_with_timeout(cmd, Duration::from_secs(10));
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("invalid JSON: {}\noutput: {}", e, stdout));
    assert!(
        parsed.get("enabled").is_some(),
        "expected 'enabled' field in profile JSON"
    );
    assert!(
        parsed.get("records").is_some(),
        "expected 'records' field in profile JSON"
    );

    // R44-F2: pin the predictor key shape. Two valid states:
    //   - top-level null: predictor file failed to load entirely
    //     (rare — only on permission errors or corrupted state)
    //   - object with `copy` and `mirror` fields, each either null
    //     ("no profile yet for this mode, needs ≥5 observations")
    //     or a coefficient object — chosen because it's strictly
    //     more informative: it distinguishes "predictor never
    //     initialised" from "predictor exists, mode-specific
    //     profile not yet trained."
    //
    // The pre-fix commit message claimed top-level null in the
    // empty case, which was wrong; this assertion locks the actual
    // contract so the next reviewer can rely on it.
    let predictor = parsed
        .get("predictor")
        .expect("expected 'predictor' field in profile JSON");
    if !predictor.is_null() {
        let obj = predictor
            .as_object()
            .unwrap_or_else(|| panic!("predictor must be null or object, got: {}", predictor));
        assert!(
            obj.contains_key("copy"),
            "predictor object must include 'copy' (got: {})",
            predictor
        );
        assert!(
            obj.contains_key("mirror"),
            "predictor object must include 'mirror' (got: {})",
            predictor
        );
        // Each mode is either null (no trained profile) or an
        // object with planner+transfer coefficient blocks.
        for mode in ["copy", "mirror"] {
            let val = &obj[mode];
            if !val.is_null() {
                let mode_obj = val.as_object().unwrap_or_else(|| {
                    panic!("predictor.{} must be null or object, got: {}", mode, val)
                });
                for key in ["observations", "fallback_depth", "planner", "transfer"] {
                    assert!(
                        mode_obj.contains_key(key),
                        "predictor.{} must include '{}' (got: {})",
                        mode,
                        key,
                        val
                    );
                }
            }
        }
    }
}
