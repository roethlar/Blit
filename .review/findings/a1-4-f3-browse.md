# a1-4-f3-browse: F3 Browse pane (modules + directory tree)

**Severity**: Feature (fourth slice of milestone A.1 — adds F3)
**Status**: In progress / pending review
**Branch**: `phase5/a1`
**Commit**: filled by the sentinel commit

## What

Adds the F3 Browse screen to `blit-tui`. Operator opts in
via `blit-tui --screen f3 --remote <host>`. The pane:

1. Starts at the daemon's module list (`list_modules` RPC).
2. Cursor + Enter (or →) descends into the selected module's
   root.
3. Cursor + Enter descends into a subdirectory; ← (or `h`)
   pops one level (back to module list at module root).
4. Each navigation kicks a fresh `list` RPC for the new
   view; replies are tagged with a generation id so a stale
   result from before navigation is dropped on the floor.
5. `r` invalidates the current view and re-fetches.

a1-2's F2 and a1-3's F1 are unchanged and remain accessible
via `--screen f2` / `--screen f1`. a1-6 replaces `--screen`
with in-app F-key routing.

## Approach

### State (`browse.rs`)

```rust
pub enum BrowseView {
    Modules,                                          // top level
    Module { name: String, path: Vec<String> },       // inside a module
}

pub enum BrowseRowKind {
    Module { read_only: bool },
    Directory,
    File,
}

pub enum BrowseFetchStatus {
    Idle, Pending,
    Loaded { fetched_at: Instant },
    Error { message: String },
}

pub struct BrowseState {
    view, rows: Vec<BrowseRow>, selected,
    status, pending_request_id: u64,
}
```

Reducer surface:

- `apply_modules(Vec<Module>, fetched_at)` — replaces rows,
  refuses if the view has navigated away (stale reply).
- `apply_listing(for_module, for_path, entries, fetched_at)` —
  same, with module-name + path equality check for stale
  rejection.
- `note_fetch_error(message)` — keeps existing rows
  visible, flips status to Error.
- `descend()` — Module row → enter root; Directory row →
  push name; File row → no-op. Clears rows and resets
  status to Idle so the next loop tick kicks a fetch.
- `ascend()` — pops path; empty path pops back to Modules;
  Modules → no-op.
- `select_next` / `select_prev` — bounded cursor movement.
- `begin_fetch() -> u64` / `is_current_request(id) -> bool`
  — generation tracking, same pattern as `DaemonsState` in
  a1-3b.
- `breadcrumb() -> String` — `"modules"`, `"home"`, or
  `"home/photos/2024"`.

### Event loop (`run_f3_event_loop` in `main.rs`)

```rust
loop {
    if endpoint.is_some() && views_differ(last_fetched.as_ref(), state.view())
       && status is Idle or Error {
        kick_browse_fetch(state, endpoint, fetch_tx);
        last_fetched = Some(state.view().clone());
    }
    draw(state);
    select! {
        key = key_rx.recv() => match key_action() { Quit | Refresh | SelectNext |
            SelectPrev | Descend | Ascend },
        reply = fetch_rx.recv() => apply_browse_reply(state, reply),
    }
}
```

- `kick_browse_fetch` bumps the per-view request_id, sets
  Pending, and spawns either `list_modules::query` or
  `ls::list_remote` depending on the view shape.
- `apply_browse_reply` drops stale generations
  (`!state.is_current_request(id)`); otherwise routes
  Modules → apply_modules, Listing → apply_listing, Error →
  note_fetch_error.
- `r` keystroke bumps the generation (so any in-flight
  reply is dropped) and resets `last_fetched` so the next
  tick re-kicks.

Missing or malformed `--remote` lands a Degraded-style error
in the stats banner and leaves the keystroke path alive so
the operator can read it and quit.

### Shared `UserAction` extensions

`UserAction` gains `Descend` and `Ascend`. `key_action` now
maps:

- Enter / → / `l` → Descend
- ← / `h` → Ascend

F1 / F2 match arms explicitly ignore both new variants (no
cursor-tree semantics in those panes today; a1-6 routing will
repurpose Enter to switch panes).

### Render (`screens/f3.rs`)

Same shape as F2 / F1: header / table / stats block / footer.

- Table columns: name · kind · size · mtime.
- `kind` renders "module" / "module (ro)" / "dir" / "file".
- `size` empty for non-File rows.
- `mtime` formats `seconds_since_epoch` to `YYYY-MM-DD`
  via a self-contained Howard-Hinnant inverse (no chrono
  dep needed; covered by a unit test against known dates).
- Footer status reuses the BrowseFetchStatus variants and
  the same "loaded · Xs ago" pattern from F1.
- Stateful Table widget + `TableState::with_selected` for
  viewport-aware highlighting (same fix landed in a1-3
  round 2).

## Files changed

- `crates/blit-tui/src/browse.rs` (new): state model + 11
  unit tests.
- `crates/blit-tui/src/screens/f3.rs` (new): render
  function + format helpers + 4 unit tests.
- `crates/blit-tui/src/screens/mod.rs`: `pub mod f3;` added.
- `crates/blit-tui/src/main.rs`:
  - `mod browse;` declaration.
  - `ScreenArg::F3` plus the dispatch arm.
  - `UserAction::Descend`/`Ascend`; `key_action` recognises
    Enter/Left/Right + vim h/l.
  - F1 / F2 match arms updated to ignore the new variants.
  - `run_f3_event_loop` + `kick_browse_fetch` +
    `apply_browse_reply` + `BrowseFetchReply` /
    `BrowseFetchPayload` envelopes.
  - `views_differ` helper.
  - +3 unit tests (`key_action_maps_f3_navigation`,
    `views_differ_module_path_compare`, and `Enter` removed
    from the unmapped-keys assertion).

## Tests added

18 new unit tests:

In `browse::tests`:
- `new_starts_in_modules_view`
- `apply_modules_populates_rows_and_resets_cursor`
- `descend_into_module_switches_view_and_clears_rows`
- `descend_into_directory_pushes_onto_path`
- `descend_on_file_is_no_op`
- `ascend_pops_path_then_returns_to_modules`
- `select_next_prev_bounded`
- `apply_modules_dropped_when_view_changed` (stale reply
  during nav)
- `apply_listing_dropped_when_path_no_longer_matches`
  (stale reply during nav)
- `begin_fetch_and_is_current_request_track_generations`
- `breadcrumb_reflects_current_view`
- `note_fetch_error_preserves_rows`

In `screens::f3::tests`:
- `kind_label_covers_each_variant`
- `format_bytes_picks_correct_unit`
- `format_mtime_handles_zero_and_negative`
- `days_to_ymd_matches_known_dates`

In `main::tests`:
- `key_action_maps_f3_navigation`
- `views_differ_module_path_compare`

80 blit-tui unit tests (was 62). Workspace passes serially.

## Known gaps

1. **No multi-select / transfer trigger.** The design shows
   `space` for multi-select and `c`/`m`/`v` for copy /
   mirror / move. Future slices (a1-7+ or post-A.1) wire
   transfer actions; this slice is read-only browsing.

2. **No find modal (`/`).** Streaming `Find` results into
   a flat list is its own slice — out of scope here.

3. **No `du` calculation per cursor row.** Sizes shown come
   from the `list` RPC entry size, not a recursive subtree
   sum. The design's "subtree: 14.2 GiB across 8,442 files"
   stats line is deferred.

4. **No automatic per-view refresh.** F3 fetches once per
   navigation; `r` re-fetches manually. No background polling
   today.

5. **Subdirectory listings on slow daemons feel sluggish.**
   No prefetch / no progress-while-loading. The footer
   reads "fetching..." and the table goes empty in between.
   Future polish.

6. **No render test against TestBackend with a navigation
   trace.** Format helpers + day-math are covered; the
   stateful navigation is covered by the state-layer tests.

## Out of scope (next A.1 slices)

- **a1-5-f4-profile**: F4 reads `~/.config/blit/perf_local.jsonl`.
- **a1-6-screen-router**: F-keys to navigate between panes,
  replacing the `--screen` flag.

## Reviewer comments

(empty — pending grade)
