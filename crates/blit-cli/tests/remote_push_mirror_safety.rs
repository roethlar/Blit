//! R59 #1 regression tests for the remote-push mirror safety gates:
//!
//! - F1: daemon must refuse to purge destination entries when the
//!   client's source-side scan reports `scan_complete=false` (i.e.
//!   `unreadable_paths` is non-empty). Pre-fix the daemon purged
//!   unconditionally after upload, so a permission-denied subtree
//!   on the source caused silent dest-side data loss.
//!
//! - F2: daemon must honor the client's filter scope when planning
//!   the purge. Pre-fix it enumerated the destination with
//!   `FileFilter::default()` and treated every out-of-scope file as
//!   extraneous, so `push --include '*.bin' --mirror` deleted the
//!   destination's non-bin files even though they were never in
//!   scope for the operation.

use std::fs;
use std::process::Command;
use std::time::Duration;

mod common;
use common::{run_with_timeout, TestContext};

/// F1: incomplete scan must refuse purge. The destination's
/// only file is absent from the (partial) source manifest, so
/// pre-fix the daemon would have deleted it.
#[cfg(unix)]
#[test]
fn push_mirror_refuses_when_source_scan_incomplete() {
    use std::os::unix::fs::PermissionsExt;

    let ctx = TestContext::new();

    // Source has one readable file and one unreadable subtree.
    let src = ctx.workspace.join("src");
    fs::create_dir_all(&src).unwrap();
    fs::write(src.join("readable.txt"), b"keep").unwrap();
    let blocked = src.join("blocked");
    fs::create_dir_all(&blocked).unwrap();
    fs::write(blocked.join("inner.txt"), b"unscannable").unwrap();

    let mut perms = fs::metadata(&blocked).unwrap().permissions();
    perms.set_mode(0o000);
    fs::set_permissions(&blocked, perms).unwrap();
    struct PermGuard(std::path::PathBuf);
    impl Drop for PermGuard {
        fn drop(&mut self) {
            if let Ok(meta) = std::fs::metadata(&self.0) {
                let mut p = meta.permissions();
                p.set_mode(0o755);
                let _ = std::fs::set_permissions(&self.0, p);
            }
        }
    }
    let _guard = PermGuard(blocked.clone());

    // Destination has a file the source doesn't know about. Pre-fix
    // the daemon would purge it.
    fs::write(ctx.module_dir.join("extra.txt"), b"would-be-deleted").unwrap();

    let dest_remote = format!("127.0.0.1:{}:/test/", ctx.daemon_port);
    let mut cli_cmd = Command::new(&ctx.cli_bin);
    cli_cmd
        .arg("--config-dir")
        .arg(&ctx.config_dir)
        .arg("mirror")
        .arg("--yes")
        .arg(format!("{}/", src.display()))
        .arg(&dest_remote);

    let output = run_with_timeout(cli_cmd, Duration::from_secs(60));
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // The CLI may surface the error either as a daemon refusal or as
    // its own pre-flight detection of unreadable paths — both are
    // valid R47-F4 / R59 F1 behaviors. What matters is that the
    // destination file was NOT purged.
    assert!(
        ctx.module_dir.join("extra.txt").exists(),
        "destination's extra.txt was purged despite incomplete source scan; \
         stdout:\n{stdout}\nstderr:\n{stderr}"
    );
    if output.status.success() {
        // If the operation reports success, the daemon must at minimum
        // have NOT purged (asserted above). Some shell stacks bubble
        // the unreadable-path warning as exit-success — accept that.
    } else {
        // More common path: operation fails. The error message should
        // mention the scan / unreadable / refused / permission cluster.
        assert!(
            stderr.contains("scan")
                || stderr.contains("permission")
                || stderr.contains("unreadable")
                || stderr.contains("refusing")
                || stderr.contains("incomplete"),
            "expected scan-incomplete refusal; stderr was: {stderr}"
        );
    }
}

/// F2: filtered-subset mirror must not purge destination entries
/// that are outside the user's filter scope. Pre-fix the daemon
/// enumerated the destination unfiltered and would purge `kept.log`
/// because it wasn't in the source manifest — even though the
/// user's `--exclude '*.log'` clearly indicates "don't touch log
/// files at the destination."
#[test]
fn push_mirror_filter_keeps_out_of_scope_destination_entries() {
    let ctx = TestContext::new();

    let src = ctx.workspace.join("src");
    fs::create_dir_all(&src).unwrap();
    fs::write(src.join("kept.txt"), b"a").unwrap();
    fs::write(src.join("noisy.log"), b"b").unwrap();

    // Destination pre-state.
    fs::write(ctx.module_dir.join("kept.txt"), b"older").unwrap();
    fs::write(ctx.module_dir.join("preserve.log"), b"out-of-scope").unwrap();

    let dest_remote = format!("127.0.0.1:{}:/test/", ctx.daemon_port);
    let mut cli_cmd = Command::new(&ctx.cli_bin);
    cli_cmd
        .arg("--config-dir")
        .arg(&ctx.config_dir)
        .arg("mirror")
        .arg("--yes")
        .arg("--exclude")
        .arg("*.log")
        .arg(format!("{}/", src.display()))
        .arg(&dest_remote);

    let output = run_with_timeout(cli_cmd, Duration::from_secs(60));
    assert!(
        output.status.success(),
        "mirror with --exclude should succeed; stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    // kept.txt updated (in-scope, present at source).
    assert_eq!(
        fs::read(ctx.module_dir.join("kept.txt")).unwrap(),
        b"a",
        "in-scope file must be updated by mirror"
    );
    // preserve.log MUST still exist — it's out of scope for the
    // --exclude '*.log' filter, so the daemon's purge enumerator
    // should never have seen it.
    assert!(
        ctx.module_dir.join("preserve.log").exists(),
        "out-of-scope destination file was purged (R59 #1 F2 regression)"
    );
    assert_eq!(
        fs::read(ctx.module_dir.join("preserve.log")).unwrap(),
        b"out-of-scope",
        "out-of-scope destination file was overwritten or truncated"
    );
}
