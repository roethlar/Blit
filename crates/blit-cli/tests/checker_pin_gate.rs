//! The diagnostic checker pin is refused on remote routes, and refused
//! BEFORE anything reaches the operator.
//!
//! cr-ls1-13: the previous guard for this asserted only on the returned
//! error, so moving both validations back after the startup banner and the
//! destructive prompt left it green — the reviewer demonstrated exactly that.
//! Ordering is only observable at the process boundary, so it is asserted
//! here, against the real binary's real output.
//!
//! No network is involved: the gate fires during argument/route validation,
//! before any connection is attempted. That is the property under test.

use std::process::Command;
use std::time::Duration;

mod common;
use common::{cli_bin, run_with_timeout};

/// Deliberately NOT passing `--yes`: if the gate ever moves back below the
/// destructive confirmation, mirror and move would block reading stdin and
/// the timeout would fail the test. Either way it cannot silently pass.
fn run_gated(subcommand: &str, source: &str, destination: &str) -> std::process::Output {
    let mut cmd = Command::new(cli_bin());
    cmd.arg(subcommand)
        .arg("--checkers")
        .arg("4")
        .arg(source)
        .arg(destination)
        .stdin(std::process::Stdio::null());
    run_with_timeout(cmd, Duration::from_secs(30))
}

#[test]
fn a_remote_checker_pin_is_refused_before_any_banner_or_prompt() {
    // `blit.invalid` is reserved by RFC 6761 and can never resolve, so even a
    // total failure of the gate cannot contact a real host. The gate must
    // fire long before that matters.
    for (subcommand, source, destination) in [
        ("copy", "blit.invalid:/src/", "/tmp/blit-gate-dst/"),
        ("mirror", "blit.invalid:/src/", "/tmp/blit-gate-dst/"),
        ("move", "blit.invalid:/src/", "/tmp/blit-gate-dst/"),
    ] {
        let output = run_gated(subcommand, source, destination);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let combined = format!("{stdout}{stderr}");

        assert!(
            !output.status.success(),
            "`blit {subcommand} --checkers 4` against a remote endpoint must \
             fail\nstdout:\n{stdout}\nstderr:\n{stderr}"
        );
        assert!(
            combined.contains("--checkers"),
            "the refusal must name the flag\nstdout:\n{stdout}\nstderr:\n{stderr}"
        );

        // THE ORDERING ASSERTIONS. Moving the gate back below the banner or
        // the confirmation reintroduces these strings.
        assert!(
            !combined.contains(&format!("starting {subcommand}")),
            "`{subcommand}` printed its startup banner before refusing; \
             route validation must run first\nstdout:\n{stdout}\nstderr:\n{stderr}"
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

/// The adaptive default must not be caught by the same gate — otherwise the
/// check would be "reject remote transfers", not "reject a pin that cannot
/// take effect". Uses a local pair so the run completes without a network.
#[test]
fn the_adaptive_default_is_not_refused() {
    let tmp = tempfile::tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    std::fs::create_dir_all(&src).unwrap();
    std::fs::write(src.join("f.txt"), b"payload").unwrap();

    let mut cmd = Command::new(cli_bin());
    cmd.arg("copy")
        .arg("--yes")
        .arg(format!("{}/", src.display()))
        .arg(format!("{}/", dst.display()))
        .stdin(std::process::Stdio::null());
    let output = run_with_timeout(cmd, Duration::from_secs(30));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "a default (adaptive) run must not be refused\nstderr:\n{stderr}"
    );
    assert!(
        !stderr.contains("--checkers"),
        "the adaptive default must never mention the pin\nstderr:\n{stderr}"
    );
    assert_eq!(std::fs::read(dst.join("f.txt")).unwrap(), b"payload");
}
