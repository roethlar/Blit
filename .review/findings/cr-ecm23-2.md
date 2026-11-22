# cr-ecm23-2: a hold with no word of the hashes dropped its error

**Severity**: HIGH — a hold whose first look failed held nothing, and the completion opened the name and reported the file written
**Status**: Reviewed, found incomplete — `combined-r25` (Codex openreview, 2026-11-22) reviewed `c89b1eb1..19025839` — the six `combined-r24` fixes — with `guard_confirmed: true`. "On the Unix sink path without a recorded hash baseline, a failed `set_permissions(0600)` only logs a warning. The sink can then patch a file whose permissions it failed to restrict. Propagate that error through the file's completion." The remaining work is cr-ecm24-5; the Windows half stays open, blocked
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `99c909fe`
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `combined-r24` over `24d4064f..c89b1eb1` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r24.codex.md`. It completes cr-ecm22-3 (`eb283420`).

## Evidence
"On Unix, with no recorded hash baseline, failure of the first metadata read becomes an empty handle. Completion then reopens the path. Injecting the fault at that first read produced one file reported written and zero failures. Return the acquisition error and require a verified handle for completion. Windows also discards a supplied baseline here, leaving its documented zero-block completion issue open."

## Intake
Admitted. On Unix after this fix, every completion either has the handle its hold took or a failure its hold left: no hold returns nothing there. So "require a verified handle for completion" holds by construction, and no separate check is added for a case that cannot arise. The Windows half — a hold that keeps nothing and does not use the baseline — is cr-ecm22-3's Windows half, blocked: no Windows host answers.

## What
`hold_partial_unfinished` returns its error. On Unix the hold with no word of the hashes takes it as the file's failure, as the hashed and whole holds do. Elsewhere it is logged and passed over, as before. The test seam `IDENTITY_FAULTS` also reaches its first look.

## Guard proof
- `sink::tests::a_hold_with_no_word_of_the_hashes_fails_when_it_cannot_be_had` (Unix): a hold with no baseline whose first look fails (the seam) — the completion reports the file failed, none written.
- Mutation, restored byte for byte: a hold that cannot be had passed over (red).
- The resume, sink and session tests (296) pass. Windows cross-clippy clean.

## Files changed
- `crates/blit-core/src/remote/transfer/sink.rs` — `hold_partial`, `hold_partial_unfinished`, two tests' calls; guard.

## Known gaps
- Windows: blocked, as above.
