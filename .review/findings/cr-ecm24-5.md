# cr-ecm24-5: a hold that could not make its partial private wrote it anyway

**Severity**: HIGH — on the wire's hold with no word of the hashes, a refused `set_permissions(0600)` was only logged, and the partial was patched with the mode it had
**Status**: Fixed on Unix, awaiting review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix cr-ecm24-5`)
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `combined-r25` over `c89b1eb1..19025839` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r25.codex.md`. It completes cr-ecm23-2 (`99c909fe`).

## Evidence
"On the Unix sink path without a recorded hash baseline, a failed `set_permissions(0600)` only logs a warning. The sink can then patch a file whose permissions it failed to restrict. Propagate that error through the file's completion."

## Intake
Admitted as the reviewer put it.

## What
`hold_partial_unfinished` (Unix) returns the refusal to make the partial private as its error. The hold with no word of the hashes takes it as the file's failure (cr-ecm23-2), so no block is written to the file and its completion fails.

## Guard proof
- `sink::tests::a_hold_that_cannot_make_its_partial_private_fails_the_file` (Unix): a `0644` destination held with no baseline while making it private is refused (the `PRIVATE_FAULTS` seam) — its completion fails, nothing written.
- Mutation, restored byte for byte: the refusal only logged (red).
- The resume, sink and session tests (311) pass. Windows cross-clippy clean.

## Files changed
- `crates/blit-core/src/remote/transfer/sink.rs` — `hold_partial_unfinished`, its seam; guard.

## Known gaps
- None known on Unix. Elsewhere than Unix, the hold keeps nothing (cr-ecm22-3's Windows half, blocked).
