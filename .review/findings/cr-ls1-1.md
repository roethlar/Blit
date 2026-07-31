# cr-ls1-1 — APPLY measures only the tail drain, so apply backpressure lands in no phase

**Severity**: HIGH
**Status**: VERIFIED-CLOSED. Round-2 dispatch over `a0b5d83d..35948e70`
returned `guard_confirmed: true` and did not re-raise this finding; the
reviewer independently ran the red/green itself. Record:
`.review/results/ls-1-range.codex.r2.json`.
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

## Fix

New `LocalPhase::ApplyBackpressure` times the `run.queue(payload).await`
call in `diff_chunk_and_apply_local` — the single seam where the bounded
queue blocks the diff loop. Same shape as the ENUMERATE_BACKPRESSURE split
already used on the source side, deliberately, because it is the same
problem: a bounded channel making one party's slowness look like another
party's cost, or like nobody's.

APPLY (drain) is kept rather than replaced. The two together distinguish
"the writer was still working after the reader finished" (drain) from "the
writer paced the reader throughout" (backpressure), which is the
distinction needed to pick a phase to attribute.

**Guard**: `transfer_session::local::tests::a_slow_sink_is_attributed_to_apply_backpressure`.
A `SlowSink` wrapper adds a fixed 40 ms per payload behind the real sink,
with `DEFAULT_PAYLOAD_PREFETCH * 4` files each just over 1 MiB so every one
plans as its own `File` payload and the queue provably backs up. Asserts
both that every push is sampled and that the accumulated wait clears a
floor derived from `(FILES - queue_depth) * delay / 2`.

The floor half matters: the FIRST version of this fixture used only 8 files
and passed with 17,700 ns of backpressure — the entire backlog fit in the
queue, nothing ever blocked, and the test would have "passed" while
testing nothing. Caught by the assertion failing, not by inspection.

**Guard proof**: removing the timing span at the queue seam reds the test
on `backpressure.samples > 0`; restore verified byte-identical by SHA-256
(`976D71B1…49205`), green again after.
