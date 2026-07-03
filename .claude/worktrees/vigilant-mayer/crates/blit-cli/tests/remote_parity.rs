use std::fs;
use std::process::Command;
use std::time::Duration;

mod common;
use common::{run_with_timeout, TestContext};

#[cfg(unix)]
#[test]
fn test_push_tcp_negotiation() {
    let ctx = TestContext::new();
    let src_dir = ctx.workspace.join("src");
    fs::create_dir_all(&src_dir).expect("src dir");
    fs::write(src_dir.join("push_tcp.txt"), b"push-tcp-test").expect("write file");

    let dest_remote = format!("127.0.0.1:{}:/test/", ctx.daemon_port);
    let mut cli_cmd = Command::new(&ctx.cli_bin);
    cli_cmd
        .arg("--config-dir")
        .arg(&ctx.config_dir)
        .arg("mirror")
        .arg("--yes")
        .arg("--trace-data-plane")
        .arg(&src_dir)
        .arg(&dest_remote);

    let output = run_with_timeout(cli_cmd, Duration::from_secs(60));
    assert!(output.status.success(), "blit-cli failed");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("[data-plane-client]"),
        "expected TCP data plane usage, got stderr:\n{}",
        stderr
    );

    let dest_file = ctx.module_dir.join("push_tcp.txt");
    assert!(dest_file.exists(), "remote file missing");
    let bytes = fs::read(&dest_file).expect("read remote file");
    assert_eq!(bytes, b"push-tcp-test");
}

#[cfg(unix)]
#[test]
fn test_pull_tcp_negotiation() {
    let ctx = TestContext::new();
    let dest_dir = ctx.workspace.join("dest");
    
    // Setup remote file
    fs::write(ctx.module_dir.join("pull_tcp.txt"), b"pull-tcp-test").expect("write file");

    let src_remote = format!("127.0.0.1:{}:/test/", ctx.daemon_port);
    let mut cli_cmd = Command::new(&ctx.cli_bin);
    cli_cmd
        .arg("--config-dir")
        .arg(&ctx.config_dir)
        .arg("mirror")
        .arg("--yes")
        // .arg("--trace-data-plane") // Not wired for pull yet
        .arg(&src_remote)
        .arg(&dest_dir);

    let output = run_with_timeout(cli_cmd, Duration::from_secs(60));
    assert!(output.status.success(), "blit-cli failed");

    let stdout = String::from_utf8_lossy(&output.stdout);
    // Verify we did NOT fall back to gRPC
    assert!(
        !stdout.contains("[gRPC fallback]"),
        "expected TCP data plane (no fallback), got stdout:\n{}",
        stdout
    );
    assert!(
        stdout.contains("Pull complete"),
        "expected success message, got stdout:\n{}",
        stdout
    );

    let dest_file = dest_dir.join("pull_tcp.txt");
    if !dest_file.exists() {
        let _ = Command::new("ls").arg("-R").arg(&ctx.workspace).status();
        panic!("local file missing at {}", dest_file.display());
    }
    let bytes = fs::read(&dest_file).expect("read local file");
    assert_eq!(bytes, b"pull-tcp-test");
}

#[cfg(unix)]
#[test]
fn test_pull_grpc_fallback() {
    let ctx = TestContext::new();
    let dest_dir = ctx.workspace.join("dest");
    
    // Setup remote file
    fs::write(ctx.module_dir.join("pull_grpc.txt"), b"pull-grpc-test").expect("write file");

    let src_remote = format!("127.0.0.1:{}:/test/", ctx.daemon_port);
    let mut cli_cmd = Command::new(&ctx.cli_bin);
    cli_cmd
        .arg("--config-dir")
        .arg(&ctx.config_dir)
        .arg("mirror")
        .arg("--yes")
        .arg("--force-grpc")
        .arg(&src_remote)
        .arg(&dest_dir);

    let output = run_with_timeout(cli_cmd, Duration::from_secs(60));
    assert!(output.status.success(), "blit-cli failed");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("[gRPC fallback]"),
        "expected gRPC fallback message, got stdout:\n{}",
        stdout
    );

    let dest_file = dest_dir.join("pull_grpc.txt");
    if !dest_file.exists() {
        println!("STDOUT:\n{}", stdout);
        println!("STDERR:\n{}", String::from_utf8_lossy(&output.stderr));
        let _ = Command::new("ls").arg("-R").arg(&ctx.workspace).status();
        panic!("local file missing at {}", dest_file.display());
    }
    let bytes = fs::read(&dest_file).expect("read local file");
    assert_eq!(bytes, b"pull-grpc-test");
}

#[cfg(unix)]
#[test]
fn test_push_grpc_fallback() {
    let ctx = TestContext::new();
    let src_dir = ctx.workspace.join("src");
    fs::create_dir_all(&src_dir).expect("src dir");
    fs::write(src_dir.join("push_grpc.txt"), b"push-grpc-test").expect("write file");

    let dest_remote = format!("127.0.0.1:{}:/test/", ctx.daemon_port);
    let mut cli_cmd = Command::new(&ctx.cli_bin);
    cli_cmd
        .arg("--config-dir")
        .arg(&ctx.config_dir)
        .arg("mirror")
        .arg("--yes")
        .arg("--force-grpc")
        .arg(&src_dir)
        .arg(&dest_remote);

    let output = run_with_timeout(cli_cmd, Duration::from_secs(60));
    assert!(output.status.success(), "blit-cli failed");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("[gRPC fallback]"),
        "expected gRPC fallback message, got stdout:\n{}",
        stdout
    );

    let dest_file = ctx.module_dir.join("push_grpc.txt");
    assert!(dest_file.exists(), "remote file missing");
    let bytes = fs::read(&dest_file).expect("read remote file");
    assert_eq!(bytes, b"push-grpc-test");
}
