# cr-ecm25-3: a read error while one destination was hashed ended the transfer

**Severity**: HIGH — an `EIO` on the second block of one destination being hashed propagated out of the hashing, and the grant sender returned early, abandoning every other file
**Status**: Reviewed, found incomplete — `combined-r27` (Codex openreview, 2026-11-22) reviewed `05ff50d7..e025a3e2` — the six `combined-r26` fixes — with `guard_confirmed: true`: "The patch takes the right approach by carrying the local writer through metadata stamping, staging private partials in a held directory, and discarding incomplete wire hashes." "An injected `ESTALE` on the final wire metadata check escaped the hash computation and propagates out of the grant sender, abandoning unrelated files. An injected local destination-block `EIO` failed that copy despite a readable source and writable directory. Discard unverifiable hashes and retry the affected file whole." The remaining work is cr-ecm26-3
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `b61b6c6c`
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `combined-r26` over `19025839..05ff50d7` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r26.codex.md`. It completes cr-ecm24-4 (`f6cb0495`), whose record named this gap.

## Evidence
"An injected `EIO` on the second destination block propagated out of hash computation. The grant sender then returns early, abandoning unrelated files. Discard the incomplete hashes and request the whole file when the error is confined to that file."

## Intake
Admitted. A read of one destination file is that file's matter (D-2026-07-09-1). If the volume itself fails, the writer meets it again at that file's hold and ends the session there (cr-ecm21-5).

## What
`compute_resume_block_hashes`: a read that fails while the file is hashed drops the hashes taken so far, and the file comes whole (`ResumeBaseline::Whole`, no hashes). A test seam (`HASH_READ_FAULTS`) fails a read by path and block.

## Guard proof
- `own_incomplete_copy_tests::a_read_that_fails_while_hashing_sends_the_file_whole`: a 5,000-byte destination whose second block's read fails — no hashes, `Whole`, no error.
- Mutation, restored byte for byte: a read that fails ending the transfer (red).
- The resume, sink and session tests (316) pass. Windows cross-clippy clean.

## Files changed
- `crates/blit-core/src/transfer_session/mod.rs` — `compute_resume_block_hashes`, the seam; guard.

## Known gaps
- A failed look at the file after its hashes (the version check) still ends the transfer; no seam reaches it, and a look at a file that is held open does not fail for that file alone.
