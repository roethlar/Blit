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
//! The dual-daemon mini-harness for the delegated case is another
//! clone of the remote_remote.rs pattern; consolidation is w9-3.

use std::fs;
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

use serde::Serialize;
use tempfile::tempdir;

mod common;
use common::{run_with_timeout, ChildGuard, TestContext};

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

#[derive(Serialize)]
struct DaemonConfig {
    daemon: DaemonSection,
    #[serde(rename = "module")]
    modules: Vec<ModuleSection>,
    #[serde(skip_serializing_if = "Option::is_none")]
    delegation: Option<DelegationSection>,
}

#[derive(Serialize)]
struct DaemonSection {
    bind: String,
    port: u16,
    no_mdns: bool,
}

#[derive(Serialize)]
struct ModuleSection {
    name: String,
    path: PathBuf,
    comment: Option<String>,
    read_only: bool,
    delegation_allowed: bool,
}

#[derive(Serialize)]
struct DelegationSection {
    allow_delegated_pull: bool,
    allowed_source_hosts: Vec<String>,
}

fn wait_for_port(port: u16, label: &str) {
    let mut ready = false;
    for _ in 0..50 {
        if TcpStream::connect(("127.0.0.1", port)).is_ok() {
            ready = true;
            break;
        }
        thread::sleep(Duration::from_millis(100));
    }
    assert!(ready, "{label} failed to listen on {port}");
}

#[allow(clippy::too_many_arguments)]
fn spawn_daemon(
    workspace: &Path,
    bin: &Path,
    port: u16,
    name: &str,
    module_path: &Path,
    read_only: bool,
    delegation_enabled: bool,
) -> ChildGuard {
    let config = DaemonConfig {
        daemon: DaemonSection {
            bind: "127.0.0.1".into(),
            port,
            no_mdns: true,
        },
        modules: vec![ModuleSection {
            name: "test".into(),
            path: module_path.to_path_buf(),
            comment: None,
            read_only,
            delegation_allowed: true,
        }],
        delegation: delegation_enabled.then(|| DelegationSection {
            allow_delegated_pull: true,
            allowed_source_hosts: vec!["127.0.0.1".to_string()],
        }),
    };

    let config_path = workspace.join(format!("{name}.toml"));
    let toml = toml::to_string(&config).expect("serialize config");
    fs::write(&config_path, toml).expect("write config");

    let child = Command::new(bin)
        .arg("--config")
        .arg(&config_path)
        .arg("--bind")
        .arg("127.0.0.1")
        .arg("--port")
        .arg(port.to_string())
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn daemon");

    wait_for_port(port, &format!("daemon {name}"));
    ChildGuard::new(child)
}

#[test]
fn delegated_pull_to_read_only_destination_is_rejected() {
    let work = tempdir().expect("tempdir");
    let workspace = work.path().to_path_buf();

    let module_src = workspace.join("module_src");
    let module_dst = workspace.join("module_dst");
    fs::create_dir_all(&module_src).expect("src module");
    fs::create_dir_all(&module_dst).expect("dst module");
    fs::write(module_src.join("payload.txt"), b"must-not-land").expect("seed src");

    let config_dir = workspace.join("cli-config");
    fs::create_dir_all(&config_dir).expect("cli config");

    let exe_path = std::env::current_exe().expect("current_exe");
    let bin_dir = exe_path
        .parent()
        .expect("deps dir")
        .parent()
        .expect("bin dir")
        .to_path_buf();
    let cli_bin = bin_dir.join(if cfg!(windows) { "blit.exe" } else { "blit" });
    let daemon_bin = bin_dir.join(if cfg!(windows) {
        "blit-daemon.exe"
    } else {
        "blit-daemon"
    });

    let src_port = common::pick_unused_port();
    let dst_port = common::pick_unused_port();
    assert_ne!(src_port, dst_port);

    let _src = spawn_daemon(
        &workspace,
        &daemon_bin,
        src_port,
        "daemon_src",
        &module_src,
        false,
        false,
    );
    // Destination: delegation enabled, module read-only — the
    // delegated_pull read-only gate is what must fire.
    let _dst = spawn_daemon(
        &workspace,
        &daemon_bin,
        dst_port,
        "daemon_dst",
        &module_dst,
        true,
        true,
    );

    let src_remote = format!("127.0.0.1:{src_port}:/test/");
    let dst_remote = format!("127.0.0.1:{dst_port}:/test/");
    let mut cmd = Command::new(&cli_bin);
    cmd.arg("--config-dir")
        .arg(&config_dir)
        .arg("copy")
        .arg(&src_remote)
        .arg(&dst_remote);
    let output = run_with_timeout(cmd, Duration::from_secs(60));

    assert_read_only_rejection(&output, "delegated pull");
    assert!(
        fs::read_dir(&module_dst)
            .expect("dst module readable")
            .next()
            .is_none(),
        "read-only destination module must stay untouched"
    );
}
