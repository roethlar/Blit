# d-55-f3-mirror: `m` mirrors a remote source → local

**Severity**: Feature (designed — TUI_DESIGN §5.3 keymap)
**Status**: In progress / pending review
**Branch**: `phase5/a1`
**Commit**: `3d1a1ad`

## What

TUI_DESIGN §5.3's F3 keymap lists `m: mirror  v: move  D: delete`.
`p` (pull/copy) shipped in d-35/d-53; `D` (delete) in d-45/d-50.
d-55 adds `m` (mirror): a remote→local pull that also deletes
local files absent from the source — rsync `--mirror`.

## Approach

### Reuse the verified pull machine + a destructive gate

A mirror is a pull plus a purge, so it rides the existing
`F3PullState` rather than a parallel machine. The additions:

- `EnteringDest` gains a `mirror: bool`; `begin_mirror(source)`
  opens the same dest prompt as `begin` but flagged. Typing /
  backspace / the input router are all unchanged.
- A new `ConfirmMirror { source, dest, dest_root }` state. On
  `Enter`, `begin_run` for a mirror does NOT launch — it
  resolves the dest and transitions to `ConfirmMirror`. A copy
  still launches directly. Empty dest still keeps the prompt
  open for both.
- `confirm_mirror()` (y) bumps the run id and transitions to
  `Running { mirror: true }`, handing back a `PullLaunch` with
  `mirror: true`. `cancel_mirror()` (n/Esc) → Idle.

**Why a confirm gate?** A mirror purges local files — it's
destructive, like F3 delete (d-45), which is already gated
behind y/N. Same modal treatment: a dedicated
`handle_f3_mirror_confirm_keystroke` swallows stray keys so a
`p`/`m`/`/` can't stack a prompt or move the cursor
mid-confirm; `?` / Ctrl-c / F-keys still bubble.

### Dest resolution shared with copy (data-loss guard)

The mirror purge target MUST resolve identically to a copy —
a mirror that purged the wrong (un-nested) directory would be
a data-loss bug. `launch` and the `ConfirmMirror` gate both go
through one extracted `resolve_dest` helper, and a test asserts
the mirror's `dest_root` equals the copy's for the same input.

### Execution (purge after progress teardown)

`spawn_f3_pull` gains a `mirror` param. It sets `mirror_mode`
on the PullSync, then — per `run_pull_sync`'s documented
lifecycle — drops the progress channel, drains the forwarder,
and only then calls `apply_pull_mirror_purge(&outcome, mirror)`
(a no-op for a plain pull). A purge failure surfaces as the
op's footer error.

### Footer + key

The footer verb switches pull→mirror across prompt
("mirror → …_"), confirm ("mirror → …? deletes extraneous
y/N", red), running ("mirroring →"), and done ("mirrored").
`m` is free in the global key map — no divergence needed
(unlike du `u` / dump `s` / batch-pull `P`). `M` stays the F4
local mirror; case-distinct.

## Files changed

- `crates/blit-tui/src/f3pull.rs`: `mirror` flag on
  EnteringDest/Running/Done/PullLaunch; `ConfirmMirror` state;
  `begin_mirror` / `is_confirming_mirror` / `confirm_mirror` /
  `cancel_mirror`; extracted `resolve_dest`; `is_busy` guard.
- `crates/blit-tui/src/main.rs`: `spawn_f3_pull` mirror param +
  purge; `UserAction::F3MirrorBegin` + `m` mapping + dispatch;
  routing guard + `handle_f3_mirror_confirm_keystroke`; bridge
  threads mirror + ConfirmMirror.
- `crates/blit-tui/src/screens/f3.rs`: `F3PullDisplay` mirror
  fields + `ConfirmMirror` variant; verb-switching footer.
- `crates/blit-tui/src/help.rs`: `m` keymap row; modal 44→45.

## Tests

+14 (488 → 502):

f3pull.rs (9): begin_mirror flags the prompt; mirror Enter →
ConfirmMirror (not Running); empty mirror dest keeps prompt;
confirm → Running+launch with mirror flag; cancel → Idle;
confirm no-op when not confirming; mirror dest resolves like
copy; Done carries the mirror flag; begin is no-op while
confirming.

main.rs (5): `m` → F3MirrorBegin (`M` still TransferMirror);
confirm `y` launches (under a tokio runtime — the spawn needs
a reactor); `n`/`N`/`Esc` cancel; stray keys swallowed; `?` /
Ctrl-c / F-keys bubble.

The purge execution itself needs a live daemon (manual); the
state machine, routing, dest resolution, and footer wiring are
unit-tested.

## Known gaps

1. **Purge/delete counts not surfaced.** The Done footer shows
   transferred files/bytes (like a pull) but not how many local
   files the mirror deleted. `apply_pull_mirror_purge` returns
   `LocalPurgeStats`; threading the deleted count into the
   reply/footer is a clean follow-up (would touch
   `F3PullReply`/`apply_done`, deliberately deferred to keep the
   reply shape stable this slice).
2. **Single-source only.** No batch mirror over the multi-select
   set yet — the batch pair to `m`, mirroring how d-53 added
   batch pull to `p`. Follow-up.

## Out of scope

- Batch mirror (marked set).
- Surfacing purge stats.
- F3 `v` move (separate designed key).

## Reviewer comments

(empty — pending grade)
