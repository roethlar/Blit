# m-jobs-2-cancel-rpc: CancelJob RPC + `blit jobs cancel` CLI

**Severity**: Feature (new RPC + CLI verb)
**Status**: In progress / pending review
**Branch**: `phase5/m-jobs`
**Commit**: filled by the sentinel commit

## What

Second sub-slice of M-Jobs. m-jobs-1 introduced the
`ActiveJobs::cancel(id) -> CancelOutcome` API and the
delegated_pull spawn-closure race against its cancellation
token. This slice exposes both over the wire: a new
`CancelJob` gRPC + a `blit jobs cancel <remote> <id>` CLI
verb consuming it. After this slice the cancel path is
end-to-end testable against a real daemon.

## Approach

The three internal `CancelOutcome` variants map cleanly onto
gRPC status codes:

- `Cancelled` → `Code::Ok` + a `CancelJobResponse` echoing the
  transfer_id back (for the CLI's confirmation message).
- `Unsupported` → `Code::FailedPrecondition` with a message
  explaining why (kind isn't cancellable today — CLI is in
  the byte path for push / pull / pull_sync, so cancel from
  the originating client instead).
- `NotFound` → `Code::NotFound` with the requested id in the
  message.

Plus an explicit `InvalidArgument` early-return when
`req.transfer_id` is empty — saves a no-op map lookup and
gives the CLI a clearer error.

On the library side, the symmetric typed view —
`CancelJobOutcome` — collapses the status-code matching into
one place. CLI / TUI consumers match on the enum and never
re-derive gRPC semantics.

CLI verb shape:

```
blit jobs cancel <remote> <transfer_id> [--json]
```

Exit codes:

| Outcome      | Exit |
|--------------|------|
| Cancelled    | 0    |
| NotFound     | 1    |
| Unsupported  | 2    |

Distinct codes let shell scripts branch on the outcome
without parsing stderr.

## Files changed

- `proto/blit.proto`: `rpc CancelJob(CancelJobRequest)
  returns (CancelJobResponse)` on the `Blit` service plus
  the two messages.
- `crates/blit-daemon/src/service/core.rs`:
  - `+use ... CancelOutcome` from `active_jobs`.
  - `+use blit_core::generated::{CancelJobRequest, CancelJobResponse}`.
  - `+async fn cancel_job` handler implementation.
  - `+#[tokio::test]` block of 4 handler tests.
- `crates/blit-app/src/admin/jobs.rs`:
  - `+CancelJobOutcome` enum (Cancelled / NotFound /
    Unsupported).
  - `+pub async fn cancel(remote, transfer_id) ->
    Result<CancelJobOutcome>` mapping status codes to
    enum variants.
- `crates/blit-cli/src/cli.rs`:
  - `+JobsCommand::Cancel(JobsCancelArgs)`.
  - `+JobsCancelArgs { remote, transfer_id, json }`.
- `crates/blit-cli/src/jobs.rs`:
  - `+run_jobs_cancel` dispatcher entry point.
  - `+print_cancel_human` + `+print_cancel_json` formatters.
  - Updated `run_jobs` to dispatch the new variant.
- `crates/blit-cli/tests/remote_remote.rs`:
  - Two Blit-impl test doubles grew `cancel_job` stubs.
- `crates/blit-core/tests/pull_sync_with_spec_wire.rs`:
  - `SpyServer` grew a `cancel_job` stub.

## Tests added

In `service::core::tests`:

- `cancel_job_ok_for_delegated_pull` — happy path:
  register, fire RPC, assert echoed transfer_id + the
  guard's cancellation token reads as cancelled.
- `cancel_job_failed_precondition_for_non_delegated_kind` —
  loops over Push / Pull / PullSync, asserts
  FailedPrecondition AND that the row's token was NOT
  fired. Catches the regression that would otherwise let
  the daemon acknowledge an uncancellable cancel.
- `cancel_job_not_found_for_unknown_transfer_id` —
  unknown id returns NotFound.
- `cancel_job_invalid_argument_for_empty_id` — empty
  string rejected with InvalidArgument before any table
  lookup.

Workspace: 532 passed (was 528; +4 handler tests).

## Known gaps

1. **No integration test of the CLI verb.** The handler
   tests exercise the daemon-side path; the CLI side is
   uncovered by unit tests in this slice. Same posture as
   `blit jobs list` (b-5) — an end-to-end smoke test can
   land separately.

2. **Cancel-during-Drop race is not exercised.** If a
   transfer's spawn task is mid-Drop (table entry removed,
   but the handler hasn't fully observed the cancel) the
   RPC will return NotFound for what looks like an active
   transfer. This is correct behavior — there's nothing
   left to cancel — but is mildly surprising for clients
   that just read it from `GetState.active[]`. The
   forthcoming Subscribe events (milestone C) will give
   clients a TransferComplete signal to disambiguate.

3. **CLI exit codes 1 and 2 aren't documented in the man
   page.** Out of scope for this slice; a docs-only
   follow-up before 0.1.0 can fold in the cancel exit-code
   table.

## Reviewer comments

(empty — pending grade)
