# cr-ssc5-2: Resume hashes are read from the lossy-text file instead of the raw-name file

**Severity**: HIGH — resume hashes are computed at the lossy-text path while blocks are applied at the raw-name path; a resumed raw-name file can be stamped complete with stale or corrupt contents
**Status**: Verified
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `3c7d1b95 + b90613b7 + 42beaa3f`
**Reviewer**: codex / gpt-5.6-sol / xhigh / standard (D-2026-07-31-3 standing codereview; range e14d1b72..69390931 (ssc-5), record .review/results/ssc-5-range.codex.json)

## Evidence
crates/blit-core/src/transfer_session/mod.rs:5501 — compute_resume_block_hashes receives only header.relative_path; lines 5983-5994 consequently resolve it with the text-only safe_join functions.

## Predicted observable failure
If both the raw-byte destination path and a distinct lossy-text path exist, resume hashes the latter but applies blocks to the former. Matching blocks can be omitted incorrectly, after which the raw-name file is stamped complete with stale or corrupt contents. Without a lossy counterpart, resume unnecessarily retransmits the whole file.

## Reviewer's suggested approach
Pass the FileHeader or raw_relative_path into compute_resume_block_hashes and resolve with safe_join_contained_named/safe_join_named. Test resume with different files at the raw and lossy paths.

## What
`compute_resume_block_hashes` takes the header's raw bytes and resolves with the `_named` path-safety variants, exactly as the sink resolves the record.

## Guard proof
Linux-only session pin (proven on magneto): the lossy-text neighbour is an exact copy of the source and the raw file is stale by mtime; with the fix the raw file is re-landed, with the raw bytes ignored the stale bytes are stamped as resumed (red). Two follow-ups sharpened the fixture so the defect is observable.

## Known gaps
The guard is `cfg`-gated to byte-keyed Unix and runs only on Linux CI/hosts.
