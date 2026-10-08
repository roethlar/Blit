# cr-jl3a-3: a local or attached run's record drops a non-UTF-8 name's exact bytes

**Severity**: MEDIUM (reviewer: MEDIUM)
**Status**: Open
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: —
**Reviewer**: codex / gpt-5.6-sol / xhigh / frontier (fallback grade), codex-cli 0.159.3; openreview jl3a-r1 over `00f31c59..2454462b` (record `.review/results/jl3a-r1.codex.json`); owner goal of 2026-10-07

## Evidence
crates/blit-cli/src/run_log.rs:395 — finish_record converts string totals into Failure values and unconditionally sets raw to None, despite the event log retaining escaped raw bytes.

## Predicted observable failure
A failed Unix filename containing non-UTF-8 bytes is stored only under its lossy text representation, so jobs retry cannot reliably target the original file and could target a different valid name.

## Reviewer's suggested approach
Propagate the audit lane's raw-name mapping into RunRecord construction, or derive failures from the completed local EventLog, preserving escaped bytes alongside display text.

## Intake
Admitted. `finish_record` sets every failure's `raw` to none, though the run log holds the escaped bytes of each non-UTF-8 name; jl-4's retry needs them to name the file exactly.
