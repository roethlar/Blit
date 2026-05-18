# d-6-f4-verify-checksum-toggle: Verify checksum mode

**Severity**: Feature (closes the "future polish" the
d-2 Verify slice explicitly called out)
**Status**: In progress / pending review
**Branch**: `phase5/a1`
**Commit**: filled by the sentinel commit

## What

`H` (Hash) toggles the Verify form between two
`compare_trees` modes:

- **size+mtime** (default): rsync-style fast comparison.
  Two files match if their length + modification time
  match. Catches the common case but misses edits that
  preserve mtime (file restored from snapshot, manual
  `touch`, mv across filesystems with some tools).
- **checksum**: per-file content hash. Slower but
  authoritative — what `blit check --checksum` does on
  the CLI.

The current mode is rendered in the Verify block:

```
Mode: size+mtime (fast, rsync default · H to toggle)
Mode: checksum (per-file content compare · H to toggle)
```

(The checksum line uses magenta so the operator's eye
catches the slower mode they opted into.)

## Approach

### VerifyState

New `use_checksum: bool` field (default `false`) +
methods:

- `use_checksum() -> bool` — read for the next run.
- `toggle_checksum()` — flips the flag AND calls
  `invalidate_run()`. Mode changes invalidate prior
  results because the displayed counts wouldn't match
  the header's mode label otherwise.

### Spawn helper

`spawn_verify_run` gains a `use_checksum: bool` parameter
that threads through to
`blit_app::check::compare_trees(_, _, use_checksum, false, _)`.
`one_way=false` stays hardcoded — F4 Verify defaults to
two-way, matching `blit check`'s default.

### Key dispatch

- `UserAction::ToggleVerifyChecksum` mapped to capital `H`.
- Lowercase `h` stays bound to `Ascend` (F3 navigation),
  so the toggle is uppercase-only — same case-convention
  the F4 transfer triggers use (`C`/`M`/`V`).
- F4 arm calls `app.verify.toggle_checksum()`.

### Edit interplay

Editing the Verify form already invalidates pending
runs via `insert_char` / `backspace` calling
`invalidate_run()`. The new toggle uses the same hook —
no special-case wiring needed.

## Files changed

- `crates/blit-tui/src/verify.rs`:
  - `use_checksum: bool` field.
  - `use_checksum()` / `toggle_checksum()` methods.
  - Module doc updated.
- `crates/blit-tui/src/main.rs`:
  - `UserAction::ToggleVerifyChecksum`.
  - `key_action` maps `H`.
  - F4 arm calls toggle.
  - `spawn_verify_run` gains the `use_checksum`
    parameter.
  - Call site passes `app.verify.use_checksum()`.
- `crates/blit-tui/src/screens/f4.rs`:
  - `render_verify` reads `verify.use_checksum()` and
    renders one of two mode labels.

## Tests

+5 unit tests (152 → 157):

In `verify::tests`:
- `new_state_uses_size_mtime_compare`
- `toggle_checksum_flips_the_flag`
- `toggle_checksum_invalidates_done_result`
- `toggle_checksum_drops_in_flight_reply`

In `main::tests`:
- `key_action_maps_verify_checksum_toggle` — `H` →
  `ToggleVerifyChecksum`, `h` stays `Ascend`.

`cargo fmt`, `cargo clippy --workspace --all-targets
-- -D warnings`, and `cargo test --workspace` all green.

## Known gaps

1. **No persistence across TUI restarts.** Mode resets
   to `size+mtime` on each launch. A future polish (e-3
   `tui.toml`) could remember the operator's last mode.

2. **No "checksum re-run" hint after a Done result.**
   If the operator runs in size+mtime mode, sees
   suspicious results, and wants to verify by checksum,
   they have to press `H` then `Enter` again. A future
   slice could add a "press H to re-verify with checksum"
   hint to the Done banner.

3. **No `one_way` toggle.** `compare_trees` accepts a
   `one_way: bool` flag for "treat src→dst only,
   ignore dst-only entries". The F4 form always uses
   two-way. Out of scope; `blit check` defaults the same.

## Out of scope (next slices)

- **e-3 themes / config** — `~/.config/blit/tui.toml`
  (would also persist mode toggles).
- **Per-file progress events** during local transfers.
- **F3 multi-select** + transfer trigger from the
  browse-tree cursor.

## Reviewer comments

(empty — pending grade)
