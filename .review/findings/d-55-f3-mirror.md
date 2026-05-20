# d-55-f3-mirror: `m` mirrors a remote source → local

**Severity**: Feature (designed — TUI_DESIGN §5.3 keymap)
**Status**: In progress / pending review (round 2)
**Branch**: `phase5/a1`
**Commit**: `c2116f1` (round 1: `3d1a1ad`)

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

`spawn_f3_pull` gains a `mirror` param. It sets `mirror_mode`,
then — per `run_pull_sync`'s documented lifecycle — drops the
progress channel, drains the forwarder, and only then calls
`apply_pull_mirror_purge(&outcome, mirror)` (a no-op for a
plain pull). A purge failure surfaces as the op's footer error.

**Round 2 (reviewer reopen — the load-bearing fix).** Round 1
set only `PullSyncExecution.mirror_mode`, which is just the
receive-side `track_paths` flag. The wire
`TransferOperationSpec` — which tells the daemon to *compute*
the delete list — is built from `options`
(`RemotePullClient::build_spec_from_options`), and round 1 left
`options: PullSyncOptions::default()` (`mirror_mode = false`),
so the daemon emitted `MirrorMode::Off`,
`outcome.report.paths_to_delete` was empty, and
`apply_pull_mirror_purge` deleted nothing. The "mirror" ran the
confirm + copy but silently behaved like a plain pull.

Fix: a `f3_pull_options(mirror)` helper sets
`options.mirror_mode = mirror`, matching how the CLI builds it
(`blit-cli/src/transfers/remote.rs`). Now both the spec
(daemon-side delete-list computation) and the purge flag are
mirror-enabled.

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

main.rs (6): `m` → F3MirrorBegin (`M` still TransferMirror);
confirm `y` launches (under a tokio runtime — the spawn needs
a reactor); `n`/`N`/`Esc` cancel; stray keys swallowed; `?` /
Ctrl-c / F-keys bubble. **R2:** `f3_mirror_options_build_mirror_enabled_spec`
— the F3 mirror builds a non-`Off` (`FilteredSubset`) wire spec
via `build_spec_from_options`, while a copy stays `Off`. This is
the regression guard the reviewer asked for: it would have
caught round 1's behaves-like-a-plain-pull bug.

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

### Round 1 (reopened)

> F3 mirror never asks the daemon for mirror mode, so no purge
> list is produced. `spawn_f3_pull` builds the execution with
> `options: PullSyncOptions::default()`; the separate
> `PullSyncExecution.mirror_mode` is only `track_paths`, while
> the wire spec is built from `options` →
> `build_spec_from_options` emits `MirrorMode::Off`. Pressing
> `m`/`y` copies but the daemon computes no deletions. Set
> `PullSyncOptions.mirror_mode = mirror` (as the CLI does) and
> add regression coverage that the mirror builds a
> mirror-enabled spec.

**Response (c2116f1):** Fixed exactly as directed —
`f3_pull_options(mirror)` sets `options.mirror_mode`, and the
new `f3_mirror_options_build_mirror_enabled_spec` test asserts
the spec is `FilteredSubset` (non-`Off`) for a mirror and `Off`
for a copy, via the same `build_spec_from_options` the reviewer
traced. Validation re-run green (503 tests, fmt + clippy).
