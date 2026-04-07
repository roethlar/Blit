use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

mod common;
use common::{run_with_timeout, TestContext};

fn utils_bin() -> PathBuf {
    let exe_path = std::env::current_exe().expect("current_exe");
    let deps_dir = exe_path.parent().expect("test binary directory");
    let bin_dir = deps_dir.parent().expect("deps parent directory");
    let name = if cfg!(windows) {
        "blit-utils.exe"
    } else {
        "blit-utils"
    };
    let utils = bin_dir.join(name);

    if !utils.exists() {
        let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .expect("workspace root");
        let output = Command::new("cargo")
            .current_dir(workspace_root)
            .arg("build")
            .arg("-p")
            .arg("blit-utils")
            .arg("--bin")
            .arg("blit-utils")
            .output()
            .expect("invoke cargo build for blit-utils");
        assert!(
            output.status.success(),
            "cargo build blit-utils failed:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    utils
}

// ── scan ──────────────────────────────────────────────────────────────

#[test]
fn test_utils_scan() {
    let utils = utils_bin();

    // Run with a very short wait so the test completes quickly.
    // Test daemon has no_mdns: true, so we expect no results — just a clean exit.
    let mut cmd = Command::new(&utils);
    cmd.arg("scan").arg("--wait").arg("1");

    let output = run_with_timeout(cmd, Duration::from_secs(10));
    assert!(
        output.status.success(),
        "blit-utils scan failed:\nstderr: {}",
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
    let utils = utils_bin();

    let remote = format!("127.0.0.1:{}", ctx.daemon_port);
    let mut cmd = Command::new(&utils);
    cmd.arg("list-modules").arg(&remote);

    let output = run_with_timeout(cmd, Duration::from_secs(10));
    assert!(
        output.status.success(),
        "blit-utils list-modules failed:\nstderr: {}",
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
    let utils = utils_bin();

    let remote = format!("127.0.0.1:{}", ctx.daemon_port);
    let mut cmd = Command::new(&utils);
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
    let utils = utils_bin();

    fs::write(ctx.module_dir.join("hello.txt"), "world").expect("write file");
    fs::create_dir(ctx.module_dir.join("subdir")).expect("create subdir");

    let remote = format!("127.0.0.1:{}:/test/", ctx.daemon_port);
    let mut cmd = Command::new(&utils);
    cmd.arg("ls").arg(&remote);

    let output = run_with_timeout(cmd, Duration::from_secs(10));
    assert!(
        output.status.success(),
        "blit-utils ls failed:\nstderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("hello.txt"), "missing hello.txt in ls output");
    assert!(stdout.contains("subdir"), "missing subdir in ls output");
}

#[test]
fn test_utils_ls_local() {
    let utils = utils_bin();
    let tmp = tempfile::tempdir().expect("tempdir");
    fs::write(tmp.path().join("local.txt"), "data").expect("write");

    let mut cmd = Command::new(&utils);
    cmd.arg("ls").arg(tmp.path());

    let output = run_with_timeout(cmd, Duration::from_secs(10));
    assert!(
        output.status.success(),
        "blit-utils ls (local) failed:\nstderr: {}",
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
    let utils = utils_bin();

    fs::write(ctx.module_dir.join("j.txt"), "json").expect("write");

    let remote = format!("127.0.0.1:{}:/test/", ctx.daemon_port);
    let mut cmd = Command::new(&utils);
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
    let utils = utils_bin();

    fs::write(ctx.module_dir.join("readme.md"), "# hi").expect("write md");
    fs::write(ctx.module_dir.join("data.csv"), "a,b").expect("write csv");
    fs::create_dir(ctx.module_dir.join("deep")).expect("mkdir");
    fs::write(ctx.module_dir.join("deep/nested.md"), "# deep").expect("write nested");

    let remote = format!("127.0.0.1:{}:/test/", ctx.daemon_port);
    let mut cmd = Command::new(&utils);
    cmd.arg("find")
        .arg(&remote)
        .arg("--pattern")
        .arg(".md");

    let output = run_with_timeout(cmd, Duration::from_secs(10));
    assert!(
        output.status.success(),
        "blit-utils find failed:\nstderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("readme.md"), "missing readme.md");
    assert!(stdout.contains("nested.md"), "missing nested.md");
    assert!(!stdout.contains("data.csv"), "data.csv should not match .md");
}

#[test]
fn test_utils_find_json() {
    let ctx = TestContext::new();
    let utils = utils_bin();

    fs::write(ctx.module_dir.join("target.log"), "log").expect("write");

    let remote = format!("127.0.0.1:{}:/test/", ctx.daemon_port);
    let mut cmd = Command::new(&utils);
    cmd.arg("find")
        .arg(&remote)
        .arg("--pattern")
        .arg(".log")
        .arg("--json");

    let output = run_with_timeout(cmd, Duration::from_secs(10));
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("invalid JSON: {}\noutput: {}", e, stdout));
    let rows = parsed.as_array().expect("expected JSON array");
    assert!(
        rows.iter().any(|r| r["path"].as_str().unwrap_or("").contains("target.log")),
        "expected target.log in JSON find output: {}",
        stdout
    );
}

#[test]
fn test_utils_find_dirs_only() {
    let ctx = TestContext::new();
    let utils = utils_bin();

    fs::create_dir(ctx.module_dir.join("mydir")).expect("mkdir");
    fs::write(ctx.module_dir.join("myfile.txt"), "x").expect("write");

    let remote = format!("127.0.0.1:{}:/test/", ctx.daemon_port);
    let mut cmd = Command::new(&utils);
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
    let utils = utils_bin();

    for i in 0..10 {
        fs::write(ctx.module_dir.join(format!("item_{}.txt", i)), "x").expect("write");
    }

    let remote = format!("127.0.0.1:{}:/test/", ctx.daemon_port);
    let mut cmd = Command::new(&utils);
    cmd.arg("find")
        .arg(&remote)
        .arg("--limit")
        .arg("3");

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
    let utils = utils_bin();

    fs::write(ctx.module_dir.join("sized.bin"), vec![0u8; 1024]).expect("write");
    fs::create_dir(ctx.module_dir.join("sub")).expect("mkdir");
    fs::write(ctx.module_dir.join("sub/inner.bin"), vec![0u8; 512]).expect("write");

    let remote = format!("127.0.0.1:{}:/test/", ctx.daemon_port);
    let mut cmd = Command::new(&utils);
    cmd.arg("du").arg(&remote);

    let output = run_with_timeout(cmd, Duration::from_secs(10));
    assert!(
        output.status.success(),
        "blit-utils du failed:\nstderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("BYTES"), "expected BYTES header");
    assert!(stdout.contains("FILES"), "expected FILES header");
}

#[test]
fn test_utils_du_json() {
    let ctx = TestContext::new();
    let utils = utils_bin();

    fs::write(ctx.module_dir.join("f.txt"), "hello").expect("write");

    let remote = format!("127.0.0.1:{}:/test/", ctx.daemon_port);
    let mut cmd = Command::new(&utils);
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
    let utils = utils_bin();

    let remote = format!("127.0.0.1:{}:/test/", ctx.daemon_port);
    let mut cmd = Command::new(&utils);
    cmd.arg("df").arg(&remote);

    let output = run_with_timeout(cmd, Duration::from_secs(10));
    assert!(
        output.status.success(),
        "blit-utils df failed:\nstderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Module:"), "expected 'Module:' in df output");
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
    let utils = utils_bin();

    let remote = format!("127.0.0.1:{}:/test/", ctx.daemon_port);
    let mut cmd = Command::new(&utils);
    cmd.arg("df").arg(&remote).arg("--json");

    let output = run_with_timeout(cmd, Duration::from_secs(10));
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("invalid JSON: {}\noutput: {}", e, stdout));
    assert!(parsed["total_bytes"].is_u64(), "expected total_bytes as u64");
    assert!(parsed["free_bytes"].is_u64(), "expected free_bytes as u64");
    assert!(parsed["module"].is_string(), "expected module as string");
}

// ── rm ────────────────────────────────────────────────────────────────

#[test]
fn test_utils_rm_file() {
    let ctx = TestContext::new();
    let utils = utils_bin();

    let file = ctx.module_dir.join("doomed.txt");
    fs::write(&file, "bye").expect("write");

    let remote = format!("127.0.0.1:{}:/test/doomed.txt", ctx.daemon_port);
    let mut cmd = Command::new(&utils);
    cmd.arg("rm").arg("--yes").arg(&remote);

    let output = run_with_timeout(cmd, Duration::from_secs(10));
    assert!(
        output.status.success(),
        "blit-utils rm failed:\nstderr: {}",
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
    let utils = utils_bin();

    fs::create_dir_all(ctx.module_dir.join("rmdir/child")).expect("mkdir");
    fs::write(ctx.module_dir.join("rmdir/child/f.txt"), "x").expect("write");

    let remote = format!("127.0.0.1:{}:/test/rmdir", ctx.daemon_port);
    let mut cmd = Command::new(&utils);
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
    let utils = utils_bin();

    // Attempt to delete the module root — should be refused
    let remote = format!("127.0.0.1:{}:/test/", ctx.daemon_port);
    let mut cmd = Command::new(&utils);
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
    let utils = utils_bin();

    fs::write(ctx.module_dir.join("foo.txt"), "f").expect("write");
    fs::write(ctx.module_dir.join("foobar.txt"), "fb").expect("write");
    fs::write(ctx.module_dir.join("baz.txt"), "b").expect("write");

    let remote = format!("127.0.0.1:{}:/test/", ctx.daemon_port);
    let mut cmd = Command::new(&utils);
    cmd.arg("completions").arg(&remote).arg("--prefix").arg("foo");

    let output = run_with_timeout(cmd, Duration::from_secs(10));
    assert!(
        output.status.success(),
        "blit-utils completions failed:\nstderr: {}",
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
    let utils = utils_bin();

    fs::write(ctx.module_dir.join("file.txt"), "f").expect("write");
    fs::create_dir(ctx.module_dir.join("dirname")).expect("mkdir");

    let remote = format!("127.0.0.1:{}:/test/", ctx.daemon_port);
    let mut cmd = Command::new(&utils);
    cmd.arg("completions").arg(&remote).arg("--dirs");

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

// ── profile ───────────────────────────────────────────────────────────

#[test]
fn test_utils_profile() {
    let utils = utils_bin();

    let mut cmd = Command::new(&utils);
    cmd.arg("profile");

    let output = run_with_timeout(cmd, Duration::from_secs(10));
    assert!(
        output.status.success(),
        "blit-utils profile failed:\nstderr: {}",
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
    let utils = utils_bin();

    let mut cmd = Command::new(&utils);
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
}
