# d-48-f2-follows-browse: F2 follows the F1 daemon switch

**Severity**: Feature (closes d-47 known gap #1)
**Status**: In progress / pending review
**Branch**: `phase5/a1`
**Commit**: `f566a35`

## What

d-47 let `enter` on an F1 daemon row retarget the **F3 browser**
to that daemon, but left **F2** watching the launch daemon's
transfers — its documented known gap. d-48 closes it: switching
daemons on F1 now also re-subscribes F2, so its active/recent
transfers track the daemon you switched to. F1 `enter` becomes a
true "switch active daemon" for both panes.

## Approach

The F1 `Descend` arm runs inside `handle_pane_action`, which has
both the loop's `transfers_event_rx` and the `f2_setup_tx` in
scope — so it can drive the F2 lifecycle directly.

New `reset_f2_for_resubscribe(app, endpoint, &mut transfers_event_rx)`:
- repoints `parsed_remote` + `remote_label` to the new daemon,
- clears `transfers` (drops the old daemon's rows),
- sets status `Connecting`,
- drops the old stream (`*transfers_event_rx = None`),
- marks a setup pending and **bumps `transfers_setup_gen`**,
- returns the new generation.

The arm then calls `spawn_f2_setup_task(endpoint, gen, …)`. The
loop's existing `f2_setup_rx` arm applies the reply **only if its
gen matches** (the a1-6b round-3 gate), so a slow setup reply
from the *previous* daemon — spawned before the switch — is
dropped instead of populating the stream for the wrong daemon.
This reuses the exact startup / `r`-refresh path; no new
lifecycle, just retargeted.

`browse_target` (F3) and `parsed_remote` (F2) now both move to
the selected daemon on `enter`, so the two panes stay in sync
while remaining distinct fields (a future "browse X while
watching Y" could re-diverge them).

## Files changed

- `crates/blit-tui/src/main.rs`:
  - `reset_f2_for_resubscribe` helper.
  - F1 `Descend` arm: `retarget_browse` (F3) + reset + spawn (F2).
  - 1 test.

## Tests

+1 test (461 → 462):

- `reset_f2_for_resubscribe_repoints_and_bumps_generation` —
  repoints `parsed_remote`, drops the old `event_rx`, marks a
  setup pending, bumps the generation by exactly 1, sets status
  `Connecting`, and clears the old rows.

The spawn + gen-gated reply application is the same path the
startup and `r`-refresh tests already exercise; the new state
reset is the part unique to d-48 and is unit-tested.

## Known gaps

1. **No multi-daemon F2.** F2 watches exactly one daemon at a
   time (the active one). TUI_DESIGN's "single pane of glass"
   ideal is to watch *all* discovered daemons at once; that's a
   bigger feature (N concurrent Subscribe streams + merge). d-48
   delivers "F2 follows the active daemon," which is the
   single-stream step.
2. **In-flight transfers on the old daemon vanish from view.**
   Switching daemons clears F2's rows for the previous daemon
   (they're still running server-side; `blit jobs watch` or
   switching back shows them). Acceptable — F2 reflects the
   active daemon.

## Out of scope

- Concurrent multi-daemon F2.
- Preserving the previous daemon's F2 rows after a switch.

## Reviewer comments

(empty — pending grade)
