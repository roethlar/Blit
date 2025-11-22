use std::fs;
use std::process::Command;
use std::time::Duration;

mod common;
use common::{run_with_timeout, TestContext};

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
    assert!(stdout.contains("file1.txt"), "missing file1.txt in list output");
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
    assert!(stdout.contains("10"), "expected size 10 in output, got:\n{}", stdout);
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
    assert!(stdout.contains("Total"), "expected 'Total' header in output");
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
    assert!(stdout.contains("find_me.txt"), "expected find_me.txt in output, got:\n{}", stdout);
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
