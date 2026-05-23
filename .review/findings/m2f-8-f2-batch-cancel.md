# m2f-8-f2-batch-cancel: batch X cancels each row's own daemon

**Severity**: Feature / correctness (multi-daemon F2 batch cancel)
**Status**: In progress / pending review
**Branch**: `phase5/a1`
**Commit**: `dfdaabd`

## What

m2f-7 fixed single cancel (`K`) to target the selected row's daemon,
but explicitly deferred batch cancel (`X`). With F2 showing rows from
every discovered daemon, `X` still fanned a single `CancelJob` set at
`parsed_remote` — so any active row owned by a *different* daemon was
never cancelled (its `transfer_id` is foreign to `parsed_remote` →
`NotFound`; fail-safe, but the cancel never lands). m2f-8 makes `X`
snapshot every active `(daemon, transfer_id)` at prompt creation and
dispatch one `CancelJob` per target against that row's own daemon.

## Approach

- `snapshot_active_targets(transfers) -> Vec<(String, String)>` —
  replaces `snapshot_active_ids`; captures `(source_daemon,
  transfer_id)` for every active row.
- `spawn_cancels_for_targets(targets, &mut seq, &tx) -> usize` — one
  `CancelJob` RPC per target, each resolved through
  `cancel_endpoint(daemon)` (the same host[:port] → connectable
  `RemoteEndpoint` parse used by single cancel). Returns the count
  actually dispatched; targets whose daemon identity won't parse are
  skipped (defensive — discovered rows always carry a real identity).
- `F2CancelStatus::ConfirmingBatch { targets }` freezes the target set
  at `X` press (the active list may change before the operator answers
  `y`), mirroring m2f-7's freeze-at-prompt contract for single cancel.
- `ConfirmedCancel::Batch(Vec<(String, String)>)` carries the frozen
  pairs to the confirm `y` path.
- The `X` handler no longer gates on `parsed_remote`; it's a no-op only
  when `snapshot_active_targets` is empty.

## Files changed

- `crates/blit-tui/src/main.rs`: `snapshot_active_targets`;
  `spawn_cancels_for_targets`; `ConfirmingBatch { targets }`;
  `ConfirmedCancel::Batch(Vec<(String, String)>)`; `X` handler +
  confirm-batch `y` arm target each row's daemon; render-bridge
  destructure (`targets.len()`); test-construction + assertion
  updates.

## Tests

589 total (+1 net; two `snapshot_active_ids` tests renamed/rewritten,
one `spawn_cancels_for_targets` test added):
- `snapshot_active_targets_captures_all_active_rows` — two daemons
  (`nas`, `skippy:9001`), asserts both `(daemon, id)` pairs captured.
- `snapshot_active_targets_empty_state` — fresh state → empty (pairs
  with the dispatcher's no-op guard).
- `spawn_cancels_for_targets_skips_malformed_and_counts_valid` — two
  valid + one empty-daemon target → dispatches 2, seq advances twice.
- Existing freeze test (`confirming_batch_freezes_ids_at_prompt_
  creation`) updated to assert the frozen `(daemon, id)` pairs.

The `X` → per-daemon `CancelJob` wiring is integration (live daemons).

## Scope

Batch `X` cancel only. This closes the last cancel-correctness gap in
multi-daemon F2 (single `K` was m2f-7).

## Remaining multi-daemon F2 follow-ups

- **m2f-9 / discovery:** auto re-fan when the mDNS daemon list changes
  (today new daemons are picked up only on `r`); per-daemon
  reconnect / degraded state.

## Reviewer comments

(empty — pending grade)
