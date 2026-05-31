# audit-7d2-extract-display-f3: extract F3 state→display mappers from main.rs

**Severity**: Refactor / code-health
**Status**: In progress / pending review
**Branch**: `phase5/a1`
**Commit**: `315f923`
**Parent finding**: `audit-7-code-health` (audit-7d — the 11K-line
blit-tui `main.rs` behavior-preserving module split). **Part 2 of N.**

## What

Continues audit-7d's incremental split of the ~11.3K-line `main.rs`.
This slice extracts the cohesive cluster of **pure** F3 state→display
mapping helpers (zero `AppState` coupling, no async, no I/O) into a new
`crate::display_f3` module:

- `f3_pull_to_display(&f3pull::F3PullStatus) -> screens::f3::F3PullDisplay`
  (d-35) — bridges the F3 pull state machine to the renderer-facing
  display struct, including the destructive-confirm detail line.
- `confirm_detail(f3pull::PullKind) -> &'static str` (d-55/d-57) — the
  per-kind "what gets removed" line. Only `f3_pull_to_display` calls it,
  so it stays **private** to the module (not re-exported).
- `f3_du_to_display(&f3du::F3DuStatus, Option<&str>) -> screens::f3::F3DuDisplay`
  (d-41) — du state→display with the path-match gating (an outdated
  subtree total never renders against the wrong row).
- `f3_del_to_display(&f3del::F3DelStatus, Option<&str>) -> screens::f3::F3DelDisplay`
  (d-45/d-50) — delete state→display with single-row gate-path gating
  vs. batch (`None`) always-show semantics.

## Approach (behavior-preserving)

Verbatim move — no logic change. A crate-root
`use crate::display_f3::{f3_del_to_display, f3_du_to_display, f3_pull_to_display}`
re-exposes the three public mappers so every call site (the `run_router`
render path at main.rs ~987-989) **and** the existing inline unit tests
(which reach them via the test module's `use super::*`) resolve
unchanged — no per-site edits, no test moves. `confirm_detail` is private
within the new module since it has a single in-module caller. The
compiler + `clippy -D warnings` + the full blit-tui suite are the
behavior-preservation proof.

The four fns previously sat interleaved with other display helpers
(`push_present_verb`/`push_past_verb`/`f1_push_status` — the F1-push
cluster) and the `BatchPull` struct; those were left in place and will
move in later slices, so this slice's deletions are non-contiguous (three
edits) but each fn moved verbatim.

## Files changed

- `crates/blit-tui/src/display_f3.rs` (new): the 3 `pub(crate)` mappers +
  the private `confirm_detail`, with `use crate::{f3del, f3du, f3pull, screens};`
  so the moved bodies' bare `f3pull::`/`screens::` paths resolve.
- `crates/blit-tui/src/main.rs`: `mod display_f3;` + the `use`; the 4 fn
  definitions removed.

## Verification

`cargo fmt --all --check`, `cargo clippy --workspace --all-targets -D
warnings`, `cargo test --workspace` all green; blit-tui 630 tests pass
(including the moved fns' unit tests — the `f3_du_to_display` path-gating
tests ~8312+, `f3_del_to_display` tests ~8204+, and the
`f3_pull_to_display` render test).

## Next slices (audit-7d, planned)

Further behavior-preserving extractions from main.rs, each its own slice:
the F1-push display cluster (`push_present_verb`/`push_past_verb`/
`f1_push_status`, `f1_trigger_prompt`, `cancel_status_to_display`/
`cancel_status_remaining_ttl`); the background-task plumbing
(`spawn_*`/`run_*` + their Reply structs, `del_wire_path`); the F1 trigger
planning (`plan_f1_trigger`/`plan_f1_delegated`/`TriggerOutcome`); and
ultimately the `run_router` event loop / `handle_pane_action` key dispatch
/ render orchestration. Done incrementally so each stays
compiler+test-verifiable.

## Reviewer comments

Verified. Moving the F3 state→display mapping helpers (`f3_pull_to_display`, `f3_du_to_display`, and `f3_del_to_display`) to `display_f3.rs` is correct and preserves TUI rendering logic. Keeping `confirm_detail` private to the module is also correct. Verified with clean clippy/fmt and all 630 workspace tests passing.
