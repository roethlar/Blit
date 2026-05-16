# a0-delegated-execution: Phase 5 A.0 — delegated remote→remote orchestration

**Severity**: Refactor (no behavior change)
**Status**: In progress / pending review
**Branch**: `phase5/blit-app-extract`
**Commit**: filled by the sentinel commit

## What

A.0 sub-slice 4 of the `transfers/remote*.rs` track. Moves the
delegated-pull orchestration from
`crates/blit-cli/src/transfers/remote_remote_direct.rs` into
`blit_app::transfers::remote`. Before this slice the CLI owned
~165 LOC of streaming-RPC handling, error mapping, and payload
dispatch — none of it specific to clap or presentation. After
this slice the CLI's inner function is a ~50-LOC wrapper.

New in `blit_app::transfers::remote`:

- `DelegatedPullExecution` — input struct, primitive fields:
  `src`, `dst`, `options: PullSyncOptions`,
  `trace_data_plane: bool`, `relay_fallback_suggestable: bool`,
  `dst_label: String`. The `relay_fallback_suggestable` flag is
  a CLI-policy bit baked into error-mapping (copy / mirror
  callers: true; move callers: false, since R53-F2 refuses
  `--relay-via-cli` on move).
- `DelegatedPullOutcome` — `summary: DelegatedPullSummary`,
  `src: RemoteEndpoint`, `dst: RemoteEndpoint`. CLI's
  `DeferredDelegatedState` is now a type alias.
- `run_delegated_pull(execution, progress, on_started) ->
  Result<outcome>` — builds the `DelegatedPullRequest`,
  connects the destination's `BlitClient`, consumes the
  streamed payload (`ManifestBatch` / `BytesProgress` /
  `Summary` / `Error`), maps errors via [`map_delegated_error`],
  returns the summary.
- `on_started: FnMut(&DelegatedPullStarted)` — callback that
  fires once when the destination emits its `Started` event.
  CLI uses it to print the verbose-mode diagnostic
  `[delegation] destination pulling from <ep> (<n> stream(s))`
  without baking presentation into the library. The M-C
  `AppProgressEvent` reshape (out of A.0 scope) will replace
  the callback with a stream variant that CLI and TUI consume
  uniformly.

Pure helpers moved alongside:

- `map_delegated_error` — pub (TUI will want it too).
- `destination_spec_fields` — pub.
- `normalize_for_request` — private.
- `DelegatedBytesProgressState` + `report_bytes_progress` —
  private.

Their four unit tests moved into
`blit_app::transfers::remote::tests` so `cargo test -p blit-app`
exercises them directly. Precedent: the a0-remote-helpers
round-1 reopen made test-locality a workflow standard.

## Approach

Same pattern as the pull-execution and push-execution slices:
primitive input struct, async orchestration function in
`blit-app`, CLI wrapper for clap → struct translation + presentation.

The only meaningful change vs prior slices is the `on_started`
callback. Other slices had pure (non-presentation) library
functions, but delegated-pull's existing CLI code emitted one
verbose-mode diagnostic eprintln mid-stream. Two options were
considered:

1. **Drop the live emission** — library returns the
   `Started` info in the outcome; CLI prints AFTER the call
   returns. Downside: the diagnostic arrives late (after the
   transfer completes), which contradicts the user's
   expectation of a real-time `[delegation]` signal.
2. **Callback hook** — library accepts an
   `FnMut(&DelegatedPullStarted)` invoked when the event
   arrives. CLI passes a closure that emits the eprintln in
   verbose+non-JSON mode; TUI will pass a no-op or wire it to
   its own status bar.

Picked (2). The callback is a targeted stopgap — small API
surface, no presentation in the library, easy to delete once
the `AppProgressEvent` reshape lands and `Started` becomes a
proper event variant.

## Files changed

- `crates/blit-app/src/transfers/remote.rs`
  - +10 imports (`BlitClient`, `DelegatedPullPhase`,
    `DelegatedPayload`, `BytesProgress`, `DelegatedPullRequest`,
    `DelegatedPullStarted`, `DelegatedPullSummary`,
    `RemoteSourceLocator`, `RemotePath` already present,
    `tonic::Code`).
  - +`DelegatedPullExecution` struct (6 fields).
  - +`DelegatedPullOutcome` struct (3 fields).
  - +`DelegatedBytesProgressState` struct (private).
  - +`run_delegated_pull` async function (~90 LOC).
  - +`map_delegated_error` (pub), `destination_spec_fields`
    (pub), `normalize_for_request` (private),
    `report_bytes_progress` (private).
  - +4 unit tests in the `tests` module.
  - Module doc updated to list the new items.

- `crates/blit-cli/src/transfers/remote_remote_direct.rs`
  - Removed `pub struct DeferredDelegatedState { ... }` body.
  - Added `pub type DeferredDelegatedState =
    DelegatedPullOutcome`.
  - `run_remote_to_remote_direct_inner` rewritten as wrapper.
  - Removed `map_delegated_error`,
    `destination_spec_fields`, `normalize_for_request`,
    `DelegatedBytesProgressState`, `report_bytes_progress`
    (all now in `blit-app`).
  - Removed the four-test `tests` module (relocated to
    `blit-app`).
  - Imports trimmed (12 items dropped — all now used only
    inside `blit-app`).

## Tests added

None new. The four pre-existing unit tests moved from the CLI
to `blit-app`; workspace total unchanged at 496 passed.

## Known gaps

- `transfers/dispatcher` (`run_transfer` / `run_move` /
  `TransferKind`) is the next slice — it's the entry point
  that picks between push / pull / delegated, so it'll lean
  on the three execution structs from this slice + the prior
  pull-execution + push-execution slices.
- Endpoints clap-coupled gates (`ensure_remote_pull_supported`
  / `ensure_remote_push_supported`) still take `&TransferArgs`;
  reshape to primitive inputs is a later A.0 slice.
- M-C `AppProgressEvent` reshape (which will eliminate the
  `on_started` callback) is out of A.0 scope per the TUI
  design doc.

## Reviewer comments

(empty — pending grade)
