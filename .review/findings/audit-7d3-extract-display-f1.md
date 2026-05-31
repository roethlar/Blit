# audit-7d3-extract-display-f1: extract F1 state→display mappers from main.rs

**Severity**: Refactor / code-health
**Status**: In progress / pending review
**Branch**: `phase5/a1`
**Commit**: `1e50f7d`
**Parent finding**: `audit-7-code-health` (audit-7d — the 11K-line
blit-tui `main.rs` behavior-preserving module split). **Part 3 of N.**

## What

Continues audit-7d's incremental split of the ~11.3K-line `main.rs`.
This slice extracts the cohesive cluster of **pure** F1 state→display
mapping helpers (zero `AppState` coupling, no async, no I/O) into a new
`crate::display_f1` module:

- `f1_trigger_prompt(&f1trigger::F1TriggerState) -> Option<screens::f1::TriggerPrompt>`
  (d-58/d-59/d-62/d-65/d-71) — bridges the F1 trigger modal to the
  renderer struct, including the direction-aware destructive-confirm
  detail (push move deletes the local source; delegated move deletes the
  remote source, classified via `parse_transfer_endpoint`).
- `f1_push_status(&f1push::F1PushState) -> Option<screens::f1::PushStatusDisplay>`
  (d-61) — bridges the F1 push state machine to the renderer struct.
- `push_present_verb`/`push_past_verb(f3pull::PullKind, bool) -> &'static str`
  (d-65/d-68/d-70) — the push-footer verb tables. Only `f1_push_status`
  calls them, so they stay **private** to the module (not re-exported).

## Approach (behavior-preserving)

Verbatim move — no logic change (the only delta is rustfmt re-wrapping
the two mapper signatures onto multiple lines in the fresh file; bodies
are byte-identical). A crate-root
`use crate::display_f1::{f1_push_status, f1_trigger_prompt}` re-exposes
the two public mappers so every call site (the `run_router` render path
at main.rs ~963-964) **and** the existing inline unit tests (which reach
`f1_trigger_prompt` via the test module's `use super::*` at ~9106) resolve
unchanged — no per-site edits, no test moves. The compiler +
`clippy -D warnings` + the full blit-tui suite are the
behavior-preservation proof.

The cluster references only sibling submodules (`f1trigger`, `f1push`,
`f3pull`), `screens`, and `blit_app::endpoints` — no main.rs-local types
— so it is a clean closed set. The F2 cancel mappers
(`cancel_status_to_display`/`cancel_status_remaining_ttl`) were
deliberately left for a later slice because they couple to the
main.rs-local `F2CancelStatus` type.

## Files changed

- `crates/blit-tui/src/display_f1.rs` (new): the 2 `pub(crate)` mappers +
  the 2 private verb helpers, with `use crate::{f1push, f1trigger, f3pull, screens};`.
- `crates/blit-tui/src/main.rs`: `mod display_f1;` + the `use`; the 4 fn
  definitions removed.

## Verification

`cargo fmt --all --check`, `cargo clippy --workspace --all-targets -D
warnings`, `cargo test --workspace` all green; blit-tui 630 tests pass
(including `f1_trigger_prompt_move_detail_follows_source_direction` and
the F1 push footer tests).

## Next slices (audit-7d, planned)

The F2 cancel display cluster (`cancel_status_to_display`/
`cancel_status_remaining_ttl`, likely moving `F2CancelStatus` with them);
the config-reload helpers (`reload_tui_config`/`classify_reload`); the
background-task plumbing (`spawn_*`/`run_*` + Reply structs,
`del_wire_path`); the F1 trigger planning
(`plan_f1_trigger`/`plan_f1_delegated`/`TriggerOutcome`); and ultimately
the `run_router` event loop / `handle_pane_action` / render orchestration.

## Reviewer comments

Verified. The verbatim extraction of F1 state→display mapping helpers (`f1_trigger_prompt`, `f1_push_status`) and the associated private verb helpers to the new `display_f1` module is correct, behavior-preserving, and keeps `main.rs` cleaner. Verified with clean clippy/fmt and all 630 workspace tests passing.
