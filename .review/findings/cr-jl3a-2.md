# cr-jl3a-2: one fixed staging file breaks atomic record updates under concurrency

**Severity**: MEDIUM (reviewer: MEDIUM)
**Status**: Open
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: —
**Reviewer**: codex / gpt-5.6-sol / xhigh / frontier (fallback grade), codex-cli 0.159.3; openreview jl3a-r1 over `00f31c59..2454462b` (record `.review/results/jl3a-r1.codex.json`); owner goal of 2026-10-07

## Evidence
crates/blit-core/src/job_record.rs:323 — every writer to a record uses the same <path>.tmp staging path without record-scoped synchronization.

## Predicted observable failure
Concurrent local jobs operations that reconcile the same detached record can share and rename one staging file; the forced two-writer guard produced one ENOENT failure, and other interleavings can expose or persist a partially overwritten record.

## Reviewer's suggested approach
Give every write a uniquely created sibling temporary file, sync it, atomically replace the destination, sync the parent directory, and clean up only that invocation's temporary file.

## Intake
Admitted. Every write of a document stages through `<path>.tmp`, so two writers of one record (two `jobs` commands settling the same detached run) can rename each other's staging file. Each write gets its own staging file, and the folder is synced after the rename.
