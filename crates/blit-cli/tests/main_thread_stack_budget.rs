//! win-1: the `blit` binary's MAIN thread must fit a small stack on every
//! platform, not only on the ones that happen to give it 8 MiB.
//!
//! `#[tokio::main]` polls the whole command future on the process's main
//! thread. Windows gives that thread 1 MiB; macOS and Linux give it the
//! `RLIMIT_STACK` soft limit, 8 MiB by default. A debug build's async poll
//! frames hold every nested future inline, and the SOURCE_SIDE_CONTAINMENT
//! work (the ssc-6 retry passes inlined a second session into every route
//! arm) pushed the DESTINATION routes past 1 MiB: every local copy, mirror,
//! move and pull died on Windows with `thread 'main' has overflowed its
//! stack`, while the same suite passed on macOS and Linux. The fix boxes
//! the session futures where one large future nests another; this test is
//! the guard that makes the same failure visible on the platforms CI and
//! developers actually run first.
//!
//! On Unix every run goes through `sh -c 'ulimit -S -s <BUDGET_KIB> && exec
//! blit …'`: the kernel sizes the main thread of the exec'd process from
//! the soft limit, so `blit` runs with exactly that much main-thread stack.
//! On Windows the binary runs as is — its main thread is the 1 MiB the
//! failure happened on.
//!
//! The budget, measured on macOS aarch64 debug (DEVLOG 2026-09-30,
//! "win-1"): before the boxing the shapes below needed 960–1216 KiB of
//! main-thread stack (push 960, pull 1024, copy/mirror 1088, mirror with a
//! retry pass 1216) — all over this budget; after it every shape needs
//! 496 KiB, which is the argument parser's own floor (clap's derived
//! `augment_args` frames), not the transfer's.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::Duration;

mod common;
use common::{cli_bin, run_with_timeout, TestContext};

/// The main-thread stack `blit` gets on Unix, in KiB. Red before win-1's
/// boxing on every shape here, green after with a ~1.5x margin over the
/// parser floor.
#[cfg(unix)]
const BUDGET_KIB: u32 = 768;

const EXIT_PARTIAL_FAILURE: i32 = 2;

/// `blit <args>` with its main thread held to the budget.
fn budgeted_blit(config_dir: Option<&Path>, args: &[String]) -> Command {
    #[cfg(unix)]
    let mut cmd = {
        let mut cmd = Command::new("/bin/sh");
        cmd.arg("-c")
            .arg(format!("ulimit -S -s {BUDGET_KIB} && exec \"$0\" \"$@\""))
            .arg(cli_bin());
        cmd
    };
    #[cfg(windows)]
    let mut cmd = Command::new(cli_bin());
    if let Some(dir) = config_dir {
        cmd.arg("--config-dir").arg(dir);
    }
    cmd.args(args);
    cmd
}

fn run(config_dir: Option<&Path>, args: &[String]) -> Output {
    run_with_timeout(budgeted_blit(config_dir, args), Duration::from_secs(120))
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

/// The run finished with `code` — not a signal, not a stack overflow.
fn assert_exit(output: &Output, code: i32, shape: &str) {
    let stderr = text(&output.stderr);
    assert!(
        !stderr.contains("overflowed its stack"),
        "{shape}: blit overflowed its main-thread stack\nstderr:\n{stderr}"
    );
    assert_eq!(
        output.status.code(),
        Some(code),
        "{shape}: unexpected exit ({:?})\nstdout:\n{}\nstderr:\n{stderr}",
        output.status,
        text(&output.stdout)
    );
}

/// Small files in subdirectories plus one large enough to ride its own
/// record, so the shard and the single-file paths both run.
fn source_tree(root: &Path) {
    for i in 0..24u8 {
        let path = root.join(format!("d{}/f{i}.txt", i % 3));
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, vec![i; 1000 + usize::from(i) * 100]).unwrap();
    }
    fs::write(root.join("big.bin"), vec![7u8; 3_000_000]).unwrap();
}

fn slash(path: &Path) -> String {
    format!("{}/", path.display())
}

fn args(parts: &[&str]) -> Vec<String> {
    parts.iter().map(|part| part.to_string()).collect()
}

fn local_fixture() -> (tempfile::TempDir, PathBuf, PathBuf) {
    let tmp = tempfile::tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst).unwrap();
    source_tree(&src);
    (tmp, src, dst)
}

#[test]
fn local_copy_fits_the_main_thread_budget() {
    let (_tmp, src, dst) = local_fixture();
    let mut argv = args(&["copy", "--yes"]);
    argv.extend([slash(&src), slash(&dst)]);
    assert_exit(&run(None, &argv), 0, "local copy");
    assert_eq!(fs::read(dst.join("big.bin")).unwrap(), vec![7u8; 3_000_000]);
}

/// Mirror with a retry pass: a DIRECTORY occupies one file's destination
/// path, so the main pass reports it, the retry pass runs a second session
/// and reports it again, and the run exits 2.
#[test]
fn local_mirror_with_a_retry_pass_fits_the_main_thread_budget() {
    let (_tmp, src, dst) = local_fixture();
    fs::create_dir_all(dst.join("d0/f0.txt")).unwrap();
    fs::write(dst.join("stale.txt"), b"extraneous").unwrap();
    let mut argv = args(&[
        "mirror",
        "--yes",
        "--retries",
        "1",
        "--diagnostics-no-retry-wait",
    ]);
    argv.extend([slash(&src), slash(&dst)]);
    let output = run(None, &argv);
    assert_exit(&output, EXIT_PARTIAL_FAILURE, "local mirror + retry");
    assert!(
        text(&output.stderr).contains("retrying 1 file(s)"),
        "the retry pass ran\nstderr:\n{}",
        text(&output.stderr)
    );
    assert!(!dst.join("stale.txt").exists(), "the mirror deleted");
}

#[test]
fn local_move_fits_the_main_thread_budget() {
    let (_tmp, src, dst) = local_fixture();
    let mut argv = args(&["move", "--yes"]);
    argv.extend([slash(&src), slash(&dst)]);
    assert_exit(&run(None, &argv), 0, "local move");
    assert!(!src.exists(), "the move removed the source");
    assert_eq!(fs::read(dst.join("big.bin")).unwrap(), vec![7u8; 3_000_000]);
}

fn remote(ctx: &TestContext, path: &str) -> String {
    format!("127.0.0.1:{}:/test/{path}", ctx.daemon_port)
}

#[test]
fn push_fits_the_main_thread_budget() {
    let ctx = TestContext::new();
    let src = ctx.workspace.join("push-src");
    source_tree(&src);
    let mut argv = args(&["copy", "--yes"]);
    argv.extend([slash(&src), remote(&ctx, "pushed/")]);
    assert_exit(&run(Some(&ctx.config_dir), &argv), 0, "push");
    assert_eq!(
        fs::read(ctx.module_dir.join("pushed/big.bin")).unwrap(),
        vec![7u8; 3_000_000]
    );
}

#[test]
fn pull_fits_the_main_thread_budget() {
    let ctx = TestContext::new();
    source_tree(&ctx.module_dir);
    let dst = ctx.workspace.join("pulled");
    let mut argv = args(&["copy", "--yes"]);
    argv.extend([remote(&ctx, ""), slash(&dst)]);
    assert_exit(&run(Some(&ctx.config_dir), &argv), 0, "pull");
    assert_eq!(fs::read(dst.join("big.bin")).unwrap(), vec![7u8; 3_000_000]);
}

#[test]
fn pull_mirror_fits_the_main_thread_budget() {
    let ctx = TestContext::new();
    source_tree(&ctx.module_dir);
    let dst = ctx.workspace.join("pulled");
    fs::create_dir_all(&dst).unwrap();
    fs::write(dst.join("stale.txt"), b"extraneous").unwrap();
    let mut argv = args(&["mirror", "--yes"]);
    argv.extend([remote(&ctx, ""), slash(&dst)]);
    assert_exit(&run(Some(&ctx.config_dir), &argv), 0, "pull mirror");
    assert!(!dst.join("stale.txt").exists(), "the mirror deleted");
}

/// A pull whose one blocked file fails on the main pass and on the retry
/// pass — the destination route with a second session behind it.
#[test]
fn pull_with_a_retry_pass_fits_the_main_thread_budget() {
    let ctx = TestContext::new();
    source_tree(&ctx.module_dir);
    let dst = ctx.workspace.join("pulled");
    fs::create_dir_all(dst.join("d0/f0.txt")).unwrap();
    let mut argv = args(&[
        "copy",
        "--yes",
        "--retries",
        "1",
        "--diagnostics-no-retry-wait",
    ]);
    argv.extend([remote(&ctx, ""), slash(&dst)]);
    let output = run(Some(&ctx.config_dir), &argv);
    assert_exit(&output, EXIT_PARTIAL_FAILURE, "pull + retry");
    assert!(
        text(&output.stderr).contains("retrying 1 file(s)"),
        "the retry pass ran\nstderr:\n{}",
        text(&output.stderr)
    );
}
