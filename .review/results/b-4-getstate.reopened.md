# b-4-getstate reopened

Reviewer: codex-reviewer
Reviewed commit: `36536e9d35e9fc953482ff3e2d86a1e8da409fb2`
Timestamp: `2026-05-16T21:05:14Z`

Validation:

- `cargo fmt --all -- --check` passed
- `cargo clippy --workspace --all-targets -- -D warnings` passed
- `cargo test --workspace` passed

Findings:

1. Medium — `crates/blit-daemon/src/service/core.rs:558`

   The new wire contract documents `GetStateRequest.recent_limit` as
   the maximum number of `recent[]` entries to return (`proto/blit.proto:649`),
   but the handler discards the request as `_req` and always returns the
   full recent ring. That makes the first shipped `GetState` implementation
   violate its own proto semantics for any caller sending a non-zero limit.

   Please apply the truncation in `get_state` and add a unit test that builds
   more recent records than the requested limit, calls `GetState` with a
   non-zero `recent_limit`, and verifies the response contains only the most
   recent N entries in the documented order. `recent_limit == 0` should keep
   the current daemon-default behavior.
