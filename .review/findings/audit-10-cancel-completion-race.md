# audit-10-cancel-completion-race: Cancel/completion race in delegated_pull

**Severity**: Bug
**Status**: In progress / pending review
**Branch**: `phase5/a1`
**Commit**: `3601f1e`

## What

Ground-up audit found a race condition in the `DelegatedPull` spawn closure that
can cause a successfully completed transfer to be recorded as "cancelled via
CancelJob."

**`crates/blit-daemon/src/service/core.rs:762-792`** — The `tokio::select!` is
`biased`, so branches are evaluated in declaration order:

1. `tx.closed()` (client disconnect)
2. `cancel_token.cancelled()` (CancelJob fired)
3. `handle_delegated_pull(...)` (handler completed)

If `handle_delegated_pull` returns `Ok(())` (success) at the exact same moment the
cancellation token is fired by a `CancelJob` RPC, the `biased` select may resolve
the `cancel_token.cancelled()` branch *before* it evaluates the `handler_ok`
branch. The `outcome` becomes `None`, and the subsequent match at line 813
records:

```rust
(None, Some("cancelled via CancelJob".to_string()))
```

The transfer is pushed into the recent ring as a failure caused by CancelJob,
even though it actually completed successfully.

## Approach

Use an atomic or a oneshot channel to determine whether the handler actually
completed before recording the outcome. A simple fix: if `handle_delegated_pull`
returns `Some(true)`, that should be authoritative regardless of the cancel
token state at the select boundary. The select could be restructured so the
handler branch sets a flag that the cancel branch checks, or the outcome could
be determined by polling `try_wait` on the handler future instead of a naked
select.

## Files changed

TBD by coder. Primarily `crates/blit-daemon/src/service/core.rs`.

## Tests

- Simulate simultaneous completion + CancelJob firing; verify the transfer is
  recorded as successful, not cancelled.
- Existing delegated_pull and cancel tests must still pass.

## Resolution (commit `3601f1e`)

Extracted the inline `tokio::select!` into a generic async helper
`resolve_delegated_pull_outcome(handler, tx_closed, cancelled, detach)`
and **ordered the handler branch first** in the `biased` select.

`biased` polls branches in declaration order, so with the handler first:
- a `Ready` handler wins even when the cancel token / client-hangup
  signal are *also* ready at the same poll — a real result (success or
  failure) is now authoritative over a simultaneous cancel;
- a still-`Pending` handler falls through to the `tx_closed` branch
  (gated by `!detach`) and then the `cancelled` branch, so a running
  transfer remains cancellable / hangup-abortable exactly as before.

Production calls the helper with
`handle_delegated_pull(...)`, `tx.closed()`, `cancel_token.cancelled()`,
`detach`. Making it a free generic fn lets the ordering invariant be
unit-tested with synthetic futures (no full handler stand-up needed) —
the genuine race window is sub-microsecond and not reproducible by wall
clock, so testing the select's resolution contract directly is the
honest coverage here.

**Tests (blit-daemon, +3):**
`resolve_pull_handler_completion_wins_over_simultaneous_cancel`
(`ready(true)` and `ready(false)` each beat a simultaneous
`ready`-cancel + `ready`-hangup),
`resolve_pull_pending_handler_yields_to_cancel` (a `pending` handler
still resolves to `None` via cancel — proves transfers stay cancellable),
`resolve_pull_detach_disables_client_hangup` (`detach=true` + ready
`tx_closed` + no cancel ⇒ the future does not resolve, asserted via a
50 ms timeout). Full workspace gate green.

## Reviewer comments

(empty — pending review)
