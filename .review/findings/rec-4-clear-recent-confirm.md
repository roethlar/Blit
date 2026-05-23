# rec-4-clear-recent-confirm: Y/n confirmation on the F2 clear-recent action

**Severity**: Feature (recent-persistence follow-up)
**Status**: In progress / pending review
**Branch**: `phase5/a1`
**Commit**: `3673ee1`
**Parent**: rec-3 (F2 `E` clear-recent). Owner-requested enhancement
(2026-05-23): "single Y/n confirmation prompt, otherwise good"
(memory `audit-owner-decisions`).

## What

rec-3 made F2 `E` clear the recent list immediately. The owner asked for
a single confirmation first. Now `E` arms a `clear recent? y/N` footer
prompt; `y` clears, `n`/`Esc` aborts. Active transfers and planner
telemetry remain untouched (the clear path is unchanged from rec-3).

## Approach

Reuse the existing F2 confirm state machine (`F2CancelStatus`) rather
than a parallel flag, so the established plumbing applies for free:

- **`F2CancelStatus::ConfirmingClearRecent`** (new, no fields — the
  clear is the whole list) + included in `is_confirming()`.
- **`E`** (`UserAction::ClearRecent`), now **guarded** so it only arms
  the confirm when no cancel is mid-cycle (`!is_sending() &&
  !is_confirming()`) — the single y/N prompt is never ambiguous. The
  actual clear (empty local `recent` view + fan a `ClearRecent` RPC to
  each watched daemon — the rec-3 path) moves into the `y` handler.
- **`y`** (`TransferMirrorConfirm if is_confirming()`): an early branch
  handles `ConfirmingClearRecent` (reset to Idle, then clear + fan-out),
  before the cancel-confirm logic (which carries payload this variant
  doesn't).
- **`n` / `Esc`**: already revert any confirming state to Idle — applies
  unchanged, so abort works for free.
- **K / Shift+X** (cancel): already guard on `is_sending() ||
  is_confirming()`, so they won't start over an active clear-recent
  confirm — mutual exclusion for free.
- **Footer**: `F2CancelDisplay::ConfirmingClearRecent` →
  `clear recent? y/N` (yellow), mirroring the cancel-confirm fragment.

## Files changed

- `crates/blit-tui/src/main.rs`: `F2CancelStatus::ConfirmingClearRecent`;
  `is_confirming()`; guarded `E`; `y` early branch; `cancel_status_to_display`
  mapping; tests.
- `crates/blit-tui/src/screens/f2.rs`: `F2CancelDisplay::ConfirmingClearRecent`
  + footer render arm.

## Tests

`blit-tui` 624 (+2):

- `f2_clear_recent_confirm_is_a_confirming_state` — `ConfirmingClearRecent`
  is a confirming (not sending) state, so Esc/`n`/K-guard all apply.
- `cancel_status_to_display_renders_confirming_clear_recent` — maps to the
  `clear recent? y/N` footer display variant.

(Consistent with the existing cancel-confirm test depth — predicates +
display mapping are unit-tested; the full keystroke→dispatch flow is
exercised at runtime, as for the cancel/mirror confirms.)

## Reviewer comments

(empty — pending review)
