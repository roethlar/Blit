# d-30-batch-cancel: F2 batch cancel via Shift+X

**Severity**: Feature (polish — closes d-22 known gap #2)
**Status**: In progress / pending review
**Branch**: `phase5/a1`
**Commit**: filled by the sentinel commit

## What

d-22 known gap #2 called out:

> **No batch cancel.** Operator with 10 active
> transfers and a need to abort all of them presses
> `K` 10 times. A future polish could add `Shift+K`
> for cancel-all-active.

d-30 lands the polish, using `Shift+X` (capital `X`)
since crossterm can't disambiguate `K` from `Shift+K`
(both yield `Char('K')`). Mnemonic: cross out
everything.

```
live · cancel 5 transfers? y/N         ← if confirm_cancel
live · sent 5 cancel requests          ← after y or directly fired
                                          (auto-hides on d-23 TTL)
```

`[transfer] confirm_cancel = true` gates `Shift+X` the
same way as single-cancel `K` — operators get a single
y/N prompt for the whole batch.

## Approach

### State

Two new `F2CancelStatus` variants:

```rust
ConfirmingBatch { count: usize },
BatchInitiated { count: usize, finished_at: Instant },
```

`is_confirming()` extended to cover `ConfirmingBatch`,
so Esc routing and dispatch gating work identically to
single-cancel.

`BatchInitiated` has a `finished_at` and rides the d-23
TTL infrastructure — same auto-hide as the per-transfer
Done variant. `cancel_status_remaining_ttl` recognizes
it; the loop's sleep budget tightens to the TTL
deadline just like single-cancel.

### Dispatch

New `UserAction::CancelAllActiveTransfers` mapped to
`KeyCode::Char('X')`. F2 dispatch arm:

```rust
UserAction::CancelAllActiveTransfers => {
    if app.cancel_status.is_sending() || app.cancel_status.is_confirming() {
        // mid-cycle — ignore
    } else if let Some(endpoint) = app.parsed_remote.clone() {
        let count = app.transfers.active_count();
        if count == 0 { /* silent no-op */ }
        else if tui_config.transfer.confirm_cancel {
            app.cancel_status = F2CancelStatus::ConfirmingBatch { count };
        } else {
            let count = spawn_batch_cancels(...);
            app.cancel_status = F2CancelStatus::BatchInitiated { count, finished_at };
        }
    }
}
```

The d-29 `y` arm extends to handle both confirm
variants:

```rust
UserAction::TransferMirrorConfirm if app.cancel_status.is_confirming() => {
    match &app.cancel_status {
        F2CancelStatus::Confirming { transfer_id } => { /* single-cancel path */ }
        F2CancelStatus::ConfirmingBatch { .. } => {
            let count = spawn_batch_cancels(...);
            app.cancel_status = F2CancelStatus::BatchInitiated { count, finished_at };
        }
        _ => {}
    }
}
```

### Spawn helper

```rust
fn spawn_batch_cancels(
    transfers: &TransfersState,
    endpoint: &RemoteEndpoint,
    cancel_request_seq: &mut u64,
    tx: &mpsc::Sender<CancelReply>,
) -> usize {
    let ids: Vec<String> = transfers.active_rows()
        .into_iter().map(|r| r.transfer_id.clone()).collect();
    let count = ids.len();
    for id in ids {
        *cancel_request_seq += 1;
        spawn_cancel_transfer(*cancel_request_seq, endpoint.clone(), id, tx.clone());
    }
    count
}
```

Each RPC dispatched independently with its own
`request_id`. The reply arm's generation guard
(comparing `request_id` to current
`Sending.request_id`) discards batch replies since
they don't match any `Sending` state. That's fine —
the operator-visible feedback is the BatchInitiated
fragment, and per-transfer outcomes propagate via the
Subscribe stream's `TransferComplete` / `TransferError`
events. No new state-machine bookkeeping needed.

### Renderer

Two new `F2CancelDisplay` variants:

- `ConfirmingBatch { count }` — yellow,
  `cancel N transfers? y/N`.
- `BatchInitiated { count }` — green,
  `sent N cancel requests`.

Both inherit the d-23 TTL bridge path. ConfirmingBatch
has no TTL (prompt stays until answered);
BatchInitiated has a TTL and the bridge returns Hidden
past the deadline, same as single-cancel Done/Error.

### Help overlay

New row under "F1 · F2 · F3 navigation":

```
X   cancel ALL active transfers (F2) — Shift+x; honors confirm_cancel
```

Modal height 35 → 36 to fit. d-16 R2 regression test
gains an `X` presence check.

## Files changed

- `crates/blit-tui/src/main.rs`:
  - `F2CancelStatus::{ConfirmingBatch, BatchInitiated}`
    variants.
  - `is_confirming()` extended.
  - `UserAction::CancelAllActiveTransfers`.
  - `Char('X')` keymap.
  - F2 dispatch arm + extended TransferMirrorConfirm arm.
  - `spawn_batch_cancels` helper.
  - `cancel_status_to_display` covers the new variants.
  - `cancel_status_remaining_ttl` recognizes
    BatchInitiated's finished_at.
  - 9 new tests.
- `crates/blit-tui/src/screens/f2.rs`:
  - `F2CancelDisplay::{ConfirmingBatch, BatchInitiated}`
    variants.
  - Footer renderer matches them.
  - Module-doc layout + variant block updated.
- `crates/blit-tui/src/help.rs`:
  - `X` row added under F1·F2·F3 nav.
  - Modal height 35 → 36.
  - Keymap test asserts `X` presence.

## Tests

+9 tests (325 → 334):

- `key_action_maps_shift_x_to_cancel_all` — pins the
  keymap.
- `f2_cancel_status_confirming_batch_predicates` —
  is_confirming = true, is_sending = false.
- `f2_cancel_status_batch_initiated_predicates` —
  is_confirming = false, is_sending = false (terminal-ish).
- `cancel_status_to_display_renders_confirming_batch` —
  bridge maps ConfirmingBatch → display ConfirmingBatch
  with count preserved.
- `cancel_status_to_display_renders_batch_initiated_within_ttl`
  — bridge maps BatchInitiated within TTL → display
  BatchInitiated.
- `cancel_status_to_display_hides_batch_initiated_past_ttl`
  — past TTL the bridge returns Hidden.
- `cancel_status_remaining_ttl_confirming_batch_returns_none`
  — prompt has no deadline.
- `cancel_status_remaining_ttl_batch_initiated_returns_positive`
  — drives the loop's wakeup budget.
- `esc_cancels_confirm_routes_f2_confirming_batch` —
  Esc routing covers the batch prompt.

`cargo fmt`, `cargo clippy --workspace --all-targets
-- -D warnings`, and `cargo test --workspace` all green.

## Known gaps

1. **No per-transfer outcome on the cancel fragment.**
   The footer just says "sent N cancel requests". If
   3 of 5 cancels fail (e.g., NotFound), the operator
   sees no per-transfer breakdown — they have to read
   the Subscribe stream events. d-22 known gap #1
   noted this trade-off: aggregating N independent
   replies into the single-cell cancel fragment would
   need a new state structure.

2. **No "are you sure for batch?" wording.** The
   prompt is terse (`cancel N transfers? y/N`) to fit
   the single-line footer. Operators with a habit of
   pressing y reflexively get the same risk profile
   as the single-cancel confirm.

3. **No batch limit.** `Shift+X` cancels every active
   row regardless of count. Pathologically large
   batches (1000+ transfers) spawn 1000 tokio tasks
   in a tight loop. In practice F2 active counts stay
   small (< 50), but a future polish could cap at
   N=100 and warn.

## Out of scope (next slices)

- **Hot-reload tui.toml.**
- **F3 filter regex/glob mode.**

## Reviewer comments

(empty — pending grade)
