# audit-7d4-extract-display-f2: extract F2 cancel state→display mappers from main.rs

**Severity**: Refactor / code-health
**Status**: In progress / pending review
**Branch**: `phase5/a1`
**Commit**: `0ed685a`
**Parent finding**: `audit-7-code-health` (audit-7d — the 11K-line
blit-tui `main.rs` behavior-preserving module split). **Part 4 of N.**

## What

Continues audit-7d's incremental split of `main.rs`. This slice extracts
the two **pure** F2 cancel state→display helpers into a new
`crate::display_f2` module:

- `cancel_status_to_display(&F2CancelStatus, Instant, Duration) -> screens::f2::F2CancelDisplay`
  (d-22/d-23) — bridges the F2 cancel state machine to the renderer
  struct, including the d-23 TTL auto-hide of terminal fragments.
- `cancel_status_remaining_ttl(&F2CancelStatus, Instant, Duration) -> Option<Duration>`
  (d-24/d-30) — the deadline the event loop reads to bound its tick
  budget so a short `cancel_status_ttl_ms` isn't masked by a longer
  `live_tick.interval_ms`.

## Approach (behavior-preserving)

Verbatim move — no logic change. Unlike the earlier 7d slices (which
moved their state enums' display mappers alongside types that lived in
sibling submodules), **`F2CancelStatus` itself stays in `main.rs`**: it is
the `AppState.cancel_status` field's enum and the event loop mutates it in
~20 places (`app.cancel_status = F2CancelStatus::…`), so moving it would
force a re-export churn with no benefit. The two mappers only *read* it,
so the new module names it via the crate-root path
(`use crate::{screens, F2CancelStatus}`) — a child module may reference an
ancestor-module-private item, so `F2CancelStatus` need not be made `pub`.

A crate-root `use crate::display_f2::{cancel_status_remaining_ttl, cancel_status_to_display}`
re-exposes both mappers so every call site (the `run_router` render path
at ~975 and the tick-budget computation at ~1040) **and** the ~25 inline
unit tests (which reach them — and `F2CancelStatus::*` directly — via the
test module's `use super::*`) resolve unchanged. The compiler +
`clippy -D warnings` + the full blit-tui suite are the
behavior-preservation proof.

## Files changed

- `crates/blit-tui/src/display_f2.rs` (new): the 2 `pub(crate)` mappers,
  with `use crate::{screens, F2CancelStatus};` + `use std::time::Instant;`.
- `crates/blit-tui/src/main.rs`: `mod display_f2;` + the `use`; the 2 fn
  definitions removed.

## Verification

`cargo fmt --all --check`, `cargo clippy --workspace --all-targets -D
warnings`, `cargo test --workspace` all green; blit-tui 630 tests pass
(including the full `cancel_status_to_display_*` and
`cancel_status_remaining_ttl_*` suites that exercise both moved fns).

## Next slices (audit-7d, planned)

The config-reload helpers (`reload_tui_config`/`classify_reload`); the
background-task plumbing (`spawn_*`/`run_*` + Reply structs,
`del_wire_path`); the F1 trigger planning
(`plan_f1_trigger`/`plan_f1_delegated`/`TriggerOutcome`); the tick-budget
pure helpers (`compute_tick_budget`/`min_opt`); and ultimately the
`run_router` event loop / `handle_pane_action` / render orchestration.
