# d-5-f4-local-move: F4 `V` move trigger

**Severity**: Feature (closes the c/m/v triad from TUI_DESIGN)
**Status**: In progress / pending review
**Branch**: `phase5/a1`
**Commit**: filled by the sentinel commit

## What

Completes the F4 transfer triad listed in TUI_DESIGN §5:
`C` copy, `M` mirror, **`V` move**. Move = copy + delete
source. Reuses the same Verify-form Source/Destination
pair that copy and mirror already use, and shares the
mirror-style confirmation prompt — but with a louder
"DELETE the SOURCE" warning since move is the most
destructive of the three triggers.

The implementation mirrors the CLI's `blit move`
(`crates/blit-cli/src/transfers/mod.rs:430-503`)
including the R47-F4 data-loss safety gate: if the copy
pass produces `unreadable_paths`, the source is left
intact and an error surfaces in the F4 transfer block.

## Approach

### TransferKind / TransferStatus

```rust
pub enum TransferKind {
    Copy,
    Mirror,
    Move,  // new
}

pub enum TransferStatus {
    Idle,
    ConfirmingMirror,
    ConfirmingMove,  // new — same shape, different prompt
    Running { kind },
    Done { kind, summary, finished_at },
    Error { kind, message },
}
```

`is_confirming_move()` symmetric to
`is_confirming_mirror()`. `is_confirming()` aggregates
both for the `y` / `n` handlers. `is_busy()` already
covers `is_running() || is_confirming()` so adding
`ConfirmingMove` to the model automatically gates a
second trigger while the confirm is open.

### Move executor

New `perform_local_move(src, dst)` async helper. After
the copy returns:

1. Inspect `summary.unreadable_paths`. If non-empty,
   return an error mentioning the count + first few
   paths. The CLI uses the same phrasing
   (`crates/blit-cli/src/transfers/mod.rs:455`); the TUI
   adapts the message for the single-line transfer
   banner.
2. `tokio::fs::remove_dir_all(src)` (or `remove_file`)
   based on `src.is_dir()`. If the source already doesn't
   exist by the time we get here, treat as success — the
   post-condition holds.

`spawn_local_move` wraps `perform_local_move` and ferries
the reply back through the same `TransferReply` channel
as `spawn_local_transfer`, so the unified loop's apply
arm doesn't need a new variant.

### Key dispatch

- `UserAction::TransferMove` mapped to capital `V`.
- F4 arm: same prepare-then-confirm shape as
  `TransferMirror`:
  ```rust
  UserAction::TransferMove if can_start_transfer(app) => {
      match prepare_local_transfer(...) {
          Ok(_) => app.transfer.begin_confirm_move(),
          Err(msg) => app.transfer.note_validation_error(Move, msg),
      }
  }
  ```
- `UserAction::TransferMirrorConfirm` gains a second arm
  guarded on `app.transfer.is_confirming_move()` —
  re-validates and spawns `spawn_local_move`.
- `UserAction::TransferCancel` already used
  `is_confirming()` (introduced in d-4 R2 design), so it
  picks up move cancellation for free.

### Render

`render_transfer` block in `f4.rs` gains a
`ConfirmingMove` arm:

```text
move will DELETE the SOURCE after copy · [y/N] to confirm
```

Title bar updated: `Local transfer (C copy · M mirror · V move)`.
Idle banner: `press C to copy · M to mirror · V to move ...`.

## Files changed

- `crates/blit-tui/src/transfer.rs`:
  - `TransferKind::Move` + label.
  - `TransferStatus::ConfirmingMove`.
  - `is_confirming_move`, `is_confirming`,
    `begin_confirm_move`.
  - `cancel_confirm` extended to handle both confirm
    states.
- `crates/blit-tui/src/screens/f4.rs`:
  - ConfirmingMove render arm.
  - Title + Idle hint updated for the c/m/v triad.
- `crates/blit-tui/src/main.rs`:
  - `UserAction::TransferMove`.
  - `key_action` maps `V`.
  - F4 dispatch: `TransferMove`, second
    `TransferMirrorConfirm` arm for move.
  - `spawn_local_move` + `perform_local_move`.

## Tests

+2 unit tests + 1 async test (150 → 152):

- `key_action_maps_transfer_triggers` extended: pins `V`
  → `TransferMove`, lowercase `v` stays unmapped.
- `transfer_state_move_confirm_lifecycle`: Idle →
  ConfirmingMove → cancel back to Idle. Asserts
  `is_confirming_move() && !is_confirming_mirror()` so
  the dispatcher's separate confirm arms route
  correctly.
- `perform_local_move_deletes_source_after_copy` (tokio
  async): tempdir, write source, run move, assert
  destination has the bytes AND source is gone.

`cargo fmt`, `cargo clippy --workspace --all-targets
-- -D warnings`, and `cargo test --workspace` all green.

## Known gaps

1. **No "moved N bytes" custom Done message.** The Done
   line reuses copy/mirror's "moved · N planned · N
   copied · N bytes" format — fine, but a more
   move-specific phrasing ("moved N files, source
   purged") would be a polish slice.

2. **No undo.** Once the source-delete runs, the move is
   irreversible. The CLI doesn't undo either — this is
   parity, not a regression.

3. **No selective filter.** `LocalMirrorOptions` has
   include/exclude support, but the F4 form doesn't
   expose filters yet. R49-F1 in the CLI rejects move
   + filters anyway (because filtered-out source files
   would still get purged), so a future filter-aware
   form on F4 will need the same gate.

## Out of scope (next slices)

- **Per-file progress events** during the copy phase.
- **F3 multi-select** + transfer trigger from the
  browse-tree cursor (the design's "select-then-act"
  flow).
- **Checksum mode toggle for Verify.**
- **e-3 themes / config** — `~/.config/blit/tui.toml`.

## Reviewer comments

(empty — pending grade)
