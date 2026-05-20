# d-57-f3-move: `v` moves a remote source → local

**Severity**: Feature (designed — TUI_DESIGN §5.3 keymap)
**Status**: In progress / pending review (round 2)
**Branch**: `phase5/a1`
**Commit**: `7c6ef5a` (round 1: `a194c5a`)

## What

TUI_DESIGN §5.3's F3 keymap is `m: mirror  v: move  D: delete`.
`m` (d-55/56) and `D` (d-45/50) shipped; d-57 adds `v` (move),
completing the trio. A move receives the remote source to a
local dest, then **deletes the remote source** — rsync
`--remove-source-files` semantics, matching `blit move <remote>
<local>`.

## Approach

### Generalize `mirror: bool` → `PullKind`

d-55 modeled mirror as a `mirror: bool`. Move is a third,
mutually-exclusive flavor, so this slice replaces the bool with
`PullKind { Copy, Mirror, Move }` threaded through
`EnteringDest` / `Confirm` / `Running` / `Done` / `PullLaunch`.
`PullKind::is_destructive()` (Mirror|Move) drives the confirm
gate; `verbs()` gives the footer tense triple.

`ConfirmMirror` became the kind-carrying `Confirm` state, so
mirror and move share one destructive gate + one keystroke
handler (`handle_f3_destructive_confirm_keystroke`, renamed
from the mirror-specific one).

**Screens stay decoupled.** `F3PullDisplay` never learns about
`PullKind` — the bridge maps the kind to plain `&'static str`
verbs (and a confirm `detail` string), so `screens/f3.rs`
renders generically.

### Execution — pull then delete source, same task

`v` opens the dest prompt (`begin_move`), commit routes through
`Confirm` ("move → dest? deletes the remote source y/N"), and
`y` (`confirm_destructive`) launches a `Move`-kind run. The
spawn task runs the receive, and **only after a successful
receive** deletes the remote source via
`delete_remote_path(&source, rel_path)` (the same Purge the CLI
move uses). The delete is folded into the existing single spawn
task — no new channel — and its removed-file count rides the
d-56 `deleted` field into the Done footer.

### Data-loss guards (mirroring the CLI move)

1. **`require_complete_scan = true`** on the move's
   `PullSyncOptions` — the daemon refuses a partial source
   scan, so a successful pull means the *whole* source was
   copied before we delete it. Without this, files skipped by
   an incomplete scan would survive the copy but be deleted
   with the source. This matches
   `run_remote_pull_transfer_deferred(.., true)` in the CLI.
2. **Delete only after pull success.** A failed/partial receive
   returns `Err` and the source is never touched. A *delete*
   failure (copy succeeded) surfaces as
   "received but failed to delete remote source: …" so the
   operator knows the source remains.
3. **Read-only gate.** `v` is refused on a read-only module
   (you can't delete from it) — unlike `m`, which only writes
   locally.
4. **Module-root rejection (round 2).** The move source must be
   a deletable *path*, never a module root — gated through the
   same `is_deletable_remote_path` check F3 delete uses. See the
   round-2 note below.

`Move` is NOT a mirror: its wire spec stays `MirrorMode::Off`
(the source delete is a separate Purge, not a destination
mirror-purge).

## Files changed

- `crates/blit-tui/src/f3pull.rs`: `PullKind` enum;
  `mirror:bool`→`kind`; `Confirm` (was `ConfirmMirror`);
  `begin_move` / `confirm_destructive` / `cancel_destructive` /
  `is_confirming_destructive` (renamed from `*_mirror`).
- `crates/blit-tui/src/main.rs`: `f3_pull_options(kind)`
  (mirror_mode for Mirror, require_complete_scan for Move);
  `spawn_f3_pull` kind param + move source-delete;
  `UserAction::F3MoveBegin` + `v` mapping + dispatch (read-only
  gate); routing + renamed destructive-confirm handler; bridge
  maps kind→verb/detail.
- `crates/blit-tui/src/screens/f3.rs`: `F3PullDisplay` carries
  `verb`/`detail` strings (not `PullKind`); renderer + doc.
- `crates/blit-tui/src/help.rs`: `v` keymap row; modal 45→46.

## Tests

509 total (was 504; net + after renames):

f3pull.rs: `begin_move_opens_prompt_in_move_kind`;
`begin_run_on_destructive_routes_to_confirm` (both kinds);
`confirm_destructive_launches_with_move_kind`;
`cancel_destructive` (move); `pull_kind_verbs_and_destructive`;
the mirror tests retargeted to the shared destructive API.

main.rs: `f3_move_options_require_complete_scan_and_not_mirror`
(the data-loss guard — move sets the scan flag, stays
`MirrorMode::Off`); `key_action` `v`→F3MoveBegin (`V` still F4
move); `handle_f3_destructive_confirm_keystroke_y_launches_move`;
bridge maps kind→past verb.

The remote source-delete needs a live daemon (manual); the
state machine, kind plumbing, options safety, routing, and
footer are unit-tested.

## Known gaps

1. **Single-source only.** No batch move over the marked set
   (the batch pair to `v`), as batch-pull (d-53) was to `p`.
2. **Source-delete is whole-path.** `delete_remote_path` purges
   the source path; partial-tree move (delete only the copied
   subset) isn't modeled — but `require_complete_scan` makes
   "copied subset" == "whole source", so this is sound.

## Out of scope

- Batch move.
- Multi-daemon F2; F1 `t` trigger-transfer.

## Reviewer comments

### Round 1 (reopened)

> `v` can launch an impossible module-root move. `F3MoveBegin`
> only checks `current_module_read_only()` before deriving the
> source; in the modules view `pull_source_endpoint` maps a
> module row to `Module { rel_path: "" }`, and the move task
> then calls `delete_remote_path` with that empty path after a
> successful receive. The daemon rejects empty/root purge paths,
> so `v` on a module row copies the whole module locally then
> fails the source delete. The read-only gate is also incomplete
> for top-level module rows (`current_module_read_only` only
> tracks a descended module). Either reject module-root moves up
> front like F3 delete, or implement whole-module move; and the
> read-only decision for top-level rows must use the selected
> row's `BrowseRowKind::Module { read_only }`.

**Response (7c6ef5a):** Took the "reject module-root moves up
front" option — gated the derived source through the same
`is_deletable_remote_path` check F3 delete uses. A module root
(empty/`.` rel path) is now refused before the prompt opens, so
the copy-then-fail sequence can't happen. This subsumes the
top-level read-only concern: a top-level module row IS a module
root, so it's rejected here regardless of its read-only flag;
descended read-only modules are still caught by
`current_module_read_only()`. Added
`move_gate_rejects_module_root_accepts_paths`. 510 tests green,
fmt + clippy clean.
