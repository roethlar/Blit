# a1-2-f2-transfers reopened

Reviewed sha: `da7646edd601b2ae88bda1ecdf04e3927f1b38c0`
Reviewed at: 2026-05-18T03:56:43Z
Reviewer: reviewer
Verdict: reopened

## Findings

### 1. Startup can miss transfers between GetState and Subscribe

Severity: Medium

Location: `crates/blit-tui/src/main.rs:241`

Round 2 fixes the detached-input-task issue and the idle `Connecting` status, but the live state still has a startup race: `run_event_loop` awaits the initial `jobs::query(&endpoint, 0)` before creating the Subscribe receiver with `spawn_subscribe_forwarder(...)`. Any transfer that starts after the snapshot is taken but before the Subscribe stream is registered is not present in the initial `active[]`/`recent[]` state and its `TransferStarted` event is missed.

That gap breaks the F2 pane in common cases. If the transfer remains active after the stream connects, later `TransferProgress` events are discarded because `TransfersState::apply_event` returns false for unknown ids. If it completes, the `TransferComplete` event can only produce a recent row with default/blank kind, peer, module, and path because the active row was never known.

Please make the snapshot/stream handshake race-safe before verifying this as a live transfers pane. A reasonable shape is to open Subscribe first, buffer events while taking the snapshot, then merge them into the snapshot with idempotent recent-row handling. The important contract is that a transfer starting during TUI startup must either appear as active or end up as a complete recent row with correct metadata, not disappear or render as a blank terminal event.

## Validation

- `cargo fmt --all -- --check` passed.
- `cargo clippy --workspace --all-targets -- -D warnings` passed.
- `cargo test -p blit-tui` passed.
- `cargo test --workspace` passed.
