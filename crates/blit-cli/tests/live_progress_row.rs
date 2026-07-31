//! The `-p` live row's redirected-stderr posture (clp-2 residue c).
//!
//! A test harness — like a pipe, a `tee`, or a log file — gives the
//! process a non-terminal stderr, and indicatif hides the bar there. The
//! row must NOT attach its progress sink in that case: the sink is also
//! what gates blit-core's enumeration heartbeat, so attaching it would
//! leave the redirected stream with no row (nothing can be drawn) and no
//! liveness line either. This binary asserts the liveness survives.

use std::fs;
use std::process::Command;
use std::time::Duration;

mod common;
use common::{cli_bin, run_with_timeout};

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

/// The sink-less baseline the above must match: `-p` changes nothing
/// about a redirected stream.
#[test]
fn plain_redirected_run_keeps_the_enumeration_liveness() {
    let output = copy_with_progress(&[]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains(ENUMERATION_LIVENESS),
        "the sink-less run's stderr is unchanged:\n{stderr}"
    );
}
