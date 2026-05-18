# d-1-f4-profile-lifecycle: F4 [c]/[d]/[e] keys for perf-history lifecycle

**Severity**: Feature (first slice of milestone D)
**Status**: In progress / pending review
**Branch**: `phase5/a1` (sticking with the F4 changes from
A.1 rather than opening a new branch — same touch points)
**Commit**: filled by the sentinel commit

## What

Wires the perf-history lifecycle keys onto F4:

- `c` — clear the local perf-history file
  (`blit_core::perf_history::clear_history`).
- `d` — disable history recording
  (`set_perf_history_enabled(false)`).
- `e` — re-enable history recording
  (`set_perf_history_enabled(true)`).

These were deferred from `a1-5-f4-profile` because they
mutate persistent state and the TUI A.1 scope was strictly
read-only. With the router and unified loop landed, this
slice is small enough to ship on its own.

Each action triggers a `begin_fetch` + `spawn_profile_fetch`
so the F4 banner immediately reflects the post-action
state (record count drops to 0 after clear; the "history
recording: disabled" line flips after toggle).

The Ctrl-c quit shortcut is preserved — bare lowercase `c`
maps to `ProfileClear`; Ctrl-c continues to map to `Quit`
via the prior `should_quit` check.

## Approach

### `UserAction` extensions

```rust
enum UserAction {
    // ...existing variants...
    ProfileClear,   // 'c'
    ProfileDisable, // 'd'
    ProfileEnable,  // 'e'
}
```

`key_action` recognizes bare `c` / `d` / `e` (uppercase
stays unmapped — case-sensitive per the design). F1/F2/F3
match arms with `_ => {}` wildcards naturally ignore the
new variants; only F4's pane action match interprets them.

### Helpers

Two thin wrappers around `blit_core::perf_history`:

```rust
fn apply_profile_clear(profile_state: &mut ProfileState) {
    match clear_history() {
        Ok(_) => {}
        Err(err) => profile_state.note_fetch_error(...)
    }
}

fn apply_profile_set_enabled(profile_state: &mut ProfileState, enabled: bool) { ... }
```

Both are sync (file I/O on the config dir) — fast enough
to call from the async handler without `spawn_blocking`.
Errors surface in the profile state's `Error` banner.

### Render

F4 footer adds three new key hints:

```
status · q/Esc quit · r refresh · c clear · d disable · e enable
```

## Files changed

- `crates/blit-tui/src/main.rs`:
  - `UserAction` gains three variants.
  - `key_action` maps `c` / `d` / `e`.
  - F4 `handle_pane_action` arm interprets the new
    variants; each calls a helper + kicks a profile
    re-fetch.
  - New `apply_profile_clear` / `apply_profile_set_enabled`
    helpers.
- `crates/blit-tui/src/screens/f4.rs`: footer key-hint
  spans.

## Tests

+1 unit test:
- `key_action_maps_profile_lifecycle_keys`: covers `c`/`d`/
  `e` → correct variants; uppercase stays unmapped; Ctrl-c
  still maps to Quit (not ProfileClear).

96 blit-tui unit tests (was 95). Workspace passes
serially.

## Known gaps

1. **No confirmation modal for `c`.** Design says `[c]
   clear` with no confirmation; this slice follows that.
   Future polish could add a confirmation dialog given how
   destructive the action is.

2. **No success indicator.** A successful clear/disable/
   enable is signaled only by the resulting state visible
   in the report (record count, history-enabled line). A
   future polish could flash a transient "cleared" /
   "disabled" / "enabled" message in the status banner.

3. **Inline file I/O.** `clear_history` and
   `set_perf_history_enabled` touch the config dir
   synchronously from the async handler. The operations
   are small and fast in practice (single-file ops); a
   pathologically slow filesystem could briefly stall the
   event loop. `spawn_blocking` is a future polish.

## Out of scope (next D slices)

- **d-2-f4-verify**: Verify pane (source/destination input
  fields + check execution + diff render).
- **d-3-f4-diagnostics**: Diagnostics dump button.

## Round 2 (sha filled by sentinel)

Reviewer caught a real bug: round-1's F4 arms called the
mutating helper THEN unconditionally kicked
`begin_fetch + spawn_profile_fetch`. If the mutation
failed, `note_fetch_error` set the banner to
`Error(message)`, but `begin_fetch` flipped it right back
to `Pending`. The subsequent successful read landed
`Loaded` and the operator never saw the failure.

### Fix: helpers return Result; caller only refetches on Ok

`apply_profile_clear` and `apply_profile_set_enabled` now
return `Result<(), String>` (no longer take a `&mut
ProfileState`). A new helper `apply_lifecycle_outcome(state,
result) -> bool` decides what to do with the outcome:

- `Ok(())` → returns `true`, status untouched, caller
  proceeds to kick a re-fetch.
- `Err(msg)` → calls `state.note_fetch_error(msg)`, returns
  `false`, caller SKIPS the re-fetch so the error banner
  survives.

F4 match arms:

```rust
UserAction::ProfileClear => {
    let outcome = apply_profile_clear();
    if apply_lifecycle_outcome(&mut app.profile, outcome) {
        let id = app.profile.begin_fetch();
        spawn_profile_fetch(id, app.profile_reply_tx.clone());
    }
}
```

### Tests

+2 unit tests:
- `apply_lifecycle_outcome_preserves_error_and_skips_fetch`
  — Err path returns false; status is the Error banner;
  caller knows not to refetch.
- `apply_lifecycle_outcome_ok_signals_refetch_without_banner_change`
  — Ok path returns true; status untouched (caller's
  `begin_fetch` flips it to Pending).

98 blit-tui unit tests (was 96). Workspace passes serially.

## Reviewer comments

(empty — pending grade)
