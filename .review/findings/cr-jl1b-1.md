# cr-jl1b-1: a served job refused at open leaves no log

**Severity**: MEDIUM (reviewer: MEDIUM)
**Status**: Fixed — awaiting re-review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `6e514464`
**Reviewer**: codex / gpt-5.6-sol / xhigh / frontier (fallback grade), codex-cli 0.159.3; openreview jl1b-r1 (record `.review/results/jl1b-r1.codex.json`); dispatched under the owner's goal of 2026-10-07 ("review with codex each slice")

## Evidence
crates/blit-daemon/src/service/transfer.rs:287 — with_log_start awaits endpoint resolution before calling JobLog::start; when resolution returns Err, crates/blit-daemon/src/job_logs.rs:264 makes close return without writing anything.

## Predicted observable failure
A push or pull rejected for an unknown module, read-only destination, or invalid/escaping path still receives a job ID and recent-jobs record, but `blit jobs log` returns NOT_FOUND, losing the forensic record for that failure.

## Reviewer's suggested approach
Start the role-specific log after SessionOpen is parsed but before invoking the resolver; add the resolved local root only on success, and always close the log with the resolver error on refusal.

## Intake
Admitted. The log starts only after the open resolves, so an unknown module, a read-only module or a bad path gives a job record but no log. Fix: start the log as soon as the SessionOpen names the role, before resolving; note the local root only on success; the dispatcher's close records the refusal.

## What
`with_log_start` starts the job's log as soon as the open names the resolver's role — before the endpoint resolves — and notes the local root only when it does. A refused open (unknown or read-only module, bad path) now leaves a log: run-start with the requested endpoint, then the dispatcher's close records the refusal as `run-end` failed with its reason.

## Guard proof
`transfer_session_e2e::a_served_job_refused_at_open_is_logged`: a push to an unknown module leaves a destination-role log naming `/nope` and ending failed with the refusal. Mutation — start the log after resolving, as before — makes it fail (the log never exists); restored, green.
