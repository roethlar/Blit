# m2f-1-f2-source-daemon: tag F2 transfer rows with their source daemon

**Severity**: Feature (TUI_DESIGN §5.2 — F2 single-pane across daemons)
**Status**: In progress / pending review
**Branch**: `phase5/a1`
**Commit**: `aeac25d`

## What

First sub-slice of **multi-daemon F2** (the design's
single-pane-of-glass: watch transfers across *all* discovered
daemons, not just one). This slice is the **foundation only** —
behavior-preserving, F2 still watches the single `parsed_remote`
daemon. It establishes the per-row source-daemon model that the
fan-out (m2f-2) needs to label rows by daemon.

## Approach

- `ActiveRow` / `RecentRow` gain `source_daemon: String` — the
  daemon whose Subscribe stream (or snapshot) reported the
  transfer, distinct from `peer` (the transfer's other endpoint).
- Threaded through the apply path:
  `apply_event(source_daemon, …)`,
  `replace_from_snapshot(source_daemon, …)`, and
  `drain_startup_events(rx, source_daemon, …)`. The
  `From<ActiveTransfer>` / `From<TransferRecord>` impls default it
  empty (the wire types carry no daemon); callers set it.
- A Complete/Error carries the **active row's** recorded source
  daemon to the recent row (fallback to the param if the row was
  never seen).
- `f2_source_label(app)` supplies the label (the watched daemon's
  `host`) at the call sites; today that's always `parsed_remote`.

## Why render is deferred to m2f-2

A per-row "daemon" column is only meaningful once F2 watches more
than one daemon — with a single stream every row would just repeat
the value already in the F2 header. So m2f-1 wires + populates +
tests the field (it's `pub`, so no dead-code), and m2f-2 adds the
fan-out *and* the column together.

## Files changed

- `crates/blit-tui/src/state.rs`: `source_daemon` on both row
  structs + `From` impls; `apply_event` / `replace_from_snapshot`
  take `source_daemon`; Complete/Error carry it to recent; 2 tests.
- `crates/blit-tui/src/main.rs`: `f2_source_label` helper;
  `drain_startup_events` gains the param; live `apply_event`,
  setup `replace_from_snapshot`, and `refresh_via_get_state`
  thread the watched-daemon label.
- `crates/blit-tui/src/screens/f2.rs`: test-helper `RecentRow`
  initializer updated (no render change).

## Tests

578 total (+2):

- `rows_record_source_daemon` — a Started tags the active row
  "nas"; the following Complete carries "nas" to the recent row.
- `snapshot_tags_rows_with_source_daemon` —
  `replace_from_snapshot("skippy", …)` tags both active + recent.

## Multi-daemon F2 sub-slice plan

- **m2f-1 (this):** per-row source-daemon model.
- **m2f-2:** persistent merged, daemon-tagged event channel; one
  Subscribe forwarder per discovered daemon (from the mDNS list);
  F2 watches all; render the source-daemon column.
- **m2f-3:** dynamic discovery (subscribe to daemons appearing
  later) + per-daemon reconnect / degraded state.

## Reviewer comments

(empty — pending grade)
