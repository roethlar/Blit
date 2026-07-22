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

/// codex otp-10a F1: `blit move` to a remote destination must land the
/// source bytes even when the destination already holds a SAME-SIZE,
/// NEWER file — the copy-shaped SizeMtime compare would skip that cell,
/// and move's source-delete would then destroy the only copy of the
/// source content. The move verb pushes with IgnoreTimes (transfer
/// unconditionally), so the destination ends with the source bytes and
/// deleting the source is safe.
#[test]
fn move_lands_source_bytes_over_same_size_newer_destination() {
    let ctx = TestContext::new();
    let src_dir = ctx.workspace.join("src");
    fs::create_dir_all(&src_dir).expect("src dir");
    let src_file = src_dir.join("clash.txt");
    fs::write(&src_file, "source-bytes").expect("write source");

    // Same size, different content, newer mtime at the destination.
    let dest_file = ctx.module_dir.join("clash.txt");
    fs::write(&dest_file, "dest---bytes").expect("seed dest");
    let newer = filetime::FileTime::from_unix_time(
        filetime::FileTime::from_last_modification_time(
            &fs::metadata(&src_file).expect("src meta"),
        )
        .unix_seconds()
            + 60,
        0,
    );
    filetime::set_file_mtime(&dest_file, newer).expect("bump dest mtime");

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
        "blit move failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    assert_eq!(
        fs::read(&dest_file).expect("read dest"),
        b"source-bytes",
        "move must overwrite the same-size-newer destination with the source bytes"
    );
    assert!(
        !src_file.exists(),
        "source deleted only after its bytes landed"
    );
}

/// otp-10b-2 (codex otp-10a F1 mirrored on pull): a remote→local move
/// through the actual binary must land the REMOTE source's bytes over
/// a same-size-newer LOCAL destination before deleting the remote
/// source — the pull twin of the push-move pin above. Pre-cutover the
/// old pull's SizeMtime compare would skip the file and the
/// source-delete would destroy the only copy of its content.
#[test]
fn pull_move_lands_source_bytes_over_same_size_newer_destination() {
    let ctx = TestContext::new();

    let remote_file = ctx.module_dir.join("clash.txt");
    fs::write(&remote_file, "source-bytes").expect("write remote source");

    // Same size, different content, newer mtime at the LOCAL destination.
    let dest_dir = ctx.workspace.join("dest");
    fs::create_dir_all(&dest_dir).expect("dest dir");
    let dest_file = dest_dir.join("clash.txt");
    fs::write(&dest_file, "dest---bytes").expect("seed dest");
    let newer = filetime::FileTime::from_unix_time(
        filetime::FileTime::from_last_modification_time(
            &fs::metadata(&remote_file).expect("remote meta"),
        )
        .unix_seconds()
            + 60,
        0,
    );
    filetime::set_file_mtime(&dest_file, newer).expect("bump dest mtime");

    // A single-file remote source (a module-root move cannot delete
    // the module root itself); the rsync rule drops it into dest_dir.
    let src_remote = format!("127.0.0.1:{}:/test/clash.txt", ctx.daemon_port);
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
        "blit move failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    assert_eq!(
        fs::read(&dest_file).expect("read dest"),
        b"source-bytes",
        "pull-move must overwrite the same-size-newer destination with the remote bytes"
    );
    assert!(
        !remote_file.exists(),
        "remote source deleted only after its bytes landed"
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

// The former Windows ignore is deliberately gone. Retained CI stderr showed
// the old push protocol stalled on the native `nested\c.txt` need-list echo,
// before source deletion. Commit 48c5a11 canonicalized that echo; the unified
// session now preserves the source's POSIX wire path directly. Keep this exact
// nested push-move active on every supported OS so that Windows guards the
// original end-to-end failure rather than only lower-level nested pushes.
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
    // Symmetric to the push direction: the recursive move must remove the
    // source directory tree, not just unlink the files.
    assert!(
        !remote_sub.join("inner").exists(),
        "remote source nested dir inner/ should be removed"
    );
    assert!(
        !remote_sub.exists(),
        "remote source tree/ should be fully removed after the move"
    );
}
