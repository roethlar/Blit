# a1-6-screen-router: in-app F-key navigation between panes

**Severity**: Feature (sixth and final core slice of milestone A.1)
**Status**: In progress / pending review
**Branch**: `phase5/a1`
**Commit**: filled by the sentinel commit

## What

Adds in-app F-key navigation: F1 → Daemons, F2 → Transfers,
F3 → Browse, F4 → Profile. Operator can hit any F-key
from any pane to switch.

A tab strip at the top of every pane shows which one is
active (bold + cyan) and which are available (dim).

`--screen` keystroke is preserved as the *initial* pane;
default flipped from F2 to F1 (the natural entry point —
operator scans the LAN, then drills in).

## Approach

### LoopOutcome return

Each per-pane event loop (`run_f1_event_loop`,
`run_f2_event_loop`, etc.) now returns
`Result<LoopOutcome>` where:

```rust
pub enum LoopOutcome {
    Quit,
    Navigate(Screen),
}
```

The pane loops intercept `UserAction::Navigate(target)` in
their keystroke match arms and return
`Ok(LoopOutcome::Navigate(target))`. The router (top-level
`run_router`) then re-enters the named pane's loop.

### F-key recognition

`key_action` checks for `KeyCode::F(n)` before any other
key — F1..F4 map to `UserAction::Navigate(Screen::FN)`.
F5+ are intentionally unmapped (reserved for future help /
settings / etc.).

### Tab strip

`screens::split_for_tabs(area) -> (tab_area, body_area)`
splits the available area into a one-line tab strip on top
and the rest for the body. `screens::render_tab_strip`
paints the active pane in inverse-video / cyan and the
others dim.

Each pane's `render` was renamed `render_into` and now
accepts an explicit `area: Rect` (the `body_area` from the
router's split). The pane's internal layout is unchanged.

### `Screen` enum + `ScreenArg` translation

`Screen { F1, F2, F3, F4 }` is the internal pane
identifier used by `LoopOutcome::Navigate` and the tab
strip. `From<ScreenArg>` translates the CLI value-enum
into a `Screen` for the router's initial pane.

## Files changed

- `crates/blit-tui/src/screens/mod.rs`: `split_for_tabs` +
  `render_tab_strip` helpers; the `f3`/`f4` pub mod lines
  unchanged.
- `crates/blit-tui/src/screens/{f1,f2,f3,f4}.rs`: rename
  `render` → `render_into`, drop the area parameter from
  `frame.area()` and accept it from the caller.
- `crates/blit-tui/src/main.rs`:
  - `Screen` enum + `From<ScreenArg>` impl.
  - `LoopOutcome { Quit, Navigate(Screen) }`.
  - `UserAction::Navigate(Screen)` variant.
  - `key_action` recognises F1..F4.
  - Each `run_fN_event_loop` returns `Result<LoopOutcome>`
    and intercepts Navigate.
  - Each pane's draw closure splits the frame area and
    paints the tab strip.
  - New `run_router` function loops on the active pane's
    outcome.
  - `main` dispatches through `run_router`.
  - `Args.screen` default flipped F2 → F1.

## Tests added

2 new unit tests in `main::tests`:

- `key_action_maps_f_keys_to_navigate`: F1..F4 → Navigate;
  F5/F12 unmapped.
- `screen_arg_to_screen_mapping_is_total`: each ScreenArg
  variant maps to its Screen counterpart (catches future
  drift if a ScreenArg variant gets added without
  updating the `From` impl).

94 blit-tui unit tests (was 92). Workspace passes
serially.

## Known gaps

1. **State loss on navigation (Medium UX cost).** Each
   `LoopOutcome::Navigate` returns to the router, which
   re-enters the destination pane's loop from scratch.
   This means:
   - F1 reruns mDNS discovery on every visit (~5s rescan
     interval; first scan blocks the operator for up to
     1.5s).
   - F2 reopens the Subscribe stream and re-fires
     GetState on every visit (~2× RTT to the daemon).
   - F3 forgets the operator's tree position.
   - F4 re-reads `perf_local.jsonl` on every visit.

   State preservation requires hoisting per-pane state +
   background tasks into a shared `AppState` and unifying
   the four loops into one. That's a substantial refactor
   on top of the routing primitive landed here.

   **Split**: a follow-up finding `a1-6b-state-preservation`
   tracks the work. Adding it to REVIEW.md alongside this
   sentinel.

2. **No `?` help overlay.** Design references `?: help` in
   the status bar. Future polish.

3. **No status bar.** The design shows a unified status bar
   at the bottom of every pane (`tab: switch panel │
   enter: drill in │ / : search │ ? : help │ q`). Today
   each pane has its own footer; consolidating them is
   future polish.

4. **Tab strip is non-interactive.** Operator clicks the
   F-key, not a mouse-on-tab gesture. Mouse support is
   a polish slice.

## Out of scope (next slices)

- **a1-6b-state-preservation**: hoist all four panes'
  state + background tasks into a shared `AppState` so
  navigation doesn't restart fetches.
- **Milestone D (Verify/diagnostics screens)** — see
  REVIEW.md open findings.
- **Milestone E (Polish: themes, refresh rates, config)**
  — including the unified status bar, `?` help overlay,
  mouse-on-tab navigation.

## Reviewer comments

(empty — pending grade)
