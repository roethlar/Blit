# d-59-f1-trigger-mirror: copy/mirror toggle in the F1 trigger

**Severity**: Feature (designed — TUI_DESIGN §3 / §5.2)
**Status**: In progress / pending review
**Branch**: `phase5/a1`
**Commit**: `1725414`

## What

d-58 shipped the F1 `t` trigger-transfer modal as copy-only.
TUI_DESIGN describes the launcher as "the same flow as copy with
a `--mirror` flag in the options modal" (§3 mapping table,
`blit mirror` row). d-59 adds that flag: a copy ⇄ mirror mode
toggle in the modal.

## Approach

- `F1TriggerState::Editing` gains a `mirror: bool` (copy by
  default); `toggle_mirror()` flips it, bound to the **Up/Down**
  arrows in the keystroke handler (arrows aren't text, so they
  don't collide with path entry). `take()` now returns
  `(source, dest, mirror)`.
- The Enter handler branches on `mirror`:
  - **copy** → `start_pull` (direct launch + spawn), as d-58.
  - **mirror** → routes through the *verified* F3 destructive
    confirm: `begin_mirror(source)` → push the dest chars →
    `begin_run()` (lands in `Confirm`, returns `None` — **no
    spawn**) → jump to F3. The operator confirms y/N on F3,
    where the destructive-confirm handler + the mirror execution
    already live (d-55/57). So d-59 adds **no new execution
    path** — it reuses the trigger hand-off pattern and the F3
    confirm gate wholesale.
- Render: the prompt shows a mode tag — green `[copy]` /
  red `[mirror]` (red flags the destructive option) — and the
  hint gains `↑↓ copy/mirror`. `TriggerPrompt` carries the
  `mirror` bool; `screens/f1.rs` stays decoupled from
  `F1TriggerState`.

## Files changed

- `crates/blit-tui/src/f1trigger.rs`: `mirror` field;
  `toggle_mirror`; `take` returns the triple; module doc.
- `crates/blit-tui/src/main.rs`: Up/Down → `toggle_mirror`;
  Enter branches copy vs mirror (mirror → F3 confirm gate);
  bridge passes `mirror`.
- `crates/blit-tui/src/screens/f1.rs`: `TriggerPrompt.mirror`;
  mode tag + hint in `render_trigger`.

## Tests

526 total (was 524):

f1trigger.rs: begin starts in copy mode;
`toggle_mirror_flips_mode_and_take_reports_it`.

main.rs: `handle_f1_trigger_keystroke_mirror_routes_to_f3_confirm`
— Up flips to mirror, Enter lands at the F3 `Confirm` gate (NOT
a direct launch / not Running) and jumps to F3. The copy-path
Enter test (d-58) still covers the direct launch.

The mirror execution itself is the verified F3 path (d-55/57);
d-59's tests cover the toggle + the copy-vs-mirror hand-off
branch.

## Known gaps

1. **Move not offered.** The toggle is copy ⇄ mirror only
   (matching the design's "copy with --mirror"); move from a
   typed spec isn't exposed (move needs the careful module-root
   gating F3 `v` does against a browsed row).
2. **Push / remote→remote still pending** (d-58 gap #1) — the
   trigger remains remote→local.
3. **No inline parse-error feedback** (d-58 gap #2).

## Out of scope

- Push / remote→remote triggers; F1 `d` diagnostics.

## Reviewer comments

(empty — pending grade)
