# d-2-f4-verify: F4 Verify pane (source/destination form + compare_trees)

**Severity**: Feature (second slice of milestone D)
**Status**: In progress / pending review
**Branch**: `phase5/a1`
**Commit**: filled by the sentinel commit

## What

Adds the F4 Verify form. Operator hits `Tab` to enter
editing mode (Source field → Destination → none → Source
…), types local paths, presses `Enter` to run
`blit_app::check::compare_trees`. The Verify block on F4
renders the form fields plus a result line showing
match/diff/missing counts. `Esc` (while editing) drops
focus without quitting the TUI.

Atomic scope: local paths, size+mtime mode, two-way
comparison. Mode toggle (size+mtime ↔ checksum) is a
deferred polish. Remote endpoints are out of scope per
the design (`blit check` itself is local-only — see
TUI_DESIGN §5.4).

## Approach

### State (`verify.rs`)

```rust
pub enum VerifyFocus { None, Source, Destination }

pub enum VerifyStatus {
    Idle,
    Running,
    Done { result: CheckResult, finished_at: Instant },
    Error { message: String },
}

pub struct VerifyState {
    pub source: String,
    pub destination: String,
    focus: VerifyFocus,
    status: VerifyStatus,
    request_id: u64,    // generation guard
}
```

Surface:
- `cycle_focus()` / `clear_focus()` — Tab / Esc
- `insert_char(c)` / `backspace()` — text edits (no-op
  when focus == None)
- `begin_run() -> u64` / `apply_result(id, r)` / `apply_error(id, msg)`
  — same generation pattern as `transfers_setup_gen` to
  drop stale replies when the operator edits and re-runs.
- `can_run() -> bool` — both fields non-empty and not
  already running.
- Editing (`insert_char`/`backspace`) invalidates `Done`
  or `Error` status by flipping back to `Idle`, so the
  operator's most recent counts aren't shown alongside a
  freshly-edited path.

### Run task

`spawn_verify_run(gen, source, destination, tx)` runs
`compare_trees` on a `spawn_blocking` task. The reply
`VerifyReply { request_id, result: Result<CheckResult,
String> }` lands on the unified loop's `verify_run_rx`.

### Key dispatch

The unified event loop's keystroke arm gains a
text-input-mode branch: when F4 is active AND
`verify.focus().is_editing()`, the new
`handle_verify_keystroke(&key, &mut app, &verify_run_tx)`
helper intercepts. It returns:
- `true` when the keystroke was consumed (text edit, Tab,
  Enter, Esc, Backspace).
- `false` to fall through to the action dispatcher
  (F-keys for navigation, Ctrl-c for emergency quit).

Tab works in both modes: from non-editing it enters the
form; from editing it cycles through the fields and back
to non-editing.

Esc while editing → `clear_focus()` (does NOT quit). Esc
while not editing → Quit (unchanged).

Navigating away from F4 (via F1/F2/F3 keys) clears focus
so the next F4 visit starts in action-key mode (so
`c`/`d`/`e`/`r` work as profile lifecycle keys again).

### Render (`screens/f4.rs`)

F4 layout becomes five regions: header / records summary /
predictor block / Verify block / footer. The Verify block
has four lines:

```
Source: /tmp/a▏           ← cursor caret when focused
Destin: /tmp/b
Mode: size+mtime (checksum toggle deferred)
matches: 42 · differ: 1 · missing-on-src: 0 · missing-on-dst: 3 · errors: 0
```

When focused, the field's value spans get a cyan inverse-
video style + cursor caret. Empty unfocused fields render
"(empty)" in dim gray.

Footer hint line swaps between "Tab/Enter/Esc" while
editing and "q/Esc/r/c/d/e + Tab verify" while not.

## Files changed

- `crates/blit-tui/src/verify.rs` (new): `VerifyState`,
  `VerifyFocus`, `VerifyStatus` + 9 unit tests.
- `crates/blit-tui/src/screens/f4.rs`:
  `render_into` signature gains `&VerifyState`; new
  `render_verify` block + `field_line` helper; footer
  swaps hints based on focus.
- `crates/blit-tui/src/main.rs`:
  - `mod verify;` declaration.
  - AppState gains `verify` field.
  - Unified loop's keystroke arm dispatches to
    `handle_verify_keystroke` when F4 is active and the
    form has focus.
  - Tab handling (cycle focus on F4 in either mode).
  - Navigation away from F4 clears focus.
  - New `VerifyReply` envelope + `spawn_verify_run` + a
    new select! arm to apply replies.
  - New verify_run_rx channel set up in the router.
  - `handle_verify_keystroke` async helper.

## Tests

+9 unit tests in `verify::tests`:

- `new_state_starts_idle_with_no_focus`
- `cycle_focus_walks_three_states_then_returns_to_none`
- `focus_is_editing_only_in_field_states`
- `insert_char_targets_focused_field`
- `backspace_pops_focused_field`
- `editing_invalidates_done_or_error_status`
- `can_run_requires_both_fields_and_not_running`
- `apply_result_drops_stale_generation`
- `clear_focus_resets_to_none`

107 blit-tui unit tests (was 98). Workspace passes
serially.

## Known gaps

1. **No checksum mode toggle.** Design's `(•) size+mtime
   ( ) checksum` radio is out of scope. Future polish.

2. **No diff drill-down.** Result is a single counts line.
   The design's row-by-row diff render (matches /
   size-diff / mtime-diff / missing-on-side) is deferred;
   a sub-pane that lists `result.differing[]` entries
   could land alongside cursor navigation.

3. **No tilde / relative path expansion.** Operator must
   type absolute paths. `~` and `./` expansion is future
   polish.

4. **Inline `KeyCode::Char` accept ASCII only by
   convention.** Unicode characters work through
   `Char(c)` but visual fit in a single-line field is
   not tested for wide chars.

5. **No history of past runs.** Each Enter overwrites the
   previous result. A scrollback or "last 5 runs" view
   could land as polish.

## Out of scope (next D slices)

- **d-3-f4-diagnostics**: `dump` snapshot button that
  saves a JSON file (reuses the same source/dest form
  inputs).
- **Mode toggle for checksum.**
- **Result drill-down (per-file diff listing).**

## Round 2 (sha filled by sentinel)

Reviewer caught a real race: round 1 allowed text edits
during `Running`, but the edits only invalidated `Done` /
`Error` — they didn't bump the generation. The in-flight
`compare_trees` task's reply could land against the now-
edited paths and render an old comparison.

### Fix: edits bump generation + collapse Running to Idle

Refactor `insert_char` and `backspace` to call a new
`invalidate_run` helper whenever they actually mutate a
field. `invalidate_run`:

1. Bumps `request_id`. Any in-flight reply tagged with the
   prior generation will be dropped by `apply_result` /
   `apply_error` on arrival.
2. Collapses status: `Running` / `Done` / `Error` → `Idle`.
   The renderer stops showing "running compare_trees..."
   immediately so the operator doesn't see a status line
   that won't be honored.

### Tests

+2 unit tests in `verify::tests`:

- `edit_during_running_drops_in_flight_reply`: type into
  the form mid-run, then deliver the old reply — must be
  refused and status must still be `Idle`. Both
  `apply_result` and `apply_error` are exercised against
  the stale generation.
- `backspace_during_running_drops_in_flight_reply`: same
  shape but using Backspace instead of typing.

109 blit-tui unit tests (was 107). Workspace passes
serially.

## Reviewer comments

(empty — pending grade)
