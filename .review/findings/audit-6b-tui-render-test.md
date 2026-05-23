# audit-6b-tui-render-test: F4 render-doesn't-panic via ratatui TestBackend

**Severity**: Test Gap
**Status**: In progress / pending review
**Branch**: `phase5/a1`
**Commit**: `267f093`
**Parent finding**: `audit-6-test-gaps` (item 2).

## What

audit-6 item 2 flagged the absence of TUI render tests. F2, F3, and the
help overlay already drive their renders through a `TestBackend`, and
F4's tests cover its pure line-builders (predictor/summary/verify-
preview) — but F4's actual `render_into` (Profile + Verify + Diagnostics
+ Transfer, with the fixed-height vertical layout) was never rendered
through a real backend.

## Approach (no production change)

Added a `render_tests` module to `screens/f4.rs`:

- `f4_renders_default_state_without_panic` — renders the full F4 pane
  with default (empty/idle) `ProfileState`/`VerifyState`/
  `DiagnosticsState`/`TransferState` at 120×40, exercising the layout
  arithmetic and every sub-render's widget construction.
- `f4_renders_tiny_area_without_panic` — renders at 8×3, far below the
  ~23-row layout, asserting the `Layout` clamps gracefully rather than
  panicking on an under-sized/zero span.

## Files changed

- `crates/blit-tui/src/screens/f4.rs`: `render_tests` module (2 tests).
  No production change.

## Scope

One sub-item of audit-6. Remaining: 6e (pull-move/push-move). 6a/6c/6d/6f/
6g + (now) 6b cover the other items.

## Reviewer comments

(empty — pending review)
