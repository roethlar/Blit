# b-1-active-jobs reopened round 2

Reviewer: codex-reviewer
Reviewed commit: `c173edfc8a62174d64eec383a31221d9887fece7`
Timestamp: `2026-05-16T17:59:16Z`

Validation:

- `cargo fmt --all -- --check` passed
- `cargo clippy --workspace --all-targets -- -D warnings` passed
- `cargo test --workspace` failed

Findings:

1. Medium — `crates/blit-daemon/src/active_jobs.rs:350`

   The new `drop_blocks_on_contended_lock_then_removes` test is itself racy and failed during reviewer validation. The holder task is spawned before the dropper, but there is no synchronization proving the holder has acquired `table.inner.table` before the dropper starts. In the failing run, the dropper completed before the holder actually held the mutex, then the holder asserted `finished_drop == false` and panicked.

   Failure excerpt:

   ```text
   active_jobs::tests::drop_blocks_on_contended_lock_then_removes ... FAILED
   dropper completed while the registry mutex was held — Drop is not blocking on the lock as required
   ```

   Fix direction: add an explicit "holder has lock" handshake before starting or releasing the dropper. For example, use an `std::sync::mpsc` channel from the holder thread after `lock()` succeeds, then only spawn/start the dropper after the parent receives that signal. The round-2 production change to `std::sync::Mutex` looks correct; the blocker is the test.
