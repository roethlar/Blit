# audit-7d8-extract-exec-plan: extract transfer-execution builders from main.rs

**Severity**: Refactor / code-health
**Status**: In progress / pending review
**Branch**: `phase5/a1`
**Commit**: `d47cc24`
**Parent finding**: `audit-7-code-health` (audit-7d — the 11K-line
blit-tui `main.rs` behavior-preserving module split). **Part 8 of N.**

## What

Continues audit-7d's incremental split of `main.rs`. This slice extracts
the cohesive cluster of **pure** transfer-execution builders into a new
`crate::exec_plan` module:

- `f3_pull_options(PullKind) -> PullSyncOptions` (d-55 R2/d-57) — the F3
  pull options: `mirror_mode` for Mirror, `require_complete_scan` for
  Move. Matches the CLI's options-field contract.
- `build_f1_push_execution(PathBuf, RemoteEndpoint, PullKind) -> PushExecution`
  (d-65 R2) — the F1 push execution; mirror gates `MirrorMode::All` +
  `require_complete_scan` on `mirror` (client-side partial-scan guard).
- `build_delegated_execution(RemoteEndpoint, RemoteEndpoint, PullKind) -> DelegatedPullExecution`
  (d-70) — the remote→remote delegated execution; options via
  `f3_pull_options(kind)`, always attached.

All three are pure (no async, no `AppState`, no I/O).

## Approach (behavior-preserving)

Verbatim move — no logic change. `remove_local_source` (the d-65 move
source-delete) was deliberately **left in main.rs**: it does filesystem
I/O, so it isn't a pure builder and belongs with the spawn task. The
moved cluster's only non-extern deps are `f3pull::PullKind`
(`use crate::f3pull`) and `RemoteEndpoint`
(`use blit_core::remote::endpoint::RemoteEndpoint`); the `blit_app`/
`blit_core` execution types are reached by absolute paths inside the
bodies. `build_delegated_execution` calls `f3_pull_options` within the
same module, so it resolves directly.

A crate-root `use crate::exec_plan::{build_delegated_execution, build_f1_push_execution, f3_pull_options}`
re-exposes all three so the non-test callers resolve unchanged:
`f3_pull_options` in `spawn_f3_pull`, `build_f1_push_execution` in
`spawn_f1_push`, `build_delegated_execution` in `spawn_f1_delegated_pull`.
The inline unit tests reach them via the test module's `use super::*`.
All three have non-test callers, so all three are re-exported at crate
root. The compiler + `clippy -D warnings` + the full blit-tui suite are
the behavior-preservation proof.

## Files changed

- `crates/blit-tui/src/exec_plan.rs` (new): the 3 `pub(crate)` fns +
  `use crate::f3pull;` + `use blit_core::remote::endpoint::RemoteEndpoint;`.
- `crates/blit-tui/src/main.rs`: `mod exec_plan;` + the `use`; the 3 fn
  definitions removed.

## Verification

`cargo fmt --all --check`, `cargo clippy --workspace --all-targets -D
warnings`, `cargo test --workspace` all green; blit-tui 630 tests pass
(including `build_f1_push_execution_gates_mirror_purge_on_complete_scan`,
`build_delegated_execution_mirror_options`, and the `f3_pull_options`
option-matrix tests).

## Next slices (audit-7d, planned)

The F1 trigger planning cluster
(`plan_f1_trigger`/`plan_f1_delegated`/`needs_container_slash` + the
`TriggerOutcome` type) is the next reasonably-self-contained pure
candidate. After that, what remains is the genuinely AppState-coupled
core: the `spawn_*`/`run_*` task plumbing (borrows channels/state, spawns
tokio tasks) and the `run_router` event loop / `handle_pane_action` /
render orchestration. Will stop and report to the owner before forcing a
risky split if no clean pure-helper clusters remain.
