# cr-uv9-2: a move's descriptor was read before it was looked at

**Severity**: MEDIUM — a descriptor that is another file is read for as long as the file that was seen before it is refused
**Status**: Verified — `combined-r21` (Codex openreview, 2026-11-22) reviewed `c15ebf5a..b92c9a17` — the seven `combined-r20` fixes — with `guard_confirmed: true`: "Move hashing checks the descriptor before and after reading"
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `4222bd3d`
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `combined-r20` over `17f4eb97..c15ebf5a` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r20.codex.md`. It completes cr-uv8-1 (`81b6c829`).

## Evidence
"A mismatched regular-file descriptor advanced by all 65,536 bytes in my probe before Blit rejected it. The final check protects the receipt. An initial check also avoids reading an unrelated file for the length of a potentially large source."

cr-uv8-1 looked once, after the read: enough for what is known of the file, not for what is read of one that is not it.

## Intake
Admitted as the reviewer put it.

## What
`moved::content_of` looks at the descriptor before a byte is read: not the file as it was seen — another file, or this one changed already — and it is an error with nothing read. It looks again once read, as before.

## Guard proof
- `moved::content_tests::a_file_that_is_not_the_one_is_not_read`: another file of the same 70,000 bytes and time, and the file itself rewritten with its time put back — each an error, the descriptor's position still 0.
- Mutation, restored byte for byte: the look before the read dropped (red).
- cr-uv8-1's guards pass unchanged.

## Files changed
- `crates/blit-core/src/moved.rs` — `content_of`; guard.

## Known gaps
- None beyond cr-uv8-1's.
