//! End-to-end regression tests for F2 (canonical containment).
//!
//! The lexical `safe_join` chokepoint catches `../`, absolute paths,
//! Windows-shaped roots, etc. — but it doesn't follow symlinks. F2
//! adds canonical-containment so a symlink inside a module that
//! points outside the module root cannot be followed by daemon
//! read/write operations.
//!
//! These tests spin up a real daemon, place a symlink inside the
//! exposed module pointing at a sibling directory outside, and
//! attempt to read/list/find through the symlink. The daemon must
//! reject every such request with a permission-denied containment
//! error.

use std::fs;
use std::process::Command;
use std::time::Duration;

mod common;
use common::{run_with_timeout, TestContext};

#[cfg(unix)]
fn place_escape_symlink(ctx: &TestContext) -> std::path::PathBuf {
    use std::os::unix::fs::symlink;
    // Create a sibling directory outside the module containing
    // sensitive content.
    let outside = ctx.workspace.join("outside");
    fs::create_dir_all(&outside).expect("outside dir");
    fs::write(outside.join("victim.txt"), b"SENSITIVE").expect("victim");

    // Place a symlink inside the module that points at it.
    let escape = ctx.module_dir.join("escape");
    symlink(&outside, &escape).expect("place escape symlink");
    outside
}

#[cfg(unix)]
#[test]
fn f2_pull_through_symlink_rejected() {
    let ctx = TestContext::new();
    let outside = place_escape_symlink(&ctx);

    let dest_dir = ctx.workspace.join("dest");
    fs::create_dir_all(&dest_dir).expect("dest dir");

    let remote_src = format!("127.0.0.1:{}:/test/escape", ctx.daemon_port);
    let mut cli_cmd = Command::new(&ctx.cli_bin);
    cli_cmd
        .arg("--config-dir")
        .arg(&ctx.config_dir)
        .arg("copy")
        .arg("--yes")
        .arg(&remote_src)
        .arg(&dest_dir);
    let output = run_with_timeout(cli_cmd, Duration::from_secs(60));

    assert!(
        !output.status.success(),
        "daemon should have refused symlink-escape pull, but it succeeded"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.to_lowercase().contains("containment")
            || stderr.to_lowercase().contains("escapes module root"),
        "expected containment error, got stderr:\n{stderr}"
    );

    // The sensitive file outside the module must not have been
    // copied to the destination — neither at the top level nor
    // under any subdir.
    let leaked = dest_dir.join("victim.txt");
    assert!(
        !leaked.exists(),
        "victim.txt must not have been copied via the symlink escape"
    );
    // And of course the original outside file is untouched.
    assert!(outside.join("victim.txt").exists());
}

#[cfg(unix)]
#[test]
fn f2_push_destination_path_symlink_rejected() {
    // R13-F1 regression: a push whose destination_path traverses
    // an in-module symlink pointing outside the module root must
    // be refused at the push handshake — not just at per-file
    // write time. Mirror purge in particular enumerates the
    // (post-mutation) module path before any per-entry check fires,
    // so the handshake-level rejection is load-bearing.
    let ctx = TestContext::new();
    let _outside = place_escape_symlink(&ctx);

    // Source has a single file we'd be pushing.
    let src_dir = ctx.workspace.join("src");
    fs::create_dir_all(&src_dir).expect("src dir");
    fs::write(src_dir.join("payload.txt"), b"would-overwrite").expect("payload");

    // Destination URL targets *through* the in-module symlink.
    // The daemon should refuse the push before any write.
    let dest_remote = format!("127.0.0.1:{}:/test/escape/", ctx.daemon_port);
    let mut cli_cmd = Command::new(&ctx.cli_bin);
    cli_cmd
        .arg("--config-dir")
        .arg(&ctx.config_dir)
        .arg("mirror")
        .arg("--yes")
        .arg(&format!("{}/", src_dir.display()))
        .arg(&dest_remote);
    let output = run_with_timeout(cli_cmd, Duration::from_secs(60));

    assert!(
        !output.status.success(),
        "daemon should have refused push through escape symlink, but it succeeded"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.to_lowercase().contains("containment")
            || stderr.to_lowercase().contains("escapes module root"),
        "expected containment error, got stderr:\n{stderr}"
    );

    // The outside directory must not have been touched — neither
    // a payload.txt copied through, nor the victim.txt overwritten.
    let leaked = ctx.workspace.join("outside").join("payload.txt");
    assert!(!leaked.exists(), "payload must not leak through symlink");
    let victim_bytes =
        fs::read(ctx.workspace.join("outside").join("victim.txt")).expect("read victim");
    assert_eq!(victim_bytes, b"SENSITIVE", "victim.txt must be untouched");
}

#[cfg(unix)]
#[test]
fn f2_legitimate_intra_module_symlink_works() {
    // Sanity: an intra-module symlink (e.g., latest -> v1) is a
    // legitimate use case and must NOT be rejected.
    use std::os::unix::fs::symlink;
    let ctx = TestContext::new();

    let v1_dir = ctx.module_dir.join("v1");
    fs::create_dir_all(&v1_dir).expect("v1 dir");
    fs::write(v1_dir.join("hello.txt"), b"v1 content").expect("v1/hello");
    symlink(&v1_dir, ctx.module_dir.join("latest")).expect("latest symlink");

    let dest_dir = ctx.workspace.join("dest");
    fs::create_dir_all(&dest_dir).expect("dest dir");

    let remote_src = format!("127.0.0.1:{}:/test/latest/hello.txt", ctx.daemon_port);
    let mut cli_cmd = Command::new(&ctx.cli_bin);
    cli_cmd
        .arg("--config-dir")
        .arg(&ctx.config_dir)
        .arg("copy")
        .arg("--yes")
        .arg(&remote_src)
        .arg(&dest_dir);
    let output = run_with_timeout(cli_cmd, Duration::from_secs(60));

    if !output.status.success() {
        panic!(
            "intra-module symlink pull should succeed, got:\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let copied = dest_dir.join("hello.txt");
    assert!(copied.exists(), "expected file copied through symlink");
    assert_eq!(fs::read(&copied).unwrap(), b"v1 content");
}
