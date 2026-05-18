# d-20-f2-recent-throughput: throughput column on F2 recent

**Severity**: Feature (polish — pairs F2 recent table
with the F4 transfer Done banner's d-10 throughput)
**Status**: In progress / pending review
**Branch**: `phase5/a1`
**Commit**: filled by the sentinel commit

## What

F2's recent table gains a `throughput` column on the
right. Each completed row computes
`bytes / duration_ms` and renders the result in binary
units (KiB/s, MiB/s, GiB/s) right next to the
`bytes` and `duration` columns:

```
transfer_id  kind   peer    module/path  bytes      duration  throughput
abc123       pull   peer-A  mod/file     500 MiB    61.2s     8.2 MiB/s
def456       push   peer-B  mod/file     1.20 GiB   2.0s      614.4 MiB/s
ghi789       pull   peer-C  mod/file     0 B        FAIL      -
```

Pre-d-20 the operator could read bytes + duration and
mentally divide — useful but tedious for a long recent
list. The column makes "this transfer was slow"
visible at a glance.

Suppressed (rendered as `-`) when the rate would be
meaningless: failed transfer, zero bytes, or
sub-millisecond duration. Same rules as d-10's
`format_rate` on F4.

## Approach

### Recent table

`render_recent_table` gains a 7th column. Width
allocation: `Length(20), 14, 20, Min(20), 10, 10, 12` —
shrunk the duration column from 12 to 10, added a
12-char throughput column. The flex `Min(20)` for
module/path absorbs the budget change.

### `format_recent_throughput`

New helper inline in `screens/f2.rs`:

```rust
fn format_recent_throughput(row: &RecentRow) -> String
```

Returns `"-"` for the three meaningless cases (failed,
zero bytes, zero duration). Otherwise computes
`bytes/duration_ms * 1000` in u128 to avoid overflow,
then picks a binary tier (B/s, KiB/s, MiB/s, GiB/s).

Could have factored with d-10's `format_rate` in
`screens/f4.rs` — the logic is nearly identical — but
the d-10 helper takes a `Duration` while the F2 row
has a `duration_ms: u64`, and centralizing them across
modules would need a shared helpers module. Left as a
future refactor.

## Files changed

- `crates/blit-tui/src/screens/f2.rs`:
  - `render_recent_table` widths + header gain the
    throughput column.
  - `recent_row_to_table_row` appends the throughput
    cell.
  - `format_recent_throughput(row)` helper.

## Tests

+6 unit tests (228 → 234):

In `screens::f2::tests`:
- `recent_throughput_dash_for_failed_transfer`
- `recent_throughput_dash_for_zero_bytes`
- `recent_throughput_dash_for_zero_duration`
- `recent_throughput_kibibytes`
- `recent_throughput_mebibytes`
- `recent_throughput_bytes_tier_for_slow_transfers`

`cargo fmt`, `cargo clippy --workspace --all-targets
-- -D warnings`, and `cargo test --workspace` all green.

## Known gaps

1. **Average only.** The throughput value is averaged
   over the transfer's full duration. A transfer that
   ran fast then slow shows the same number as one
   that ran slow then fast. The CLI emits a similar
   averaged figure on its summary, so the parity is
   consistent.

2. **No TiB/s tier.** Stops at GiB/s. Highly unlikely
   to hit but a fast NVMe-to-NVMe local copy might —
   future polish can add the tier mirror-image to
   d-10's `format_rate`.

3. **Duplicate logic with d-10.** `format_rate` in
   `screens/f4.rs` does the same math against a
   `Duration` instead of `u64 ms`. Future refactor
   could host both in `crate::format` or similar.

## Out of scope (next slices)

- **TiB/s tier on both surfaces.**
- **Shared `format_rate` helper module.**
- **Color-coded throughput** (red for "below threshold",
  green for "above").

## Reviewer comments

(empty — pending grade)
