# d-25-f2-tib-tier: TiB/s + TiB tiers on F2

**Severity**: Feature (polish — cross-pane consistency)
**Status**: In progress / pending review
**Branch**: `phase5/a1`
**Commit**: filled by the sentinel commit

## What

F2 had three byte/rate surfaces capped at `GiB` / `GiB/s`
while F4's `format_rate` already grew a `TiB/s` tier in
d-10. A hypothetical 2 TiB/s transfer rendered as

- F4 Done banner: `2.0 TiB/s`
- F2 recent-row throughput: `2048.0 GiB/s`
- F2 recent-row bytes: `2048.00 GiB` (for the 2 TiB
  transfer's total)
- F2 active-row throughput: `2048.00 GiB/s` (since the
  active-row column wraps `format_bytes(throughput_bps)`)

That's the same number rendered four different ways
depending on which pane the operator was on. d-25 closes
the gap by adding `TiB` to `format_bytes` and `TiB/s` to
`format_recent_throughput`, matching F4's tier list.

## Approach

### `format_bytes`

```rust
if n >= 1 << 40 {
    format!("{:.2} TiB", n as f64 / (1u64 << 40) as f64)
} else if n >= 1 << 30 {
    format!("{:.2} GiB", n as f64 / (1u64 << 30) as f64)
} /* ...rest unchanged... */
```

`format_bytes` is called from three F2 surfaces:

1. Recent-row bytes column.
2. Active-row bytes-progress column (via
   `format_bytes_progress`, which prepends to
   `format_bytes(completed)`).
3. Active-row throughput column (via the literal
   `format!("{}/s", format_bytes(row.throughput_bps))`).

All three inherit the TiB tier in one shot.

### `format_recent_throughput`

```rust
const TIB: f64 = GIB * 1024.0;
if bytes_per_sec >= TIB {
    format!("{:.1} TiB/s", bytes_per_sec / TIB)
} else if bytes_per_sec >= GIB { /* ... */ }
```

Tier list now matches F4's `format_rate` exactly. The
"-" fallbacks for failed / zero-byte / zero-duration
rows are unchanged.

### Source-doc sync

f2.rs module-doc layout-header comment bumped from
`d-23 / d-24` to `d-23 / d-24 / d-25`. Each formatter's
rustdoc mentions the d-25 tier change and explains
why the alignment matters.

## Files changed

- `crates/blit-tui/src/screens/f2.rs`:
  - `format_bytes` grows a TiB tier (first branch).
  - `format_recent_throughput` grows a TiB/s tier (first
    branch).
  - Module-doc layout-header lists d-25.
  - Both formatter docstrings call out the d-25 tier
    and the F4 alignment motivation.

## Tests

+3 tests (268 → 271):

In `screens::f2::tests`:
- `format_bytes_picks_correct_unit` (existing) — asserts
  `1 TiB` → `"1.00 TiB"` and `2 TiB` → `"2.00 TiB"`
  appended to the existing matrix.
- `format_bytes_tib_boundary_promotes_unit` (new) — pins
  the GiB → TiB boundary: `(1<<40) - 1` → `"1024.00 GiB"`,
  `(1<<40)` → `"1.00 TiB"`.
- `recent_throughput_tebibytes` (new) — 2 TiB/s →
  `"2.0 TiB/s"`. Mirrors F4's
  `format_rate_gibibytes_per_second` style.
- `recent_throughput_tib_boundary_promotes_unit` (new) —
  pins the boundary: exactly 1 TiB/s → `"1.0 TiB/s"`,
  1023 GiB/s → `"1023.0 GiB/s"`.

`cargo fmt`, `cargo clippy --workspace --all-targets
-- -D warnings`, and `cargo test --workspace` all green.

## Known gaps

1. **No PiB tier.** Rates above 1024 TiB/s render as
   `"NNNN.N TiB/s"`. Not worth adding until someone
   actually demos a PiB/s transfer.

2. **Tiers aren't deduplicated across f2 / f4.** Both
   files declare their own `KIB/MIB/GIB/TIB` constants.
   A shared helper module (`format::si_binary` or
   similar) could host the tier table. Out of scope for
   d-25 — the duplication is 5 lines each, and the
   formatters diverge slightly (F2 returns `String`,
   F4 returns `Option<String>`).

## Out of scope (next slices)

- **Cancel confirmation prompt** (d-22 known gap #1).
- **Batch cancel Shift-K** (d-22 known gap #2).
- **F3 module filter.**
- **Hot-reload tui.toml.**

## Reviewer comments

(empty — pending grade)
