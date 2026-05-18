# d-3-f4-diagnostics: F4 diagnostics dump (`s` key)

**Severity**: Feature (third slice of milestone D; closes D's core scope)
**Status**: In progress / pending review
**Branch**: `phase5/a1`
**Commit**: filled by the sentinel commit

## What

Adds the F4 Diagnostics block — operator presses `s` to
write a JSON snapshot of the current Verify form's
Source/Destination pair to disk. The dump reuses
`blit_app::diagnostics::dump::endpoint_snapshot` so the
file shape matches the CLI's `blit diagnostics dump
--json` output.

Mnemonic: `s` ("snapshot") was chosen over `d` because
`d` is bound to `ProfileDisable`. TUI_DESIGN §5.4 listed
`[d] dump` and `[d] disable` on the same screen — this
slice resolves the conflict.

## Approach

### State (`diagnostics.rs`)

```rust
pub enum DiagnosticsStatus {
    Idle,
    Running,
    Done { path: PathBuf, written_at: Instant },
    Error { message: String },
}

pub struct DiagnosticsState {
    status: DiagnosticsStatus,
    request_id: u64,
}
```

Surface: `begin_dump() -> u64`, `apply_done(id, path)`,
`apply_error(id, msg)`. Generation-gated like every
other async surface in this crate.

### Spawn helper

`spawn_diagnostics_dump(id, source, destination, tx)`
runs the dump on `tokio::task::spawn_blocking`. The
synchronous core (`run_diagnostics_dump`):

1. Parses both endpoints via `parse_transfer_endpoint`.
2. Builds a JSON Value using `endpoint_snapshot` for
   each + `same_device` + `endpoint_display`.
3. Writes to
   `<config-dir>/diagnostics-<unix-ms>.json` (uses
   `blit_core::perf_history::config_dir` for parity with
   the CLI's location).
4. Returns the resulting `PathBuf`.

Errors at any step are flattened into a `String` so the
F4 status banner can render them.

### Key dispatch

`UserAction::DiagnosticsDump` added. `key_action` maps
`s` to it. F1/F2/F3 ignore (wildcard arm). F4's match
arm checks both fields are non-empty before kicking the
spawn — empty form → no-op rather than dump
`null`/`null`.

When F4's Verify form is in editing mode (`is_editing()`),
the `s` keystroke goes through `handle_verify_keystroke`
as a text edit, NOT as a snapshot trigger. Same context-
sensitive routing pattern that already protects `c`/`d`/
`e`/`r`.

### Render

F4 layout gains a 3-line Diagnostics block between the
Verify form and the footer. Status line:

- `Idle`: dim gray "press `s` to dump..."
- `Running`: yellow "writing diagnostics snapshot..."
- `Done`: green "wrote /path/to/file.json"
- `Error`: red "error: <message>"

Footer in action mode advertises the new key:
`status · q quit · r refresh · c clear · d disable · e enable · s snapshot · tab verify`.

## Files changed

- `crates/blit-tui/src/diagnostics.rs` (new):
  `DiagnosticsState`, `DiagnosticsStatus`, 5 unit tests.
- `crates/blit-tui/src/main.rs`:
  - `mod diagnostics;` declaration.
  - `AppState` gains `diagnostics` field + `diagnostics_reply_tx`.
  - New `DiagnosticsReply` envelope + select! arm.
  - `spawn_diagnostics_dump` + `run_diagnostics_dump`
    helpers.
  - `UserAction::DiagnosticsDump` + `key_action` `s`
    mapping.
  - F4 action arm with the empty-fields guard.
- `crates/blit-tui/src/screens/f4.rs`: new
  `render_diagnostics` block; `render_into` signature
  gains `&DiagnosticsState`; footer adds `s snapshot`.
- `crates/blit-tui/Cargo.toml`: `serde_json = "1"`
  (needed for the snapshot construction).

## Tests

+5 unit tests in `diagnostics::tests`:

- `new_state_is_idle`
- `begin_dump_increments_request_id`
- `apply_done_writes_path_when_current`
- `apply_done_drops_stale_generation`
- `apply_error_records_message`

114 blit-tui unit tests (was 109). Workspace passes
serially.

## Known gaps

1. **No path picker.** Operator types absolute paths into
   the Verify form; the dump reuses those values. Future
   polish could add a path-completion picker.

2. **No "open file in $EDITOR" affordance.** The dump
   path is rendered in the status line but the operator
   has to open it themselves. A future polish could spawn
   `$EDITOR` on the file.

3. **No history of past dumps.** Each dump overwrites the
   status line. Files accumulate in
   `~/.config/blit/diagnostics-*.json`; the operator can
   `ls` to find them. A future polish could surface the
   last N dump paths in the F4 footer.

4. **`s` key is also a printable char.** When the Verify
   form is being edited it's correctly routed as text
   input (`handle_verify_keystroke`). Outside editing mode
   it triggers the dump. This matches the existing
   `c`/`d`/`e`/`r` context-sensitive routing.

## Out of scope (next slices)

- Mode toggle (size+mtime ↔ checksum) for Verify — d-2b.
- Per-file diff drill-down for Verify — d-2c.
- Result list of past dumps + open-in-editor — future
  polish.

## Round 2 (sha filled by sentinel)

Reviewer caught that round-1's JSON shape didn't match
the CLI's `blit diagnostics dump --json` output:

- Missing `invocation` field.
- Missing `rsync_resolution { source_is_contents,
  destination_is_container, pre_resolve_destination,
  resolved_destination, resolution_changed }` block.
- `destination` and `same_device` were computed against
  the un-resolved destination, so container targets (e.g.
  `cp src /dst/`) would report the wrong effective path.

### Fix

`run_diagnostics_dump` now mirrors the CLI's
`run_diagnostics_dump` (`crates/blit-cli/src/diagnostics.rs:160`):

1. Parses source + raw destination.
2. Calls `blit_app::transfers::resolution::resolve_destination`
   to compute the effective destination.
3. Calls `source_is_contents` + `dest_is_container` for
   the resolution-flag block.
4. Calls `endpoint_snapshot` for source AND
   resolved-destination (not pre-resolved).
5. Calls `same_device` against the resolved destination.
6. Emits the full top-level shape including `invocation`
   and `rsync_resolution`.

The pure JSON-building work is extracted into a
`build_diagnostics_snapshot` helper so the regression
tests can compare shape without touching disk.
`run_diagnostics_dump` becomes a thin "build snapshot +
write to disk" wrapper.

### Tests

+2 regression tests:

- `tui_dump_shape_matches_cli_contract`: asserts the
  top-level JSON has the same keys as the CLI's output
  (`blit_version`, `invocation`, `source`, `destination`,
  `rsync_resolution`, `same_device`) and that
  `rsync_resolution` has all five fields the CLI emits.
- `tui_dump_resolves_destination_for_container_targets`:
  builds a real temp src directory + temp container
  destination (trailing-slash), then asserts
  `source_is_contents=false`, `destination_is_container=true`,
  `pre_resolve_destination != resolved_destination`, and
  `resolution_changed=true`. This is the bug-bait case
  the reviewer called out — the TUI must NOT report the
  un-resolved destination.

116 blit-tui unit tests (was 114). Workspace passes
serially.

## Reviewer comments

(empty — pending grade)
