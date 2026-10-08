# cr-jl3a-1: an incomplete daemon log can misclassify a detached run

**Severity**: HIGH (reviewer: HIGH)
**Status**: Open
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: —
**Reviewer**: codex / gpt-5.6-sol / xhigh / frontier (fallback grade), codex-cli 0.159.3; openreview jl3a-r1 over `00f31c59..2454462b` (record `.review/results/jl3a-r1.codex.json`); owner goal of 2026-10-07

## Evidence
crates/blit-core/src/job_record.rs:615 — any fetched log takes precedence over the daemon job record: an unfinished partial immediately returns Going, while a finished log without run-end is synthesized as Interrupted.

## Predicted observable failure
If logging stops mid-run or final compression fails, a detached transfer that actually completed can remain waiting forever or be persisted as interrupted; its counts and failed-file set can also be falsely recorded as complete, undermining later retry.

## Reviewer's suggested approach
Parse log completeness explicitly, and when summary/run-end is absent or LogIncomplete is present, consult the daemon's active/recent job state for terminal status and counts while retaining available log details and marking the failure list incomplete.

## Intake
Admitted. `ask_daemon` trusts any log it fetched: an unfinished one means "going" and a finished one without a `run-end` is synthesized as interrupted, and either way its failure list is taken as complete. A log is authoritative only when it is finished, has its `run-end` and `summary`, and holds no `log-incomplete`; otherwise the daemon's job state decides (active: going; recent: its terminal status and counts), the log's failures kept but marked truncated.
