# d-22-f2-cancel-selected: K cancels selected transfer

**Severity**: Feature (the payoff for d-21's cursor work)
**Status**: In progress / pending review
**Branch**: `phase5/a1`
**Commit**: filled by the sentinel commit

## What

Pressing `K` (kill) on F2 cancels the cursor-selected
active transfer via the daemon's `CancelJob` RPC. The
footer surfaces the lifecycle:

```
live · cancelling abc-123...           ← Sending
live · cancelled abc-123               ← Done(Cancelled)
live · cancel: id abc-123 not found    ← Done(NotFound)
live · cancel unsupported for abc-123: …  ← Done(Unsupported)
live · cancel abc-123 failed: …        ← Error (transport)
```

Pre-d-22 the only way to cancel a transfer mid-flight
was `blit jobs cancel <id>` from a separate shell. The
operator watching F2 can now act directly.

## Approach

### Selection contract

Builds on d-21 R2's id-anchored cursor.
`TransfersState::selected_active_id()` returns
`Option<&str>` — `Some` only when the cursor is
anchored on a live row. Returns `None` if the cursor
isn't set or if the previously selected transfer has
terminated (id no longer in `active`). The dispatcher
checks this before firing the RPC.

### Status machine

New `F2CancelStatus` enum on `AppState`:

```rust
enum F2CancelStatus {
    Idle,
    Sending { transfer_id, request_id },
    Done { outcome: CancelJobOutcome },
    Error { transfer_id, message },
}
```

Generation guard via `cancel_request_seq: u64` on
`AppState`. Each dispatch bumps the seq + stamps the
new value into `Sending`. The reply arm compares the
reply's `request_id` to the current `Sending` gen —
stale replies drop. (Operator can't fire a second `K`
while `is_sending()` is true; this guard is for the
case where the first cancel completes after the
operator does something else, then they K-cancel a
different transfer.)

### Dispatch

`UserAction::CancelSelectedTransfer` mapped to `K`.
F2 dispatch:

```rust
UserAction::CancelSelectedTransfer => {
    if app.cancel_status.is_sending() {
        // already in flight — ignore
    } else if let (Some(id), Some(endpoint)) =
        (app.transfers.selected_active_id()..., app.parsed_remote.clone())
    {
        app.cancel_request_seq += 1;
        app.cancel_status = F2CancelStatus::Sending { ... };
        spawn_cancel_transfer(rid, endpoint, id, tx);
    }
}
```

Silently no-ops when: no remote configured, cursor
not anchored, or a cancel is already in flight.

### Spawn helper

```rust
fn spawn_cancel_transfer(request_id, endpoint, transfer_id, tx)
```

Thin wrapper around `blit_app::admin::jobs::cancel`.
The outcome (Cancelled / NotFound / Unsupported) lands
in the reply; transport failures land as `Err(String)`.

### Render

New `F2CancelDisplay` enum in `screens/f2.rs` — the
renderer-facing snapshot so the screens layer doesn't
reach into main.rs's types. `cancel_status_to_display`
in main.rs bridges. Footer renders a fragment
(yellow for Sending, green for Cancelled, red for the
failure variants).

### Help overlay

New row: `K  cancel selected transfer (F2)`. Section
renamed `F1 · F2 · F3 navigation` to reflect F2 now
having a cursor too. Modal height 32 → 34 to fit.

## Files changed

- `crates/blit-tui/src/state.rs`:
  - `selected_active_id()` public accessor (deferred
    from d-21 R2).
- `crates/blit-tui/src/main.rs`:
  - `F2CancelStatus` enum + `CancelReply` struct.
  - AppState gains `cancel_status` /
    `cancel_reply_tx` / `cancel_request_seq`.
  - `UserAction::CancelSelectedTransfer` + `K`
    keymap.
  - F2 dispatch arm.
  - `spawn_cancel_transfer` helper.
  - `cancel_status_to_display` bridge.
  - select! arm applies the reply with gen guard.
- `crates/blit-tui/src/screens/f2.rs`:
  - `F2CancelDisplay` enum.
  - `render_into` + `render_footer` gain the
    `cancel: &F2CancelDisplay` parameter.
  - Footer renders the cancel fragment + adds a
    `K cancel selected` hint.
- `crates/blit-tui/src/help.rs`:
  - F1·F2·F3 nav section gains a `K` row.
  - Modal height 32 → 34.

## Tests

+1 test (242 → 243):

In `main::tests`:
- `key_action_maps_cancel_selected_transfer` — pins
  `K` → `CancelSelectedTransfer`.
- `key_action_returns_none_for_unmapped_keys` updated
  to drop the stale "K is unmapped" assertion (with
  a comment explaining the d-22 takeover).

The status-machine + dispatch + render path is
exercised manually; a full end-to-end test would need
a fake daemon serving CancelJob, which is out of
scope for this slice.

`cargo fmt`, `cargo clippy --workspace --all-targets
-- -D warnings`, and `cargo test --workspace` all green.

## Known gaps

1. **No confirmation prompt.** `K` fires immediately.
   Cancel is reversible (the daemon just stops sending
   bytes; data already on disk stays), so a confirm
   prompt would be friction for marginal safety gain.
   A future polish could add an opt-in
   `[transfer] confirm_cancel` config field.

2. **No batch cancel.** Operator with 10 active
   transfers and a need to abort all of them presses
   `K` 10 times. A future polish could add `Shift+K`
   for cancel-all-active.

3. **Footer fragment doesn't auto-clear.** A Done
   /Error status persists until the operator triggers
   another action. Acceptable — they see the outcome
   stayed visible — but a future polish could
   auto-revert to Idle after N seconds.

## Out of scope (next slices)

- **Cancel confirmation prompt.**
- **Batch cancel (Shift-K).**
- **Auto-clear cancel-fragment timeout.**

## Reviewer comments

(empty — pending grade)
