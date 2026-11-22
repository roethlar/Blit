# cr-ecm28-4: the hashed hold kept the new mode when marking the partial failed

**Severity**: MEDIUM — under a 4,096-byte file-size limit, marking an eight-byte partial unfinished (4,097 bytes) failed with `EFBIG`: its bytes were kept, but its mode went from `0644` to `0600`
**Status**: Fixed on Unix, awaiting review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix cr-ecm28-4`)
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `combined-r29` over `10145127..e3ea9517` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r29.codex.md`. It completes cr-ecm27-4 (`50c6b3be`).

## Evidence
"With a 4,096-byte file-size limit, the public sink failed to mark an eight-byte partial unfinished at 4,097 bytes. It reported `EFBIG` and preserved all eight bytes, but changed the mode from `0644` to `0600`. Collect the required identity before changing the length and restore the mode on acquisition failure."

## Intake
Admitted as the reviewer put it.

## What
`hold_hashed`: once the partial is made private, its last look, the identity it is held by (learned now, before its length changes) and marking it unfinished each put its mode back if they fail. Before, the rollback was let go before the mark. A seam (`MARK_FAULTS`) refuses the mark by path.

## Guard proof
- `sink::tests::a_hashed_hold_that_cannot_mark_its_partial_gets_the_mode_back` (Unix): a `0644` eight-byte destination held as hashed, the mark refused (`EFBIG`) — its mode and bytes as they were, and the file fails.
- Mutation, restored byte for byte: a refused mark leaving the new mode (red).
- The resume, sink and session tests (333) pass. Windows cross-clippy clean.

## Files changed
- `crates/blit-core/src/remote/transfer/sink.rs` — `hold_hashed`, `mark_fault`, its seam; guard.

## Known gaps
- None known.
