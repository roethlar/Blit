Reviewed sha: `96075cb9b6543b252ba771488cb0f264c7bb10c5`

# Reopened: d-68-f1-remote-remote-copy

## Finding

**Medium** - remote-source pulls to relative local destinations are now rejected as delegated destinations.

Round 2 fixed malformed remote-looking destinations, but the destination classifier is now too broad for the existing remote-to-local path. With a remote source, `plan_f1_trigger` calls `parse_transfer_endpoint(dest)` and delegates any `Ok(Endpoint::Remote(_))` result. A bare relative local destination like `backup` parses as `RemotePath::Discovery` because `RemoteEndpoint::parse` treats bare `server` / `server:port` as discovery endpoints. That means `nas:/photos/ -> backup` no longer falls through to `app.f3_pull.start_pull(...)`; it enters `plan_f1_delegated` and is rejected with "destination needs a module".

This regresses the pre-d-68 remote-to-local trigger behavior: relative local destinations are valid for `F3PullState::start_pull`; it only rejects blank destinations and then resolves the raw local path.

Relevant code:

- `crates/blit-tui/src/main.rs:3540` parses every remote-source destination through `parse_transfer_endpoint`.
- `crates/blit-tui/src/main.rs:3541` treats every parsed remote endpoint as delegated, including `RemotePath::Discovery`.
- `crates/blit-tui/src/main.rs:3648` then rejects discovery destinations as missing a module.
- `crates/blit-core/src/remote/endpoint.rs:90` shows why a bare `backup` parses as discovery.
- `crates/blit-tui/src/f3pull.rs:405` shows the remote-to-local pull path only requires a nonblank destination.

The fix needs to distinguish "valid remote transfer destination" from "bare discovery endpoint". For remote-source triggers, a destination should only take the delegated path when it is a remote module/root destination (`host:/module/...` or `host://...`). A bare discovery parse should not steal ordinary relative local destinations from the remote-to-local pull path.

## Gates

- `cargo fmt --all -- --check` passed.
- `cargo clippy --workspace --all-targets -- -D warnings` passed.
- `cargo test --workspace` passed: 564 tests.
