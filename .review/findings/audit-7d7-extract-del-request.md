# audit-7d7-extract-del-request: extract F3 delete request-building helpers from main.rs

**Severity**: Refactor / code-health
**Status**: In progress / pending review
**Branch**: `phase5/a1`
**Commit**: `b18e5f9`
**Parent finding**: `audit-7-code-health` (audit-7d — the 11K-line
blit-tui `main.rs` behavior-preserving module split). **Part 7 of N.**

## What

Continues audit-7d's incremental split of `main.rs`. This slice extracts
the cohesive cluster of **pure** F3-delete request-building helpers into a
new `crate::del_request` module:

- `del_wire_path(&Path) -> String` (d-45 R2) — the module-relative Purge
  wire path (forward-slash joined regardless of client OS), a named
  wrapper over `blit_app::endpoints::rel_path_to_string`.
- `build_delete_request(Vec<RemoteEndpoint>, batch) -> Option<(RemoteEndpoint, Vec<String>, String, Option<String>)>`
  (d-50) — assembles the single-`Purge` request from resolved
  cursor/marked endpoints: filters non-deletable targets, converts to wire
  rel-paths, and shapes the label + gate_path (batch vs single).
- `is_deletable_remote_path(&RemoteEndpoint) -> bool` (d-45) — refuses
  module roots / empty rel-paths / bare-host Discovery endpoints (mirrors
  `blit rm`'s guard). The dispatcher gates the confirm prompt on it.

All three are pure (no async, no `AppState`).

## Approach (behavior-preserving)

Verbatim move — no logic change. The cluster's only non-extern dependency
is the `RemoteEndpoint` type (`use blit_core::remote::endpoint::RemoteEndpoint`);
everything else (`blit_app::admin::rm`, `blit_app::endpoints`) is reached
by absolute extern-crate paths inside the fn bodies. `build_delete_request`
calls the other two within the same module, so they resolve directly.

A crate-root `use crate::del_request::{build_delete_request, del_wire_path, is_deletable_remote_path}`
re-exposes all three so every non-test caller resolves unchanged:
`build_delete_request` at the `handle_pane_action` dispatcher (~2030),
`is_deletable_remote_path` at the dispatcher (~1948) and in
`plan_f1_trigger`/`plan_f1_delegated` (the move-direction guard),
`del_wire_path` in `spawn_f3_pull`/`spawn_f1_delegated_pull`. The ~10
inline unit tests reach them via the test module's `use super::*`. All
three have non-test callers, so all three are re-exported at crate root
(no test-local-import needed). The compiler + `clippy -D warnings` + the
full blit-tui suite are the behavior-preservation proof.

## Files changed

- `crates/blit-tui/src/del_request.rs` (new): the 3 `pub(crate)` fns +
  `use blit_core::remote::endpoint::RemoteEndpoint;`.
- `crates/blit-tui/src/main.rs`: `mod del_request;` + the `use`; the 3 fn
  definitions removed.

## Verification

`cargo fmt --all --check`, `cargo clippy --workspace --all-targets -D
warnings`, `cargo test --workspace` all green; blit-tui 630 tests pass
(including `del_wire_path_is_forward_slash_joined`, the
`build_delete_request` single-vs-batch tests, and the
`is_deletable_remote_path` guard tests).

## Next slices (audit-7d, planned)

Remaining clusters are progressively more AppState-coupled: the F1 trigger
planning (`plan_f1_trigger`/`plan_f1_delegated`/`needs_container_slash` +
the `TriggerOutcome` type) is the next reasonably-self-contained pure
candidate; then the request/execution builders
(`build_f1_push_execution`/`build_delegated_execution`/`f3_pull_options`);
and ultimately the `spawn_*`/`run_*` task plumbing and the `run_router`
event loop / `handle_pane_action` / render orchestration. Will stop and
report to the owner before forcing a risky split if no clean pure-helper
clusters remain.
