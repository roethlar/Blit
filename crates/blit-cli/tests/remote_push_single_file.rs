//! Regression tests for the "single-file remote push crash" bug.
//!
//! See `docs/bugs/single-file-remote-push-crash.md`. A `blit copy FILE
//! server:/mod/` invocation must actually push the file, not fail with
//! "opening FILE/ during payload planning: Not a directory".

use std::fs;
use std::process::Command;
use std::time::Duration;

mod common;
use common::{run_with_timeout, TestContext};

#[cfg(unix)]
#[test]
fn push_single_file_to_container_dir() {
    let ctx = TestContext::new();
    let src_file = ctx.workspace.join("hello.txt");
    fs::write(&src_file, b"hello world").expect("write source file");

    let dest_remote = format!("127.0.0.1:{}:/test/", ctx.daemon_port);
    let mut cli_cmd = Command::new(&ctx.cli_bin);
    cli_cmd
        .arg("--config-dir")
        .arg(&ctx.config_dir)
        .arg("copy")
        .arg("--yes")
        .arg(src_file.to_string_lossy().to_string())
        .arg(&dest_remote);

    let output = run_with_timeout(cli_cmd, Duration::from_secs(60));
    assert!(
        output.status.success(),
        "blit-cli failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let dest_file = ctx.module_dir.join("hello.txt");
    assert!(
        dest_file.exists(),
        "expected {} to exist on remote after single-file push",
        dest_file.display()
    );
    let bytes = fs::read(&dest_file).expect("read remote file");
    assert_eq!(bytes, b"hello world");
}

#[cfg(unix)]
#[test]
fn push_single_file_rename() {
    // `blit copy FILE server:/mod/new.txt` (no trailing slash on dest)
    // — rsync semantics say rename.
    let ctx = TestContext::new();
    let src_file = ctx.workspace.join("orig.txt");
    fs::write(&src_file, b"rename-me").expect("write source");

    let dest_remote = format!("127.0.0.1:{}:/test/renamed.txt", ctx.daemon_port);
    let mut cli_cmd = Command::new(&ctx.cli_bin);
    cli_cmd
        .arg("--config-dir")
        .arg(&ctx.config_dir)
        .arg("copy")
        .arg("--yes")
        .arg(src_file.to_string_lossy().to_string())
        .arg(&dest_remote);

    let output = run_with_timeout(cli_cmd, Duration::from_secs(60));
    assert!(
        output.status.success(),
        "blit-cli failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let dest_file = ctx.module_dir.join("renamed.txt");
    assert!(
        dest_file.exists(),
        "expected {} to exist on remote after rename",
        dest_file.display()
    );
    let bytes = fs::read(&dest_file).expect("read remote file");
    assert_eq!(bytes, b"rename-me");
}
