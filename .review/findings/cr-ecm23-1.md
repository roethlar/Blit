# cr-ecm23-1: a wire resume's fresh partial took the destination's place before its identity was known

**Severity**: HIGH — a hold whose look at its fresh partial failed left the destination replaced by eleven bytes of nothing, and nothing held
**Status**: Fixed on Unix, awaiting review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix cr-ecm23-1`)
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `combined-r24` over `24d4064f..c89b1eb1` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r24.codex.md`. It completes cr-ecm22-3 (`eb283420`).

## Evidence
"The wire path renames the file before obtaining its identity. With the repository's identity fault injected, I observed the original `0123456789` replaced by eleven zero bytes before any payload arrived. Obtain the identity on the staged handle and finish preparation before the rename."

## Intake
Admitted as the reviewer put it.

## What
`fresh_partial_ready(dst, unfinished, ready)` runs `ready` on the staged file — private and at its length — before it is renamed over the destination; an error there removes the staged file and leaves the destination as it stood. `fresh_partial` is it with nothing to learn. The wire's whole hold (`hold_whole`) learns which file the partial is in `ready` (`HeldPartial::identity_of`), so a look that fails is the file's failure with the destination untouched.

## Guard proof
- `sink::tests::a_fresh_partial_is_known_before_it_takes_the_name` (Unix): a destination `0123456789` held whole while the look fails (the `IDENTITY_FAULTS` seam) — the destination as it stood, nothing beside it, and its completion failed.
- Mutation, restored byte for byte: the identity learned after the rename (red).
- The resume, sink and session tests (295) pass. Windows cross-clippy clean.

## Files changed
- `crates/blit-core/src/copy/file_copy/resume.rs` — `fresh_partial_ready`; `fresh_partial` through it.
- `crates/blit-core/src/copy/file_copy/mod.rs`, `crates/blit-core/src/copy/mod.rs` — its export.
- `crates/blit-core/src/remote/transfer/sink.rs` — `HeldPartial::identity_of`, `hold_whole`; guard.

## Known gaps
- Unix only. The Windows hold keeps no handle (cr-ecm22-3's Windows half, blocked).
