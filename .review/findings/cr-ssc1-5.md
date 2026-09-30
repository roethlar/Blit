# cr-ssc1-5: Treat opened-handle metadata failures before announcement as per-file skips

**Severity**: MEDIUM — one per-inode metadata error still ends the whole run, against D-2026-09-28-2
**Status**: Open
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: (filled after the fix commit)
**Reviewer**: codex / gpt-5.6-sol / xhigh / standard (D-2026-07-31-3 standing codereview; range f74b0b1a..6bc3d08f, record .review/results/ssc-1-range.codex.json)

## Evidence
crates/blit-core/src/remote/transfer/data_plane.rs:499 and crates/blit-core/src/transfer_session/mod.rs:3105 — both carriers propagate OpenedSourceFile::len errors instead of using their pre-announcement skip paths.

Verified against master after ssc-3 (2026-09-30) by the orchestrator before admission.

## Predicted observable failure
On a filesystem where opening succeeds but metadata on the opened handle returns a per-inode error, one file aborts the entire transfer rather than being reported as a contained source failure; the remaining manifest is not transferred.

## Reviewer's suggested approach
Handle `len()` errors alongside open errors: emit FileSkipped/send_skip before announcing a file record and continue with the next need.

## What
(coder fills in)

## Guard proof
(red/green proof of the new guard; mutation described)

## Known gaps
(none yet)
