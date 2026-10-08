# cr-jl3afix1-1: a lossy-name collision gives the failed file the wrong bytes

**Severity**: MEDIUM (reviewer: MEDIUM)
**Status**: Open
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: —
**Reviewer**: codex / gpt-5.6-sol / xhigh / frontier (fallback grade), codex-cli 0.159.3; openreview jl3afix1-r1 over `7241a572..32280602` (record `.review/results/jl3afix1-r1.codex.json`); owner goal of 2026-10-07

## Evidence
crates/blit-core/src/run_log.rs:366 — raw identities are collapsed into a first-wins HashMap keyed only by lossy text; crates/blit-cli/src/run_log.rs:299 then recovers failure bytes solely through that text, while crates/blit-core/src/transfer_session/mod.rs:4436 records the second colliding manifest entry as the rejected failure.

## Predicted observable failure
With two distinct Unix filenames that collapse to the same display string, the first entry lands and the second is rejected, but the local RunRecord identifies the failure with the first entry's bytes. The record is forensically false, and future jobs retry can select the already-landed file while leaving the actual failed file absent.

## Reviewer's suggested approach
Add optional raw identity to FileFailed/FileFailure at the point the failure is created, preserve it through summaries and RunTotals, and construct RunRecord failures directly from those structured identities; verify two colliding non-UTF-8 names end to end on Linux.

## Intake
Admitted. When two names collapse to one text, the first entry lands and the second is rejected as a duplicate, but the record's lookup by text gives that failure the first entry's bytes. The failure must carry its own bytes from where it is made. The plan's jl-3a known-gap line about raw bytes is also stale since cr-jl3a-3.
