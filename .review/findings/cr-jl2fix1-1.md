# cr-jl2fix1-1: a dry run's projected bytes come from the bytes actually written

**Severity**: MEDIUM (reviewer: MEDIUM)
**Status**: Open
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: —
**Reviewer**: codex / gpt-5.6-sol / xhigh / frontier (fallback grade), codex-cli 0.159.3; openreview jl2fix1-r1 over `8288d2f1..8cf9380d` (record `.review/results/jl2fix1-r1.codex.json`); owner goal of 2026-10-07

## Evidence
crates/blit-core/src/run_log.rs:535 — the new dry-run diagnostic formats totals.bytes_copied, which crates/blit-cli/src/run_log.rs:266 obtains from LocalMirrorSummary.total_bytes; crates/blit-core/src/transfer_session/local.rs:351 defines that field as bytes actually written, while the correct planned_bytes value is already available at run_log.rs:506.

## Predicted observable failure
A dry run over the test's 5-byte file logs both that 5 B was planned and that it would have copied 0 B; every nonempty dry run therefore records a misleading projected volume.

## Reviewer's suggested approach
Format the dry-run projection with planned_files and planned_bytes from the audit lane, keep totals for actual or discarded work, and strengthen a_run_that_writes_nothing_logs_no_copies to require the exact 5 B projection.

## Intake
Admitted. A dry run writes nothing, so its written-bytes total is 0; the projection must come from what the run planned.
