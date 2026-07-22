# release-cli-daemon-test-startup-race — make CLI integration startup reliable

**Severity**: HIGH — the required workspace gate can fail because CLI tests
connect before their temporary daemon is ready, so a release candidate cannot
be validated reliably.
**Status**: Open
**Branch**: `master`
**Commit**: Pending

## Evidence

The rel-1 full `cargo test --workspace` run failed
`test_utils_df` and `test_utils_list_modules_json` in
`crates/blit-cli/tests/blit_utils.rs`. `test_utils_df` reported connection
refused to its allocated loopback port; the JSON test returned unsuccessful
without a product assertion failure. Both exact isolated reruns passed
immediately.

The fail-then-pass shape is a startup/lifecycle race in the shared temporary
daemon fixture, not a deterministic command failure. STATE already recorded
historical `blit_utils` instability under load; the full release gate makes it
a current blocker.

## Predicted observable failure

Parallel or loaded workspace runs can attempt a CLI connection before the test
daemon is listening, producing nondeterministic red CI and hiding real product
failures among retries.

## What

Trace the fixture's port allocation, daemon spawn, readiness check, and teardown.
Replace timing assumptions with an owned process plus bounded positive readiness
proof, and add a deterministic guard that withholds readiness long enough to
prove clients cannot escape early.

## Known gaps

Root cause and fix are not yet implemented. No hosted run has tested this local
head.
