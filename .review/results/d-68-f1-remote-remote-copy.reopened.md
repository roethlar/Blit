Reviewed sha: `1dcdc66be6bd42645fef1dac49a845ee0f29a8d1`

# Reopened: d-68-f1-remote-remote-copy

## Finding

**Medium** - malformed remote-looking destinations still mis-route as local pull paths.

`plan_f1_trigger` only delegates when `parse_transfer_endpoint(dest)` returns `Ok(Endpoint::Remote(_))`. If parsing fails, the error is ignored and the remote-source copy path continues into `app.f3_pull.start_pull(source, dest.to_string())`.

That leaves the same class of silent mis-route for common invalid remote inputs. Example: `nas:/photos/ -> skippy:/backup` is likely intended as remote-to-remote, but `RemoteEndpoint::parse` rejects it because module paths need a slash after the module (`skippy:/backup/` or `skippy:/backup/path`). `parse_transfer_endpoint` deliberately propagates that remote-shaped error, but this branch drops it and launches a remote-to-local pull into a local path named `skippy:/backup`.

Relevant code:

- `crates/blit-tui/src/main.rs:3535` ignores parse errors while detecting remote-to-remote.
- `crates/blit-tui/src/main.rs:3544` launches the remote-to-local pull with the raw destination after that ignored error.
- `crates/blit-app/src/endpoints.rs:58` documents the intended behavior: remote-shaped typos must error instead of becoming local paths.

The fix should parse the destination once for a remote source and distinguish all outcomes: `Ok(Remote)` delegates, `Ok(Local)` remains remote-to-local, and `Err(_)` rejects with an invalid-destination message instead of falling through.

## Gates

- `cargo fmt --all -- --check` passed.
- `cargo clippy --workspace --all-targets -- -D warnings` passed.
- `cargo test --workspace` passed: 562 tests.
