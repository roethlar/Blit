# cr-ecm26-3: a destination that could not be compared still failed, on the wire and locally

**Severity**: HIGH on the wire, MEDIUM locally — an `ESTALE` on the wire's last look after hashing ended the transfer, abandoning unrelated files; a local destination block that could not be read (`EIO`) failed that copy
**Status**: Verified — `combined-r28` (Codex openreview, 2026-11-22) reviewed `e025a3e2..10145127` — the six `combined-r27` fixes — with `guard_confirmed: true`: "The cursor reset, local rewrite after a failed block read, and whole-file fallback after the final wire metadata check all fit that approach. I would keep them"
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `c7ca4d8d`
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `combined-r27` over `05ff50d7..e025a3e2` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r27.codex.md`. It completes cr-ecm25-3 (`b61b6c6c`) and cr-ecm25-5 (`660eb9d0`).

## Evidence
"An injected `ESTALE` on the final wire metadata check escaped the hash computation and propagates out of the grant sender, abandoning unrelated files. An injected local destination-block `EIO` failed that copy despite a readable source and writable directory. Discard unverifiable hashes and retry the affected file whole."

## Intake
Admitted as the reviewer put it.

## What
- Wire (`compute_resume_block_hashes`): the look after the last block, failing, leaves the hashes unverified, and the file comes whole. The hash-read seam reaches it at block `usize::MAX`.
- Local (`resume_copy_through`): a destination block that cannot be read is written, and so is every block after it — the rest of the file cannot be compared. A seam (`DST_READ_FAULTS`) fails a block's read by path and offset.

## Guard proof
- `own_incomplete_copy_tests::a_failed_look_after_hashing_sends_the_file_whole`: no hashes, `Whole`, no error.
- `resume::tests::a_block_that_cannot_be_read_is_written_with_the_rest`: an identical 4 KiB destination whose second block cannot be read — the copy succeeds, 3,072 bytes written, the source's bytes on disk.
- Mutations, each restored byte for byte: the wire's last look ending the transfer (red); a local block that cannot be read failing the copy (red).
- The resume, sink and session tests (326) pass. Windows cross-clippy clean.

## Files changed
- `crates/blit-core/src/transfer_session/mod.rs` — `compute_resume_block_hashes`, the seam's word; guard.
- `crates/blit-core/src/copy/file_copy/resume.rs` — `resume_copy_through`, `dst_read_fault`, its seam; guard.

## Known gaps
- A block that cannot be written still fails the copy, as it must: the destination itself refuses.
