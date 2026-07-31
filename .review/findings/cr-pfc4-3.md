# cr-pfc4-3: An all-failed reconciliation makes progress appear never started

**Severity**: LOW — when every sent file fails at the destination,
initiator verbose/JSON progress takes the manifest-only terminal branch
and omits the final transfer event despite reconciled transfer work.
**Status**: Verified
**Branch**: — (default-branch mode)
**Commit**: `f2916090`

Reviewer provenance (generation pass): codex / gpt-5.6-sol / xhigh /
standard; codex-cli 0.146.0; range `72079331..bdc3e4a4`
(`.review/results/pfc-4-range.codex.json`).

## Evidence
`progress.rs:161-170` retracts files/bytes while recording
`files_failed`; `started()` at `:180-182` ignores `files_failed`;
`blit-cli/src/transfers/remote.rs:126-153` therefore selects the
manifest-only terminal branch.

## Predicted observable failure
All-failed initiator-SOURCE run: JSON/verbose output reports only
enumeration, no final transfer event.

## What
The observed-work gate derives from post-retraction counters instead of
being monotonic over observed work.

## Approach
(planned) `started()` treats nonzero `files_failed` as started work (the
minimal correct fix per the reviewer's better_approach).

## Files changed
(filled with the fix commit)

## Guard proof
(planned) Totals with full retraction + nonzero files_failed report
started() == true; red when the files_failed clause is removed.

## Coder dispute (if any)
None.

## Known gaps
None.

## Reviewer comments
Reviewer: codex / gpt-5.6-sol / xhigh; codex-cli 0.146.0 (detached, disposable worktree). Reviewed `f291609095ca7cf88bce63c651143bab03c8ff5a` base `2b5c091b3dc9d22b106f89305701a38095eb4371`. guard_confirmed **true** (reviewer dropped the clause itself: targeted red, restore green after mtime bump; confirmed clean/never-started behavior unchanged, all 14 ProgressTotals tests green). Verdict: **accepted**. 2026-07-31T08:28Z. Record: `.review/results/cr-pfc4-3.codex.json`.

As built: `started()` treats nonzero `files_failed` as observed work (with the rationale in its doc). Guard `a_fully_retracted_session_still_reads_as_started` — full retraction to files=0/bytes=0 with files_failed=1 must still read started — FAILED red with the clause removed (verified this session), green restored.
