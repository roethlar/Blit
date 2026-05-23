# m2f-3-f2-merge-snapshot: additive per-daemon snapshot hydration

**Severity**: Feature (multi-daemon F2 foundation) + latent-bug fix
**Status**: In progress / pending review
**Branch**: `phase5/a1`
**Commit**: `7202418`

## What

Third sub-slice of multi-daemon F2. Adds
`TransfersState::merge_snapshot(source_daemon, state, fetched_at)` —
the **additive** counterpart to `replace_from_snapshot`. It hydrates
ONE daemon's `GetState` without disturbing other daemons' rows:
drops only that daemon's active + recent entries, then inserts the
snapshot's. The fan-out (m2f-4) calls it per watched daemon so each
hydrates independently into the shared view; initial F2 setup keeps
the full `replace_from_snapshot` (fresh start, nothing else watched
yet).

## Latent bug fixed (refresh identity)

`refresh_via_get_state` (the `r` re-query) tagged rows with
`endpoint.host` — host-only — while `f2_source_label` / the live
stream / the reset label use `host_port_display()` (m2f-2 R2). For a
non-default-port daemon the refresh-hydrated rows
(`source_daemon = "nas"`) would have a different composite key from
the stream rows (`"nas:9444"`), so a refresh would never reconcile
with the live rows (orphans/dupes). Refresh now uses
`host_port_display()` + `merge_snapshot`.

## Why behavior-preserving (single daemon)

With one daemon, `merge_snapshot` (drop-this-daemon + insert) and
`replace_from_snapshot` (clear-all + insert) produce identical state.
The refresh identity fix only changes behavior for a non-default-port
daemon, where it's strictly more correct.

## Recent-ring note

Recent rows carry no completion timestamp, so a precise cross-daemon
time interleave isn't possible; `merge_snapshot` groups a daemon's
recent rows by merge order and the global ring stays bounded by
`TUI_RECENT_CAP`. A timestamp-based interleave is a possible later
refinement.

## Files changed

- `crates/blit-tui/src/state.rs`: `merge_snapshot` + test.
- `crates/blit-tui/src/main.rs`: `refresh_via_get_state` uses
  `host_port_display()` + `merge_snapshot`; setup arm comment notes
  m2f-4 will switch it to merge.

## Tests

582 total (+1): `merge_snapshot_is_additive_per_daemon` — daemon A
(2 active) + daemon B (1) coexist (3 rows); re-merging A (1 active)
replaces only A's rows, leaving B's. Existing single-daemon
hydration tests unchanged.

## Multi-daemon F2 sub-slice plan

- m2f-1 ✓ source_daemon on rows · m2f-2 ✓ composite (daemon,id) key.
- **m2f-3 (this):** additive `merge_snapshot`.
- **m2f-4:** persistent merged, daemon-tagged event channel + a
  Subscribe forwarder per discovered daemon (each merge_snapshot +
  stream) + render the source-daemon column.
- **m2f-5:** dynamic discovery (subscribe to daemons appearing
  later) + per-daemon reconnect; multi-daemon cancel.

## Reviewer comments

(empty — pending grade)
