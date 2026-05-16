# a0-final-cleanup: Phase 5 A.0 — drop CLI shim re-exports

**Severity**: Refactor (no behavior change)
**Status**: In progress / pending review
**Branch**: `phase5/blit-app-extract`
**Commit**: filled by the sentinel commit

## What

Last A.0 sub-slice. Walks the remaining CLI `transfers/*.rs`
files and drops the now-unnecessary shim re-exports that were
added during earlier slices as backwards-compat stop-gaps —
deliberate decisions at the time to keep slices atomic and
avoid touching every call site in a single commit. With the
moves complete, the re-exports can go.

## Re-exports removed

From `crates/blit-cli/src/transfers/mod.rs`:

- `pub use endpoints::{Endpoint, format_remote_endpoint,
  parse_transfer_endpoint}` — double-shim through the local
  `endpoints` module that itself re-exported from
  `blit_app::endpoints`. Callers now import from blit-app
  directly.
- `pub(crate) use blit_app::transfers::resolution::{
  dest_is_container, resolve_destination, source_is_contents}`
  — only `resolve_destination` is used inside
  `transfers/mod.rs` itself, so it stays as a private `use`;
  the other two are imported directly by `diagnostics.rs`.
- `pub use blit_app::transfers::dispatch::{select_transfer_route,
  TransferKind, TransferRoute}` — these were re-exported for
  `main.rs`'s benefit. `main.rs` now imports `TransferKind`
  from `blit_app::transfers::dispatch` directly;
  `select_transfer_route` / `TransferRoute` are only used
  inside `transfers/mod.rs::run_transfer`.

From `crates/blit-cli/src/transfers/endpoints.rs`:

- `pub use blit_app::endpoints::{Endpoint,
  parse_transfer_endpoint, format_remote_endpoint,
  ensure_remote_destination_supported,
  ensure_remote_source_supported}` — every call site now
  imports those names from `blit_app::endpoints` directly.
- File reduced to the two clap-arg adapter wrappers
  (`ensure_remote_pull_supported` /
  `ensure_remote_push_supported`) — its sole reason to exist.

## Call sites updated

- `crates/blit-cli/src/main.rs` — `TransferKind` imported
  from `blit_app::transfers::dispatch`.
- `crates/blit-cli/src/diagnostics.rs` —
  `parse_transfer_endpoint` from `blit_app::endpoints`;
  resolution helpers from `blit_app::transfers::resolution`.
  Removed a stale doc comment that pointed at the now-gone
  `crate::transfers::*` re-export.
- `crates/blit-cli/src/transfers/remote.rs` — `Endpoint` +
  `format_remote_endpoint` from `blit_app::endpoints`.
- `crates/blit-cli/src/transfers/remote_remote_direct.rs` —
  `format_remote_endpoint` from `blit_app::endpoints`.
- `crates/blit-cli/src/transfers/mod.rs` — endpoint helpers,
  dispatch primitives, resolution `resolve_destination` all
  imported directly from blit-app.

## State after this slice

The only remaining `pub`/`pub(crate)` items in CLI
`transfers/mod.rs` are the actual entry points
(`run_transfer`, `run_move`) and the clap-arg filter helpers
(`build_filter` / `build_filter_spec`). No more pass-through
re-exports.

CLI `transfers/` content tally:
- `endpoints.rs` (28 LOC) — 2 clap-arg adapter wrappers only.
- `mod.rs` (~675 LOC) — `run_transfer` / `run_move` entry
  points, `filter_inputs` / `build_filter` / `build_filter_spec`
  wrappers, `display_endpoint` / `collapse_slashes` formatters,
  `confirm_destructive_operation` interactive prompt.
- `remote.rs` (~575 LOC) — `run_remote_pull_transfer` /
  `run_remote_push_transfer` clap wrappers, progress monitor
  spawn (`spawn_progress_monitor_with_options`), JSON / human
  presentation (`print_pull_json`, `print_push_json`,
  `describe_pull_result`, `describe_push_result`,
  `print_deferred_pull_result`, `print_deferred_push_result`).
- `remote_remote_direct.rs` (~205 LOC) — `run_remote_to_remote_direct`
  clap wrapper, JSON / human presentation
  (`print_delegated_json`, `describe_delegated_result`,
  `print_deferred_delegated_result`).
- `local.rs` (~350 LOC) — local-transfer clap wrapper +
  presentation (out of scope for this slice; not audited
  for shim re-exports because it was already direct).

Every remaining symbol in CLI `transfers/*` is either
(a) clap-coupled, (b) presentation (`println` / `eprintln` /
`print_*_json`), or (c) interactive (stdin prompt). Pure
orchestration logic now lives in `blit-app`.

## Files changed

- `crates/blit-cli/src/main.rs` — 1-line import change.
- `crates/blit-cli/src/diagnostics.rs` — 2-line import change
  + 6-line doc comment refresh.
- `crates/blit-cli/src/transfers/endpoints.rs` — file
  rewritten (35 → 28 LOC); only the two adapter wrappers
  remain.
- `crates/blit-cli/src/transfers/mod.rs` — import block
  reorganized; 3 `pub use` re-exports removed.
- `crates/blit-cli/src/transfers/remote.rs` — 1-line import
  change.
- `crates/blit-cli/src/transfers/remote_remote_direct.rs` —
  1-line import change.

## Tests added

None new. Workspace stays at 503 passed (same as the
pre-slice baseline). Cleanup is pure import-graph plumbing.

## Known gaps

A.0 done after this slice verifies. Next milestones per
`docs/plan/TUI_DESIGN.md` §8:

- B — `GetState` + `ActiveJobs` + recent ring
- M-Jobs — daemon-owned lifecycle + `CancelJob` + `detach`
- C — `Subscribe` + byte-level instrumentation
- A.1 — the TUI itself

## Reviewer comments

(empty — pending grade)
