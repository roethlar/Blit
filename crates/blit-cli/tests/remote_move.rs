use std::fs;
use std::process::Command;
use std::time::Duration;

mod common;
use common::{run_with_timeout, TestContext};

#[test]
fn test_remote_move_local_to_remote() {
    let ctx = TestContext::new();
    let src_dir = ctx.workspace.join("src");
    fs::create_dir_all(&src_dir).expect("src dir");
    let src_file = src_dir.join("move_me.txt");
    fs::write(&src_file, "move content").expect("write file");

    let dest_remote = format!("127.0.0.1:{}:/test/", ctx.daemon_port);
    let mut cli_cmd = Command::new(&ctx.cli_bin);
    cli_cmd
        .arg("--config-dir")
        .arg(&ctx.config_dir)
        .arg("move")
        .arg("--yes")
        .arg(&src_dir)
        .arg(&dest_remote);

    let output = run_with_timeout(cli_cmd, Duration::from_secs(60));
    assert!(output.status.success(), "blit move failed");

    // Verify destination file exists
    let dest_file = ctx.module_dir.join("move_me.txt");
    assert!(dest_file.exists(), "remote file missing");
    let bytes = fs::read(&dest_file).expect("read remote file");
    assert_eq!(bytes, b"move content");

    // Verify source file is deleted
    assert!(!src_file.exists(), "source file should have been deleted");
    // Verify source directory is deleted (since we moved the dir content, but `blit move src dest` usually moves the dir content if src is a dir)
    // Wait, `blit move src dest` behavior depends on if src is a file or dir.
    // If src is a dir, it mirrors the dir content and then deletes the src dir.
    assert!(!src_dir.exists(), "source directory should have been deleted");
}

#[test]
fn test_remote_move_remote_to_local() {
    let ctx = TestContext::new();
    let dest_dir = ctx.workspace.join("dest");
    
    // Setup remote file
    let remote_file = ctx.module_dir.join("remote_move.txt");
    fs::write(&remote_file, "remote move content").expect("write file");

    let src_remote = format!("127.0.0.1:{}:/test/remote_move.txt", ctx.daemon_port);
    let mut cli_cmd = Command::new(&ctx.cli_bin);
    cli_cmd
        .arg("--config-dir")
        .arg(&ctx.config_dir)
        .arg("move")
        .arg("--yes")
        .arg(&src_remote)
        .arg(&dest_dir);

    let output = run_with_timeout(cli_cmd, Duration::from_secs(60));
    assert!(output.status.success(), "blit move failed");

    // Verify local file exists
    let dest_file = dest_dir.join("remote_move.txt");
    assert!(dest_file.exists(), "local file missing");
    let bytes = fs::read(&dest_file).expect("read local file");
    assert_eq!(bytes, b"remote move content");

    // Verify remote file is deleted
    assert!(!remote_file.exists(), "remote file should have been deleted");
}
