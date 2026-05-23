Reviewed sha: `9531dde005df0cdd7fb8182a96f1359013d89e55`

# Reopened: d-68-f1-remote-remote-copy

## Finding

**Medium** - Windows forward-slash local destinations are still rejected before remote-to-local pull can run.

Round 3 fixes the bare-relative destination regression, but the same up-front destination parse still rejects Windows-style local absolute paths that use forward slashes. For a remote-source pull, `nas:/photos/ -> C:/tmp/out` should remain a local destination. `RemoteEndpoint::parse` explicitly classifies both `C:\path` and `C:/path` as local paths, but `parse_transfer_endpoint` then converts the `C:/path` error back into `Err` because the raw string contains `:/`. The new `plan_f1_trigger` branch treats any `Err` as "invalid destination", so the pull never reaches `app.f3_pull.start_pull(...)`.

This is a d-68 regression for remote-to-local triggers: before this slice, the remote-source branch passed the raw destination string directly to the F3 pull machine, so a Windows `C:/...` local destination was not rejected by the transfer endpoint parser.

Relevant code:

- `crates/blit-tui/src/main.rs:3540` parses every remote-source destination through `parse_transfer_endpoint`.
- `crates/blit-tui/src/main.rs:3555` rejects every parse error as an invalid destination.
- `crates/blit-app/src/endpoints.rs:62` turns any failed parse containing `:/` into `Err`.
- `crates/blit-core/src/remote/endpoint.rs:255` shows `C:/path` is intended to be recognized as local, not as a malformed remote.

The fix needs either a classifier that can distinguish "remote-shaped typo" from local Windows drive paths, or a transfer endpoint parser fix that preserves the lower-level Windows-drive local classification. Please add a regression test for `plan_f1_trigger("nas:/photos/", "C:/tmp/out", Copy, false)` launching the remote-to-local pull path.

## Gates

- `cargo fmt --all -- --check` passed.
- `cargo clippy --workspace --all-targets -- -D warnings` passed.
- `cargo test --workspace` passed: 565 tests.
