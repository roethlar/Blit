# m2f-2-f2-composite-key: key F2 transfers by (daemon, transfer_id)

**Severity**: Feature (multi-daemon F2 correctness foundation)
**Status**: In progress / pending review
**Branch**: `phase5/a1`
**Commit**: `08a0642`

## What

Second sub-slice of multi-daemon F2 (after m2f-1's per-row
`source_daemon`). Daemon-minted `transfer_id`s are `t<ms>-<n>` —
unique only *within* a daemon (the counter resets on restart, see
`crates/blit-daemon/src/active_jobs.rs` `mint_transfer_id`). Once
F2 fans out across daemons (m2f-3), two daemons can mint the same
id, so the F2 view must key by **(daemon, transfer_id)** or it
would collapse/evict unrelated transfers.

## Approach

`row_key(source_daemon, transfer_id)` = `"{daemon}\u{1f}{id}"`
(unit separator can't appear in a host or id). Threaded everywhere
the `active` map or cursor used a bare id:

- `active: HashMap<String, ActiveRow>` now keyed by the composite.
  `apply_event` (Started insert, Progress lookup, Complete/Error
  remove) and `replace_from_snapshot` build the key from the
  `source_daemon` param (added in m2f-1) + the event/snapshot id.
- **Recent-id dedup** matches `id AND source_daemon`, so a terminal
  transfer on daemon A no longer suppresses a same-id transfer on
  daemon B.
- **Cursor** anchors on the composite key (field renamed
  `selected_active_id` → `selected_active_key`); the public
  `selected_active_id()` still returns the bare id (the CancelJob
  target) by looking the row up by composite key. `select_next/
  prev/first/last` + `selected_active_index` updated.

## Why behavior-preserving for one daemon

With a single daemon every key is `row_key(parsed_remote, id)` —
consistent, so lookups/cursor/dedup behave exactly as before. The
verified d-21/d-22 cursor + dedup tests pass unchanged (579 total,
+1).

## Files changed

- `crates/blit-tui/src/state.rs`: `row_key` helper; `active` map +
  cursor + dedup re-keyed; field rename; `active` doc updated; 1
  test.

## Tests

579 total (+1):

- `same_id_from_two_daemons_stays_distinct` — `nas`/`t1` and
  `skippy`/`t1` are two rows; completing `nas`/`t1` leaves
  `skippy`/`t1` active; the recent `nas`/`t1` doesn't dedup-suppress
  `skippy`/`t1`.
- All existing single-daemon cursor/dedup/snapshot tests pass
  unchanged (the guardrail for the re-key).

## Multi-daemon F2 sub-slice plan

- m2f-1 ✓: per-row `source_daemon`.
- **m2f-2 (this):** composite `(daemon, id)` key.
- **m2f-3:** persistent merged, daemon-tagged event channel; one
  Subscribe forwarder per discovered daemon (mDNS list); per-daemon
  snapshot merge; render the source-daemon column; multi-daemon
  cancel.
- **m2f-4:** dynamic discovery (subscribe to daemons appearing
  later) + per-daemon reconnect / degraded state.

## Reviewer comments

(empty — pending grade)
