# b-3-recent-ring: TransferRecord ring + outcome capture

**Severity**: Feature (no behavior change visible on the wire)
**Status**: In progress / pending review
**Branch**: `phase5/getstate`
**Commit**: filled by the sentinel commit

## What

Lands the recent-runs ring from §6.3 of
`docs/plan/TUI_DESIGN.md`. `ActiveJobs` now drains an active
row into a bounded ring of `TransferRecord` entries on every
guard drop; spawn closures in `service/core.rs` report
success/failure via `ActiveJobGuard::record_outcome` before
dropping the guard. After this slice the daemon has
everything `GetState.recent[]` needs to read — just not the
RPC handler itself (b-4).

## Approach

Two related additions, both on `crates/blit-daemon/src/active_jobs.rs`:

1. **The ring.** A `VecDeque<TransferRecord>` behind a
   `std::sync::Mutex` on the existing `Inner`, sized by a
   `recent_limit: usize` field. Drop pushes a record at the
   back and trims oldest-first when over capacity. The
   `DEFAULT_RECENT_LIMIT = 50` constant is the single source
   of truth for the default; `ActiveJobs::with_recent_limit(0)`
   is a legal way to disable history entirely.

2. **Outcome capture.** `ActiveJobGuard` gains a
   `Mutex<Option<RecordedOutcome>>` cell + a
   `record_outcome(&self, ok, error_message)` setter.
   Last-writer-wins; in practice spawn closures call it
   exactly once. Drop reads the cell, falls back to
   `ok=false` + `"cancelled before outcome recorded"` if
   never set (spawn panic, cancel-before-outcome) — so a
   silently-dropped panic still leaves an audit trail on the
   ring rather than vanishing.

`TransferRecord` mirrors the wire shape:

```
transfer_id, kind, peer, module, path,
start_unix_ms, duration_ms, ok, error_message
```

`duration_ms` is computed at Drop time as
`unix_ms_now().saturating_sub(row.start_unix_ms)` — saturating
so a backwards clock jump between registration and drain
doesn't underflow.

Missing wire fields (`bytes`, `files`) come from milestone C's
write-loop instrumentation; deferred per the design doc.

## Files changed

- `crates/blit-daemon/src/active_jobs.rs`:
  - `+DEFAULT_RECENT_LIMIT` constant.
  - `+TransferRecord` struct.
  - `+RecordedOutcome` private struct + outcome cell on
    `ActiveJobGuard`.
  - `+ActiveJobs::with_recent_limit` constructor; `new()`
    delegates.
  - `+ActiveJobs::recent()` snapshot method.
  - `+ActiveJobGuard::record_outcome(ok, error_message)`.
  - `Drop` impl rewritten to: drain the active row, build a
    `TransferRecord`, push (with trim) when `recent_limit > 0`.
  - `+build_record` and `+push_recent` private helpers.
  - Module doc updated with the `b-3-recent-ring` scope
    section.

- `crates/blit-daemon/src/service/core.rs`:
  - `+outcome_from_status(&Result<_, Status>) -> (bool,
    Option<String>)` helper.
  - `push`, `pull`, `pull_sync` spawn closures call
    `job.record_outcome(...)` after the handler returns,
    before dropping the guard.
  - `delegated_pull`'s select-shaped outcome
    (`Option<bool>`) is inlined to the same `(ok, err_msg)`
    pair before `record_outcome`. The phased error message
    isn't available at this level today; the C milestone's
    Subscribe events will carry structured errors.

## Tests added

- `drop_with_recorded_outcome_pushes_to_recent` — record is
  on the ring with `ok=true`, fields preserved.
- `drop_with_error_outcome_carries_message` — failure
  message preserved.
- `drop_without_recorded_outcome_marks_cancelled` — silent
  drop regression test: panic / cancel-before-outcome
  leaves a placeholder record.
- `recent_ring_bounded_evicts_oldest` — 5 entries into a
  size-3 ring, the 3 survivors are the most recent in
  oldest-first order.
- `recent_ring_zero_limit_disables_history` — `with_recent_limit(0)`
  drains the active table but pushes nothing onto the ring.

Workspace: 514 passed (was 509; +5).

## Known gaps

1. **`bytes` / `files` on the record are absent.** Milestone
   C's write-loop instrumentation will fill them via a
   guard update path analogous to `set_endpoint`. The
   `TransferRecord` struct is wire-shape complete except
   for those two fields.

2. **`GetState` RPC not yet implemented.** Out of scope for
   this slice; b-4 wires it.

3. **`delegated_pull` failure case carries a generic
   placeholder message.** The handler returns `bool`
   today, not `Result<_, Status>`; the actual phased error
   was already sent on the data plane via `handler_tx`.
   Routing that message back to the ActiveJobs ring would
   require either reshaping `handle_delegated_pull`'s return
   or sharing a side channel — both bigger than this slice
   should be. C's Subscribe events will carry the
   structured shape end-to-end.

4. **Drop runs inside a sync-mutex critical section.** Two
   mutexes touched (table + recent). The path is short
   (HashMap remove + VecDeque push + len-check), but a
   future GetState handler that takes the table mutex AND
   the recent mutex in opposite orders could deadlock.
   Current code only takes them sequentially; b-4 will
   need to follow the same order (table first, recent
   second) and have a doc comment to that effect.

## Reviewer comments

(empty — pending grade)
