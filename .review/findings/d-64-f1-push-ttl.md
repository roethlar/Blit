# d-64-f1-push-ttl: auto-hide the F1 push outcome footer

**Severity**: Feature (closes d-61 known gap #3)
**Status**: In progress / pending review
**Branch**: `phase5/a1`
**Commit**: `0ae7069`

## What

The F1 push Done/Error footer (`pushed N · X → dest` /
`push failed: …`) persisted until the next push began — every
other transfer outcome (F3 pull d-38, F3 delete d-52, F2 cancel
d-23) auto-hides on a TTL. d-64 brings the push in line.

## Approach

Direct port of the verified pull TTL recipe (d-38/d-40):

- `F1PushStatus::Done` / `Error` gain `finished_at: Instant`;
  `apply_done` / `apply_error` re-gain an `at` param to stamp it.
- `F1PushState` gets `is_terminal()`,
  `clear_terminal_if_expired(now, ttl)`, and
  `terminal_remaining(now, ttl)` — same shape as
  `F3PullState`.
- The frame loop calls `clear_terminal_if_expired(now, push_ttl)`.
- `compute_tick_budget` folds the F1 push `terminal_remaining`
  (gated on `Screen::F1`) into the sleep budget via a nested
  `min_opt`, so a short `push_status_ttl_ms` collapses the budget
  exactly like the d-24 cancel / d-40 pull TTLs — a long
  `live_tick.interval_ms` can't delay it.
- `needs_live_tick` returns true while a push terminal shows, so
  the loop keeps ticking until the auto-hide fires.

### Config

A new `[transfer] push_status_ttl_ms` knob (default 5000, clamped
`[250, 60000]`), mirroring `delete_status_ttl_ms` exactly:
constants, clamped accessor, `Default`, and the schema-doc line.
Read each frame so a `Ctrl+R` reload retunes it live.

## Files changed

- `crates/blit-tui/src/f1push.rs`: `finished_at` on Done/Error;
  `at` param on apply_*; `is_terminal` / `clear_terminal_if_expired`
  / `terminal_remaining`; TTL tests.
- `crates/blit-tui/src/config.rs`: `push_status_ttl_ms` field +
  constants + clamped accessor + Default + schema doc; 2 tests.
- `crates/blit-tui/src/main.rs`: frame clear; `push_remaining` in
  the tick budget; `needs_live_tick` push terminal; select arm
  passes `at`.

## Tests

550 total (was 544):

f1push.rs: Done/Error are terminal (Running/Idle aren't); clear
hides after TTL; clear is a no-op on Running; terminal_remaining
some-within / none-after / none-on-running.

config.rs: default 5000; parse + floor/ceiling clamp for
`push_status_ttl_ms`.

The tick-budget integration mirrors the verified pull path; the
TTL math is unit-tested.

## Known gaps

1. **Push mirror/move, remote→remote** still pending — d-64 is
   the last of the push UX-polish items.

## Out of scope

- Push mirror/move; remote→remote; F1 `d` diagnostics;
  multi-daemon F2.

## Reviewer comments

(empty — pending grade)
