//! w9-4: read-only-module enforcement tests
//! (tests-readonly-module-enforcement-untested).
//!
//! The daemon refuses writes to `read_only: true` modules in three
//! places — push control stream (push/control.rs), purge
//! (core.rs::purge_inner), and delegated pull (delegated_pull.rs) —
//! and before this file no test in the workspace ever configured a
//! read-only module, so a dropped gate (mirror-deletion blast
//! radius) would have passed the full validation suite.
//!
//! The dual-daemon delegated case runs on the shared `common` spawn
//! primitives (consolidated by w9-3).

use std::fs;
use std::process::Command;
use std::time::Duration;

mod common;
use common::{run_with_timeout, DaemonOptions, TestContext};

fn assert_read_only_rejection(output: &std::process::Output, what: &str) {
    assert!(
        !output.status.success(),
        "{what} against a read-only module must fail\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("read-only"),
        "{what} must surface the read-only rejection, got stderr:\n{stderr}"
    );
}

#[test]
fn push_to_read_only_module_is_rejected_and_module_untouched() {
    let ctx = TestContext::new_read_only();

    let src_dir = ctx.workspace.join("src");
    fs::create_dir_all(&src_dir).expect("src dir");
    fs::write(src_dir.join("payload.txt"), b"must-not-land").expect("write src");

    let dest = format!("127.0.0.1:{}:/test/", ctx.daemon_port);
    let src_arg = format!("{}/", src_dir.display());
    let mut cmd = Command::new(&ctx.cli_bin);
    cmd.arg("--config-dir")
        .arg(&ctx.config_dir)
        .arg("copy")
        .arg(&src_arg)
        .arg(&dest);
    let output = run_with_timeout(cmd, Duration::from_secs(60));

    assert_read_only_rejection(&output, "push");
    assert!(
        fs::read_dir(&ctx.module_dir)
            .expect("module dir readable")
            .next()
            .is_none(),
        "read-only module must stay untouched after a rejected push"
    );
}

/// design-5: with many files the client is mid-manifest-send when the
/// daemon's rejection lands, so the request-stream send fails before
/// the response is read — and pre-fix the user saw "failed to send
/// push request payload" instead of the read-only reason. This was the
/// first failure the w9-1/w9-4 ungating surfaced on CI (macOS and
/// Windows lost the race; local single-file runs won it). The client
/// now harvests the daemon's terminal status on send failure.
#[test]
fn push_rejection_reason_survives_midmanifest_send_failure() {
    let ctx = TestContext::new_read_only();

    let src_dir = ctx.workspace.join("src");
    fs::create_dir_all(&src_dir).expect("src dir");
    for i in 0..500 {
        fs::write(src_dir.join(format!("f{i}.txt")), b"x").expect("write src");
    }

    let dest = format!("127.0.0.1:{}:/test/", ctx.daemon_port);
    let src_arg = format!("{}/", src_dir.display());
    let mut cmd = Command::new(&ctx.cli_bin);
    cmd.arg("--config-dir")
        .arg(&ctx.config_dir)
        .arg("copy")
        .arg(&src_arg)
        .arg(&dest);
    let output = run_with_timeout(cmd, Duration::from_secs(60));

    assert_read_only_rejection(&output, "many-file push");
    assert!(
        fs::read_dir(&ctx.module_dir)
            .expect("module dir readable")
            .next()
            .is_none(),
        "read-only module must stay untouched after a rejected many-file push"
    );
}

#[test]
fn purge_on_read_only_module_is_rejected_and_file_survives() {
    let ctx = TestContext::new_read_only();

    // Seed the module on disk directly — the daemon serves it
    // read-only, but the filesystem itself is writable.
    let victim = ctx.module_dir.join("keep.txt");
    fs::write(&victim, b"survives").expect("seed module file");

    let target = format!("127.0.0.1:{}:/test/keep.txt", ctx.daemon_port);
    let mut cmd = Command::new(&ctx.cli_bin);
    cmd.arg("--config-dir")
        .arg(&ctx.config_dir)
        .arg("rm")
        .arg(&target)
        .arg("-y");
    let output = run_with_timeout(cmd, Duration::from_secs(60));

    assert_read_only_rejection(&output, "rm (purge)");
    assert_eq!(
        fs::read(&victim).expect("file must still exist"),
        b"survives",
        "rejected purge must not delete anything"
    );
}

// ---------------------------------------------------------------
// Delegated pull: needs a delegation-enabled destination whose
// module is read-only. The gate fires before the destination ever
// contacts the source, but a real source daemon keeps the test
// honest about ordering.
// ---------------------------------------------------------------

#[test]
fn delegated_pull_to_read_only_destination_is_rejected() {
    // The context's own daemon is the plain source; the destination
    // has delegation enabled and a read-only module — the
    // delegated_pull read-only gate is what must fire.
    let ctx = TestContext::new();
    fs::write(ctx.module_dir.join("payload.txt"), b"must-not-land").expect("seed src");

    let dst = ctx.spawn_second_daemon(
        "daemon_dst",
        &DaemonOptions {
            read_only: true,
            delegation: true,
            ..Default::default()
        },
    );

    let src_remote = format!("127.0.0.1:{}:/test/", ctx.daemon_port);
    let dst_remote = format!("127.0.0.1:{}:/test/", dst.port);
    let mut cmd = Command::new(&ctx.cli_bin);
    cmd.arg("--config-dir")
        .arg(&ctx.config_dir)
        .arg("copy")
        .arg(&src_remote)
        .arg(&dst_remote);
    let output = run_with_timeout(cmd, Duration::from_secs(60));

    assert_read_only_rejection(&output, "delegated pull");
    assert!(
        fs::read_dir(&dst.module_dir)
            .expect("dst module readable")
            .next()
            .is_none(),
        "read-only destination module must stay untouched"
    );
}
