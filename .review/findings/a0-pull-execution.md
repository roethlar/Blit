# a0-pull-execution: Phase 5 A.0 — pull entry-point orchestration

**Severity**: Refactor (no behavior change)
**Status**: In progress / pending review
**Branch**: `phase5/blit-app-extract`
**Commit**: (filled by sentinel after commit)

## What

A.0 sub-slice 2 of the `transfers/remote.rs` track. Moves the
pull entry-point's orchestration body (the part that does work,
as opposed to the part that prints) into `blit-app` so the
future TUI can run a pull without re-implementing the
connect → enumerate → pull_sync → mirror-purge sequence.

New in `blit_app::transfers::remote`:

- `PullExecution` — input struct, primitive fields only
  (`remote: RemoteEndpoint`, `dest_root: PathBuf`,
  `options: PullSyncOptions`, `compute_checksums: bool`,
  `mirror_mode: bool`, `remote_label: String`). No clap, no
  presentation. CLI builds this from `&TransferArgs`; TUI will
  build it directly.
- `PullExecutionOutcome` — captured state for the printer:
  `report`, `actual_dest`, `mirror_purge_stats`. Same field
  shape as the pre-slice CLI `DeferredPullState`.
- `run_remote_pull(execution, progress) -> Result<outcome>` —
  the orchestration: connect, enumerate manifest, pull_sync,
  apply mirror-delete list. R46-F6 ordering preserved (purge
  runs *before* return so JSON callers get a single document).

CLI side (`crates/blit-cli/src/transfers/remote.rs`):

- `DeferredPullState` is now `pub type DeferredPullState =
  PullExecutionOutcome` — preserves the public name that
  `transfers::mod` already imports.
- `run_remote_pull_transfer_inner` is now a thin wrapper: build
  filter spec, spawn progress monitor, construct
  `PullExecution`, call library, handle progress lifecycle,
  print via `print_deferred_pull_result`. Body shrank from ~110
  to ~60 LOC.

## Approach

Same pattern as previous A.0 sub-slices (filter, resolution,
remote-helpers): a primitive input struct, a pure async
function in `blit-app`, a CLI wrapper that handles clap → struct
translation and presentation.

Progress monitor stays in CLI. The library function borrows
`Option<&RemoteTransferProgress>` for the duration of
`pull_sync`; CLI keeps ownership of the channel handle + monitor
task and drains them after `run_remote_pull` returns. This
keeps the R53-F1 `suppress_final_line` policy (presentation
concern) in the CLI where the monitor is spawned.

`format_remote_endpoint(&remote)` was the one CLI-side call
inside the orchestration that produced the error-context label.
Replaced with a `remote_label: String` field on `PullExecution`
so the library function has no CLI module dependency. CLI
passes the same `format_remote_endpoint(...)` result it would
have computed inline; TUI passes whatever string it shows the
user in the picker.

## Files changed

- `crates/blit-app/src/transfers/remote.rs`
  - +5 imports (`PullSyncOptions`, `RemotePullReport`,
    `RemoteTransferProgress`, `RemoteEndpoint`,
    `RemotePullClient`, `Context`).
  - +`PullExecution` struct (6 fields).
  - +`PullExecutionOutcome` struct (3 fields).
  - +`run_remote_pull` async function (~50 LOC).
  - Module doc updated to list the new items.

- `crates/blit-cli/src/transfers/remote.rs`
  - Removed `pub struct DeferredPullState { ... }` body.
  - Added `pub type DeferredPullState = PullExecutionOutcome`.
  - `run_remote_pull_transfer_inner` rewritten as wrapper —
    builds `PullExecution`, delegates to `run_remote_pull`,
    keeps progress lifecycle + printer.
  - Imports: replaced `delete_listed_paths,
    enumerate_local_manifest` with `run_remote_pull,
    PullExecution, PullExecutionOutcome` (the helpers from the
    previous slice are now called by the library only).

## Tests added

None new. Existing pull-sync integration tests at
`crates/blit-cli/tests/remote_pull_*.rs` and
`crates/blit-cli/tests/remote_pull_mirror.rs` exercise the new
flow through the unchanged CLI entry-points; the workspace
test count is unchanged.

## Known gaps

- Push side (`run_remote_push_transfer_inner`) is the next
  sub-slice. Push has additional complexity: the
  `Endpoint::Local | Endpoint::Remote` dispatch builds the
  `TransferSource` trait object, which currently uses CLI's
  `super::endpoints::Endpoint`. Either move that enum into
  blit-app, or have CLI pre-construct the
  `Arc<dyn TransferSource>` before calling the library.
- `transfers/remote_remote_direct.rs` and the dispatcher
  (`run_transfer` / `run_move` / `TransferKind`) follow after
  push.
- M-C `AppProgressEvent` reshape (channel reshape to support
  TUI consumption) is its own pause point — explicitly out of
  scope for the A.0 track per the TUI design doc.

## Reviewer comments

### Round 1 (reviewed sha `7f75539`) — reopened

Validation green (fmt + clippy + workspace tests).
Reviewer: `codex-reviewer`. Two medium-severity findings:

1. **Behavior regression** — mirror-pull progress monitor stays
   open through local mirror purge. Pre-slice,
   `run_remote_pull_transfer_inner` dropped `progress_handle`
   and awaited the monitor task immediately after `pull_sync`,
   **then** ran `delete_listed_paths`. After the round-1 move,
   `run_remote_pull` performed `delete_listed_paths` before
   returning, while the CLI couldn't drop the progress handle
   until the library call returned. For a mirror pull with
   progress enabled and a large delete list, the monitor would
   keep emitting stale transfer progress ticks during purge.
   Violates the no-behavior-change refactor contract and
   contradicts the doc comment claiming `progress` is borrowed
   only for the duration of the PullSync RPC.

   Fix direction: split the app layer so the caller controls
   the progress lifecycle boundary. App layer returns the
   pull report (+ delete list) after `pull_sync`; CLI/TUI
   closes the progress channel; then an app-layer mirror-purge
   helper runs.

2. **Workflow contract** — the slice's `.review/findings/`
   doc and `.review/ready/` sentinel were untracked at commit
   time. `REVIEW.md` got a pending row later as part of the
   unrelated `a0-remote-helpers` round-4 sentinel commit, but
   the ready file for this slice was never in git. Defeats the
   file-based workflow's goal of removing the user from the
   loop — a fresh worktree or a second reviewer wouldn't see
   the pending request.

   Fix direction: commit the finding doc, sentinel, and
   `REVIEW.md` update **in the sentinel commit**.

### Round 2 (sha pending) — addresses both findings

**Finding 1 — progress lifecycle:**

Library split into:

- `PullSyncExecution` (input, same shape as round-1
  `PullExecution`).
- `PullSyncOutcome` (intermediate: `report` + `actual_dest`,
  no purge yet).
- `run_pull_sync(execution, progress) -> PullSyncOutcome` —
  connect + enumerate + PullSync RPC. **Does not purge.**
- `apply_pull_mirror_purge(&PullSyncOutcome, mirror_mode) ->
  Option<LocalPurgeStats>` — applies the delete list, no-op
  on non-mirror or empty-list cases.
- `PullExecutionOutcome` (final shape, caller-composed).

CLI lifecycle restored to pre-Phase-5 ordering:

1. `run_pull_sync` (progress monitor live).
2. `drop(progress_handle)` + `task.await` (monitor torn down).
3. `apply_pull_mirror_purge` (no progress).
4. `print_deferred_pull_result` (R51-F4 defer respected).

R46-F6 was about ordering relative to *printing* (step 4
happens after step 3), not relative to the monitor. Round 1
conflated the two.

**Finding 2 — workflow contract:**

This finding doc, the new sentinel
(`.review/ready/a0-pull-execution.json`), and the `REVIEW.md`
row update are all in the *same* commit as the workflow
metadata for round 2, per the contract. The code commit that
precedes it references this finding doc in its body so the
sentinel + doc + REVIEW.md update are atomically visible to
any worktree the moment the sentinel commit lands.

Also bundled: `.review/coder-wait.sh` extractor hardened to
tolerate pretty-printed JSON in `verified.json` files (round-3
of `a0-remote-helpers` had the wait script exit 1 because the
reviewer wrote `"sha": "..."` with whitespace and the
extractor's regex was compact-only). Defensive — the reviewer
has since started normalizing to compact, but the script
should work either way.
