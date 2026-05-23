# audit-7d1-extract-progress-accum: extract pure progress helpers from main.rs

**Severity**: Refactor / code-health
**Status**: In progress / pending review
**Branch**: `phase5/a1`
**Commit**: `5112705`
**Parent finding**: `audit-7-code-health` (audit-7d — the 11K-line
blit-tui `main.rs` behavior-preserving module split). **Part 1 of N.**

## What

`main.rs` is ~11,360 lines. audit-7d splits it into modules without
behavior change, in small reviewable slices. This first slice extracts a
cohesive cluster of **pure** helpers (zero `AppState` coupling, no async)
into a new `crate::progress_accum` module:

- `accumulate_pull_progress` / `accumulate_push_progress` /
  `accumulate_delegated_progress` — fold a `ProgressEvent` into running
  `(files, bytes)` totals with the per-direction semantics each transfer
  path emits (the receive double-count avoidance, push bytes-on-complete,
  delegated cumulative-deltas).
- `pull_throughput` — the warm-up-suppressed average-throughput formula.
- `du_total_from_entries` — the F3 du-total reducer.

## Approach (behavior-preserving)

Verbatim move — no logic change. A crate-root
`use crate::progress_accum::{..}` re-exposes the five names so every call
site (the F3 pull / F1 push / delegated spawn tasks) **and** the existing
inline unit tests (which reach them via the test module's `use super::*`)
resolve unchanged — no per-site edits, no test moves. The compiler +
`clippy -D warnings` + the full blit-tui suite are the
behavior-preservation proof.

## Files changed

- `crates/blit-tui/src/progress_accum.rs` (new): the 5 `pub(crate)` fns.
- `crates/blit-tui/src/main.rs`: `mod progress_accum;` + the `use`; the
  5 fn definitions removed (the `F3PullProgress`/`F1PushProgress` message
  structs interleaved with them were preserved in place).

## Verification

`cargo fmt --check`, `cargo clippy --workspace --all-targets -D
warnings`, `cargo test --workspace` all green; blit-tui 630 tests pass
(including the moved fns' unit tests — `accumulate_*`, `pull_throughput_*`,
`du_total_from_entries`).

## Next slices (audit-7d, planned)

Further behavior-preserving extractions from main.rs, each its own slice:
the background-task plumbing (`spawn_*`/`run_*` + their Reply structs),
the state→display mapping helpers (`*_to_display`, `f1_*_prompt`), the F1
trigger planning (`plan_f1_trigger`/`plan_f1_delegated`/`TriggerOutcome`),
and ultimately the `run_router` event loop / key dispatch / render
orchestration. Done incrementally so each stays compiler+test-verifiable.

## Reviewer comments

Verified. The verbatim extraction of the five pure progress/throughput helper functions (`accumulate_pull_progress`, `accumulate_push_progress`, `accumulate_delegated_progress`, `pull_throughput`, and `du_total_from_entries`) from `main.rs` into the new `progress_accum` module is clean, behavior-preserving, and introduces zero behavior change. Crate-root re-exports successfully keep existing call sites and inline tests working out-of-the-box. Verified with clean clippy/fmt and all 630 workspace tests passing.
