# cr-ls1-1 — APPLY measures only the tail drain, so apply backpressure lands in no phase

**Severity**: HIGH
**Status**: admitted
**Source**: `ls-1-range` codex dispatch over `a0b5d83d..d67b44fd`
Reviewer provenance (generation pass): codex / gpt-5.6-sol / xhigh /
workspace-write (detached, disposable worktree); codex-cli 0.146.0.
Record: `.review/results/ls-1-range.codex.json`.

## The finding, as returned

> `crates/blit-core/src/transfer_session/local.rs:470` starts APPLY timing
> only after the payload sender closes, while
> `crates/blit-core/src/transfer_session/mod.rs:5008` can spend most of the
> transfer awaiting the bounded apply queue outside every timer. When the
> sink is slower than planning — as expected on the target small-file/SMB
> workload — the queue wait paces production and the final drain can be near
> zero even though apply dominated wall time. The hard-gate report can
> therefore select the wrong phase; measure pipeline work or at least queue
> backpressure separately instead of treating tail drain as APPLY.

## Why this is admitted rather than argued

It is correct, and it is the specific failure ls-1 step (0) exists to
prevent. The instrument's whole purpose is to stop a phase being blamed for
another phase's cost. The `run.queue(payload).await` loop sits between the
PLAN timer and nothing at all: the COMPARE timer covers only
`diff_chunk_verdicts`, the PLAN timer covers only `plan_chunk`, and the
queue wait that follows is inside no span. On the owner's actual shape — a
slow SMB destination, where the sink is expected to be the slow party —
that untimed wait is exactly where the wall clock would go.

The landing note already conceded that APPLY is drain-only and that work
keeping pace with compare is "invisible by design". That concession
understated the problem: the issue is not only that fast apply is invisible,
it is that SLOW apply is invisible too, because it manifests as producer
backpressure rather than as drain. A report built on this could show every
phase small against a large `session_wall_ns` and offer no candidate at all,
or worse, be read as "no phase dominates".

## Repair direction (not yet implemented)

Time the queue wait. The `queue()` call site in `diff_chunk_and_apply_local`
is a single seam, matching the ENUMERATE_BACKPRESSURE split already used on
the source side — the same problem with the same shape, so it should get the
same treatment rather than a new mechanism. A guard must prove the new span
is non-vacuous under a genuinely slow sink, not merely present.

## Verification bar for closing

- The queue wait is attributed to a named phase, and the sum of attributed
  phases accounts for the session wall clock within a stated tolerance on a
  fixture where the sink is deliberately slow.
- Red/green proven by revert, with the revert shown to actually red the
  intended assertion (see cr-ls1-2: a guard that stays green under revert
  does not count).
