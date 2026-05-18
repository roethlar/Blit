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

(empty — pending grade)
