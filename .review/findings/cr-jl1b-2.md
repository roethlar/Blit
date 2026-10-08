# cr-jl1b-2: unbounded progress queues sit ahead of the bounded log writer

**Severity**: HIGH (reviewer: HIGH)
**Status**: Fixed — awaiting re-review
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

## What
- `RemoteTransferProgress` gains an optional **audit lane** (`progress::audit_lane(capacity)` → `AuditSender`/`AuditReceiver`, a bounded flume channel, depth `AUDIT_LANE_DEPTH` = 1024) attached with `with_audit`. The per-file facts and phase changes a log keeps — `FileComplete`, `FileFailed`, `Deleted`, `DiffComplete`, `DeleteBegin` — go on it as well as on the unbounded UI lane, so UI consumers are unchanged.
- The reporters that feed it are now `async` and await the lane at their call sites, all in async code: the destination's record sites, the data-plane receive and send loops, the source's in-stream sender (its `report_files` closure became an async helper), and the phase reporters. The mirror pass's `report_deleted` runs on its blocking thread and sends blocking.
- Planned totals are shared counters on the lane (`PlannedTotals`), not events, so `report_manifest_batch` stays synchronous.
- Daemon: `JobLog::audit_lane()` makes the job's one lane and spawns the relay that drains it into the log. The jobs-row relays no longer touch the log. `close` waits for the relay to drain (bounded at 30 s, then says so in the log) before writing the closing events.

## Guard proof
- `progress::tests::a_full_audit_lane_makes_the_producer_wait`: a lane of one holds a producer back; every event arrives in order, planned totals add up, and the UI lane still sees all seven.
- `job_logs::tests::closing_a_log_waits_for_its_lane_to_drain`: 1000 events, then close at once; all 1000 are in the log.
- Mutations, each red alone, then restored:
  - an unbounded lane → the backpressure test
  - closing without draining → the drain test
  - no lane attached on the served route → four served e2e tests
  - no lane attached on the delegated route → the delegated e2e test
