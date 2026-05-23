# audit-7d6-extract-tick-budget: extract tick-budget pure helpers from main.rs

**Severity**: Refactor / code-health
**Status**: In progress / pending review
**Branch**: `phase5/a1`
**Commit**: `a99a136`
**Parent finding**: `audit-7-code-health` (audit-7d — the 11K-line
blit-tui `main.rs` behavior-preserving module split). **Part 6 of N.**

## What

Continues audit-7d's incremental split of `main.rs`. This slice extracts
the two **pure** sleep-budget helpers into a new `crate::tick_budget`
module:

- `compute_tick_budget(needs_live_tick, live_tick_interval, cancel_remaining) -> Option<Duration>`
  (d-24) — picks the loop's `live_tick` sleep budget: the shorter of the
  tick interval and a pending auto-hide deadline, or `None` to sleep on
  real events only.
- `min_opt(a, b) -> Option<Duration>` (d-40) — the nearer of two optional
  deadlines, used to merge the F2-cancel and F3-pull auto-hide deadlines
  before handing the result to `compute_tick_budget`.

Both are pure `Duration`/`Option` math with no `AppState` coupling and no
crate-type dependencies.

## Approach (behavior-preserving)

Verbatim move — no logic change. A crate-root
`use crate::tick_budget::{compute_tick_budget, min_opt}` re-exposes both so
the event-loop call site (main.rs ~1066-1069, where the budget is computed
each iteration) **and** the inline unit tests (which reach them via the
test module's `use super::*`) resolve unchanged — no per-site edits, no
test moves. Both have non-test callers (the event loop), so both are
re-exported at crate root (no test-local-import dance like 7d5's
`classify_reload`). The compiler + `clippy -D warnings` + the full
blit-tui suite are the behavior-preservation proof.

## Files changed

- `crates/blit-tui/src/tick_budget.rs` (new): the 2 `pub(crate)` fns (no
  imports needed — they use fully-qualified `std::time::Duration`).
- `crates/blit-tui/src/main.rs`: `mod tick_budget;` + the `use`; the 2 fn
  definitions removed.

## Verification

`cargo fmt --all --check`, `cargo clippy --workspace --all-targets -D
warnings`, `cargo test --workspace` all green; blit-tui 630 tests pass
(including `min_opt_picks_the_shorter_deadline` and the
`compute_tick_budget_*` suite).

## Next slices (audit-7d, planned)

The remaining clusters get progressively more AppState-coupled: the
background-task plumbing (`spawn_*`/`run_*` + Reply structs,
`del_wire_path`); the F1 trigger planning
(`plan_f1_trigger`/`plan_f1_delegated`/`TriggerOutcome`); and ultimately
the `run_router` event loop / `handle_pane_action` / render orchestration.
Will stop and report to the owner before forcing a risky split if no clean
pure-helper clusters remain.
