# d-17-verify-result-preview: show first diff/missing/error on Done

**Severity**: Feature (polish — debugging surface
upgrade on the F4 Verify result banner)
**Status**: In progress / pending review
**Branch**: `phase5/a1`
**Commit**: filled by the sentinel commit

## What

The Verify Done banner has surfaced summary counts
since d-2, but the counts were the entire story:

```
matches: 100 · differ: 3 · missing-on-src: 0 · missing-on-dst: 5 · errors: 0 · 432ms
```

"3 files differ" — but WHICH 3? Pre-d-17 the operator
either had to dump a diagnostics snapshot, or run
`blit check --json` in a terminal, or just guess.

d-17 surfaces the first entry from each non-empty
category directly below the count line:

```
matches: 100 · differ: 3 · missing-on-src: 0 · missing-on-dst: 5 · errors: 0 · 432ms
  differ[0]: src/config.toml — size 1024 vs 2048
  missing-on-dst[0]: src/cache/index.bin
```

Up to 3 preview lines render (limited so the verify
block doesn't crowd out the predictor / transfer
blocks). The category order — differ → missing-on-dst →
missing-on-src → errors — surfaces the most actionable
mismatches first.

## Approach

### Preview helper

New `verify_preview_lines(result: &CheckResult, max: usize) -> Vec<Line<'static>>`:

- Skips empty categories.
- Emits one line per category, formatted with the
  category name + first entry's path (+ reason for
  `differing`, + message for `errors`).
- Stops at `max` lines so the caller can size the
  block.
- Yellow for warnings (differ / missing), red for
  errors.

### Layout adjustment

Verify block constraint grew from `Length(6)` to
`Length(9)` to accommodate up to 3 preview lines below
the existing 4-line content (source / dest / mode /
status). Predictor block's `Min(5)` weakened to
`Min(2)` — the predictor still renders, but on a tight
terminal the preview wins the budget battle. Predictor
coefficients are reference info; preview lines are
actionable debugging.

### Render integration

`render_verify`'s `lines` vec gets `push`ed the preview
lines only when `status` is `Done`. Idle / Running /
Error states render the same 4 content lines they did
before (the extra layout space stays blank).

## Files changed

- `crates/blit-tui/src/screens/f4.rs`:
  - Verify constraint `Length(6)` → `Length(9)`.
  - Predictor constraint `Min(5)` → `Min(2)`.
  - `verify_preview_lines` helper.
  - `render_verify` appends preview lines on Done.
  - +4 unit tests covering the preview helper.

## Tests

+4 unit tests (191 → 195):

In `screens::f4::tests`:
- `verify_preview_empty_result_returns_no_lines` —
  clean compare emits no preview.
- `verify_preview_shows_first_differ_first` — order +
  formatting of the first two non-empty categories.
- `verify_preview_caps_at_max_even_with_all_categories`
  — `max` honored regardless of how many categories
  have entries.
- `verify_preview_shows_errors_in_red` — error
  formatting carries the path and message.

`cargo fmt`, `cargo clippy --workspace --all-targets
-- -D warnings`, and `cargo test --workspace` all green.

## Known gaps

1. **Only first entry per category.** A compare with
   100 differing files shows just `differ[0]`. The
   operator can still see the count and dump a
   diagnostics snapshot for the full list. A future
   slice could add a "press D to dump differing list"
   shortcut.

2. **No line-wrapping for long paths.** A 200-char
   diff entry truncates at the verify block's width
   (~78 chars on a standard terminal). Operator would
   need to widen the terminal or open the diagnostics
   dump to see the full path.

3. **Predictor block can be tight on small terminals.**
   The Min(2) constraint means as low as 2 lines for
   predictor coefficients (which normally need 4-5).
   Acceptable trade-off — predictor is reference info,
   verify preview is actionable.

## Out of scope (next slices)

- **e-3 themes / config** — `~/.config/blit/tui.toml`.
- **Long-path wrapping in the verify block.**
- **Per-file progress** events during local transfers.

## Reviewer comments

(empty — pending grade)
