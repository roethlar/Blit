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

    // Source has trailing slash -> rsync "merge contents" semantics:
    // files under src/ land directly in the module root.
    let src_arg = format!("{}/", src_dir.display());
    let dest_remote = format!("127.0.0.1:{}:/test/", ctx.daemon_port);
    let mut cli_cmd = Command::new(&ctx.cli_bin);
    cli_cmd
        .arg("--config-dir")
        .arg(&ctx.config_dir)
        .arg("move")
        .arg("--yes")
        .arg(&src_arg)
        .arg(&dest_remote);

    let output = run_with_timeout(cli_cmd, Duration::from_secs(60));
    assert!(output.status.success(), "blit move failed");

    // Verify destination file exists at module root (merged).
    let dest_file = ctx.module_dir.join("move_me.txt");
    assert!(dest_file.exists(), "remote file missing");
    let bytes = fs::read(&dest_file).expect("read remote file");
    assert_eq!(bytes, b"move content");

    // Verify source is deleted after the move.
    assert!(!src_file.exists(), "source file should have been deleted");
    assert!(
        !src_dir.exists(),
        "source directory should have been deleted"
    );
}

#[test]
fn test_remote_move_remote_to_local() {
    let ctx = TestContext::new();
    let dest_dir = ctx.workspace.join("dest");
    // Pre-create dest as a directory so the rsync resolver treats it as
    // a container (places `remote_move.txt` inside it) rather than an
    // exact rename target.
    fs::create_dir_all(&dest_dir).expect("create dest dir");

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
    assert!(
        !remote_file.exists(),
        "remote file should have been deleted"
    );
}

// audit-6e: the single-file move tests above cover both remote directions,
// but a move of a multi-file *directory tree* exercises the recursive
// copy-then-delete-source path — the real partial-deletion data-loss
// surface. These assert every file lands at the destination AND the entire
// source side is removed (source deleted only after the verified copy).

#[test]
fn test_remote_move_local_to_remote_directory_tree() {
    let ctx = TestContext::new();
    let src_dir = ctx.workspace.join("tree_src");
    fs::create_dir_all(src_dir.join("nested")).expect("src dirs");
    fs::write(src_dir.join("a.txt"), "alpha").expect("write a");
    fs::write(src_dir.join("b.txt"), "bravo").expect("write b");
    fs::write(src_dir.join("nested/c.txt"), "charlie").expect("write c");

    // Trailing slash -> merge contents into the module root.
    let src_arg = format!("{}/", src_dir.display());
    let dest_remote = format!("127.0.0.1:{}:/test/", ctx.daemon_port);
    let mut cli_cmd = Command::new(&ctx.cli_bin);
    cli_cmd
        .arg("--config-dir")
        .arg(&ctx.config_dir)
        .arg("move")
        .arg("--yes")
        .arg(&src_arg)
        .arg(&dest_remote);

    let output = run_with_timeout(cli_cmd, Duration::from_secs(60));
    assert!(
        output.status.success(),
        "blit move (push dir) failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Every file landed at the module root (merged), nested tree preserved.
    assert_eq!(fs::read(ctx.module_dir.join("a.txt")).expect("a"), b"alpha");
    assert_eq!(fs::read(ctx.module_dir.join("b.txt")).expect("b"), b"bravo");
    assert_eq!(
        fs::read(ctx.module_dir.join("nested/c.txt")).expect("c"),
        b"charlie"
    );

    // Entire source tree removed only after the verified copy.
    assert!(!src_dir.join("a.txt").exists(), "src a.txt not deleted");
    assert!(
        !src_dir.join("nested/c.txt").exists(),
        "src nested/c.txt not deleted"
    );
    assert!(!src_dir.exists(), "source tree should be fully removed");
}

#[test]
fn test_remote_move_remote_to_local_directory_tree() {
    let ctx = TestContext::new();
    // Remote subdir with a nested tree.
    let remote_sub = ctx.module_dir.join("tree");
    fs::create_dir_all(remote_sub.join("inner")).expect("remote dirs");
    fs::write(remote_sub.join("x.txt"), "x-data").expect("write x");
    fs::write(remote_sub.join("inner/y.txt"), "y-data").expect("write y");

    let dest_dir = ctx.workspace.join("pulled");
    fs::create_dir_all(&dest_dir).expect("dest dir");

    // Trailing slash -> merge contents into dest_dir.
    let src_remote = format!("127.0.0.1:{}:/test/tree/", ctx.daemon_port);
    let mut cli_cmd = Command::new(&ctx.cli_bin);
    cli_cmd
        .arg("--config-dir")
        .arg(&ctx.config_dir)
        .arg("move")
        .arg("--yes")
        .arg(&src_remote)
        .arg(&dest_dir);

    let output = run_with_timeout(cli_cmd, Duration::from_secs(60));
    assert!(
        output.status.success(),
        "blit move (pull dir) failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Files arrived locally, nested tree preserved.
    assert_eq!(fs::read(dest_dir.join("x.txt")).expect("x"), b"x-data");
    assert_eq!(
        fs::read(dest_dir.join("inner/y.txt")).expect("y"),
        b"y-data"
    );

    // Remote source files removed after the verified copy.
    assert!(
        !remote_sub.join("x.txt").exists(),
        "remote source x.txt should be deleted"
    );
    assert!(
        !remote_sub.join("inner/y.txt").exists(),
        "remote source inner/y.txt should be deleted"
    );
}
