# cr-ecm31-1: a hashing that failed part-way discarded the version it had

**Severity**: HIGH — when hashing a destination failed after its first look (a read, the hash-count cap, the look after the last block, or the file changing while hashed), it was sent `Whole` with nothing of what it had found. The hold then looked for itself, and could remove a file that came to the name before the first write.
**Status**: Fixed on Unix, awaiting review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix cr-ecm31-1`)
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `combined-r32` over `93ecf7b2..714ed0b7` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r32.codex.md`. It completes cr-ecm30-1 (`125231cb`).

## Evidence
"A focused check confirmed that a read error after the first block returns `Whole`, discarding the version already obtained. The receiver can then remove a replacement that arrived before the first write. Carry the last trustworthy observation through that fallback, or fail the individual file for this pass."

## Intake
Admitted: the last trustworthy observation is carried.

## What
In `compute_resume_block_hashes`, every fallback after a look sends `WholeOf` with what was found:
- the first look at the opened file failing → the path's look;
- a read failing, or the hash count passing its cap → the file as it stood before it was read (`before`);
- the look after the last block failing → `before`;
- that look finding the file changed → the file as that look found it.

Where the name itself is that file and not a link to it, `leaf` takes the same version, so the hold's look at the name agrees with its look at the file. Only a look at the name, or at its end, that fails other than as `NotFound` still sends `Whole`, observation unknown.

## Guard proof
- `transfer_session::tests::a_hashing_that_fails_part_way_keeps_what_it_found` (Unix): a read refused on the second block, and the look after the last refused. Each sends `WholeOf { leaf: Some(v), end: Some(v) }`, with `v` the file as it stood.
- Mutations, each restored byte for byte:
  - a failed read sending the file as unknown → red;
  - a failed last look sending the file as unknown → red.
- The resume, sink, session, copy and data-plane tests (360) pass. The older guards of these fallbacks still find the file sent whole.

## Files changed
- `crates/blit-core/src/transfer_session/mod.rs` — `compute_resume_block_hashes`; guard.

## Known gaps
- A file that keeps changing while it is hashed is sent with its latest version. If it changes again before the hold, the hold finds it changed and fails it for this pass; the retry compares afresh (D-2026-11-23-2).
