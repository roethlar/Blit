# d-71-f1-delegated-move: remote→remote delegated move

**Severity**: Feature (TUI_DESIGN §1 "move … between any two endpoints")
**Status**: In progress / pending review (round 2)
**Branch**: `phase5/a1`
**Commit**: `c18c493` (R1 `be0121a`)

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

## Round 2 (fix)

**Reviewer (data-loss):** the remote→remote path delegated with the
raw parsed destination, skipping the `resolve_destination` step the
CLI runs before every copy/mirror/move. A non-trailing-slash source +
a container dest (`nas:/photos/2024` → `skippy:/backup/`) should
resolve to `skippy:/backup/2024`; without it the daemon writes into
the dest root and a delegated move then deletes the source — data
loss to the wrong target. The R1 tests used a trailing-slash source,
hiding the missing basename-append.

**Fix (`c18c493`):** call `resolve_destination(src, dest, &source,
Endpoint::Remote(dst_ep))` before `plan_f1_delegated`, for all
delegated kinds. It's a no-op for trailing-slash ("copy contents")
sources (so the verified d-68 copy / d-70 mirror behavior, which used
trailing-slash sources, is unchanged) and preserves the Remote variant
(infallible rebind). Two tests: a non-trailing source appends the
basename (asserted via the launched run's resolved label,
`backup/2024`); a trailing-slash source keeps the dest root.

## Reviewer comments

(empty — pending round-2 grade)
