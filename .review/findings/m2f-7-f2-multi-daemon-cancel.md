# m2f-7-f2-multi-daemon-cancel: single cancel targets the row's daemon

**Severity**: Feature / correctness (multi-daemon F2 cancel)
**Status**: In progress / pending review
**Branch**: `phase5/a1`
**Commit**: `bbd0084`

## What

With F2 showing rows from every daemon (m2f-5/6), `K`
(cancel-selected) still sent `CancelJob` to `parsed_remote`. So
cancelling a transfer that belongs to a *different* daemon hit the
wrong daemon — the foreign `transfer_id` returns `NotFound`
(fail-safe: no wrong transfer cancelled, but the intended cancel
never landed). m2f-7 makes `K` target the **selected row's** daemon.

## Approach

- `TransfersState::selected_active_daemon()` — the cursor row's
  `source_daemon`, with the same fall-off contract as
  `selected_active_id` (both resolve the same composite-keyed row).
- `F2CancelStatus::Confirming` gains `daemon`, captured at `K` press
  (the cursor may move before the operator answers `y`). The confirm
  `y` path cancels against that captured daemon.
- `cancel_endpoint(daemon)` parses the row's `host[:port]` identity
  back into a connectable `RemoteEndpoint` — `CancelJob` only needs
  the control plane (host:port), no module — so a Discovery endpoint
  is fine.
- `ConfirmedCancel::Single` now carries `{ id, daemon }`.

## Scope

Single cancel (`K` + its confirm) only. Batch `X` still targets
`parsed_remote` — per-daemon batch grouping is **m2f-8**. Both paths
are currently fail-safe (a foreign id → `NotFound`, never a wrong
cancellation), so deferring batch is safe.

## Files changed

- `crates/blit-tui/src/state.rs`: `selected_active_daemon()` + test.
- `crates/blit-tui/src/main.rs`: `Confirming.daemon`; `K` handler +
  confirm `y` handler target the row's daemon; `ConfirmedCancel::Single
  { id, daemon }`; `cancel_endpoint`; render-bridge destructure;
  test-construction updates; `cancel_endpoint` test.

## Tests

588 total (+2): `selected_active_daemon_matches_cursor_row` (daemon +
id come from the same selected row across two daemons);
`cancel_endpoint_round_trips_daemon_identity` (host:port preserved,
default-port form). The K→CancelJob wiring to the row's daemon is
integration (live daemons).

## Remaining multi-daemon F2 follow-ups

- **m2f-8:** batch `X` cancel per-daemon grouping.
- **m2f-9 / m2f-7-discovery:** auto re-fan when the mDNS daemon list
  changes (today new daemons are picked up only on `r`); per-daemon
  reconnect / degraded state.

## Reviewer comments

(empty — pending grade)
