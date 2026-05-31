# d-7-f4-verify-one-way-toggle: Verify direction toggle

**Severity**: Feature (the twin of d-6 — exposes
`compare_trees`'s second tuning flag)
**Status**: In progress / pending review
**Branch**: `phase5/a1`
**Commit**: filled by the sentinel commit

## What

`O` toggles the Verify form between two-way and one-way
direction:

- **two-way** (default): reports both `missing-on-src`
  AND `missing-on-dst`. Matches `blit check`'s default
  and the rsync `--existing`-free shape.
- **one-way**: skips the dst-walk so the operator only
  sees `missing-on-dst`. Useful for "did src reach dst?"
  audits where extras at dst don't matter (think:
  "verify backup landed" without flagging the backup's
  own pre-existing files).

This pairs with d-6's checksum toggle — together the two
keys expose the same tunables `blit check` already
exposes via `--checksum` and `--one-way`.

## Approach

### VerifyState

New `one_way: bool` field (default `false`) + methods:

- `one_way() -> bool`
- `toggle_one_way()` — flips the flag + calls
  `invalidate_run()` (same contract as
  `toggle_checksum`).

The two toggles are independent — flipping one doesn't
touch the other, so the operator can mix and match
(e.g. one-way + checksum for "thorough audit of what
src has").

### Spawn helper

`spawn_verify_run` signature gains a `one_way: bool`
parameter that threads through to
`compare_trees(_, _, use_checksum, one_way, _)`.

### Key dispatch

- `UserAction::ToggleVerifyOneWay` mapped to capital `O`.
- Lowercase `o` stays unmapped (reserved for future
  "open" / "open in editor" polish).
- F4 arm calls `app.verify.toggle_one_way()`.

### Render

The mode hint line now surfaces both flags in one string:

```
Mode: size+mtime · two-way · H toggles hash · O toggles direction
Mode: checksum   · one-way · H toggles hash · O toggles direction
```

Magenta whenever **either** non-default is on — so the
operator's eye catches the deviation regardless of which
flag they flipped.

## Files changed

- `crates/blit-tui/src/verify.rs`:
  - `one_way: bool` field.
  - `one_way()` / `toggle_one_way()` methods.
  - Module doc updated.
- `crates/blit-tui/src/main.rs`:
  - `UserAction::ToggleVerifyOneWay`.
  - `key_action` maps `O`.
  - F4 arm calls toggle.
  - `spawn_verify_run` gains the `one_way` parameter.
  - Call site passes `app.verify.one_way()`.
- `crates/blit-tui/src/screens/f4.rs`:
  - `render_verify` composes the dual mode line.

## Tests

+5 unit tests (157 → 162):

In `verify::tests`:
- `new_state_uses_two_way_compare`
- `toggle_one_way_flips_the_flag`
- `toggle_one_way_invalidates_done_result`
- `checksum_and_one_way_toggles_are_independent`

In `main::tests`:
- `key_action_maps_verify_one_way_toggle` — `O` →
  `ToggleVerifyOneWay`, `o` stays unmapped.

`cargo fmt`, `cargo clippy --workspace --all-targets
-- -D warnings`, and `cargo test --workspace` all green.

## Known gaps

1. **No persistence.** Toggle resets to default on each
   TUI launch. A future `tui.toml` config (e-3) could
   remember the operator's last setting.

2. **No "direction-aware" Done banner.** The Done line
   still reports `missing-on-src: N` even when in
   one-way mode (where that count is always 0 because
   the walk was skipped). Cosmetic only — the mode label
   tells the operator the count is meaningless in
   one-way mode — but a future polish could hide
   irrelevant rows.

## Out of scope (next slices)

- **e-3 themes / config** — would persist these toggles.
- **Per-file progress events** during local transfers.
- **F3 multi-select** + transfer trigger from the
  browse-tree cursor.

## Reviewer comments

(empty — pending grade)
