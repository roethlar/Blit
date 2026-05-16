# b-1-active-jobs reopened

Reviewer: codex-reviewer
Reviewed commit: `a842a00485d2e55a0c0c1b2d733214bf74ea2491`
Timestamp: `2026-05-16T17:51:41Z`

Validation:

- `cargo fmt --all -- --check` passed
- `cargo clippy --workspace --all-targets -- -D warnings` passed
- `cargo test --workspace` passed

Findings:

1. Medium — `crates/blit-daemon/src/active_jobs.rs:205`

   `ActiveJobGuard::drop` removes the row inline only when `try_lock()` succeeds. If the table is contended, it spawns an unawaited cleanup task. That means a completed/cancelled transfer can remain visible to the next `snapshot()` after the guard has already dropped, and during runtime shutdown the spawned cleanup may never run. This weakens the advertised RAII contract ("Drop removes the row on every termination path") and will matter once `GetState.active[]` starts reading the table.

   The table operations are purely in-memory and short. Prefer a synchronous removal path, for example by using a standard mutex for the registry, or otherwise make the eventual-removal semantics explicit and covered by a test that forces the contended path. The desired contract for `active[]` should be deterministic: after the guard is dropped, the row is gone.

2. Low — `crates/blit-daemon/src/active_jobs.rs:299`

   `transfer_ids_unique_under_concurrent_registers` relies on fixed sleeps (`20ms`, `5ms`, `20ms`) to prove all spawned tasks have registered and later drained. That can flake on a loaded CI machine or under sanitizer/instrumented runs. Use explicit synchronization instead: have each task signal after registration, wait for all signals before asserting the live snapshot count, then join all tasks and wait for deterministic cleanup.
