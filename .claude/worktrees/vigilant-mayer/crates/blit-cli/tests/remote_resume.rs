use std::fs;
use std::process::Command;
use std::time::Duration;

mod common;
use common::{run_with_timeout, TestContext};

/// Test that --resume mode works for remote pull with partial files.
/// Creates a partial local file, runs pull with --resume, verifies only
/// the differing blocks are transferred.
#[cfg(unix)]
#[test]
fn test_pull_resume_partial_file() {
    let ctx = TestContext::new();
    let dest_dir = ctx.workspace.join("dest");
    fs::create_dir_all(&dest_dir).expect("dest dir");

    // Create a 3-block file on server (3 MiB with 1 MiB blocks)
    let server_content: Vec<u8> = (0..3 * 1024 * 1024)
        .map(|i| ((i / (1024 * 1024)) as u8) + b'A') // AAA...BBB...CCC...
        .collect();
    fs::write(ctx.module_dir.join("large.bin"), &server_content).expect("write server file");

    // Create a partial local file (only first 2 blocks, matching content)
    let local_content: Vec<u8> = server_content[..2 * 1024 * 1024].to_vec();
    fs::write(dest_dir.join("large.bin"), &local_content).expect("write partial local file");

    let src_remote = format!("127.0.0.1:{}:/test/", ctx.daemon_port);
    let mut cli_cmd = Command::new(&ctx.cli_bin);
    cli_cmd
        .arg("--config-dir")
        .arg(&ctx.config_dir)
        .arg("mirror")
        .arg("--yes")
        .arg("--resume")
        .arg(&src_remote)
        .arg(&dest_dir);

    let output = run_with_timeout(cli_cmd, Duration::from_secs(60));
    assert!(output.status.success(), "blit-cli failed: {}", String::from_utf8_lossy(&output.stderr));

    // Verify the file is now complete
    let result = fs::read(dest_dir.join("large.bin")).expect("read result file");
    assert_eq!(result.len(), server_content.len(), "file size mismatch");
    assert_eq!(result, server_content, "file content mismatch");
}

/// Test that --resume mode with identical files transfers zero blocks.
#[cfg(unix)]
#[test]
fn test_pull_resume_identical_file() {
    let ctx = TestContext::new();
    let dest_dir = ctx.workspace.join("dest");
    fs::create_dir_all(&dest_dir).expect("dest dir");

    // Create identical files on server and local
    let content: Vec<u8> = (0..1024 * 1024).map(|i| (i % 256) as u8).collect();
    fs::write(ctx.module_dir.join("same.bin"), &content).expect("write server file");
    fs::write(dest_dir.join("same.bin"), &content).expect("write local file");

    let src_remote = format!("127.0.0.1:{}:/test/", ctx.daemon_port);
    let mut cli_cmd = Command::new(&ctx.cli_bin);
    cli_cmd
        .arg("--config-dir")
        .arg(&ctx.config_dir)
        .arg("mirror")
        .arg("--yes")
        .arg("--resume")
        .arg(&src_remote)
        .arg(&dest_dir);

    let output = run_with_timeout(cli_cmd, Duration::from_secs(60));
    assert!(output.status.success(), "blit-cli failed: {}", String::from_utf8_lossy(&output.stderr));

    // Verify file is still correct
    let result = fs::read(dest_dir.join("same.bin")).expect("read result file");
    assert_eq!(result, content, "file should be unchanged");
}

/// Test that --resume with --force-grpc also works (fallback path).
#[cfg(unix)]
#[test]
fn test_pull_resume_grpc_fallback() {
    let ctx = TestContext::new();
    let dest_dir = ctx.workspace.join("dest");
    fs::create_dir_all(&dest_dir).expect("dest dir");

    // Create file on server
    let server_content = b"server-content-for-grpc-resume";
    fs::write(ctx.module_dir.join("grpc_resume.txt"), server_content).expect("write server file");

    // Create different local file (will be overwritten)
    fs::write(dest_dir.join("grpc_resume.txt"), b"old-local").expect("write local file");

    let src_remote = format!("127.0.0.1:{}:/test/", ctx.daemon_port);
    let mut cli_cmd = Command::new(&ctx.cli_bin);
    cli_cmd
        .arg("--config-dir")
        .arg(&ctx.config_dir)
        .arg("mirror")
        .arg("--yes")
        .arg("--resume")
        .arg("--force-grpc")
        .arg(&src_remote)
        .arg(&dest_dir);

    let output = run_with_timeout(cli_cmd, Duration::from_secs(60));
    assert!(output.status.success(), "blit-cli failed: {}", String::from_utf8_lossy(&output.stderr));

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("[gRPC fallback]"),
        "expected gRPC fallback message, got stdout:\n{}",
        stdout
    );

    // Verify file was updated
    let result = fs::read(dest_dir.join("grpc_resume.txt")).expect("read result file");
    assert_eq!(result, server_content, "file should have server content");
}
