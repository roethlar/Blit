# c-4-transfer-progress reopened

Verdict: Reopened
Reviewed sha: `69224e086beb20f0e5175c9e97c33cb6a36921a9`
Reviewer: `reviewer`
Timestamp: `2026-05-17T14:19:51Z`

Validation:

- `cargo fmt --all -- --check`: pass
- `cargo clippy --workspace --all-targets -- -D warnings`: pass
- `cargo test --workspace`: pass

## Findings

1. Medium - A stale `TransferProgress` can be broadcast after `TransferComplete` / `TransferError` for the same transfer.

   `tick_progress_once` snapshots active rows at `crates/blit-daemon/src/service/core.rs:208` via `ActiveJobs::snapshot_progress_samples()`, then broadcasts the returned samples at `core.rs:210-224`. The snapshot function holds the active-jobs table lock only while building the vector (`crates/blit-daemon/src/active_jobs.rs:469-496`); the lock is released before any progress event is sent.

   That leaves a race with the terminal-event ordering from c-3. A transfer task can drop the `ActiveJobGuard` and broadcast its terminal event after the progress snapshot is taken but before the ticker sends the progress event. For example, `push` builds the terminal event, drops the active row, drops the metrics guard, then sends at `core.rs:407-415`; `pull` and `pull_sync` send after row drain at `core.rs:465-468` and `core.rs:527-530`; delegated pull sends after row drain / error-counter update at `core.rs:701-721`.

   In that interleaving a subscriber can observe `TransferComplete` or `TransferError`, reconcile via `GetState` and see the transfer moved to `recent[]`, then receive a later `TransferProgress` for the already-finished transfer. That violates the lifecycle stream contract: terminal events must be final for a transfer id.

   Make progress emission atomic with row liveness. One reasonable shape is to perform the sample-and-send operation while holding the active-jobs table lock, since `broadcast::Sender::send` is synchronous and does not await; if the guard drops first, the ticker sees no row, and if the ticker samples first, the terminal send waits behind row removal and therefore follows the progress frame. Add a regression test that forces the snapshot/send gap or otherwise proves a terminal event cannot overtake a progress frame for the same transfer id.
