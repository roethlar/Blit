# a0-push-execution: Phase 5 A.0 — push entry-point orchestration

**Severity**: Refactor (no behavior change)
**Status**: In progress / pending review
**Branch**: `phase5/blit-app-extract`
**Commit**: filled by the sentinel commit

## What

Final sub-slice of the `transfers/remote.rs` track. Pulls the
push entry-point's orchestration body into `blit-app` so the
future TUI can run a push without re-implementing the
connect → source-construct → filter-wrap → push RPC sequence.

New in `blit_app::transfers::remote`:

- `PushExecution` — input struct, primitive fields only:
  `source: Endpoint`, `remote: RemoteEndpoint`,
  `filter: FileFilter`, `mirror_mode: bool`,
  `mirror_kind: MirrorMode`, `force_grpc: bool`,
  `trace_data_plane: bool`, `require_complete_scan: bool`,
  `remote_label: String`. CLI builds this from
  `&TransferArgs`; TUI builds it directly.
- `PushExecutionOutcome` — `report: RemotePushReport`,
  `destination: String`. CLI's presentation hint
  `show_progress` is intentionally absent — CLI's
  `DeferredPushState` adds it directly.
- `run_remote_push(execution, progress) -> Result<outcome>` —
  the orchestration:
  1. `RemotePushClient::connect(remote)` for the destination.
  2. Build the inner `Arc<dyn TransferSource>` from
     `execution.source`:
     - `Endpoint::Local(path)` → `FsTransferSource`.
     - `Endpoint::Remote(endpoint)` → fresh
       `RemotePullClient::connect(endpoint)`, then
       `RemoteTransferSource::new(client, root)` with `root`
       derived from `endpoint.path`
       (`RemotePath::{Module, Root, Discovery}`).
  3. Wrap with `FilteredSource` so the universal filter
     chokepoint (R49) applies on push the same way it does on
     local→local and remote→remote.
  4. Invoke `RemotePushClient::push` with the wired-up source,
     filter, mirror mode/kind, force/trace flags, complete-scan
     requirement, and the borrowed progress channel.

CLI side
(`crates/blit-cli/src/transfers/remote.rs`):

- `run_remote_push_transfer_inner` is now a thin wrapper:
  spawns the progress monitor (R53-F1 `suppress_final_line`
  policy lives here), builds filter + mirror_kind +
  `PushExecution`, calls `run_remote_push`, handles progress
  lifecycle (drop handle + drain task), composes
  `DeferredPushState`, prints (unless deferred).
- Imports trimmed: removed `Arc`, `PathBuf`,
  `FilteredSource`/`FsTransferSource`/`RemoteTransferSource`/
  `TransferSource`, `RemotePullClient`, `RemotePushClient` —
  all now used only inside `blit_app`.

Module doc on `blit_app::transfers::remote` updated to declare
the `transfers/remote.rs` move complete: after this slice
`blit-cli` retains only clap-arg wrappers and presentation
(progress monitor + JSON / human printers).

## Approach

Same pattern as the pull-execution slice: primitive input
struct, async orchestration function in `blit-app`, CLI wrapper
for clap → struct translation and presentation.

Push doesn't need the pull's pull_sync/purge split because
push has no caller-side destructive step. Mirror deletes run
on the daemon and surface via the returned `RemotePushReport`;
the progress monitor's lifetime cleanly matches the RPC.

`DeferredPushState` stays a CLI-side struct (not an alias of
`PushExecutionOutcome`) because its `show_progress` field is a
CLI-only presentation hint. The library outcome is composed
into it at the CLI boundary.

## Files changed

- `crates/blit-app/src/transfers/remote.rs`
  - +9 imports (`Endpoint`, `FileFilter`, `MirrorMode`,
    `RemotePushReport`, `FilteredSource`, `FsTransferSource`,
    `RemoteTransferSource`, `TransferSource`, `RemotePath`,
    `RemotePushClient`, `Arc`).
  - +`PushExecution` struct (9 fields).
  - +`PushExecutionOutcome` struct (2 fields).
  - +`run_remote_push` async function (~50 LOC).
  - Module doc updated to list the new items and declare the
    move complete.

- `crates/blit-cli/src/transfers/remote.rs`
  - `run_remote_push_transfer_inner` rewritten as a wrapper.
    Body shrank from ~95 to ~55 LOC.
  - Imports trimmed (removed 7 now-library-only items).
  - `DeferredPushState` unchanged (still CLI-owned).

## Tests added

None new. Existing push integration tests at
`crates/blit-cli/tests/remote_push_*.rs` and
`crates/blit-cli/tests/remote_remote*.rs` exercise the new
flow through the unchanged CLI entry-points; workspace total
unchanged at 496.

## Known gaps

- `transfers/remote_remote_direct.rs` is the next slice
  (direct remote→remote relay; separate code path from the
  remote-source-on-push case which goes through the
  pull-client adapter).
- `transfers/dispatcher` (`run_transfer` / `run_move` /
  `TransferKind`) follows after `remote_remote_direct`.
- Endpoints clap-coupled gates (`ensure_remote_pull_supported`
  / `ensure_remote_push_supported`) still take `&TransferArgs`;
  reshape to primitive inputs is a later slice.
- M-C `AppProgressEvent` reshape is out of scope for A.0 per
  the TUI design doc.

## Reviewer comments

(empty — pending grade)
