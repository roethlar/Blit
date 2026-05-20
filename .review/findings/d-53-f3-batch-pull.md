# d-53-f3-batch-pull: P pulls the marked set sequentially

**Severity**: Feature (designed — TUI_DESIGN §5.3 batch transfer)
**Status**: In progress / pending review
**Branch**: `phase5/a1`
**Commit**: `7188da6`

## What

The first batch-transfer slice and the second consumer of d-49
multi-select (after d-50 batch-delete). `P` (Shift+p) pulls every
marked F3 row into a single local destination, one at a time —
the batch pair to `p` (pull the cursor row).

## Approach

### Reuse the verified f3pull machine (no enum rework)

Rather than generalize `F3PullState` to multi-source, d-53 drives
the single-source machine sequentially via two **additive**
methods:

- `entering_dest() -> Option<&str>` — reads the dest the operator
  has typed, so the batch can capture it once.
- `start_pull(source, raw_dest)` — launches a source directly
  (skipping the prompt) for queued sources. Shares a new private
  `launch` core extracted from `begin_run` (begin_run's
  behavior is unchanged — its 31 tests stay green).

### Sequencing

A `BatchPull { remaining: VecDeque<RemoteEndpoint>, raw_dest,
done, total }` on AppState:

1. `P` resolves the marked rows to endpoints, pops the first to
   open the normal dest prompt, queues the rest.
2. The pull keystroke's `Enter` captures `entering_dest()` into
   `raw_dest` before `begin_run` consumes it.
3. The pull-reply arm calls `advance_batch_pull` on each applied
   `Done`: bump `done`, pop the next source, `start_pull` it with
   the captured dest + spawn — or clear the batch when the queue
   empties.
4. An applied `Error` aborts the rest of the batch (don't keep
   pulling past an unseen failure). `Esc` on the prompt clears
   the batch (nothing ran yet).

The footer shows `batch pull k/N` while running.

## Files changed

- `crates/blit-tui/src/f3pull.rs`: `entering_dest`, `start_pull`,
  extracted `launch`; 3 tests.
- `crates/blit-tui/src/main.rs`: `BatchPull` + AppState field;
  `F3BatchPullBegin` action + `P` key + dispatch; `Enter`
  dest-capture + `Esc` clear; reply-arm sequencing +
  `advance_batch_pull`; render passes `(done+1, total)`; 6
  AppState fixtures updated; 2 tests.
- `crates/blit-tui/src/screens/f3.rs`: `batch_pull` param +
  `batch pull k/N` footer fragment.
- `crates/blit-tui/src/help.rs`: `P` keymap row; modal height
  43→44; keymap test asserts it; test backends 44→46.

## Tests

+7 tests (478 → 485):

- `f3pull`: `entering_dest_reports_typed_dest`,
  `start_pull_launches_directly_without_prompt`,
  `start_pull_is_noop_when_busy_or_blank`.
- `main`: `key_action_maps_shift_p_to_batch_pull`,
  `advance_batch_pull_clears_when_queue_empty`.

The start-next path spawns a task (needs a live daemon), exercised
manually; the pure pieces (start_pull, queue-clear, dest-capture
methods) are unit-tested.

## Known gaps

1. **Copy only (`P` = batch pull-to-local).** The design's
   `m`/`v` (mirror/move options) and remote→remote batch are
   follow-ons — they're `PullSyncOptions` variations on this same
   queue.
2. **No mid-batch cancel.** Like single pull, a running pull
   can't be cancelled; the batch runs to completion (or aborts on
   error). A cancel-batch key could come later.
3. **Sequential, not parallel.** One pull at a time — simpler and
   avoids saturating the link; parallelism is a possible future
   knob.

## Out of scope

- Batch mirror/move (`m`/`v`) and remote→remote.
- Mid-batch cancel; parallel batch.

## Reviewer comments

### Round 1 verdict — reopened (`.review/results/d-53-f3-batch-pull.reopened.md`)

One finding:

- **Batch reused single-source destination resolution → multiple
  sources can target the same path.** Each queued source went
  through `F3PullState::launch` → `resolve_destination`, which
  treats a non-existing, slash-less dest as an *exact target /
  rename*. So marking `a.txt` + `b.txt` and pulling into
  `/tmp/out` (non-existing) made both resolve to `/tmp/out` —
  overwrite/collision. A multi-source op needs a directory
  (container) contract.

### Round 2 fix

The Enter handler now **container-normalizes** the batch
destination: if the entered dest is non-empty and lacks a
trailing slash (`needs_container_slash`), it appends one before
`begin_run`/capture. `resolve_destination` then treats it as a
container and nests every source under `<dest>/<basename>`.
Basenames are unique within a single F3 view (marks are
view-scoped), so the nested targets never collide. Single `p` is
untouched — only the batch path normalizes.

### Round 2 tests

+2 tests (485 → 487):

- `f3pull::container_dest_nests_each_source_distinctly` — the
  reviewer's regression: two sources + a container (trailing-
  slash) dest resolve to **distinct** `dest_root`s ending in
  their own basenames.
- `main::needs_container_slash_cases` — slash forced for
  `/tmp/out` & `relative/dir`; not for `/tmp/out/`, blank, or
  whitespace.

`cargo fmt --all -- --check`, `cargo clippy --workspace
--all-targets -- -D warnings`, and `cargo test --workspace` all
green.

### Lesson restated

Single-item destination semantics (exact-target-or-nest) don't
carry over to multi-item operations: N items need a container, or
they fight over one path. When fanning a single-source primitive
out to a set, re-examine its destination contract — don't just
reuse the per-item resolution.
