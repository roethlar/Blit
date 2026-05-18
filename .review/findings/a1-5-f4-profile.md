# a1-5-f4-profile: F4 Profile pane (perf history + predictor)

**Severity**: Feature (fifth slice of milestone A.1 — adds F4)
**Status**: In progress / pending review
**Branch**: `phase5/a1`
**Commit**: filled by the sentinel commit

## What

Adds the F4 Profile screen to `blit-tui`. Operator opts in
via `blit-tui --screen f4`. The pane:

1. On open, reads `~/.config/blit/perf_local.jsonl` + the
   predictor state file via `blit_app::profile::query` —
   no RPC.
2. Renders a record-count / span / total-bytes summary, the
   predictor file path, history-enabled status, and the
   per-mode (copy / mirror) coefficients with both planner
   and transfer terms.
3. `r` re-reads from disk (file might have grown since
   open).

Atomic scope for this slice is **read-only display**.
The design also lists `[c] clear`, `[d] disable`, `[e] enable`
hotkeys plus a Verify pane and a Diagnostics block; those
mutate state / wrap separate code paths and land in
subsequent slices.

## Approach

### State (`profile.rs`)

Mirrors `BrowseState`'s shape so the operator's eye finds
the same conventions:

```rust
pub enum ProfileFetchStatus {
    Idle,
    Pending,
    Loaded { fetched_at: Instant },
    Error { message: String },
}

pub struct ProfileState {
    report: Option<ProfileReport>,
    status: ProfileFetchStatus,
    pending_request_id: u64,
}
```

Surface:
- `begin_fetch() -> u64` — bumps the generation, flips to
  Pending.
- `is_current_request(id) -> bool` — for stale-result drops.
- `apply_report(ProfileReport, fetched_at)`
- `note_fetch_error(message)` — keeps the prior report
  visible so a transient failure doesn't blank the screen.

### Event loop (`run_f4_event_loop` in `main.rs`)

```rust
loop {
    draw(state);
    select! {
        key => key_action handler (Quit | Refresh; F4 ignores nav).
        reply => apply_report or note_fetch_error if request_id matches.
    }
}
```

- Initial fetch kicked unconditionally so the operator
  doesn't have to press `r` to see anything.
- `r` re-issues via `begin_fetch` + `spawn_profile_fetch`;
  any in-flight stale generation gets dropped on arrival.
- `app_profile::query(0)` is sync (file I/O); wrapped in
  `tokio::task::spawn_blocking` so a slow disk doesn't
  stall the event loop.

### Render (`screens/f4.rs`)

Four-region layout (matches F1/F2/F3 conventions):

```
┌── header (1 line) ─────────────────────────────┐
│ blit-tui · F4 Profile · <state>                │
├── records summary (Length 4) ──────────────────┤
│ Records: N · span: D days · ~Bytes total       │
│ Predictor file: <path>                         │
│ History recording: enabled | disabled          │
├── predictor block (Min 5) ─────────────────────┤
│ [copy]  n=N  fallback=D                        │
│   planner  α=...  β=...  γ=...                 │
│   transfer α=...  β=...  γ=...                 │
│ [mirror] (similar)                             │
├── footer (1 line) ─────────────────────────────┤
│ status · q quit · r refresh                    │
└────────────────────────────────────────────────┘
```

Notes on rendering:
- Predictor coefficients show BOTH planner and transfer
  sides (the wire shape from R59's `DurationCoefficients`).
  The CLI's `blit profile` does the same.
- `History recording: disabled` is yellow (warning — the
  operator should know new records won't be captured).
- Empty profile (no observations yet) shows a dim "no
  profile yet — needs ≥5 observations" line per mode.

## Files changed

- `crates/blit-tui/src/profile.rs` (new): `ProfileState` +
  `ProfileFetchStatus` + 4 unit tests.
- `crates/blit-tui/src/screens/f4.rs` (new): render
  function + format helpers + 6 unit tests.
- `crates/blit-tui/src/screens/mod.rs`: `pub mod f4;` added.
- `crates/blit-tui/src/main.rs`:
  - `mod profile;` declaration.
  - `use blit_app::profile as app_profile;`.
  - `ScreenArg::F4` plus the dispatch arm.
  - `run_f4_event_loop` + `spawn_profile_fetch` +
    `ProfileReply` envelope.

## Tests added

10 new unit tests:

In `profile::tests`:
- `new_starts_idle`
- `begin_fetch_increments_request_id`
- `apply_report_sets_loaded_and_stores_report`
- `note_fetch_error_preserves_prior_report`

In `screens::f4::tests`:
- `format_bytes_picks_correct_unit`
- `span_days_computes_from_timestamps`
- `span_days_zero_for_empty_or_single_record`
- `summary_lines_total_bytes`
- `predictor_lines_renders_coefficients_and_observations`
- `summary_lines_renders_disabled_warning`

92 blit-tui unit tests (was 82). Workspace passes serially.

## Known gaps

1. **No clear/disable/enable hotkeys.** Design specifies
   `c` / `d` / `e`. Those mutate persistent state and need
   confirmation modals; out of scope for this slice.

2. **No Verify or Diagnostics sub-block.** The design's F4
   has three sections; this slice ships Profile only. F4
   Verify + Diagnostics are tracked as follow-up work
   (will likely be `a1-5b-f4-verify` and `a1-5c-f4-diag`
   once we cross those bridges).

3. **No record-list view.** Just summary counts. A future
   slice could render the recent records as a table
   (mirroring the CLI's `blit profile --json` shape).

4. **No periodic refresh.** Operator hits `r` to refresh.
   The file changes on every completed transfer; future
   polish could `watch` the file or refresh on a timer.

5. **No render test against TestBackend.** Format helpers
   covered; full-pane render isn't.

## Out of scope (next A.1 slices)

- **a1-6-screen-router**: F-keys to navigate between panes,
  replacing the `--screen` flag.
- **F4 Verify + Diagnostics**: separate follow-up slices.

## Reviewer comments

(empty — pending grade)
