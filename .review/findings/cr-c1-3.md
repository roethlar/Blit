# cr-c1-3: Stale directory buttons remain active during navigation

**Severity**: MEDIUM — ordinary double-click / rapid clicks on a still-visible listing can navigate to a joined path the operator never asked for.
**Status**: Verified
**Branch**: —
**Commit**: `95357667`

## Evidence
`crates/blit-gui/src/app.rs` shows "Loading…" when `is_loading()` is true, then still collects `model.listing()` into clickable directory buttons. Core `NavigateTo` sets `loading` and the new `current_path` but does not clear the previous listing. Trigger: listing of `/` includes `photos/`; first click dispatches `NavigateTo(/photos)` (path now `/photos`, listing still the old rows); second click joins stale `photos` onto that path → `NavigateTo(/photos/photos)`. The first browse is superseded by generation; the pane settles on the unintended path or a missing-path error.

## Predicted observable failure
A conventional double-click (or two clicks before the first listing returns) on a remote module/directory opens `/name/name` instead of `/name`. Local disks can hide this because the first listing often returns before the second click; daemon browse is slow enough to hit it.

## What
The C1 face treats the previous listing as live while a newer browse is in flight, so stale rows can dispatch another `NavigateTo` against the already-updated `current_path`.

## Approach
The face, not the core, owns click dispatch. `interactive_listing` returns no rows while `model.is_loading()` is true, so stale names cannot be joined onto the already-updated `current_path`. Core still keeps the previous listing for the host; only the click surface goes empty. The browse pane already shows "Loading…" for that interval.

## Files changed
- `crates/blit-gui/src/lib.rs` — `interactive_listing` + `in_flight_browse_has_no_interactive_rows`
- `crates/blit-gui/src/app.rs` — browse pane renders `interactive_listing`, not `model.listing()`

## Guard proof
- `blit_gui::tests::in_flight_browse_has_no_interactive_rows` — load a directory, start a second browse, assert the model listing is still populated and `interactive_listing` is empty. Replacing the helper with `model.listing()` turns the test red; restore turns it green.

## Coder dispute
None.

## Known gaps
None.

## Reviewer comments
Reviewer: codex / gpt-5.6-sol / xhigh / standard — codex-cli 0.147.0; range `767dc1e0..3487874f`, SHAs pin-verified; capability_ok=true; 2026-08-13T05:50Z; record `.review/results/c1-3-range.codex.json` (raw `logs/c1-3-range.codex.jsonl`). Cache was empty; pair taken from this machine's `~/.codex/config.toml` and D-2026-07-31-3 (last owner-named landed-slice pair). Hook notice `clamping SessionEnd hook timeout to 3s` recorded as an environment note, not an invalidation.
Generation verdict: findings (1) — this finding admitted at intake.

**r1 (verification, 2026-08-13T06:20Z): ACCEPTED.** Reviewer: codex / gpt-5.6-sol / xhigh / standard — codex-cli 0.147.0; base `3487874f` head `95357667` pin-verified; guard_confirmed=true; capability_ok=true; record `.review/results/cr-c1-3.codex.json` (raw `logs/cr-c1-3-verify.codex.jsonl`). Reviewer independently re-ran the mutation guard in a disposable worktree at the head SHA: reverting the helper reds `in_flight_browse_has_no_interactive_rows`; restore greens it. All blit-gui tests passed.
