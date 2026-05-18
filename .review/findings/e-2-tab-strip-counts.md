# e-2-tab-strip-counts: at-a-glance counts in the tab strip

**Severity**: Feature (second slice of milestone E)
**Status**: In progress / pending review
**Branch**: `phase5/a1`
**Commit**: filled by the sentinel commit

## What

Right-side of the tab strip now shows live counts of
discovered daemons, active transfers, and recent
transfers, plus a `? help` reminder. Matches the
TUI_DESIGN §5 header line:

```
┌─ blit ────────────────────────── 3 daemons │ 1 transfer active ─┐
```

The status is visible regardless of which pane is active,
so the operator always knows whether mDNS has settled,
whether anything's transferring right now, and how many
recent completions are in the buffer — without having to
hop to F1 or F2.

## Approach

`screens::render_tab_strip` gains a fourth parameter,
`counts: TabStripCounts { daemons, active_transfers,
recent_transfers }`. The function splits the tab-strip
area horizontally into:

- left (`Min 28`): F1..F4 tabs (unchanged shape).
- right (`Length 48`): right-aligned counts +
  `? help` hint.

Right-aligned via `Paragraph::alignment(Alignment::Right)`.

The router fills `counts` from `AppState`:

```rust
let counts = TabStripCounts {
    daemons: app.daemons.rows().len(),
    active_transfers: app.transfers.active_count(),
    recent_transfers: app.transfers.recent_count(),
};
```

When no daemon has been discovered or no transfer has
landed, the line reads `0 daemons · 0 active · 0 recent ·
? help`. That's still useful — operator sees the keymap
hint and confirms the panes are populated correctly (or
explains why they're empty).

## Files changed

- `crates/blit-tui/src/screens/mod.rs`:
  - New `TabStripCounts` struct (default = zeroes).
  - `render_tab_strip` signature gains `counts`.
  - New `format_counts_line` helper.
  - +2 unit tests for the formatter.
- `crates/blit-tui/src/main.rs`: caller fills `counts`
  from `AppState` per draw.

## Tests

+2 unit tests in `screens::tests`:

- `format_counts_line_includes_all_three_numbers`
- `format_counts_line_with_zeroes`

143 blit-tui unit tests (was 141 after d-4 R2).
Workspace passes serially.

## Known gaps

1. **Right column width is fixed at 48.** On terminals
   narrower than ~76 columns the counts get clipped.
   Future polish could query the terminal width and
   shorten or hide the right block when tight.

2. **No "error" indicator.** A discovery-failed or
   subscribe-stream-broken state surfaces in the pane's
   own footer but doesn't bubble to the tab strip. A red
   dot on the affected tab key (e.g. F1) would be a
   future visual cue.

## Out of scope

- **e-3 themes / config** — `~/.config/blit/tui.toml`.
- **e-4 mouse on tabs** — clickable F1..F4.
- **e-5 unified status bar** — bottom-of-screen single
  status line replacing per-pane footers.

## Reviewer comments

### Round 1 verdict — reopened (`.review/results/e-2-tab-strip-counts.reopened.md`)

Three findings, all addressed in round 2:

1. **High — Active/recent counts ignored F4 local
   transfers.** Fix: `TransferState` gained `count_active()`
   (`1` while `is_running()`) and `count_recent()` (`1` when
   the F4 status is terminal `Done`/`Error`). The main-loop
   call site folds these into the daemon-stream totals:
   ```rust
   active_transfers: app.transfers.active_count() + app.transfer.count_active(),
   recent_transfers: app.transfers.recent_count() + app.transfer.count_recent(),
   ```
   So an F4 copy / mirror / move is visible in the tab strip
   while running, and stays visible as "recent" until the
   operator kicks another transfer.

2. **Medium — Daemons count included the synthetic Local
   row.** Fix: `DaemonsState::discovered_count()` filters
   the rows on `!is_local()`, so the count reflects mDNS
   discovery only. Pre-discovery the tab strip now reads
   `0 daemons`, matching the finding doc's empty-state
   promise.

3. **Medium — Fixed 48-col right column clipped tabs on
   common terminal widths.** Fix: `render_tab_strip` is now
   responsive across four width regimes:
   - Wide: full tab labels (" F1 Daemons " etc.) + full
     counts ("3 daemons · 1 active · 47 recent · ? help").
   - Medium: full tabs + short counts ("3d · 1a · 47r").
   - Narrow: short tab labels (" F1 " etc.) + short counts.
   - Very narrow: short tabs only, counts hidden.

   The layout gives tabs first dibs on width
   (`Constraint::Length(tab_width)`) so the primary
   navigation surface never clips. Counts shrink or
   disappear before tabs do.

### Round 2 file changes

- `crates/blit-tui/src/transfer.rs`: `count_active()` and
  `count_recent()` methods + 3 unit tests.
- `crates/blit-tui/src/daemons.rs`: `discovered_count()`
  method + 1 unit test.
- `crates/blit-tui/src/screens/mod.rs`: new responsive
  `render_tab_strip`, helpers `build_tab_spans`,
  `total_span_width`, `format_counts_full`,
  `format_counts_short` + 3 new unit tests covering
  width regimes.
- `crates/blit-tui/src/main.rs`: call site uses
  `discovered_count` + folded transfer-state counts.

### Round 2 tests

+7 tests total (143 → 150):

In `transfer::tests`:
- `count_active_is_one_while_running_zero_otherwise`
- `count_recent_is_one_after_terminal_state`
- `count_recent_counts_errors`

In `daemons::tests`:
- `discovered_count_excludes_local_row`

In `screens::tests`:
- `format_counts_short_keeps_numbers_drops_help_hint`
- `render_at_80_cols_keeps_full_tabs`
- `short_tabs_fit_narrow_terminal`

Renamed: `format_counts_line_*` → `format_counts_full_*`.

### Validation

- `cargo fmt --all -- --check` ✅
- `cargo clippy --workspace --all-targets -- -D warnings` ✅
- `cargo test -p blit-tui` ✅ 150 tests
- `cargo test --workspace` ✅
