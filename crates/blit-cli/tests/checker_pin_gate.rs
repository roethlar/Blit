//! The diagnostic checker pin is refused on remote routes, refused BEFORE
//! anything reaches the operator, and the adaptive default is not refused.
//!
//! # Fixture design, which is most of the work here
//!
//! Every case uses a **missing local source paired with a remote
//! destination**. That shape is doing three jobs at once:
//!
//! - It selects a genuinely remote route (`LocalToRemote`), so the gate is
//!   actually exercised. cr-ls1-15: a local→local pair proves nothing,
//!   because local routes are exempt from the gate by design — the reviewer
//!   added a dispatch-only rejection of `checkers=0` on remote routes and
//!   every test here stayed green.
//! - It cannot reach the network under ANY behaviour of the gate. The
//!   source-existence check sits in the dispatch arm, after the gate, so a
//!   removed gate fails on the missing source rather than connecting.
//!   cr-ls1-16: the previous fixture leaned on `blit.invalid` never
//!   resolving, but a hosts-file entry can defeat RFC 6761, and it wrote to
//!   a fixed `/tmp` path — if the gate were removed, that case could have
//!   contacted a live daemon and populated a non-temporary destination.
//! - It gives the adaptive case an unambiguous post-gate signal: reaching
//!   "source path does not exist" proves the run got PAST the gate on a
//!   remote route, which is exactly the property being claimed.
//!
//! Ordering is only observable at the process boundary, so it is asserted
//! here against the real binary's real output (cr-ls1-13).

use std::process::Command;
use std::time::Duration;

mod common;
use common::{cli_bin, run_with_timeout};

/// A local path guaranteed not to exist, inside a temp dir that is removed
/// when the guard drops.
fn missing_local_source() -> (tempfile::TempDir, String) {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("does-not-exist");
    let display = format!("{}/", path.display());
    (tmp, display)
}

/// Deliberately NOT passing `--yes`: if the gate ever moves back below the
/// destructive confirmation, mirror and move block reading stdin and the
/// timeout fails the test. It cannot silently pass either way.
fn run_gated(subcommand: &str, extra: &[&str]) -> (tempfile::TempDir, std::process::Output) {
    let (tmp, source) = missing_local_source();
    let mut cmd = Command::new(cli_bin());
    cmd.arg(subcommand);
    for arg in extra {
        cmd.arg(arg);
    }
    cmd.arg(&source)
        .arg("blit-test-host:/dst/")
        .stdin(std::process::Stdio::null());
    let output = run_with_timeout(cmd, Duration::from_secs(30));
    (tmp, output)
}

#[test]
fn a_remote_checker_pin_is_refused_before_any_banner_or_prompt() {
    for subcommand in ["copy", "mirror", "move"] {
        let (_tmp, output) = run_gated(subcommand, &["--checkers", "4"]);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let combined = format!("{stdout}{stderr}");

        assert!(
            !output.status.success(),
            "`blit {subcommand} --checkers 4` to a remote endpoint must \
             fail\nstdout:\n{stdout}\nstderr:\n{stderr}"
        );
        assert!(
            combined.contains("--checkers"),
            "the refusal must name the flag\nstdout:\n{stdout}\nstderr:\n{stderr}"
        );
        // Proves the gate beat the dispatch: the missing source is only
        // reported from inside the route arm, below the gate.
        assert!(
            !combined.contains("source path does not exist"),
            "`{subcommand}` got past the gate into the dispatch\nstdout:\n{stdout}\nstderr:\n{stderr}"
        );

        // THE ORDERING ASSERTIONS. Moving the gate back below the banner or
        // the confirmation reintroduces these strings.
        assert!(
            !combined.contains(&format!("starting {subcommand}")),
            "`{subcommand}` printed its startup banner before refusing; route \
             validation must run first\nstdout:\n{stdout}\nstderr:\n{stderr}"
        );
        assert!(
            !combined.contains("Continue?"),
            "`{subcommand}` asked the operator to confirm a destructive \
             operation that can never run\nstdout:\n{stdout}\nstderr:\n{stderr}"
        );
        assert!(
            !combined.contains("will delete"),
            "`{subcommand}` described a deletion it was never going to \
             perform\nstdout:\n{stdout}\nstderr:\n{stderr}"
        );
    }
}

/// The adaptive default must reach the REMOTE dispatch, not be swept up by
/// the same gate — otherwise the check would be "reject remote transfers"
/// rather than "reject a pin that cannot take effect".
///
/// cr-ls1-15: asserted on the post-gate error, on a remote route. A
/// local→local pair cannot show this, because local routes never reach the
/// gate at all.
#[test]
fn the_adaptive_default_reaches_the_remote_dispatch() {
    // `copy` has no destructive confirmation, so the run proceeds straight
    // from the gate into the route arm with stdin closed.
    let (_tmp, output) = run_gated("copy", &[]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{stdout}{stderr}");

    assert!(
        !combined.contains("--checkers"),
        "the adaptive default must never be refused by the pin \
         gate\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
    assert!(
        combined.contains("source path does not exist"),
        "the run must reach the remote dispatch arm — that is what proves it \
         passed the gate on a remote route\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
}
