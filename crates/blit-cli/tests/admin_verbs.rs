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

    // UTILITY_VERBS_LOCAL_PARITY slice 1: `du`'s own shape — one line for
    // the directory given, `SIZE<TAB>PATH`, no header and no line for the
    // file (that is `-a`). It used to print a `PATH BYTES FILES DIRS` table.
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines.len(), 1, "one directory, one line:\n{stdout}");
    let (size, path) = lines[0].split_once('\t').expect("SIZE<TAB>PATH");
    assert_eq!(path, remote_path);
    size.parse::<u64>().expect("a size in 1K blocks");
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

    // UTILITY_VERBS_LOCAL_PARITY slice 1: `df -k`'s shape, the operand in
    // the first column. It used to print `Module:` / `Total:` / `Free :`.
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines.len(), 2, "a header and one filesystem:\n{stdout}");
    assert!(
        lines[0].starts_with("Filesystem") && lines[0].contains("1K-blocks"),
        "{stdout}"
    );
    assert!(lines[1].starts_with(&remote_path), "{stdout}");
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
        .arg("-r")
        .arg("--yes")
        .arg(&remote_path);

    let output = run_with_timeout(cli_cmd, Duration::from_secs(10));
    assert!(output.status.success(), "blit rm -r directory failed");

    assert!(
        !ctx.module_dir.join("removeme").exists(),
        "directory should have been deleted recursively"
    );
}
