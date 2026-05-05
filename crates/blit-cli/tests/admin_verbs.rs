use std::fs;
use std::process::Command;
use std::time::Duration;

mod common;
use common::{run_with_timeout, TestContext};

#[test]
fn test_admin_list_modules() {
    let ctx = TestContext::new();

    // Discovery mode: server:port with no module path
    let discovery = format!("127.0.0.1:{}", ctx.daemon_port);
    let mut cli_cmd = Command::new(&ctx.cli_bin);
    cli_cmd
        .arg("--config-dir")
        .arg(&ctx.config_dir)
        .arg("list-modules")
        .arg(&discovery);

    let output = run_with_timeout(cli_cmd, Duration::from_secs(10));
    assert!(output.status.success(), "blit list-modules failed");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("test"),
        "expected module 'test' in output, got:\n{}",
        stdout
    );
}

#[test]
fn test_admin_list_smart_dispatch_bare_host() {
    // §2.3 of RELEASE_PLAN_v2: `blit list <bare-host>` should route
    // to list-modules. The explicit `list-modules` form still works
    // (covered above) and `ls` rejects bare hosts; this test pins
    // that the `list` alias smart-dispatches.
    let ctx = TestContext::new();

    let discovery = format!("127.0.0.1:{}", ctx.daemon_port);
    let mut cli_cmd = Command::new(&ctx.cli_bin);
    cli_cmd
        .arg("--config-dir")
        .arg(&ctx.config_dir)
        .arg("list")
        .arg(&discovery);

    let output = run_with_timeout(cli_cmd, Duration::from_secs(10));
    assert!(
        output.status.success(),
        "blit list <bare-host> failed:\nstderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    // list-modules format: "Modules on <host>:" header + "name (rw|ro)\tpath"
    assert!(
        stdout.contains("Modules on"),
        "expected list-modules header in output, got:\n{}",
        stdout
    );
    assert!(
        stdout.contains("test"),
        "expected module 'test' in output, got:\n{}",
        stdout
    );
}

#[test]
fn test_admin_list_smart_dispatch_module_path_routes_to_ls() {
    // The smart-dispatch fall-through: if the target has a module
    // path, `list` should behave like `ls` and stream directory
    // entries. Regression guard against accidentally routing every
    // `list` invocation to list-modules.
    let ctx = TestContext::new();

    fs::write(ctx.module_dir.join("first.txt"), "x").expect("write first");
    fs::write(ctx.module_dir.join("second.txt"), "y").expect("write second");

    let target = format!("127.0.0.1:{}:/test/", ctx.daemon_port);
    let mut cli_cmd = Command::new(&ctx.cli_bin);
    cli_cmd
        .arg("--config-dir")
        .arg(&ctx.config_dir)
        .arg("list")
        .arg(&target);

    let output = run_with_timeout(cli_cmd, Duration::from_secs(10));
    assert!(output.status.success(), "blit list <module/> failed");

    let stdout = String::from_utf8_lossy(&output.stdout);
    // ls format: "Listing test:/:" header + per-entry rows. The
    // important assertion is that it's NOT the list-modules header.
    assert!(
        !stdout.contains("Modules on"),
        "blit list with a module path should NOT route to \
         list-modules; got:\n{}",
        stdout
    );
    assert!(
        stdout.contains("first.txt") && stdout.contains("second.txt"),
        "expected ls-style entries in output, got:\n{}",
        stdout
    );
}

#[test]
fn test_admin_list() {
    let ctx = TestContext::new();

    // Create some files in the module
    fs::write(ctx.module_dir.join("file1.txt"), "content1").expect("write file1");
    fs::create_dir(ctx.module_dir.join("subdir")).expect("create subdir");
    fs::write(ctx.module_dir.join("subdir/file2.txt"), "content2").expect("write file2");

    let remote_path = format!("127.0.0.1:{}:/test/", ctx.daemon_port);
    let mut cli_cmd = Command::new(&ctx.cli_bin);
    cli_cmd
        .arg("--config-dir")
        .arg(&ctx.config_dir)
        .arg("ls")
        .arg(&remote_path);

    let output = run_with_timeout(cli_cmd, Duration::from_secs(10));
    assert!(output.status.success(), "blit ls failed");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("file1.txt"),
        "missing file1.txt in ls output"
    );
    assert!(stdout.contains("subdir"), "missing subdir in ls output");
}

#[test]
fn test_admin_du() {
    let ctx = TestContext::new();

    // Create a file with known size
    let content = "1234567890"; // 10 bytes
    fs::write(ctx.module_dir.join("data.txt"), content).expect("write file");

    let remote_path = format!("127.0.0.1:{}:/test/", ctx.daemon_port);
    let mut cli_cmd = Command::new(&ctx.cli_bin);
    cli_cmd
        .arg("--config-dir")
        .arg(&ctx.config_dir)
        .arg("du")
        .arg(&remote_path);

    let output = run_with_timeout(cli_cmd, Duration::from_secs(10));
    assert!(output.status.success(), "blit du failed");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("10"),
        "expected size 10 in output, got:\n{}",
        stdout
    );
    assert!(stdout.contains("BYTES"), "expected BYTES header");
}

#[test]
fn test_admin_df() {
    let ctx = TestContext::new();

    let remote_path = format!("127.0.0.1:{}:/test/", ctx.daemon_port);
    let mut cli_cmd = Command::new(&ctx.cli_bin);
    cli_cmd
        .arg("--config-dir")
        .arg(&ctx.config_dir)
        .arg("df")
        .arg(&remote_path);

    let output = run_with_timeout(cli_cmd, Duration::from_secs(10));
    assert!(output.status.success(), "blit df failed");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Free"), "expected 'Free' header in output");
    assert!(
        stdout.contains("Total"),
        "expected 'Total' header in output"
    );
}

#[test]
fn test_admin_find() {
    let ctx = TestContext::new();

    let file_path = ctx.module_dir.join("find_me.txt");
    std::fs::write(&file_path, "content").unwrap();

    let remote_path = format!("127.0.0.1:{}:/test/", ctx.daemon_port);
    let mut cli_cmd = Command::new(&ctx.cli_bin);
    cli_cmd
        .arg("--config-dir")
        .arg(&ctx.config_dir)
        .arg("find")
        .arg(&remote_path)
        .arg("--pattern")
        .arg("find_me.txt");

    let output = run_with_timeout(cli_cmd, Duration::from_secs(10));
    assert!(output.status.success(), "blit find failed");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("find_me.txt"),
        "expected find_me.txt in output, got:\n{}",
        stdout
    );
    assert!(stdout.contains("file"), "expected 'file' type in output");
}

#[test]
fn test_admin_rm() {
    let ctx = TestContext::new();

    let file_path = ctx.module_dir.join("todelete.txt");
    fs::write(&file_path, "delete me").expect("write file");

    let remote_path = format!("127.0.0.1:{}:/test/todelete.txt", ctx.daemon_port);
    let mut cli_cmd = Command::new(&ctx.cli_bin);
    cli_cmd
        .arg("--config-dir")
        .arg(&ctx.config_dir)
        .arg("rm")
        .arg("--yes")
        .arg(&remote_path);

    let output = run_with_timeout(cli_cmd, Duration::from_secs(10));
    assert!(output.status.success(), "blit rm failed");

    assert!(!file_path.exists(), "file should have been deleted");
}

#[test]
fn test_admin_complete_path() {
    let ctx = TestContext::new();

    // Create files for completion
    fs::write(ctx.module_dir.join("alpha.txt"), "a").expect("write alpha");
    fs::write(ctx.module_dir.join("alpha_2.txt"), "a2").expect("write alpha_2");
    fs::write(ctx.module_dir.join("beta.txt"), "b").expect("write beta");
    fs::create_dir(ctx.module_dir.join("alpha_dir")).expect("create alpha_dir");

    let remote_path = format!("127.0.0.1:{}:/test/", ctx.daemon_port);
    let mut cmd = Command::new(&ctx.cli_bin);
    cmd.arg("completions")
        .arg("remote")
        .arg(&remote_path)
        .arg("--prefix")
        .arg("alpha");

    let output = run_with_timeout(cmd, Duration::from_secs(10));
    assert!(
        output.status.success(),
        "blit completions failed:\nstderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("alpha.txt"),
        "expected alpha.txt in completions, got:\n{}",
        stdout
    );
    assert!(
        stdout.contains("alpha_2.txt"),
        "expected alpha_2.txt in completions, got:\n{}",
        stdout
    );
    assert!(
        stdout.contains("alpha_dir"),
        "expected alpha_dir in completions, got:\n{}",
        stdout
    );
    assert!(
        !stdout.contains("beta.txt"),
        "beta.txt should not appear in alpha completions, got:\n{}",
        stdout
    );
}

#[test]
fn test_admin_list_subdirectory() {
    let ctx = TestContext::new();

    // Create nested structure
    fs::create_dir_all(ctx.module_dir.join("sub/nested")).expect("create nested");
    fs::write(ctx.module_dir.join("sub/a.txt"), "a").expect("write a");
    fs::write(ctx.module_dir.join("sub/nested/b.txt"), "b").expect("write b");

    let remote_path = format!("127.0.0.1:{}:/test/sub/", ctx.daemon_port);
    let mut cli_cmd = Command::new(&ctx.cli_bin);
    cli_cmd
        .arg("--config-dir")
        .arg(&ctx.config_dir)
        .arg("ls")
        .arg(&remote_path);

    let output = run_with_timeout(cli_cmd, Duration::from_secs(10));
    assert!(output.status.success(), "blit ls subdir failed");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("a.txt"), "missing a.txt in subdir listing");
    assert!(
        stdout.contains("nested"),
        "missing nested/ in subdir listing"
    );
}

#[test]
fn test_admin_find_with_pattern() {
    let ctx = TestContext::new();

    // Create various files
    fs::write(ctx.module_dir.join("report.csv"), "data").expect("write csv");
    fs::write(ctx.module_dir.join("notes.txt"), "notes").expect("write txt");
    fs::create_dir(ctx.module_dir.join("subdir")).expect("create subdir");
    fs::write(ctx.module_dir.join("subdir/results.csv"), "results").expect("write nested csv");

    // --pattern is a glob (per BLIT_UTILS_PLAN). The glob matches
    // both the relative path AND the basename, so `*.csv` finds
    // `report.csv` (root) and `subdir/results.csv` (nested).
    let remote_path = format!("127.0.0.1:{}:/test/", ctx.daemon_port);
    let mut cli_cmd = Command::new(&ctx.cli_bin);
    cli_cmd
        .arg("--config-dir")
        .arg(&ctx.config_dir)
        .arg("find")
        .arg(&remote_path)
        .arg("--pattern")
        .arg("*.csv");

    let output = run_with_timeout(cli_cmd, Duration::from_secs(10));
    assert!(output.status.success(), "blit find --pattern failed");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("report.csv"),
        "missing report.csv in find output"
    );
    assert!(
        stdout.contains("results.csv"),
        "missing results.csv in find output"
    );
    assert!(
        !stdout.contains("notes.txt"),
        "notes.txt should not match .csv pattern"
    );
}

#[test]
fn test_admin_find_glob_invalid_pattern_rejected() {
    // Glob compilation can fail (e.g. unclosed character class).
    // The daemon must surface the error to the CLI rather than
    // treating the bad pattern as no-pattern (matches everything)
    // or panicking. Pre-glob (substring) implementation accepted any
    // string; with glob we get real validation.
    let ctx = TestContext::new();
    fs::write(ctx.module_dir.join("a.txt"), "x").expect("write");

    let remote_path = format!("127.0.0.1:{}:/test/", ctx.daemon_port);
    let mut cli_cmd = Command::new(&ctx.cli_bin);
    cli_cmd
        .arg("--config-dir")
        .arg(&ctx.config_dir)
        .arg("find")
        .arg(&remote_path)
        .arg("--pattern")
        .arg("[unterminated");

    let output = run_with_timeout(cli_cmd, Duration::from_secs(10));
    assert!(
        !output.status.success(),
        "blit find with malformed glob should fail, got success with stdout:\n{}",
        String::from_utf8_lossy(&output.stdout)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.to_lowercase().contains("glob")
            || stderr.to_lowercase().contains("invalid")
            || stderr.to_lowercase().contains("pattern"),
        "expected glob/invalid/pattern in stderr, got:\n{}",
        stderr
    );
}

#[test]
fn test_admin_find_glob_star_does_not_cross_path_separator() {
    // R41-F3 regression. `*` must NOT match across `/` per POSIX
    // shell-glob conventions. With `literal_separator(true)` set on
    // the daemon's GlobBuilder, `foo*.csv` matches `foo-x.csv` (a
    // basename in the root or via the basename-fallback) but does
    // NOT match `foo/bar.csv` as a path. Users wanting to cross
    // path components must use `**/`.
    let ctx = TestContext::new();
    fs::create_dir(ctx.module_dir.join("foo")).expect("mkdir foo");
    fs::write(ctx.module_dir.join("foo/bar.csv"), "x").expect("write nested");
    fs::write(ctx.module_dir.join("foo-x.csv"), "x").expect("write sibling");
    fs::write(ctx.module_dir.join("notes.txt"), "x").expect("write txt");

    let remote_path = format!("127.0.0.1:{}:/test/", ctx.daemon_port);
    let mut cli_cmd = Command::new(&ctx.cli_bin);
    cli_cmd
        .arg("--config-dir")
        .arg(&ctx.config_dir)
        .arg("find")
        .arg(&remote_path)
        .arg("--pattern")
        .arg("foo*.csv");

    let output = run_with_timeout(cli_cmd, Duration::from_secs(10));
    assert!(output.status.success(), "blit find foo*.csv failed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    // foo-x.csv is in the root → matches both as path and basename.
    assert!(stdout.contains("foo-x.csv"), "missing foo-x.csv");
    // foo/bar.csv has the basename `bar.csv` (no foo prefix), and
    // its path `foo/bar.csv` would only match if `*` crossed `/`.
    // With literal_separator(true) it does not — and the basename
    // is `bar.csv`, which `foo*.csv` also doesn't match.
    assert!(
        !stdout.contains("foo/bar.csv") && !stdout.contains("bar.csv"),
        "foo/bar.csv should NOT match foo*.csv (literal_separator); got:\n{}",
        stdout
    );
    assert!(!stdout.contains("notes.txt"));
}

#[test]
fn test_admin_find_glob_nested_with_double_star() {
    // `**/*.csv` matches across directory components. With the
    // basename fallback, plain `*.csv` already matches nested
    // entries via their basename — but the explicit `**/*.csv`
    // form is what users will reach for. Confirm both shapes work.
    let ctx = TestContext::new();
    fs::write(ctx.module_dir.join("top.csv"), "x").expect("write");
    fs::create_dir(ctx.module_dir.join("a")).expect("mkdir a");
    fs::create_dir(ctx.module_dir.join("a/b")).expect("mkdir a/b");
    fs::write(ctx.module_dir.join("a/b/deep.csv"), "x").expect("write deep");
    fs::write(ctx.module_dir.join("notes.txt"), "x").expect("write txt");

    let remote_path = format!("127.0.0.1:{}:/test/", ctx.daemon_port);
    let mut cli_cmd = Command::new(&ctx.cli_bin);
    cli_cmd
        .arg("--config-dir")
        .arg(&ctx.config_dir)
        .arg("find")
        .arg(&remote_path)
        .arg("--pattern")
        .arg("**/*.csv");

    let output = run_with_timeout(cli_cmd, Duration::from_secs(10));
    assert!(output.status.success(), "blit find **/*.csv failed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("top.csv"), "missing top.csv");
    assert!(stdout.contains("deep.csv"), "missing deep.csv");
    assert!(!stdout.contains("notes.txt"), "notes.txt should not match");
}

#[test]
fn test_admin_rm_directory() {
    let ctx = TestContext::new();

    fs::create_dir_all(ctx.module_dir.join("removeme/child")).expect("create dir tree");
    fs::write(ctx.module_dir.join("removeme/child/file.txt"), "x").expect("write file");

    let remote_path = format!("127.0.0.1:{}:/test/removeme/", ctx.daemon_port);
    let mut cli_cmd = Command::new(&ctx.cli_bin);
    cli_cmd
        .arg("--config-dir")
        .arg(&ctx.config_dir)
        .arg("rm")
        .arg("--yes")
        .arg(&remote_path);

    let output = run_with_timeout(cli_cmd, Duration::from_secs(10));
    assert!(output.status.success(), "blit rm directory failed");

    assert!(
        !ctx.module_dir.join("removeme").exists(),
        "directory should have been deleted recursively"
    );
}
