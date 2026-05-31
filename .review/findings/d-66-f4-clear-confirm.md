# d-66-f4-clear-confirm: y/N gate on the F4 profile-history clear

**Severity**: Feature (safety — destructive-action consistency)
**Status**: In progress / pending review (round 2 — base refresh)
**Branch**: `phase5/a1`
**Commit**: `0f4cd64` (R1 `82f27e4`)

## What

The F4 Profile pane's `[c] clear` hotkey deleted
`perf_local.jsonl` (the local performance-history log)
**immediately on a single keystroke**, with no confirmation.
It was the only destructive action in the TUI without a y/N
gate:

- F2 cancel → `confirm_cancel` (d-29, opt-in)
- F3 delete → read-only gate + confirm (d-46)
- F3 mirror/move → destructive confirm (d-55 / d-57)
- F1 push mirror/move → destructive confirm (d-65)
- **F4 clear → (none)** ← this gap

The wipe is permanent — the records can't be recovered — so a
reflexive `c` (especially since `c` is also the *copy* verb on
the F4 transfer block) could silently destroy history.

## Approach

Mirrors the d-65 modal-confirm pattern exactly:

- `ProfileState` gains `confirming_clear: bool` +
  `begin_clear_confirm` / `is_confirming_clear` /
  `cancel_clear_confirm`.
- `UserAction::ProfileClear` now **arms** the confirm
  (`begin_clear_confirm`) instead of clearing. The actual clear
  pipeline (`apply_profile_clear` → `apply_lifecycle_outcome` →
  re-fetch-on-success) is unchanged — it just moved behind the
  gate.
- A pre-dispatch routing guard (alongside the d-65 F1 guard and
  the F4 verify-edit guard) sends y/n/Esc to
  `handle_profile_clear_confirm_keystroke` while armed, before
  `key_action` (where bare `Esc` would otherwise map to Quit).
- `handle_profile_clear_confirm_keystroke`: `y`/`Y` → disarm +
  run the clear + re-fetch; `n`/`N`/`Esc` → disarm, no clear;
  Ctrl-c / F-keys / `?` fall through (return `false`) so the
  operator is never trapped; any other key is swallowed and the
  confirm stays armed.
- Render: `render_records_summary` shows a red
  `clear ALL local performance history? this is permanent ·
  [y / N or Esc]` line while armed — same convention as the
  Local-transfer block's mirror/move confirm (red in-block
  banner, no footer swap).

### Doc scrub

While here, removed the stale module-doc claims in
`profile.rs` and `screens/f4.rs` that `[c]/[d]/[e]` and the
Verify/Diagnostics sub-blocks were deferred "to a future
slice" — all are wired. (The reviewer reopened d-62/d-63/d-64
for exactly this kind of stale doc; pre-empting it here.) The
f4.rs layout box was also out of date — it omitted the verify /
diagnostics / transfer blocks; now corrected.

## Files changed

- `crates/blit-tui/src/profile.rs`: `confirming_clear` +
  three methods; module doc; lifecycle unit test.
- `crates/blit-tui/src/main.rs`: `ProfileClear` arms the
  confirm; routing guard; `handle_profile_clear_confirm_keystroke`;
  4 tests.
- `crates/blit-tui/src/screens/f4.rs`: red confirm banner in
  the records-summary block; module doc + layout box.

## Tests

557 total (was 552, +5):

- `profile::clear_confirm_lifecycle` — begin/is/cancel.
- `handle_profile_clear_confirm_cancel_keeps_history` — n/N/Esc
  disarm without clearing.
- `handle_profile_clear_confirm_swallows_unrelated_keys` — a
  stray key is consumed and leaves the confirm armed.
- `handle_profile_clear_confirm_lets_escape_hatches_through` —
  Ctrl-c / F-key / `?` fall through, confirm untouched.
- `handle_profile_clear_confirm_y_clears_and_disarms`
  (`#[tokio::test]`) — `y` disarms; the clear targets a tempdir
  config override (`set_config_dir`) so it never touches the
  operator's real `perf_local.jsonl`, and the spawned re-fetch
  has a runtime.

## Known gaps

1. **remote→remote (delegated)** trigger still pending (no clean
   reusable execution entry).
2. **Multi-daemon F2** still pending.

These are the substantial remaining TUI_DESIGN items; the F1/F3/
F4 interactive surface is otherwise feature-complete.

## Out of scope

- A config knob to make the clear-confirm opt-out: deliberately
  not added — unlike the reversible F2 cancel (d-29 made *that*
  opt-in), clearing history is irreversible, so the gate is
  always-on, matching the F3/F1 mirror·move confirms.

## Round 2 (base refresh — no code change)

The R1 sentinel (`82f27e4`) was a clean *Blocker* only because it was
stacked on the reopened `d-65` base, which still carried the
mirror-purge data-loss bug at that SHA. d-66's own logic was not
faulted. Refreshed onto `0f4cd64` (d-65 R2 fixed); the d-66 change
itself is byte-identical.

## Reviewer comments

(empty — pending round-2 grade)
