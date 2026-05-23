# m2f-6-f2-daemon-column: render the source-daemon column in F2

**Severity**: Feature (multi-daemon F2 — single-pane labeling)
**Status**: In progress / pending review
**Branch**: `phase5/a1`
**Commit**: `a5456cc`

## What

With the fan-out live (m2f-5 verified), F2 shows transfers from
every discovered daemon — so each row now needs to say *which*
daemon it belongs to. Adds a `daemon` column (first, host[:port])
to the F2 active **and** recent tables, reading the `source_daemon`
field that's been recorded since m2f-1.

This is the render that m2f-1 deliberately deferred: a single-daemon
column was redundant with the header, but with multiple daemons it's
the single-pane-of-glass label that distinguishes rows.

## Files changed

- `crates/blit-tui/src/screens/f2.rs`: `daemon` cell prepended to
  `active_row_to_table_row` / `recent_row_to_table_row`; `daemon`
  header + a `Length(16)` width prepended to both tables;
  module-doc layout box updated; render test.

## Tests

586 total (+1): `active_table_renders_source_daemon_column` — a
`TestBackend` render of an active row tagged `skippy:9001` asserts
both the `daemon` header and the row's daemon label appear.

## Multi-daemon F2 sub-slice plan

- m2f-1..5 ✓ verified — model foundation + fan-out (F2 watches all
  discovered daemons via merged streams).
- **m2f-6 (this):** render the per-row daemon column.
- **m2f-7:** dynamic discovery — auto re-fan when the mDNS daemon
  list changes (today a daemon appearing after setup is picked up
  only on the next `r`/d-48).
- **m2f-8:** per-daemon reconnect / degraded state.
- **m2f-9:** multi-daemon cancel (`CancelJob` to the cursor row's
  daemon, not just `parsed_remote`).

## Reviewer comments

(empty — pending grade)
