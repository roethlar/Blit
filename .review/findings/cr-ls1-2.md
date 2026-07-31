# cr-ls1-2 — The compare-subtraction guard is vacuous, and the landing records claimed otherwise

**Severity**: MEDIUM (the code defect); the FALSE RECORD is the more serious half
**Status**: admitted
**Source**: `ls-1-range` codex dispatch over `a0b5d83d..d67b44fd`
Reviewer provenance (generation pass): codex / gpt-5.6-sol / xhigh /
workspace-write (detached, disposable worktree); codex-cli 0.146.0.
Record: `.review/results/ls-1-range.codex.json`.

## The finding, as returned

> `crates/blit-core/tests/local_session.rs:1353` asserts
> `compare + repair <= 2 * session wall`. Because repair is nested within
> compare, that inequality still holds when Compare includes the repair
> time. I deliberately changed
> `crates/blit-core/src/transfer_session/mod.rs:4973` to record the full
> compare elapsed time without subtraction;
> `metadata_repair::repair_time_is_attributed_to_repair_not_to_compare`
> remained green. I restored the exact original SHA-256 bytes and confirmed
> the test green again, so the claimed red/green guard did not hold.

The reviewer returned `guard_confirmed: false` on this basis. Under the
repo's standing rule that a missing or false guard confirmation fails the
review closed, the `ls-1-range` dispatch is a FAIL, not a pass with notes.

## What was actually proven, and what was claimed

Two subtractions ship in this slice. Only one was guard-proven:

| subtraction | guard run | held? |
|---|---|---|
| ENUMERATE minus its backpressure | revert of the `measure` split | YES — reds the sample-count assertion |
| repair `measure` wrapper present at all | revert of the wrapper | YES — reds `repair.samples == 16` |
| COMPARE minus nested ATTRIBUTE_REPAIR | **never run** | **NO — vacuous** |

The commit message on `7d825d22`, the plan landing note, and the DEVLOG
entry all state that "both subtractions are red/green proven" and that
nested cost "is subtracted, not double-billed". The first is false. The
second is true of the code as written but was not demonstrated by any test,
which is the same thing as unproven.

This matters beyond the one assertion. The slice's own DEVLOG entry makes a
point of recording that an EARLIER version of the backpressure guard was
vacuous and was replaced — and then, in the same breath, ships a second
vacuous guard while claiming it was proven. Having noticed the failure mode
once is what makes the unchecked claim indefensible rather than unlucky: the
assertion was written knowing durations are unreliable in fast tests, and
the revert that would have exposed it was never run.

## Repair direction (not yet implemented)

The assertion needs to compare COMPARE against something the subtraction
actually moves. `compare + repair <= 2 * wall` is satisfied by construction
and cannot fail. A workable shape: pin that COMPARE's total excludes a
repair span that is made large enough to be unambiguous (an injected slow
repair), or assert directly on the relationship the subtraction creates
rather than on a bound that holds either way.

Whatever replaces it must be verified by running the revert the reviewer
ran — remove the `saturating_sub` at the compare seam and confirm the test
goes red.

## Record correction owed

`DEVLOG.md`, `docs/plan/LOCAL_SMALL_FILE_PATH.md` and `docs/STATE.md` each
carry the false "both subtractions red/green proven" claim. Commit
`7d825d22`'s message carries it too and cannot be edited; the correction is
recorded here and in a follow-up DEVLOG entry rather than by rewriting
history.
