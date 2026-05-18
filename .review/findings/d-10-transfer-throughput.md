# d-10-transfer-throughput: bytes/sec on transfer Done

**Severity**: Feature (polish — complements d-8's
duration display by surfacing effective throughput)
**Status**: In progress / pending review
**Branch**: `phase5/a1`
**Commit**: filled by the sentinel commit

## What

The F4 transfer Done banner now appends an effective
throughput when one is meaningful:

```
copy done · 1234 planned · 1234 copied · 5678901 bytes · 12.3s · 449 KiB/s
mirror done · 50 planned · 50 copied · 524288000 bytes · 2m 5s · 4.0 MiB/s
```

Suppressed when the rate is meaningless: zero bytes, a
duration below 1ms (Instant resolution edge), or a
sub-1-B/s rate after rounding. So a 0-file copy that
completes instantly just shows the duration ("8ms")
without a misleading "0 B/s" tail.

## Approach

### `format_rate`

New helper in `screens/f4.rs`:

```rust
fn format_rate(bytes: u64, duration: Duration) -> Option<String>
```

Returns `None` for the meaningless cases. Otherwise
formats `bytes / duration` in binary units (KiB / MiB /
GiB / TiB at 1024-step thresholds), with one-decimal
precision above the byte tier. Binary units match the
raw byte count already shown next to the rate, so the
two reconcile cleanly.

### Render

The transfer Done arm in `render_transfer` computes the
elapsed Duration once, formats the duration via
`format_elapsed`, then opportunistically appends the
rate via `format_rate`. Verify Done is untouched —
`compare_trees` doesn't produce byte counts, so there's
nothing to divide.

## Files changed

- `crates/blit-tui/src/screens/f4.rs`:
  - `format_rate(bytes, duration) -> Option<String>`
    helper + new `rate_tests` submodule.
  - `render_transfer` Done arm appends the rate string
    when `format_rate` returns `Some`.

## Tests

+6 unit tests (169 → 175):

In `screens::f4::rate_tests` (new submodule):
- `format_rate_returns_none_for_zero_bytes`
- `format_rate_returns_none_for_zero_duration`
- `format_rate_bytes_per_second`
- `format_rate_kibibytes_per_second`
- `format_rate_mebibytes_per_second`
- `format_rate_gibibytes_per_second`

`cargo fmt`, `cargo clippy --workspace --all-targets
-- -D warnings`, and `cargo test --workspace` all green.

## Known gaps

1. **No live rate during Running.** Bytes copied so
   far isn't surfaced by `LocalMirrorSummary` until
   completion — there's no in-flight byte counter on
   the local-transfer path. A future progress-events
   slice (per-file or per-byte) would unlock a live
   rate display.

2. **Decimal-formatted (e.g. "1.0 KiB/s") only when
   above the byte tier.** Sub-KiB rates show as
   integer "N B/s". Matches the precision the operator
   can usefully read at that scale.

## Out of scope (next slices)

- **e-3 themes / config** — `~/.config/blit/tui.toml`.
- **Per-file progress** events during local transfers.
- **Esc cancels mirror/move confirm** prompts.

## Reviewer comments

(empty — pending grade)
