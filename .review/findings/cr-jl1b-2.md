# cr-jl1b-2: unbounded progress queues sit ahead of the bounded log writer

**Severity**: HIGH (reviewer: HIGH)
**Status**: Admitted — open
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: —
**Reviewer**: codex / gpt-5.6-sol / xhigh / frontier (fallback grade), codex-cli 0.159.3; openreview jl1b-r1 (record `.review/results/jl1b-r1.codex.json`); dispatched under the owner's goal of 2026-10-07 ("review with codex each slice")

## Evidence
crates/blit-daemon/src/service/transfer.rs:396 — per-file events enter mpsc::unbounded_channel before the relay awaits destination_log.observe at line 402; crates/blit-core/src/remote/transfer/progress.rs:950 confirms RemoteTransferProgress uses an UnboundedSender.

## Predicted observable failure
When a high-file-count transfer produces events faster than the state volume can serialize them, paths and failure reasons accumulate without a memory bound; a sufficiently large or slow-disk run can exhaust daemon memory instead of making the transfer wait as the approved design requires.

## Reviewer's suggested approach
Give durable audit events their own bounded sink and await capacity at async production points, using blocking sends only from existing blocking workers; keep best-effort UI progress separate, and test that a tiny queue blocks and resumes without dropping events.

## Intake
Admitted. The log's facts ride the unbounded progress lane to a relay that awaits the bounded writer, so when the writer falls behind the backlog grows in memory instead of the transfer waiting — contradicting the plan's backpressure rule. Fix: a bounded audit lane of its own on `RemoteTransferProgress`; per-file reporters await it at their async sites (the delete pass, on a blocking thread, sends blocking); the daemon drains it in one relay task per job and closes the log only after the relay ends. The UI lane stays unbounded and best-effort.
