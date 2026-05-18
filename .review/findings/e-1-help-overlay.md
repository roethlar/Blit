# e-1-help-overlay: `?` opens a global help overlay

**Severity**: Feature (first slice of milestone E — polish)
**Status**: In progress / pending review
**Branch**: `phase5/a1`
**Commit**: filled by the sentinel commit

## What

Adds a `?` help overlay that lists every keybinding on
top of the active pane. Operator hits `?` to open, `?` or
Esc to close. While open, the overlay absorbs all
keystrokes EXCEPT:

- `?` / `Esc` — close the overlay.
- `Ctrl-c` — emergency quit.

F-keys, navigation, profile lifecycle, snapshot, and
text-input are all absorbed so the operator can study the
keymap without accidentally triggering a pane action.

a1-6's "known gap 2: no `?` help overlay" — closed.

## Approach

### State (`help.rs`)

Single-purpose module:

```rust
#[derive(Debug, Default, Clone, Copy)]
pub struct HelpOverlay {
    visible: bool,
}

impl HelpOverlay {
    pub fn is_visible(self) -> bool;
    pub fn toggle(&mut self);
    pub fn close(&mut self);
}

pub fn render_overlay(frame: &mut Frame, area: Rect);
```

`AppState` gains a `help: HelpOverlay` field. Visibility
persists across F-key navigation, so opening the help on
F1, switching to F2, the overlay is still up.

### Render

`render_overlay` paints a centered 64×18 modal (clamped
to area when smaller) with `Clear` underneath so it
isn't garbled by widgets beneath. Layout:

```
┌─ Help · press ? or Esc to close ──────────────────────┐
│ ▓ Navigation (global) ▓                               │
│        F1     Daemons pane                            │
│        F2     Transfers pane                          │
│        F3     Browse pane                             │
│        F4     Profile / Verify / Diagnostics          │
│         ?     toggle this help overlay                │
│   q / Esc     quit (Ctrl-c emergency)                 │
│                                                       │
│ ▓ Per-pane ▓                                          │
│         r     refresh / rescan                        │
│ ↑ ↓ / j k     cursor (F1, F3)                         │
│ Enter / → / l  descend (F3)                           │
│       ← / h   ascend (F3)                             │
│       Tab     enter / cycle Verify form (F4)          │
│ c / d / e     profile clear / disable / enable (F4)   │
│         s     diagnostics snapshot (F4)               │
└───────────────────────────────────────────────────────┘
```

### Key dispatch

`key_action` maps `Char('?')` to `UserAction::ToggleHelp`.
The unified loop's keystroke arm intercepts `ToggleHelp`
at the top match. When `app.help.is_visible()` is true,
the arm enters absorb-mode (only `?`/Esc close it; Ctrl-c
still quits).

## Files changed

- `crates/blit-tui/src/help.rs` (new):
  `HelpOverlay` + `render_overlay` + helpers + 4 unit
  tests.
- `crates/blit-tui/src/main.rs`:
  - `mod help;` declaration.
  - `AppState.help: HelpOverlay`.
  - `UserAction::ToggleHelp` variant.
  - `key_action` maps `?` → `ToggleHelp`.
  - Keystroke arm absorbs all keys while overlay is up
    (preserves `?`/Esc close + Ctrl-c).
  - Render loop calls `help::render_overlay` after the
    active pane when visible.

## Tests

+5 unit tests:

In `help::tests`:
- `toggle_flips_visibility`
- `close_sets_invisible_regardless_of_prior`
- `centered_clamps_to_area_when_smaller`
- `centered_returns_centered_rect_inside_area`

In `main::tests`:
- `key_action_maps_question_mark_to_toggle_help`

121 blit-tui unit tests (was 116). Workspace passes
serially.

## Known gaps

1. **Static keymap text.** The overlay lists every key
   regardless of which pane is active. A context-aware
   overlay (per-pane "here's what enter does *here*")
   would be future polish.

2. **No mouse close.** Clicking outside the overlay
   doesn't close it; the operator must press `?` or
   `Esc`. Mouse handling lands with the broader mouse
   support slice.

3. **No keybinding for typing `?` literally.** In Verify
   editing mode, `?` still toggles help rather than
   inserting the literal character. Operators who need
   `?` in a filename will have to drop focus (Esc) and
   type via a different path. Tracked under polish.

## Out of scope (next slices)

- **e-2 unified status bar** — replace each pane's
  footer with a single status line at the bottom of every
  screen (design §5).
- **e-3 themes / config** — `~/.config/blit/tui.toml` for
  refresh rates + color scheme.
- **e-4 mouse on tabs** — clickable F1..F4 tab strip.

## Round 2 (sha filled by sentinel)

Reviewer caught that `?` wasn't actually global: when the
F4 Verify form had focus, `handle_verify_keystroke`
intercepted every `Char(_)` as text input, so `?`
inserted the literal character instead of opening the
overlay. That's exactly the screen state where the
operator is most likely to want the keymap reference.

### Fix

`handle_verify_keystroke` now explicitly returns `false`
for `Char('?')` (without Ctrl/Alt modifiers) so the
dispatcher's `ToggleHelp` arm runs. Same exemption shape
as the existing F-key / Ctrl-c carve-outs.

```rust
if key.code == KeyCode::Char('?')
    && !key.modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
{
    return false;
}
```

(The known-gap "no keybinding for typing `?` literally"
is now load-bearing instead of incidental — operator who
needs `?` in a filename must drop focus first. Documented
in the finding doc.)

### Tests

+1 regression test (`handle_verify_keystroke_returns_false_for_question_mark`):
construct an AppState with Verify focused on Source,
send `Char('?')`, assert the handler returned `false`
AND the field is still empty (i.e. `?` wasn't inserted).

124 blit-tui unit tests (was 121). Workspace passes
serially.

## Reviewer comments

(empty — pending grade)
