# d-29-confirm-cancel: opt-in F2 cancel confirmation

**Severity**: Feature (polish — closes d-22 known gap #1)
**Status**: In progress / pending review
**Branch**: `phase5/a1`
**Commit**: filled by the sentinel commit

## What

d-22 shipped `K` as a one-keystroke cancel of the
cursor-selected F2 transfer. d-22's "Known gaps" §1
flagged the polish opportunity:

> **No confirmation prompt.** `K` fires immediately.
> Cancel is reversible ... a future polish could add
> an opt-in `[transfer] confirm_cancel` config field.

d-29 lands the opt-in. With
`[transfer] confirm_cancel = true` in `tui.toml`,
pressing `K` opens:

```
live · cancel abc-123? y/N
```

`y` confirms → fires the CancelJob RPC (same path as
the d-22 default). `n` or `Esc` aborts → cancel_status
returns to Idle, the footer fragment disappears.

Default is `false` — d-22's one-keystroke behavior is
preserved for operators who don't want the prompt.

## Approach

### Config

New `confirm_cancel: bool` on `TransferDefaults`. Default
false. The TUI schema doc now lists:

```toml
[transfer]
cancel_status_ttl_ms = 5000   # d-24
confirm_cancel = false        # d-29
```

### State machine

`F2CancelStatus` gains a `Confirming { transfer_id }`
variant. Two predicates:

```rust
impl F2CancelStatus {
    fn is_sending(&self) -> bool { /* unchanged */ }
    fn is_confirming(&self) -> bool {
        matches!(self, F2CancelStatus::Confirming { .. })
    }
}
```

Confirming has no TTL — the prompt stays until the
operator answers. `cancel_status_remaining_ttl` returns
None for it; the loop's sleep budget doesn't tighten.

### Dispatcher

F2's `K` arm branches on `tui_config.transfer.confirm_cancel`:

```rust
if tui_config.transfer.confirm_cancel {
    app.cancel_status = F2CancelStatus::Confirming { transfer_id: id };
} else {
    /* d-22 path: spawn the RPC, set Sending */
}
```

Two new F2 arms handle the confirm:

```rust
UserAction::TransferMirrorConfirm if app.cancel_status.is_confirming() => {
    /* y: promote Confirming → Sending, spawn RPC */
}
UserAction::TransferCancel if app.cancel_status.is_confirming() => {
    /* n: cancel_status = Idle */
}
```

`y` and `n` reuse the existing `TransferMirrorConfirm` /
`TransferCancel` UserActions (originally F4 mirror/move
confirm). On F2, the gate predicate is
`cancel_status.is_confirming()`; on F4, it stays
`transfer.is_confirming()`. The two state machines
share keys without sharing state.

### Esc routing

`esc_cancels_confirm` now returns true when EITHER
state machine is in confirm mode. The router intercept
resets whichever applies:

```rust
fn esc_cancels_confirm(...) -> bool {
    /* ...bare Esc... */
    && (app.transfer.is_confirming() || app.cancel_status.is_confirming())
}

if esc_cancels_confirm(&key, &app) {
    app.transfer.cancel_confirm();
    if app.cancel_status.is_confirming() {
        app.cancel_status = F2CancelStatus::Idle;
    }
    continue;
}
```

This means Esc is intercepted BEFORE
`handle_verify_keystroke` (same priority as d-12 R2),
so an operator who's Tab-ed into a Verify field
mid-cancel-confirm can still escape with Esc.

### Renderer

New `F2CancelDisplay::ConfirmingCancel { transfer_id }`
variant. The renderer-side bridge
(`cancel_status_to_display`) maps `Confirming` to it.
Footer styling: yellow, "cancel <id>? y/N". Module-doc
layout block updated.

### Help overlay

The `K` row now mentions the opt-in prompt:

```
K   cancel selected transfer (F2) — y/N prompt if [transfer] confirm_cancel
```

`y / N / Esc` row in the F4 section already covers the
y/n keys; their F2 cancel-confirm use is by-extension
since the same keys trigger the same conceptual gesture.

## Files changed

- `crates/blit-tui/src/config.rs`:
  - `TransferDefaults` gains `confirm_cancel: bool`.
  - Module-doc schema block lists the new field.
  - 4 existing test fixtures updated to use
    `..TransferDefaults::default()` so future schema
    growth doesn't break them.
- `crates/blit-tui/src/main.rs`:
  - `F2CancelStatus::Confirming` variant.
  - `is_confirming()` predicate.
  - `handle_pane_action` gains `tui_config:
    &config::TuiConfig` parameter.
  - F2 dispatch branches on `confirm_cancel` for the K
    path; new TransferMirrorConfirm / TransferCancel
    arms gated on `cancel_status.is_confirming()`.
  - `cancel_status_to_display` maps Confirming →
    `ConfirmingCancel`.
  - `cancel_status_remaining_ttl` returns None for
    Confirming (no deadline).
  - `esc_cancels_confirm` predicates both state machines.
  - Router intercept resets `cancel_status` to Idle on
    Esc when applicable.
- `crates/blit-tui/src/screens/f2.rs`:
  - `F2CancelDisplay::ConfirmingCancel` variant.
  - Footer renders the yellow `cancel <id>? y/N`
    fragment.
  - Module-doc layout sketch lists the new variant.
- `crates/blit-tui/src/help.rs`:
  - `K` row mentions the opt-in confirm prompt.

## Tests

+11 tests (314 → 325):

**`config::tests` — 4 new:**
- `transfer_default_confirm_cancel_is_false`.
- `transfer_confirm_cancel_parses_true`.
- `transfer_confirm_cancel_parses_false_explicitly`.
- `transfer_confirm_cancel_and_ttl_compose` — d-24 +
  d-29 fields independent.

**`main::tests` — 7 new:**
- `f2_cancel_status_confirming_predicates` —
  Confirming reports is_confirming=true, is_sending=false.
- `f2_cancel_status_sending_predicates` — Sending
  reports is_sending=true, is_confirming=false.
- `f2_cancel_status_idle_done_error_predicates` —
  predicate matrix for the other variants.
- `cancel_status_to_display_renders_confirming` — bridge
  emits the new display variant.
- `cancel_status_remaining_ttl_confirming_returns_none`
  — Confirming has no TTL deadline.
- `esc_cancels_confirm_routes_f2_cancel_confirm` —
  predicate returns true for F2 cancel-confirm.
- `esc_cancels_confirm_returns_false_when_neither_confirming`
  — predicate is still tight; Esc falls through to
  normal quit handling otherwise.

`cargo fmt`, `cargo clippy --workspace --all-targets
-- -D warnings`, and `cargo test --workspace` all green.

## Known gaps

1. **No "are you sure?" word.** The prompt is terse
   (`cancel <id>? y/N`) to fit the single-line footer.
   A more verbose phrasing would push the key hints
   off the right edge on narrow terminals.

2. **No fallback when remote disappears mid-prompt.**
   If the operator opens the prompt then the daemon
   connection drops, pressing `y` silently aborts (no
   endpoint = early return to Idle). The status footer
   doesn't surface the abort reason — the
   ConfirmingCancel fragment just vanishes. Future
   polish could add an Error variant to mark the
   abort.

3. **Confirm dialog isn't wide enough for noisy
   transfer IDs.** The prompt shows the full ID. UUID
   v4 IDs (~36 chars) push the right edge close to the
   key hints on 80-col terminals. Future polish could
   abbreviate.

## Out of scope (next slices)

- **Batch cancel Shift-K** (d-22 known gap #2).
- **Hot-reload tui.toml.**

## Reviewer comments

(empty — pending grade)
