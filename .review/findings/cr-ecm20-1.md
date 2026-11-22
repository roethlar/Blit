# cr-ecm20-1: a wire resume's hashes, block writes and completion were not tied to one file

**Severity**: HIGH — a destination changed or replaced after its blocks were hashed is patched with the source's other blocks and reported complete with bytes it never had
**Status**: Fixed on Unix, awaiting review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix cr-ecm20-1`)
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `combined-r21` over `c15ebf5a..b92c9a17` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r21.codex.md`. It completes cr-ecm19-4 (`af308007`).

## Evidence
"Each block and the final truncation reopen the destination by name. If the destination changes after hashing, the sender can skip a block that matched the original file, and the receiver can report the replacement complete with incorrect bytes. Retain the hashed file's identity, detect a changed baseline, and verify delivery before acknowledging completion."

## Intake
Admitted as the reviewer put it. Seen in the work: a completion whose hold had refused the file still marked whatever stood at the name one byte longer, by its name — the user's own file, which nothing was to touch.

## What
- The hold (cr-ecm20-2's) asks, of a file whose blocks were hashed, that it is that file unchanged since — its version as it was hashed (identity, size, both times) — before anything is done to it; otherwise nothing is written and the file fails. Every hold says which file it holds, and the sink keeps that until the completion.
- A block is written only into the file held: opened by its name, it must be that file (a name that lost it is not made again); otherwise the failure is held and no block more is written.
- The completion cuts and stamps only the file held — checked as it is opened — and reports the file complete only while the name holds it, or a link there leads to it (`still_its_own`, after the stamp).
- A completion whose file failed marks a file left at the source's size only when it is the file held — never whatever stands there after a hold that failed. Where the system does not say which file is which, as before.

## Guard proof
- `sink::…::a_wire_resume_works_on_the_file_it_hashed_and_no_other`: a file rewritten after it was hashed (as many bytes, its time put back) — nothing written, not marked, the file fails; another file renamed over the name after the hold, after the block, and as the completion stamps (the claim seam) — the other file whole each time (not written, not cut, not stamped), the file failing.
- Mutations, each restored byte for byte, five: the hold taking a changed file; a block written into whatever is at the name; the completion cutting whatever is at the name; the completion reported whoever holds the name; a failed hold's file marked all the same — each red.
- cr-ecm20-2's, cr-ecm19-4's and cr-win-1's guards pass. The whole suite in the scratch copy: 1,973 passed, 0 failed. Windows cross-clippy clean.

## Files changed
- `crates/blit-core/src/remote/transfer/sink.rs` — `hold_partial` and its arms, `hold_partial_unfinished`, `patch_file_block`, `finalize_resumed_file`, the block and completion payloads; guard.

## Known gaps
- Between a block's look at the file and its write, or the completion's last look and its report, the name can change again; each is a look, then an act, as every check by name is.
- A write by another hand into the held file itself, between blocks, is not seen.
- Speed: one `fstat` more per block and per completion; not measured (blocks are a megabyte or more).
- Windows: no file identity; nothing is asked there (cr-ecm20-4's Windows half).
