# d-11-freshness-tick: F1/F3/F4 footers tick live

**Severity**: Feature (closes the d-9 known gap —
freshness footers on F1/F3/F4 went stale until an
external event arrived)
**Status**: In progress / pending review
**Branch**: `phase5/a1`
**Commit**: filled by the sentinel commit

## What

The d-9 500ms live-tick gate now also fires when a
freshness footer is on screen, so "live · last scan
12s ago" / "loaded · 4s ago" actually count up
visibly:

- **F1** while `DiscoveryStatus::Live { .. }` (the "last
  scan" footer + the daemon detail "as of" line).
- **F3** while `BrowseFetchStatus::Loaded { .. }` (the
  "loaded · Xs ago" footer).
- **F4** while `ProfileFetchStatus::Loaded { .. }` (the
  "loaded · Xs ago" status span — independent of the
  d-9 transfer/verify run gate that already covered
  active F4 work).

F2 stays gated off — its renderer doesn't use `now` for
anything, so a wakeup there would just be wasted draws.

## Approach

Extend `needs_live_tick(app: &AppState) -> bool` in
`main.rs` from "transfer or verify running" to also
include pane-specific freshness conditions:

```rust
fn needs_live_tick(app: &AppState) -> bool {
    if app.transfer.is_running() || app.verify.is_running() {
        return true;
    }
    match app.current_screen {
        Screen::F1 => matches!(app.daemons.status(),
            daemons::DiscoveryStatus::Live { .. }),
        Screen::F2 => false,
        Screen::F3 => matches!(app.browse.status(),
            browse::BrowseFetchStatus::Loaded { .. }),
        Screen::F4 => matches!(app.profile.status(),
            profile::ProfileFetchStatus::Loaded { .. }),
    }
}
```

Confirm-pending and pure Idle states still return
false — there's nothing time-dependent to refresh.

500ms cadence reused from d-9. `format_since` reports
at second resolution but a 500ms wake keeps the first
second's transition (5s→6s, etc.) within ~500ms of
real time.

## Files changed

- `crates/blit-tui/src/main.rs`:
  - `needs_live_tick` extended with the per-screen
    match. Doc comment rewritten.
  - One new unit test for the pane-specific conditions.

## Tests

+1 unit test (175 → 176):

In `main::tests`:
- `needs_live_tick_covers_per_pane_freshness_footers`:
  walks F1 Scanning → Live, F2 (never ticks), F3 Idle
  → Loaded, F4 Idle → Loaded. Asserts the gate flips
  correctly at each transition.

`cargo fmt`, `cargo clippy --workspace --all-targets
-- -D warnings`, and `cargo test --workspace` all green.

## Known gaps

1. **F2 still doesn't tick.** F2's renderer doesn't
   compute anything against `now`, so freshness on F2
   would need new render code first. Not in this slice.

2. **Resource use.** Every active pane with a Live
   status now redraws every 500ms even when nothing
   moved. Ratatui diffs frames so the terminal byte
   traffic stays minimal, but CPU is non-zero. On
   battery-sensitive hosts a future polish slice could
   reduce the cadence to 1Hz once the tick exceeds a
   minute of staleness (where second-level precision
   no longer matters).

## Out of scope (next slices)

- **e-3 themes / config** — `~/.config/blit/tui.toml`.
- **Per-file progress** events during local transfers.
- **Esc cancels mirror/move confirm.**

## Reviewer comments

(empty — pending grade)
