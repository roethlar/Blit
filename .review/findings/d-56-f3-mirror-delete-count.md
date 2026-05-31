# d-56-f3-mirror-delete-count: surface mirror purge count

**Severity**: Feature (closes d-55 known gap #1)
**Status**: In progress / pending review
**Branch**: `phase5/a1`
**Commit**: `6c5d092`

## What

d-55 shipped F3 `m` mirror but documented a gap: the Done
footer reported only transferred files/bytes (like a pull),
never how many local files the mirror *deleted*. For a
destructive op that's a UX-honesty hole — the operator confirms
"deletes extraneous" but then sees no evidence of what was
removed. d-56 surfaces the purge count.

## Approach

`apply_pull_mirror_purge` already returns
`Option<LocalPurgeStats { files_deleted, dirs_deleted }>`.
d-56 threads `files_deleted` through the existing reply path —
no new lifecycle:

- `spawn_f3_pull`: `deleted = purge_stats.map(|s|
  s.files_deleted).unwrap_or(0)`, added to the reply tuple.
- `F3PullReply.result`: `(usize, u64)` → `(usize, u64, u64)`
  (files, bytes, **deleted**).
- `F3PullState::apply_done` gains a `deleted: u64` param; the
  `Done` state carries it.
- Bridge → `F3PullDisplay::Done.deleted`.
- Footer: a mirror Done appends `· N deleted` **only when**
  `deleted > 0`. A copy never deletes (always 0); a mirror that
  found nothing to remove shows no suffix — cleaner than
  "· 0 deleted".

`deleted` is always 0 on the copy path, so the suffix is
mirror-exclusive without a separate flag check beyond the
`mirror && deleted > 0` guard.

## Files changed

- `crates/blit-tui/src/f3pull.rs`: `Done.deleted`; `apply_done`
  `deleted` param; tests updated (+ renamed mirror Done test to
  assert the count).
- `crates/blit-tui/src/main.rs`: reply tuple; `spawn_f3_pull`
  purge-count capture; reply arm; bridge; +1 bridge test.
- `crates/blit-tui/src/screens/f3.rs`: `F3PullDisplay::Done.deleted`;
  footer suffix; module-doc footer sketch.

## Tests

504 total (was 503; one mirror test renamed, +1 bridge test):

- `mirror_done_carries_mirror_flag_and_deleted_count`
  (f3pull) — `apply_done(.., deleted=4, ..)` lands `deleted: 4`
  in `Done`.
- `f3_pull_to_display_done_carries_mirror_and_deleted`
  (main) — the bridge copies `mirror` + `deleted` into the
  display.

The purge itself needs a live daemon (manual); the count's flow
from reply → state → display is unit-tested. The footer suffix
is a pure conditional (`mirror && deleted > 0`).

## Known gaps

1. **`dirs_deleted` not surfaced.** Only `files_deleted` is
   shown; directory removals aren't counted in the footer.
   Files are the operator-meaningful unit; dirs are an
   implementation detail of the purge.
2. **Live count, not incremental.** The count lands with the
   terminal reply, not during the purge (the purge is fast
   relative to the transfer; no progress events for deletes).

## Out of scope

- Batch mirror (marked set).
- F3 `v` move.

## Reviewer comments

(empty — pending grade)
