# cr-ssc1-5: Treat opened-handle metadata failures before announcement as per-file skips

**Severity**: MEDIUM — one per-inode metadata error still ends the whole run, against D-2026-09-28-2
**Status**: Verified
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `458ca62c`
**Reviewer**: codex / gpt-5.6-sol / xhigh / standard (D-2026-07-31-3 standing codereview; range f74b0b1a..6bc3d08f, record .review/results/ssc-1-range.codex.json)

## Evidence
crates/blit-core/src/remote/transfer/data_plane.rs:499 and crates/blit-core/src/transfer_session/mod.rs:3105 — both carriers propagate OpenedSourceFile::len errors instead of using their pre-announcement skip paths.

Verified against master after ssc-3 (2026-09-30) by the orchestrator before admission.

## Predicted observable failure
On a filesystem where opening succeeds but metadata on the opened handle returns a per-inode error, one file aborts the entire transfer rather than being reported as a contained source failure; the remaining manifest is not transferred.

## Reviewer's suggested approach
Handle `len()` errors alongside open errors: emit FileSkipped/send_skip before announcing a file record and continue with the next need.

## What
Both carriers' pre-announcement stat on the opened handle (`send_payload_records` in `transfer_session/mod.rs`; `DataPlaneSession::send_file` in `remote/transfer/data_plane.rs`) now treat an `Err` from `OpenedSourceFile::len()` as that file's skip (`source: cannot read metadata: {err}`), exactly like an open failure — `FileSkipped` / SKIP record, nothing announced. `OpenedSourceFile::virtual_reader_stat_fails` is the test hook.

## Guard proof
`in_stream_opened_handle_stat_failure_is_skipped_and_reported` and `data_plane_opened_handle_stat_failure_is_skipped_and_reported` (`tests/source_side_containment.rs`, `Fault::StatFails` through `assert_skip_contained`: the file is reported with the metadata reason, the other two land, both ends agree, the move gate refuses). Mutations: the `?`/`map_err(tag_path)?` restored on each carrier → each guard FAILED (source faulted instead of completing; `scratchpad/cr-ssc-mutations.txt`); restored → green.

## Known gaps
None.
