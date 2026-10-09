# cr-jl4fix2-1: a lossy-name collision gives left-in-place status to the wrong file

**Severity**: HIGH (reviewer: HIGH)
**Status**: Open
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: —
**Reviewer**: codex / gpt-5.6-sol / xhigh / frontier (fallback grade), codex-cli 0.159.3; openreview jl4fix2-r1 over `e3a8bde0..47827f24` (record `.review/results/jl4fix2-r1.codex.json`); owner goal of 2026-10-07

## Evidence
crates/blit-core/src/job_record.rs:824 — left-in-place membership is stored only by display path and retained while any failure has that path; crates/blit-cli/src/jobs.rs:299 — every raw identity sharing that path is consequently classified as Blit's own leftover.

## Predicted observable failure
If two byte-distinct names collapse to the same text, one leaves an incomplete copy and later succeeds while the other remains failed, `jobs retry` misclassifies the remaining file as Blit's leftover. For an original `--ignore-existing` job it disables that protection and can overwrite a pre-existing destination file.

## Reviewer's suggested approach
Store the optional raw bytes with each left-in-place entry, remove and partition entries by exact composite identity, migrate existing text-only records conservatively, and prove the collision through detached settlement and retry execution.

## Intake
Admitted. Failures carry their own bytes since cr-jl3afix1-1, but the left-in-place set — in the engine's summary, the record and the daemon-log fold — is keyed by text alone, so two names of one text share the status. The status must be kept by the same (text, bytes) identity as the failure it belongs to, through the wire and the record, and the `--ignore-existing` split must use it.
