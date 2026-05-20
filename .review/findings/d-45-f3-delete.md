# d-45-f3-delete: D deletes the F3 cursor path with confirm

**Severity**: Feature (designed — TUI_DESIGN §5.3 `D: delete`)
**Status**: In progress / pending review
**Branch**: `phase5/a1`
**Commit**: `f0be497`

## What

The last unbuilt F3 cursor action from the design hotkey bar.
`D` (Shift+d) opens a confirm prompt for the cursor-selected
remote path; `y` issues a `Purge` RPC, `n`/`N`/`Esc` aborts:

```
status · delete nas:/home/photos/old.jpg? y/N   ← red, modal
status · deleting...                             ← yellow
status · deleted 3 file(s)                       ← green (path-gated)
status · delete failed: read-only module         ← red (path-gated)
```

## Approach

A new `f3del.rs` state machine mirroring the F3 pull / du
machines: `Idle → Confirming → Deleting → Done|Error`,
generation-guarded by `request_id`.

### Destructive-safety choices

- **Shift+D, not `d`.** Lowercase `d` is `ProfileDisable` (F4)
  and `key_action` is a global map; the capital both avoids the
  collision and makes deletion a deliberate keystroke.
- **Modal confirm.** While confirming, `handle_f3_delete_keystroke`
  swallows every key except `y`/`n`/`Esc` (and the global `?` /
  Ctrl-c / F-keys). A stray `p`/`u`/`/` can't stack another
  prompt or move the cursor mid-confirm.
- **Frozen target.** The resolved `RemoteEndpoint` is captured
  into `Confirming` at prompt-open; navigating afterward can't
  change what gets deleted (the d-30 batch-cancel lesson).
- **Module-root guard (client).** `is_deletable_remote_path`
  refuses a module root / empty rel-path / bare-host discovery
  endpoint — you can't nuke a whole module from the TUI (mirrors
  `blit rm`'s guard). Pure + unit-tested.
- **Read-only enforcement (server).** The daemon rejects `Purge`
  on a read-only module; the error flows into the footer. Relying
  on the server (the authority) is correct; client-side
  key-disable is a deferred polish (Known gaps).

### Rendering

`F3DelDisplay` bridge: `Confirming`/`Deleting` always show (an
active operation); `Done`/`Error` are **path-gated** like the
d-41 du display, so a stale outcome hides once the cursor leaves
the deleted path — no TTL machinery.

## Files changed

- `crates/blit-tui/src/f3del.rs` (new): `F3DelState` machine +
  10 unit tests.
- `crates/blit-tui/src/main.rs`:
  - `mod f3del`; AppState `f3_del` + `f3_del_reply_tx` + channel.
  - `F3DelReply`, `spawn_f3_del`, `run_f3_del`, pure
    `is_deletable_remote_path`.
  - `f3_del_to_display` bridge; `UserAction::F3DeleteBegin`; `D`
    key mapping; F3 dispatch arm (guarded); delete reply select
    arm; `handle_f3_delete_keystroke` (modal) + input-router gate.
  - 5 AppState test fixtures updated; 5 new tests.
- `crates/blit-tui/src/screens/f3.rs`: `F3DelDisplay` enum;
  footer delete fragment.
- `crates/blit-tui/src/help.rs`: `D` keymap row; modal height
  40→41; keymap test backend grown so the taller keymap renders
  un-clipped; scrollbar-fits test area grown to match.

## Tests

+17 tests (435 → 452 in blit-tui):

- `f3del::tests` (10): begin/confirm/cancel transitions, frozen
  target, generation-guarded apply_done/apply_error, stale-drop,
  begin-noop-while-deleting, confirm-none-when-not-confirming.
- `main::tests` (5): `D` → F3DeleteBegin (and `d` stays
  ProfileDisable); `is_deletable_remote_path` rejects module
  root / discovery, accepts a sub-path; `f3_del_to_display`
  Confirming always shows, Done is path-gated.
- 2 pre-existing tests updated for the new `D` binding + taller
  help modal.

`cargo fmt --all -- --check`, `cargo clippy --workspace
--all-targets -- -D warnings`, and `cargo test --workspace` all
green.

## Known gaps

1. **No client-side read-only key-disable.** The design says
   read-only modules "disable the key"; d-45 instead lets the
   prompt open and surfaces the daemon's rejection in the footer.
   Correct (server is the authority) but one extra keystroke. A
   follow-on could thread the cursor module's `read_only` flag to
   gate `D` up front.
2. **Single-path only.** `D` deletes the cursor row; the design's
   multi-select (`space`) + batch delete is a separate, larger
   feature (no selection-set exists yet).
3. **No outcome TTL.** The path-gating hides a stale result on
   cursor move, but a `Done`/`Error` for the current row persists
   until the operator navigates. Matches the du display; a TTL
   (d-38 style) could be added if it lingers in practice.

## Out of scope

- Client-side read-only gating.
- Multi-select batch delete.
- Outcome auto-hide TTL.

## Reviewer comments

(empty — pending grade)
