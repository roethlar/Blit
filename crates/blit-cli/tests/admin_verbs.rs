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

#[test]
fn test_admin_list_modules() {
    let ctx = TestContext::new();

    // Discovery mode: server:port with no module path
    let discovery = format!("127.0.0.1:{}", ctx.daemon_port);
    let mut cli_cmd = Command::new(&ctx.cli_bin);
    cli_cmd
        .arg("--config-dir")
        .arg(&ctx.config_dir)
        .arg("list")
        .arg(&discovery);

    let output = run_with_timeout(cli_cmd, Duration::from_secs(10));
    assert!(output.status.success(), "blit list (discovery) failed");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("test"),
        "expected module 'test' in discovery output, got:\n{}",
        stdout
    );
    assert!(
        stdout.contains("Modules"),
        "expected 'Modules' header in discovery output, got:\n{}",
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
        .arg("list")
        .arg(&remote_path);

    let output = run_with_timeout(cli_cmd, Duration::from_secs(10));
    assert!(output.status.success(), "blit list failed");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("file1.txt"),
        "missing file1.txt in list output"
    );
    assert!(stdout.contains("subdir"), "missing subdir in list output");
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
    // du output is raw bytes in the current implementation
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
    let port = ctx.daemon_port;

    // Create a dummy file to find in the module directory
    let file_path = ctx.module_dir.join("find_me.txt");
    std::fs::write(&file_path, "content").unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_blit-cli"))
        .arg("find")
        .arg(format!("127.0.0.1:{}:/test/", port))
        .arg("--pattern")
        .arg("find_me.txt")
        .output()
        .expect("failed to execute blit find");

    if !output.status.success() {
        println!("STDOUT:\n{}", String::from_utf8_lossy(&output.stdout));
        println!("STDERR:\n{}", String::from_utf8_lossy(&output.stderr));
    }
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

    // First try without --yes (should fail or prompt, but in non-interactive it might fail or require input)
    // Actually, the CLI might default to interactive confirmation.
    // Let's use --yes to force deletion.
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

    let utils = utils_bin();
    let remote_path = format!("127.0.0.1:{}:/test/", ctx.daemon_port);
    let mut cmd = Command::new(&utils);
    cmd.arg("completions")
        .arg(&remote_path)
        .arg("--prefix")
        .arg("alpha");

    let output = run_with_timeout(cmd, Duration::from_secs(10));
    assert!(
        output.status.success(),
        "blit-utils completions failed:\nstderr: {}",
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
        .arg("list")
        .arg(&remote_path);

    let output = run_with_timeout(cli_cmd, Duration::from_secs(10));
    assert!(output.status.success(), "blit list subdir failed");

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

    // Pattern is substring match, not glob
    let remote_path = format!("127.0.0.1:{}:/test/", ctx.daemon_port);
    let mut cli_cmd = Command::new(&ctx.cli_bin);
    cli_cmd
        .arg("--config-dir")
        .arg(&ctx.config_dir)
        .arg("find")
        .arg(&remote_path)
        .arg("--pattern")
        .arg(".csv");

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
