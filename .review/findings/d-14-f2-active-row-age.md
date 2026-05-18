# d-14-f2-active-row-age: F2 active rows show start age

**Severity**: Feature (polish — closes the d-13 known
gap on per-row freshness in the F2 active table)
**Status**: In progress / pending review
**Branch**: `phase5/a1`
**Commit**: filled by the sentinel commit

## What

F2's active table gains an `age` column on the right.
Each row reports how long ago its transfer started,
computed against `ActiveTransfer::start_unix_ms` (the
wall-clock millisecond stamp the daemon writes on the
row at start).

```
transfer_id  kind         peer      module/path        bytes      throughput  age
t-1          delegated... peer-A    mod-X/sub/file     500 KiB    1.2 MiB/s   12s
t-2          push         peer-B    backup/big.tar     2.4 GiB    8.0 MiB/s   2m
t-3          pull         peer-C    photos/2026/img    47.5 MiB   -           5h
```

Pre-d-14 the operator could tell a transfer was active
but not how long it had been running. Combined with the
existing throughput column, the age column gives a
quick "this transfer has been going for 5 hours at 8
MiB/s" read without doing wall-clock math.

The d-9/d-11/d-13 live-tick gate already covers F2
when `last_event_at` is `Some`, so the age column
visibly ticks every 500ms.

## Approach

### Render path

`render_into` captures wall-clock once per frame via
`SystemTime::now() / UNIX_EPOCH`, then threads
`now_unix_ms: u64` into `render_active_table`. Each
active row's age cell calls a new helper:

```rust
fn format_age_from_unix_ms(now_unix_ms: u64, start_unix_ms: u64) -> String
```

Magnitude tiers (matching the existing freshness
helpers on F1/F2/F3):

| Age          | Output  |
|--------------|---------|
| < 1s         | `Nms`   |
| 1s – 59s     | `Ns`    |
| 1m – 59m     | `Nm`    |
| 1h+          | `Nh`    |

Returns `"-"` for garbage / unparseable input:

- `now_unix_ms == 0` (SystemTime fell over — unlikely).
- `start_unix_ms == 0` (daemon sent a garbage row).
- `now < start` (clock skew — TUI host clock is behind
  the daemon's).

The dash is safer than a wrap-around or negative
display.

### Table layout

Active table widths gain one trailing constraint:

```rust
[Length(20), Length(14), Length(20), Min(20), Length(12), Length(12), Length(10)]
```

The `Min(20)` module/path column is the flex one, so
the new fixed 10-char age column comes off its width
on a narrow terminal. Acceptable trade-off — module
paths are routinely long enough to truncate already,
and 10 chars handles every age the format helper
produces.

## Files changed

- `crates/blit-tui/src/screens/f2.rs`:
  - `render_into` captures `now_unix_ms` once per
    frame.
  - `render_active_table` gains `now_unix_ms` param +
    new "age" column.
  - `active_row_to_table_row` gains `now_unix_ms`
    param.
  - New `format_age_from_unix_ms` helper.

## Tests

+4 unit tests (182 → 186):

In `screens::f2::tests`:
- `format_age_milliseconds`
- `format_age_seconds` (covers seconds + 1-minute
  transition).
- `format_age_minutes_and_hours`
- `format_age_returns_dash_for_garbage_inputs` —
  zero-now, zero-start, and reversed-order (clock skew)
  cases.

`cargo fmt`, `cargo clippy --workspace --all-targets
-- -D warnings`, and `cargo test --workspace` all green.

## Known gaps

1. **Wall-clock skew between TUI host and daemon host
   prints "-".** Operator running the TUI on a host
   whose clock is BEHIND the daemon's will see all ages
   render as dashes. Sub-second skew probably won't be
   visible because the comparison is integer-millisecond.
   The dash is correct (we shouldn't lie about ages we
   can't compute) but a future polish could surface a
   "clock skew detected" hint in the footer when
   multiple rows show dashes.

2. **Recent table doesn't get an age column.** The
   recent table already shows `duration_ms` (how long
   the transfer ran), which is a more useful metric for
   completed work than "how long ago it finished." Out
   of scope.

## Out of scope (next slices)

- **e-3 themes / config** — `~/.config/blit/tui.toml`.
- **Per-file progress** events during local transfers.

## Reviewer comments

(empty — pending grade)
