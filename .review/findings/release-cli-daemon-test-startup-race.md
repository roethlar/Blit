# release-cli-daemon-test-startup-race — make CLI integration startup reliable

**Severity**: HIGH — the required workspace gate can fail because CLI tests
connect before their temporary daemon is ready, so a release candidate cannot
be validated reliably.
**Status**: Fixed locally
**Branch**: `master`
**Commit**: This rel-1b implementation commit

## Evidence

The rel-1 full `cargo test --workspace` run failed
`test_utils_df` and `test_utils_list_modules_json` in
`crates/blit-cli/tests/blit_utils.rs`. `test_utils_df` reported connection
refused to its allocated loopback port; the JSON test returned unsuccessful
without a product assertion failure. Both exact isolated reruns passed
immediately.

The shared fixture's readiness proof was only a successful TCP connection.
`pick_unused_port` necessarily drops its probe listener before the daemon
binds. The claimed-port set narrows collisions inside one test process, but it
does not reserve the kernel port, and the TCP readiness check did not establish
that the response came from the child the fixture had spawned. Its child-death
check ran before each TCP probe, so it could accept another listener and return
before the losing child reported its bind failure.

The failed run did not retain daemon stderr, so it cannot prove which external
listener or exit status occupied either port retrospectively. That missing
diagnostic does not change the concrete fixture defect: it declared readiness
without an application response or process identity.

## Predicted observable failure

Parallel or loaded workspace runs can accept a foreign listener as ready, then
attempt the CLI connection after that listener or the intended child has gone,
producing nondeterministic connection-refused failures.

## What

`spawn_daemon` now keeps the child-liveness check and a bounded gRPC identity
probe in one readiness loop. The probe calls `ListModules` and accepts readiness
only when the response contains the canonical path of that fixture's unique
temporary module root. A foreign daemon cannot satisfy that identity.

`daemon_readiness_waits_for_the_owned_module_identity` feeds a foreign module
path before the owned path and proves the wait cannot return on the first
response. Mutation proof replaced the path match with "any module exists"; the
guard failed at one probe instead of two, then passed after restoration.

Validation on macOS: the focused guard passed; all 23 `blit_utils` tests passed
twice in parallel (focused integration run and the workspace run). The complete
`cargo fmt`, strict workspace clippy, workspace test, and docs gates passed.

## Known gaps

No hosted Windows/Linux run has tested this local head; publication remains an
owner-gated release step.
