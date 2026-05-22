# d-65-f1-push-mirror-move: mirror/move for the F1 push direction

**Severity**: Feature (designed — TUI_DESIGN §1 "copy / mirror / move")
**Status**: In progress / pending review
**Branch**: `phase5/a1`
**Commit**: `2e8e8a2`

## What

The F1 trigger handled local→remote **copy** push (d-61) but
not mirror/move — leaving the push direction's kind matrix
incomplete vs. remote→local (which has copy/mirror/move via
d-58…d-60). d-65 adds local→remote **mirror** and **move**.

## Approach

### Execution (reuses `run_remote_push`)

`spawn_f1_push` gains a `kind`:

- **Mirror**: `PushExecution.mirror_mode = true`,
  `mirror_kind = MirrorMode::All` (no filter → delete everything
  at the dest not in the source, matching `blit mirror`'s
  default scope). The daemon does the delete-extraneous.
- **Move**: a plain copy push, then — only after a **successful**
  push — delete the LOCAL source (`remove_local_source`: recursive
  for a dir, direct for a file). A delete failure surfaces as the
  op's error ("pushed but failed to delete local source: …"); a
  push failure never deletes. Mirrors the CLI's `run_move`
  local→remote arm.
- **Copy**: unchanged.

### Destructive confirm gate (in the trigger)

Mirror/move are destructive, so they need a y/N confirm. The
pull-side mirror/move confirm on F3 (d-60); the push side has no
F3 equivalent, so it confirms **in the trigger modal**:

- `plan_f1_trigger` now returns a `TriggerOutcome`
  (`Launched` / `NeedsConfirm` / `Rejected(msg)`). For a local
  source + destructive kind + valid remote dest, it returns
  `NeedsConfirm` unless `confirmed`.
- `F1TriggerState::Editing` gains a `confirming` flag;
  `begin_confirm` / `is_confirming` / `cancel_confirm`. The Enter
  handler opens the gate on `NeedsConfirm`; a modal
  `handle_f1_trigger_confirm_keystroke` (`y` re-runs `plan` with
  `confirmed = true` → launch + close; `n`/`Esc` → back to
  editing; else swallowed). The input router checks
  `is_confirming` before the edit handler.
- The prompt renders a red `<mode> <src> → <dst>? <detail> y/N`
  line — detail is "deletes extraneous at dest" (mirror) /
  "deletes the local source" (move).

### Footer verb

`F1PushStatus` carries the `kind`; the bridge maps it to the
push footer verb (`pushing`/`mirroring`/`moving` ·
`pushed`/`mirrored`/`moved`). Copy reads "push" (not "pull")
since this is the local→remote direction.

## Files changed

- `crates/blit-tui/src/f1push.rs`: `kind` on Running/Done/Error;
  `begin(label, kind)`; module doc.
- `crates/blit-tui/src/main.rs`: `spawn_f1_push(kind)` +
  `remove_local_source`; `plan_f1_trigger` → `TriggerOutcome`
  (+ destructive-push `NeedsConfirm`); Enter handler;
  `handle_f1_trigger_confirm_keystroke` + routing guard; push
  verb bridge.
- `crates/blit-tui/src/f1trigger.rs`: `confirming` flag +
  `begin_confirm` / `is_confirming` / `cancel_confirm`.
- `crates/blit-tui/src/screens/f1.rs`: `TriggerPrompt.confirm_detail`
  + confirm render; `PushStatusDisplay` `verb` fields + render.

## Tests

552 total (was 550):

main.rs: `..._mirror_push_confirms_then_launches` (Enter →
confirm gate, no launch; `y` → push runs, modal closed);
`..._confirm_cancel_returns_to_editing` (n/Esc → no push, back
to editing); `..._move_push_confirms_then_launches`. Existing
copy-push and validation tests still pass.

The mirror_mode wire flag + the move local-delete need a live
daemon (manual); the confirm gate, kind plumbing, and footer
verb are unit-tested.

## Known gaps

1. **remote→remote (delegated)** trigger still pending (no clean
   reusable execution entry — wires through the dispatch layer).
2. **Multi-daemon F2** still pending.

## Out of scope

- remote→remote; multi-daemon F2; F1 `d` diagnostics.

## Reviewer comments

(empty — pending grade)
