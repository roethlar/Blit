# cr-ecm27-2: the wire's first looks at a destination still ended the transfer

**Severity**: HIGH — a granted `album/photo` after `album` had become a regular file: containment passed, `ENOTDIR` escaped the hashing, and the grant sender returned early, abandoning unrelated files; the first look at the opened file failed the same way
**Status**: Verified — `combined-r29` (Codex openreview, 2026-11-22) reviewed `10145127..e3ea9517` — the five `combined-r28` fixes — with `guard_confirmed: true`: "I would keep the whole-file fallback for failed metadata reads, cleanup in the default `skip_record`, and Linux's atomic no-replace rename."
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `1a64a0c3`
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `combined-r28` over `e025a3e2..10145127` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r28.codex.md`. It completes cr-ecm25-3 (`b61b6c6c`) and cr-ecm26-3 (`c7ca4d8d`).

## Evidence
"I tested a granted `album/photo` after `album` became a regular file. Containment succeeded, but `ENOTDIR` escaped the hash calculation and propagates out of the grant sender, abandoning unrelated files. The first descriptor metadata call also propagates errors. Use the existing whole-file fallback for file-specific comparison failures at both points."

## Intake
Admitted as the reviewer put it.

## What
`compute_resume_block_hashes`: a failing look at the path, of whatever kind, sends the file whole, as one that found nothing did. So does a failing first look at the file opened. Every look the hashing takes is now the one file's matter. The hash-read seam reaches the opened file's first look at `usize::MAX - 1`.

## Guard proof
- `own_incomplete_copy_tests::a_failed_first_look_sends_the_file_whole`: `album/photo` with `album` a regular file, and a file whose first opened look fails — each `Whole`, no error.
- Mutations, each restored byte for byte: the path's look ending the transfer (red); the opened file's first look ending it (red).
- The resume, sink and session tests (328) pass. Windows cross-clippy clean.

## Files changed
- `crates/blit-core/src/transfer_session/mod.rs` — `compute_resume_block_hashes`, the seam's word; guard.

## Known gaps
- A volume that has failed is no longer told apart while hashing; the writer meets it at the file's hold (cr-ecm21-5).
