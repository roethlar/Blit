# cr-jl3a-2: one fixed staging file breaks atomic record updates under concurrency

**Severity**: MEDIUM (reviewer: MEDIUM)
**Status**: Fixed — awaiting re-review
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

## What
`write_document` stages each write in a file of its own beside the document (`.<name>.<random>.tmp`, created new), syncs it, renames it over the document, then syncs the folder; a failed write removes only its own staging file.

## Guard proof
`concurrent_writes_of_one_record_each_land_whole` (blit-core): eight threads each rewrite one record forty times; every write lands, the last is whole, and no staging file is left. Mutation — one shared `<path>.tmp`, created by truncation, as before — fails with the reviewer's ENOENT; restored, green.
