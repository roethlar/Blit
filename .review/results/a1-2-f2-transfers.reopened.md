# a1-2-f2-transfers reopened

Reviewed sha: `455ba2e6c47589cf4f5087735223b7f4d1bc5835`
Reviewed at: 2026-05-18T04:05:09Z
Reviewer: reviewer
Verdict: reopened

## Findings

### 1. Subscribe-first startup still is not causally ordered

Severity: Medium

Location: `crates/blit-tui/src/main.rs:251`

Round 3 starts the Subscribe forwarder task before `GetState`, but it does not wait for the Subscribe RPC to succeed or for the daemon-side broadcast receiver to be registered before taking the snapshot. `spawn_subscribe_forwarder(...)` returns immediately after `tokio::spawn`; the spawned task still has to connect and call `jobs::subscribe(...)`. Meanwhile `run_event_loop` immediately awaits `jobs::query(&endpoint, 0)`.

That means the original gap is still possible: `GetState` can complete before the Subscribe receiver is registered, so a transfer can still start after the snapshot but before events are actually being buffered. Please make the ordering explicit. For example, open the Subscribe stream in an awaited setup step, signal `Connected` only after the RPC succeeds, and only then issue the initial `GetState` while the already-registered receiver buffers events.

### 2. Buffered terminal events duplicate rows already present in snapshot recent[]

Severity: Medium

Location: `crates/blit-tui/src/state.rs:146`

The merge is only idempotent against `active[]`. If Subscribe is registered before `GetState`, a transfer can start and complete while the snapshot RPC is in flight. In that case the snapshot may already contain the transfer in `recent[]`, while the buffered stream still contains `TransferStarted` followed by `TransferComplete`.

With the current merge, the buffered `TransferStarted` sees no active row and inserts a new active entry; the buffered `TransferComplete` removes it and calls `push_recent`, producing a duplicate recent row for the same transfer id. The new regression test covers an empty snapshot, but not the more important "already in snapshot recent" case.

Please make `recent[]` idempotent by transfer id before replaying buffered startup events. Either ignore `TransferStarted` for ids already present in recent, or make `push_recent` replace/move an existing recent row instead of appending a duplicate. The contract should be one row per transfer id after snapshot plus buffered replay.

## Validation

- `cargo fmt --all -- --check` passed.
- `cargo clippy --workspace --all-targets -- -D warnings` passed.
- `cargo test -p blit-tui` passed.
- `cargo test --workspace` passed.
