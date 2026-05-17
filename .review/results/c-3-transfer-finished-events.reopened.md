# c-3-transfer-finished-events reopened

Verdict: Reopened
Reviewed sha: `5be5f10142e4465aee5831ba5e69bc7ba7c70052`
Reviewer: `reviewer`
Timestamp: `2026-05-17T06:40:37Z`

Validation:

- `cargo fmt --all -- --check`: pass
- `cargo clippy --workspace --all-targets -- -D warnings`: pass
- `cargo test --workspace`: pass

## Findings

1. Medium - Terminal events are broadcast before the daemon state has actually moved the transfer out of active state.

   The new terminal events are sent while `ActiveJobGuard` and the metrics active-transfer guard are still alive. For example, `push` sends `build_transfer_finished_event(...)` at `crates/blit-daemon/src/service/core.rs:332`, then drops the active-job guard at `core.rs:333` and the metrics guard at `core.rs:339`. `pull` and `pull_sync` have the same shape at `core.rs:387` before `core.rs:388`/`core.rs:389`, and `core.rs:445` before `core.rs:446`/`core.rs:447`.

   `delegated_pull` also sends the terminal event before the active job is dropped and before handler-failure errors increment the metrics counter: event at `core.rs:610`, drop at `core.rs:615`, `inc_error()` at `core.rs:629`.

   A subscriber that receives `TransferComplete` or `TransferError` and immediately refreshes `GetState` can still see the transfer in `active[]`, missing from `recent[]`, with `counters.active_transfers` still including the finished RPC. For delegated handler failures, `transfer_errors_total` can also still be stale. That contradicts the terminal-event contract: the event is the signal that the transfer has ended and should be reconcilable with `GetState`.

   Build the `DaemonEvent` while the guard is alive, then drop the active-job guard and metrics guard / update error counters, and only then broadcast the already-built terminal event. Add coverage that a terminal event observed by a subscriber is ordered after the corresponding `GetState` active/recent transition, at least for one dispatch path.
