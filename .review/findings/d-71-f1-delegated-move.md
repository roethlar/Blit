# d-71-f1-delegated-move: remote→remote delegated move

**Severity**: Feature (TUI_DESIGN §1 "move … between any two endpoints")
**Status**: In progress / pending review
**Branch**: `phase5/a1`
**Commit**: `be0121a`

## What

Completes the remote→remote kind matrix (copy d-68, mirror d-70,
**move** here) — and with it the whole F1 trigger
endpoint×kind matrix (remote→local, local→remote, remote→remote
× copy/mirror/move). A delegated move runs the delegated copy,
then deletes the remote **source**.

## Approach (mirrors the F3 remote→local move, d-57/d-60)

- **Safety — `require_complete_scan`.** Move sets it (via
  `f3_pull_options(Move)`), so the daemon refuses a partial
  source scan; a successful copy therefore means the whole
  source transferred before the delete fires. This is the
  move-only guard the CLI also forces (it passes `false` for
  delegated copy/mirror but the move path scans completely).
- **Remote-source delete.** After `Ok`, reuse
  `extract_module_and_path` + `del_wire_path` +
  `delete_remote_path(&source, &wire)` — the same calls the F3
  move uses. A delete failure surfaces as the op error
  (`"delegated but failed to delete remote source: …"`); the
  copy already landed, so the operator must know.
- **Module-root guard.** A move whose source is a module root
  (`nas:/photos/`, no subpath) is refused up front
  (`is_deletable_remote_path`), exactly like the F3 move (d-60).
- **Confirm.** Move is destructive → trigger y/N confirm
  (`NeedsConfirm` → confirmed launch), same gate as mirror.
- **Direction-aware confirm detail.** The trigger detail for
  Move now classifies the source: a remote source (delegated
  move) reads "deletes the remote source"; a local source (push
  move) "deletes the local source". Previously hard-coded to
  "deletes the local source", which was wrong for a delegated
  move.

## Files changed

- `crates/blit-tui/src/main.rs`: `plan_f1_delegated` (module-root
  guard + move now confirms instead of rejecting);
  `spawn_f1_delegated_pull` (post-copy remote-source delete);
  `f1_trigger_prompt` (direction-aware move detail); doc updates;
  3 tests (1 replaced).

## Tests

571 total (net +2 vs d-70):

- `plan_f1_trigger_remote_to_remote_move_confirms_then_launches`
  — subpath source: unconfirmed → `NeedsConfirm`; confirmed →
  delegated `Running { kind: Move }`.
- `plan_f1_trigger_remote_to_remote_move_module_root_source_rejected`
  — module-root source → `Rejected("module root")`, no launch.
- `f1_trigger_prompt_move_detail_follows_source_direction` —
  remote source → "deletes the remote source"; local source →
  "deletes the local source".

(The remote-source delete RPC itself runs only against a live
daemon; the gating, guard, and detail are unit-covered.)

## Status of the remote→remote feature

Copy / mirror / move are now all wired and confirmed. Remaining
TUI_DESIGN follow-ups touch a different area:

1. **detach + F2 visibility** for delegated transfers (needs
   multi-daemon F2).
2. **Multi-daemon F2** (the large outstanding pane work).

## Reviewer comments

(empty — pending grade)
