//! Regression tests for remote pull of subpaths.
//!
//! Four cases previously broken:
//!   1. Pull single file → container dir (appended basename)
//!   2. Pull dir → container dir (nest under basename)
//!   3. Pull dir/ (trailing slash) → merge contents
//!   4. Pull single file → exact rename target
//!
//! Case 2 and 3 were silently double-nested (`dst/gamedir/gamedir/...`)
//! because the daemon prefixed every header.relative_path with the
//! requested subpath AND the CLI resolver appended the basename to dest.
//!
//! Cases 1 and 4 silently no-op'd because the daemon enumerator skipped
//! the file-root entry at depth 0.
//!
//! All existing tests used `/test/` (empty subpath) which hid both bugs.

use std::fs;
use std::process::Command;
use std::time::Duration;

mod common;
use common::{run_with_timeout, TestContext};

fn setup_module(ctx: &TestContext) {
    fs::create_dir_all(ctx.module_dir.join("gamedir/subdir")).expect("nested dirs");
    fs::write(ctx.module_dir.join("gamedir/a.txt"), b"a-content").expect("write a");
    fs::write(ctx.module_dir.join("gamedir/subdir/b.txt"), b"b-content").expect("write b");
    fs::write(ctx.module_dir.join("single.txt"), b"solo-file-content").expect("write single");
}

fn pull(ctx: &TestContext, remote_path: &str, local_dest: &std::path::Path) {
    let src = format!("127.0.0.1:{}:{}", ctx.daemon_port, remote_path);
    let mut cmd = Command::new(&ctx.cli_bin);
    cmd.arg("--config-dir")
        .arg(&ctx.config_dir)
        .arg("copy")
        .arg("--yes")
        .arg(&src)
        .arg(local_dest);
    let output = run_with_timeout(cmd, Duration::from_secs(30));
    assert!(
        output.status.success(),
        "blit copy failed\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn pull_single_file_to_container_dir() {
    let ctx = TestContext::new();
    setup_module(&ctx);
    let dst = ctx.workspace.join("dst");
    fs::create_dir_all(&dst).expect("mkdir dst");

    pull(&ctx, "/test/single.txt", &dst);

    let f = dst.join("single.txt");
    assert!(f.exists(), "{} should exist", f.display());
    assert_eq!(fs::read(&f).unwrap(), b"solo-file-content");
    // Must NOT have created a directory at that path.
    assert!(
        f.is_file(),
        "{} should be a file, not a directory",
        f.display()
    );
}

#[test]
fn pull_single_file_rename() {
    let ctx = TestContext::new();
    setup_module(&ctx);
    let dst = ctx.workspace.join("dst");
    fs::create_dir_all(&dst).expect("mkdir dst");
    let target = dst.join("renamed.txt");

    pull(&ctx, "/test/single.txt", &target);

    assert!(target.exists(), "{} should exist", target.display());
    assert_eq!(fs::read(&target).unwrap(), b"solo-file-content");
    assert!(target.is_file());
}

#[test]
fn pull_dir_no_slash_nests_under_basename() {
    let ctx = TestContext::new();
    setup_module(&ctx);
    let dst = ctx.workspace.join("dst");
    fs::create_dir_all(&dst).expect("mkdir dst");

    pull(&ctx, "/test/gamedir", &dst);

    // Expect: dst/gamedir/a.txt and dst/gamedir/subdir/b.txt
    assert_eq!(fs::read(dst.join("gamedir/a.txt")).unwrap(), b"a-content");
    assert_eq!(
        fs::read(dst.join("gamedir/subdir/b.txt")).unwrap(),
        b"b-content"
    );
    // Regression: must NOT double-nest as dst/gamedir/gamedir/...
    assert!(
        !dst.join("gamedir/gamedir").exists(),
        "double-nesting regression detected"
    );
}

#[test]
fn pull_dir_trailing_slash_merges_contents() {
    let ctx = TestContext::new();
    setup_module(&ctx);
    let dst = ctx.workspace.join("dst");
    fs::create_dir_all(&dst).expect("mkdir dst");

    pull(&ctx, "/test/gamedir/", &dst);

    // Trailing slash → merge contents into dst (no basename prefix).
    assert_eq!(fs::read(dst.join("a.txt")).unwrap(), b"a-content");
    assert_eq!(fs::read(dst.join("subdir/b.txt")).unwrap(), b"b-content");
    assert!(
        !dst.join("gamedir").exists(),
        "merge semantics violated: 'gamedir' should not appear in dst"
    );
}
