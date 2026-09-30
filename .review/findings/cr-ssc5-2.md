# cr-ssc5-2: Resume hashes are read from the lossy-text file instead of the raw-name file

**Severity**: HIGH — resume hashes are computed at the lossy-text path while blocks are applied at the raw-name path; a resumed raw-name file can be stamped complete with stale or corrupt contents
**Status**: Open
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: (filled after the fix commit)
**Reviewer**: codex / gpt-5.6-sol / xhigh / standard (D-2026-07-31-3 standing codereview; range e14d1b72..69390931 (ssc-5), record .review/results/ssc-5-range.codex.json)

## Evidence
crates/blit-core/src/transfer_session/mod.rs:5501 — compute_resume_block_hashes receives only header.relative_path; lines 5983-5994 consequently resolve it with the text-only safe_join functions.

## Predicted observable failure
If both the raw-byte destination path and a distinct lossy-text path exist, resume hashes the latter but applies blocks to the former. Matching blocks can be omitted incorrectly, after which the raw-name file is stamped complete with stale or corrupt contents. Without a lossy counterpart, resume unnecessarily retransmits the whole file.

## Reviewer's suggested approach
Pass the FileHeader or raw_relative_path into compute_resume_block_hashes and resolve with safe_join_contained_named/safe_join_named. Test resume with different files at the raw and lossy paths.

## What
(coder fills in)

## Guard proof
(red/green proof; mutation described)

## Known gaps
(none yet)
