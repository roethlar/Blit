# audit-3a-mutex-poisoning: recover poisoned ActiveJobs mutex instead of panicking

**Severity**: Robustness
**Status**: In progress / pending review
**Branch**: `phase5/a1`
**Commit**: `198ff31`
**Parent finding**: `audit-3-panic-resilience` (delivered as two sub-slices —
audit-3a here covers the mutex poisoning; audit-3b covers the
`generate_token` RNG `expect`).

## What

Part 1 of audit-3. The non-Drop `ActiveJobs` paths locked the `table` /
`recent` mutexes with `.expect("active_jobs table poisoned")` /
`.expect("active_jobs recent poisoned")`. If any task panics while
holding one of those mutexes, the lock is poisoned and **every
subsequent** `register` / `snapshot` / `cancel` / progress-tick /
`emit_event` / `recent` / GetState read would itself panic on the
`expect`, cascading one failure into a daemon-wide outage of the
transfer registry.

The `ActiveJobGuard::Drop` path already avoided this with
`unwrap_or_else(|e| e.into_inner())`. This slice brings the non-Drop
paths in line.

## Approach

Replace every `.lock().expect("active_jobs … poisoned")` in
`active_jobs.rs` with `.lock().unwrap_or_else(|e| e.into_inner())` — the
existing Drop-path idiom. `PoisonError::into_inner` hands back the
guard, so operations proceed on the recovered state. The trade-off
(operating on possibly-inconsistent state) is acceptable: poisoning
means a prior panic already happened, so serving degraded beats taking
the whole registry down. The replacement covered all matching sites in
the module, including one occurrence inside the test module — harmless
there (poison-leniency doesn't change any test's outcome) and kept for
consistency.

## Files changed

- `crates/blit-daemon/src/active_jobs.rs`: 9 `.expect(...)` lock sites →
  `unwrap_or_else(|e| e.into_inner())`; one new test.

## Tests

`blit-daemon` 141 (was 140; +1):

- `poisoned_table_mutex_recovers_instead_of_panicking` — registers a row,
  poisons the table mutex (panics while holding the lock, with the panic
  hook suppressed so test output stays clean), asserts the mutex is
  poisoned, then asserts `snapshot()` recovers the row instead of
  panicking. Pre-fix this panicked at the `expect`.

## Scope / next

Daemon-only, no wire/behavior change on the happy path (a non-poisoned
lock behaves identically). audit-3b will make `generate_token` return a
`Result` (RNG failure → `Status::Internal`) and update its 6 callers,
completing audit-3.

## Reviewer comments

(empty — pending review)
