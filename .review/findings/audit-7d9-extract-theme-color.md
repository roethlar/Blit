# audit-7d9-extract-theme-color: extract theme-color mapping helpers from main.rs

**Severity**: Refactor / code-health
**Status**: In progress / pending review
**Branch**: `phase5/a1`
**Commit**: `6ddce2e`
**Parent finding**: `audit-7-code-health` (audit-7d — the 11K-line
blit-tui `main.rs` behavior-preserving module split). **Part 9 of N.**

## What

Continues audit-7d's incremental split of `main.rs`. This slice extracts
the two **pure** theme-color mapping helpers into a new
`crate::theme_color` module:

- `base_theme_style(Option<Color>, Option<Color>) -> Option<Style>`
  (dark-1) — the base frame style from the optional `[theme]` bg/fg;
  `None` when both unset (terminal colors show through).
- `raw_color_to_ratatui(config::RawColor) -> ratatui::style::Color` (e-7)
  — bridges the config schema's `RawColor` (which avoids leaking ratatui
  types into the schema layer) to the renderer's ratatui `Color`.

Both are pure (no `AppState`); their only deps are `ratatui::style::*`
(fully qualified) and `config::RawColor` (`use crate::config`).

## Approach (behavior-preserving)

Verbatim move — no logic change. A crate-root
`use crate::theme_color::{base_theme_style, raw_color_to_ratatui}`
re-exposes both so the `run_router` render call sites (main.rs ~891-901,
where the base layer is built from config) **and** the inline unit tests
(via the test module's `use super::*`) resolve unchanged. Both have
non-test callers, so both are re-exported at crate root. The compiler +
`clippy -D warnings` + the full blit-tui suite are the
behavior-preservation proof.

## Files changed

- `crates/blit-tui/src/theme_color.rs` (new): the 2 `pub(crate)` fns +
  `use crate::config;`.
- `crates/blit-tui/src/main.rs`: `mod theme_color;` + the `use`; the 2 fn
  definitions removed.

## Verification

`cargo fmt --all --check`, `cargo clippy --workspace --all-targets -D
warnings`, `cargo test --workspace` all green; blit-tui 630 tests pass
(including `base_theme_style_built_from_set_colors`).

## Note on the F1 trigger-planning cluster (NOT moved)

The originally-planned next candidate — `plan_f1_trigger` /
`plan_f1_delegated` — was inspected and is **not** a pure cluster:
`plan_f1_trigger(app: &mut AppState, …)` mutates `AppState` and
orchestrates the `spawn_*` tasks (it *is* part of the coupled dispatch
core). Its only genuinely pure inner pieces are `needs_container_slash`
(a one-liner string check) and the `TriggerOutcome` enum (the planners'
return type, which would have to stay reachable by the planners that
remain in main.rs). Moving those alone provides little value and isn't a
clean closed set, so they were left in place.

## Next slices (audit-7d) — approaching the coupled core

Remaining low-risk pure candidates are getting sparse. A few read-only
`&AppState` predicates may still be cleanly extractable as a small
"predicates" slice (`needs_live_tick`, `can_start_transfer`,
`esc_cancels_confirm`) along with `cancel_endpoint`/`snapshot_active_targets`,
but they read `AppState`/`TransfersState` so they'd reference those via
the crate root. Beyond that, what remains is the genuinely coupled
event-loop core: `plan_f1_*`, the `spawn_*`/`run_*` task plumbing, and the
`run_router` / `handle_pane_action` / render orchestration — which should
be left in `main.rs` (or done as its own dedicated reviewed effort)
rather than force-split. Will assess one predicates slice next, then
report to the owner that the refactor has reached the coupled core.
