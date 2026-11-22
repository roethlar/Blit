# cr-ecm21-5: a volume failure at a resume's hold was one file's

**Severity**: MEDIUM — a destination that stops taking writes during a resume's hold is held as one file's failure, and the receiver drains every payload after it into a volume that takes none
**Status**: Verified — `combined-r23` (Codex openreview, 2026-11-22) reviewed `5d2f8def..24d4064f` — the six `combined-r22` fixes — with `guard_confirmed: true`: "Checking the file version before and after hashing, ignoring empty blocks for acquisition, retaining the Unix write handle, and distinguishing volume failures from per-file failures are sound choices."
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `760119f4`
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `combined-r22` over `b92c9a17..5d2f8def` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r22.codex.md`. It completes cr-ecm20-2 (`9ee5cb63`).

## Evidence
"A read-only filesystem or exhausted volume during the hold is recorded as one file's failure. The receiver can continue draining payloads after the destination has stopped accepting writes. Apply the existing root and volume error classifier at acquisition, as the block-write path already does."

## Intake
Admitted as the reviewer put it.

## What
`hold_resume_partial` asks the one classifier every write site asks (`failure_is_containable`, cr-pfc2-1): a volume that refuses writes, or a destination root gone, ends the session; anything else at the hold stays the file's, as cr-ecm20-2 made it.

## Guard proof
- `sink::…::a_volume_that_refuses_writes_at_a_hold_ends_the_session`: a hold that meets a read-only filesystem (a seam, `HOLD_FAULTS` — no test can make a real one) returns the error; one that meets `Permission denied` holds it for the file, which fails at its completion.
- Mutation, restored byte for byte: a volume failure at a hold held as one file's (red).
- The sink tests (111) pass. Windows cross-clippy clean.

## Files changed
- `crates/blit-core/src/remote/transfer/sink.rs` — `hold_resume_partial`, `hold_partial`, a seam; guard.

## Known gaps
- None beyond cr-pfc2-1's own (a transient full volume ends the run).
