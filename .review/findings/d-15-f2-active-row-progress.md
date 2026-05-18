# d-15-f2-active-row-progress: % complete on F2 active rows

**Severity**: Feature (polish — pairs with d-14's age
column to fill out the F2 active-row context)
**Status**: In progress / pending review
**Branch**: `phase5/a1`
**Commit**: filled by the sentinel commit

## What

F2's active table now shows fraction-complete next to
the raw byte count when the daemon has measured the
total:

```
bytes
0 B · 0%
500 KiB · 50%
1.20 GiB · 75%
1.00 MiB
```

When `bytes_total == 0` (daemon hasn't measured the
plan yet — e.g. early in a remote pull) the cell falls
back to just the raw byte count, no percentage. The
operator gets a meaningful number either way.

## Approach

### `format_bytes_progress`

New helper:

```rust
fn format_bytes_progress(completed: u64, total: u64) -> String
```

Branches:

- `total == 0` → `format_bytes(completed)` (no percent —
  showing "0%" against an unknown plan would mislead).
- `completed >= total` → clamps to 100 (daemon counter
  drift is presented as "100%" not "120%").
- Otherwise → `format!("{bytes} · {percent}%")`.

Arithmetic done in `u128` to avoid overflow on a
hypothetical 16-EiB total.

### Column width

The bytes column widened from `Length(12)` to
`Length(18)` to fit "1023.99 MiB · 100%" (17 chars).
Other columns unchanged.

## Files changed

- `crates/blit-tui/src/screens/f2.rs`:
  - `format_bytes_progress(completed, total) -> String`
    helper.
  - `active_row_to_table_row` calls it instead of
    `format_bytes(row.bytes_completed)`.
  - Bytes column width 12 → 18.

## Tests

+4 unit tests (186 → 190):

In `screens::f2::tests`:
- `format_bytes_progress_omits_percent_when_total_unknown`
  — `total == 0` case.
- `format_bytes_progress_appends_percent_when_total_known`
  — 0%, 50%, 100%.
- `format_bytes_progress_clamps_overflow_to_100` —
  counter drift `completed > total`.
- `format_bytes_progress_picks_correct_byte_unit` —
  the byte tier (MiB here) still kicks in correctly
  when paired with the percent suffix.

`cargo fmt`, `cargo clippy --workspace --all-targets
-- -D warnings`, and `cargo test --workspace` all green.

## Known gaps

1. **No graphical progress bar.** Text-only percentage
   in the same column. A future polish could add a
   ratatui `Gauge` widget per row, but the current
   table layout doesn't accommodate per-cell widgets
   cleanly.

2. **Bytes column is wider on every active row, even
   when bytes_total == 0.** The 18-char column reserves
   space for the percent suffix that won't render. This
   is fine — terminal real estate is plentiful at the
   ~120-col mark — but on narrow terminals it eats into
   the flex module/path column more than pre-d-15.

## Out of scope (next slices)

- **e-3 themes / config** — `~/.config/blit/tui.toml`.
- **Graphical progress bars** on F2 active rows.

## Reviewer comments

(empty — pending grade)
