# d-4-f4-local-transfers: F4 `C` copy / `M` mirror triggers

**Severity**: Feature (closes the "TUI can't initiate transfers" gap)
**Status**: In progress / pending review
**Branch**: `phase5/a1`
**Commit**: filled by the sentinel commit

## What

Adds local-transfer triggers on F4. Operator workflow:

1. F4 (default initial pane after a1-6 routing).
2. `Tab` enters the Verify form; types Source path; `Tab`;
   types Destination path; `Esc` drops focus.
3. `C` triggers a copy. `M` triggers a mirror.
4. The new "Local transfer" block on F4 surfaces the
   running status and the final
   `planned / copied / bytes` summary on completion.

User observation that motivated this slice: "the TUI
doesn't seem to have a way to do local transfers." The
prior A.1/D scope built only browse/verify/diagnostics
operations on the SRC→DST pair; running the actual
copy/mirror was missing.

Uses the same `blit_app::transfers::local::run` code
path as `blit copy` / `blit mirror`, so semantics match
the CLI verbatim.

## Approach

### State (`transfer.rs`)

New module with:

```rust
pub enum TransferKind { Copy, Mirror }

pub enum TransferStatus {
    Idle,
    Running { kind },
    Done { kind, summary: Box<LocalMirrorSummary>, finished_at },
    Error { kind, message },
}

pub struct TransferState { status, request_id }
```

Generation guard same as `VerifyState` /
`DiagnosticsState`: stale replies from a previous run are
dropped. `summary` is `Box<...>` to keep the enum
discriminant lean (clippy::large_enum_variant — the
`LocalMirrorSummary` is ~272 bytes).

`TransferState::is_running()` is a guard the action
dispatcher checks so `C` / `M` keys don't kick a second
run while the first is in flight.

### Spawn helper

`spawn_local_transfer(id, kind, src, dst, tx)`:

```rust
let options = blit_core::orchestrator::LocalMirrorOptions {
    mirror: matches!(kind, TransferKind::Mirror),
    ..Default::default()
};
blit_app::transfers::local::run(&src, &dst, options).await
```

Returns a `TransferReply { request_id, kind, result }`.
The unified loop's apply arm routes to `app.transfer.apply_done` /
`apply_error` with the generation gate.

### Key dispatch

- `UserAction::TransferCopy` / `TransferMirror` added.
- `key_action` maps capital `C` / `M`. Lowercase `c` is
  taken by `ProfileClear`; capitals chosen so the keymap
  remains case-distinct.
- F4 arm gates on `can_start_transfer(app)`:
  `verify.source` + `verify.destination` non-empty AND
  no transfer running.

### Render

F4 layout gains a 3-line "Local transfer" block below
the Diagnostics block. Status line:

- `Idle`: dim "press C to copy or M to mirror Source → Destination"
- `Running`: yellow "copy running..." / "mirror running..."
- `Done`: green "copy done · 1234 planned · 1234 copied · 12345678 bytes"
- `Error`: red "copy failed: <message>"

## Files changed

- `crates/blit-tui/src/transfer.rs` (new):
  `TransferState`, `TransferStatus`, `TransferKind` + 6
  unit tests.
- `crates/blit-tui/src/main.rs`:
  - `mod transfer;` declaration.
  - `AppState` gains `transfer` + `transfer_reply_tx`.
  - `UserAction::TransferCopy` / `TransferMirror`.
  - `key_action` maps `C` / `M`.
  - F4 handler arms (gated by `can_start_transfer`).
  - `TransferReply` envelope + `spawn_local_transfer` +
    select! arm.
- `crates/blit-tui/src/screens/f4.rs`:
  - `render_into` signature gains `&TransferState`.
  - New `render_transfer` block.
  - Layout adds a 3-line region for it.

## Tests

+8 unit tests:

In `transfer::tests`:
- `new_is_idle_and_not_running`
- `begin_marks_running_with_kind`
- `apply_done_writes_when_current`
- `apply_done_drops_stale_generation`
- `apply_error_records_message`
- `transfer_kind_label`

In `main::tests`:
- `key_action_maps_transfer_triggers` — pins `C`/`M`
  mappings.
- Updated `key_action_maps_profile_lifecycle_keys` to
  drop the stale "uppercase C is unmapped" assertion
  (capital C now maps to TransferCopy).

129 blit-tui unit tests (was 121). Workspace passes
serially.

## Known gaps

1. **Sync block on the runtime.** `blit_app::transfers::local::run`
   already uses `spawn_blocking` internally. The wrapping
   `tokio::spawn` is so the TUI's main loop stays
   responsive while the transfer runs. Cancellation isn't
   wired — pressing `q` mid-transfer quits the TUI but
   doesn't abort the in-flight transfer (which finishes
   on its own).

2. **No progress events.** The block shows "running..."
   but no byte-level progress. The CLI emits its own
   progress; the TUI could subscribe to a similar feed
   in a future polish slice.

3. **No `V` (move) trigger.** Design lists `c`/`m`/`v`;
   `v` (move) would need a delete-after-copy semantic.
   `blit_app::transfers::local::run` doesn't expose
   that directly — move = copy + purge — so it's a
   follow-up slice.

4. **No confirmation modal for mirror.** Mirror can
   delete files at the destination. The CLI prompts;
   the TUI just runs. A future polish could add a
   confirmation modal for destructive ops.

5. **No multi-select.** Design's F3 spec called for
   `space` multi-select + `c`/`m`/`v` from the selection
   set. This slice operates on the single SRC/DST pair
   in the Verify form.

## Out of scope (next slices)

- **`V` (move) trigger.**
- **Mirror confirmation modal.**
- **Per-file progress events.**
- **F3 multi-select + transfer trigger from the
  browse-tree cursor.**
- **e-2 tab-strip counts** — still local WIP, will ship
  after this slice verifies.

## Reviewer comments

(empty — pending grade)
