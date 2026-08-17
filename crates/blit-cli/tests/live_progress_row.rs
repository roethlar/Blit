//! The `-p` live row's redirected-stderr posture (clp-2 residue c).
//!
//! A test harness — like a pipe, a `tee`, or a log file — gives the
//! process a non-terminal stderr, and indicatif hides the bar there. The
//! row must NOT attach its progress sink in that case: the sink is also
//! what gates blit-core's enumeration heartbeat, so attaching it would
//! leave the redirected stream with no row (nothing can be drawn) and no
//! liveness line either. This binary asserts the liveness survives.
//!
//! It also owns audit-16's gate end-to-end — the whole truth about when
//! that heartbeat speaks lives here, on BOTH routes (cr-a16-1): quiet by
//! default, audible under `--verbose`, audible under `-p`.

use std::fs;
use std::process::Command;
use std::time::Duration;

mod common;
use common::{cli_bin, run_with_timeout, TestContext};

/// The line blit-core prints when a manifest scan ends with no progress
/// sink attached. Its presence is the observable proof the sink stayed
/// off; with a sink the enumeration reports as events instead.
const ENUMERATION_LIVENESS: &str = "Manifest enumeration complete";

fn copy_with_progress(extra: &[&str]) -> std::process::Output {
    let tmp = tempfile::tempdir().expect("tempdir");
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).expect("mkdir src");
    fs::write(src.join("a.txt"), b"first").expect("write a");
    fs::write(src.join("b.txt"), b"second").expect("write b");

    let mut cmd = Command::new(cli_bin());
    cmd.arg("copy").arg("--yes");
    for arg in extra {
        cmd.arg(arg);
    }
    cmd.arg(format!("{}/", src.display()))
        .arg(format!("{}/", dst.display()));
    let output = run_with_timeout(cmd, Duration::from_secs(30));
    assert!(
        output.status.success(),
        "blit copy failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        fs::read(dst.join("b.txt")).expect("the transfer landed"),
        b"second"
    );
    output
}

/// `-p` with stderr redirected: no row can be drawn, so the legacy
/// enumeration lines must still reach the stream.
#[test]
fn progress_with_redirected_stderr_keeps_the_enumeration_liveness() {
    let output = copy_with_progress(&["-p"]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains(ENUMERATION_LIVENESS),
        "a hidden row must not swallow the enumeration liveness:\n{stderr}"
    );
}

/// audit-16 guard: the true default (no `-p`, no `--verbose`) must stay
/// quiet. Before the fix, a sink-less scan printed the legacy heartbeat
/// lines unconditionally regardless of verbosity.
#[test]
fn plain_redirected_run_stays_quiet_without_verbose() {
    let output = copy_with_progress(&[]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains(ENUMERATION_LIVENESS),
        "a plain run with neither -p nor --verbose must stay quiet:\n{stderr}"
    );
}

/// `--verbose` (no `-p`) restores the sink-less heartbeat end-to-end
/// through the real CLI flag — the audit-16 opt-in this binary's other
/// two cases (progress-requested, and the true default above) bracket.
#[test]
fn verbose_redirected_run_keeps_the_enumeration_liveness() {
    let output = copy_with_progress(&["--verbose"]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains(ENUMERATION_LIVENESS),
        "--verbose must restore the sink-less enumeration liveness:\n{stderr}"
    );
}

/// cr-a16-1: the same `-p` posture on the REMOTE PUSH route.
///
/// audit-16 preserved clp-2's `verbose || progress` fallback in
/// `run_local_session` but threaded only `execution.verbose` into
/// `run_remote_push`'s `FsTransferSource`, so a push with `-p` and no
/// `-v` was silent through source enumeration — the documented "(or
/// `-p`)" contract holding on one route and not the other. The CLI does
/// build a progress handle for `-p`, but that handle rides the SESSION's
/// progress lane; the SOURCE end has no sink of its own (attaching one is
/// CLI_LIVE_PROGRESS residue (d), post-1.0), so the raw heartbeat is the
/// only liveness a slow remote enumeration has.
///
/// Driven through the real binary against a real daemon because that is
/// where the CLI's `-p` decision, `PushExecution.progress`, and the source
/// construction all meet; a unit test on the expression would pin the fix
/// rather than the behaviour. Reverting `|| execution.progress` in
/// `blit_core::transfers::remote::run_remote_push` reds this and nothing
/// else — the two local-route tests above cannot see that route at all.
#[test]
fn remote_push_with_progress_keeps_the_enumeration_liveness() {
    let ctx = TestContext::new();
    let src = ctx.workspace.join("src");
    fs::create_dir_all(&src).expect("mkdir src");
    fs::write(src.join("a.txt"), b"first").expect("write a");
    fs::write(src.join("b.txt"), b"second").expect("write b");

    let mut cmd = Command::new(&ctx.cli_bin);
    cmd.arg("--config-dir")
        .arg(&ctx.config_dir)
        .arg("copy")
        .arg("--yes")
        .arg("-p")
        .arg(format!("{}/", src.display()))
        .arg(format!("127.0.0.1:{}:/test/", ctx.daemon_port));

    let output = run_with_timeout(cmd, Duration::from_secs(60));
    assert!(
        output.status.success(),
        "blit copy to the daemon failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        fs::read(ctx.module_dir.join("b.txt")).expect("the push must land"),
        b"second",
        "the fixture must actually transfer, or the liveness assertion is vacuous"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains(ENUMERATION_LIVENESS),
        "a remote push with -p (and no -v) must keep the enumeration liveness:\n{stderr}"
    );
}
