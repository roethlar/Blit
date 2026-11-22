# cr-ecm21-2: a wire resume's hashes were paired with the version taken after them

**Severity**: HIGH — a block written while later blocks were hashed is skipped by the source and the file reported complete with the wrong bytes
**Status**: Fixed, awaiting review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix cr-ecm21-2`)
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `combined-r22` over `b92c9a17..5d2f8def` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r22.codex.md`. It completes cr-ecm20-1 (`5b3bf24e`).

## Evidence
"If an already-read block changes while later blocks are being hashed, the receiver pairs the old block hash with the file's *post-read* version. The writer's version check can pass, the sender can skip that block, and Blit can report incorrect bytes as complete. Compare the opened file's version before and after a bounded read. Discard the hash list if it changed."

## Intake
Admitted as the reviewer put it.

## What
`compute_resume_block_hashes` takes the file's version before a byte is read, reads no further than the size it had then, and looks again at the end: changed, and no hash is sent — every block comes, into a fresh partial (`ResumeBaseline::Whole`). The baseline the writer checks is the version from before the read.

## Guard proof
- `transfer_session::…::a_partial_written_while_it_is_hashed_is_sent_whole`: a three-block file, unchanged — three hashes, `Hashed`; with its first block rewritten once it was hashed (a seam between blocks; as many bytes, its time put back) — no hash, `Whole`.
- Mutation, restored byte for byte: the hashes sent whatever happened as they were read (red).
- cr-ecm20-2's `hashing_a_resume_partial_changes_nothing` and the sink and session tests (268) pass. Windows cross-clippy clean.

## Files changed
- `crates/blit-core/src/transfer_session/mod.rs` — `compute_resume_block_hashes`, a seam; guard.

## Known gaps
- A change that leaves no trace in the version (a filesystem whose time of last change is coarse, a kernel that keeps it to the tick) is not seen, as for every `Version` comparison.
