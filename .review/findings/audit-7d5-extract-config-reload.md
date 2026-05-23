# audit-7d5-extract-config-reload: extract config hot-reload helpers from main.rs

**Severity**: Refactor / code-health
**Status**: In progress / pending review
**Branch**: `phase5/a1`
**Commit**: `4e728b5`
**Parent finding**: `audit-7-code-health` (audit-7d — the 11K-line
blit-tui `main.rs` behavior-preserving module split). **Part 5 of N.**

## What

Continues audit-7d's incremental split of `main.rs`. This slice extracts
the `Ctrl+R` config hot-reload helpers into a new `crate::config_reload`
module:

- `reload_tui_config(&config::TuiConfig, Instant) -> (config::TuiConfig, ReloadBanner)`
  (d-36) — the thin wrapper that re-reads `tui.toml` via `config::load`
  (the only I/O) and delegates the decision to `classify_reload`.
- `classify_reload(loaded, warning, current, now) -> (config::TuiConfig, ReloadBanner)`
  — the **pure** keep-vs-adopt decision: on a parse warning keep the
  current config (the loader returns defaults on failure, which would
  silently wipe settings) with a red banner; otherwise adopt the loaded
  config with a green banner.

## Approach (behavior-preserving)

Verbatim move — no logic change. Like the F2 slice (7d4), the AppState-
coupled type stays put: **`ReloadBanner`** is an `AppState.reload_banner`
field with its own `impl` block and is constructed directly in tests, so
it remains in `main.rs` and the new module references it (and constructs
it) via the crate-root path (`use crate::{config, ReloadBanner}`) — a
child module can name and build an ancestor-module-private struct.

Re-export wiring differs from earlier slices because of caller topology:
- `reload_tui_config` has a non-test caller (the event loop's `Ctrl+R`
  branch at main.rs ~1249), so it is re-exported at crate root
  (`use crate::config_reload::reload_tui_config`).
- `classify_reload`'s **only** non-test caller was `reload_tui_config`,
  which moved with it; its remaining callers in `main.rs` are the two
  inline unit tests. A crate-root `use` of it would be unused in the
  non-test bin build and trip `clippy -D warnings`, so it is imported
  **test-locally** (`use crate::config_reload::classify_reload;` inside
  `mod tests`) instead. Behavior unchanged; the tests resolve it.

## Files changed

- `crates/blit-tui/src/config_reload.rs` (new): the 2 `pub(crate)` fns,
  with `use crate::{config, ReloadBanner};` + `use std::time::Instant;`.
- `crates/blit-tui/src/main.rs`: `mod config_reload;` + the crate-root
  `use reload_tui_config` + the test-module `use classify_reload`; the 2
  fn definitions removed.

(The commit also folds in the leftover audit-7d3 ready-sentinel removal —
verdict-cleanup bookkeeping the coder now owns.)

## Verification

`cargo fmt --all --check`, `cargo clippy --workspace --all-targets -D
warnings`, `cargo test --workspace` all green; blit-tui 630 tests pass
(including `classify_reload_success_adopts_new` and
`classify_reload_parse_error_keeps_current`).

## Next slices (audit-7d, planned)

The tick-budget pure helpers (`compute_tick_budget`/`min_opt`); the
background-task plumbing (`spawn_*`/`run_*` + Reply structs,
`del_wire_path`); the F1 trigger planning
(`plan_f1_trigger`/`plan_f1_delegated`/`TriggerOutcome`); and ultimately
the `run_router` event loop / `handle_pane_action` / render orchestration
— stopping before the genuinely AppState-coupled core if no clean
pure-helper clusters remain.
