# cr-pfc3-2: Deferred rayon classification can miss a dead destination root

**Severity**: HIGH — a root that dies mid-shard and recovers before the
fold reclassifies root-caused errors as per-file; the mirror finishes
"incomplete" instead of session-fatal on a dead root.
**Status**: In progress — fixed, awaiting per-finding reviewer verdict
**Branch**: — (default-branch mode)
**Commit**: (filled at landing)

Reviewer provenance (generation pass): codex / gpt-5.6-sol / xhigh /
standard; codex-cli 0.146.0; range `ff38f0e6..3ae5bf4d`
(`.review/results/pfc-3-range.codex.json`).

## Evidence
`sink.rs:1046-1053`/`1065-1116` collect ALL rayon member results before
classification; only at `:1204-1207` does the fold call
`failure_is_containable`, so `destination_root_live` (`:175-181`)
observes root state AFTER the workers finished, not when each error
occurred.

## Predicted observable failure
SMB root disconnects during member writes and reconnects before the
fold (or the root is recreated): dead-root errors classify per-file,
the mirror completes contained instead of the session-fatal
destination-root error the classification principle requires.

## What
Classification is time-of-check/time-of-use split from the failure it
judges. The verdict must be taken when the error exists, in the worker.

## Approach
(planned) Each rayon worker classifies its own error at failure time
(calling the shared predicate there) and returns a preserved
fatal-vs-contained verdict; the fold honors the recorded verdict
instead of re-deriving it later. Same for the sequential wrapper.

## Files changed
(filled with the fix commit)

## Guard proof
(planned) A fixture whose root is dead at write time but restored
before the fold still returns Err (red under the fold-time
classification); ordinary member failures with a live root still
contain.

## Coder dispute (if any)
None.

## Known gaps
None.

## Reviewer comments
(pending per-finding verification after the fix)

As built: `classify_shard_member` takes the verdict (`ClassifiedMember::{Contained, Fatal}`) IN the worker at error time, calling the one shared `failure_is_containable`; `fold_shard_member_results` honors recorded verdicts and re-derives nothing; the sequential wrapper shares the seam. Guards: the TOCTOU test produces the error on a dead root, revives the root, folds — Err (red under the fold-time revert, whose panic output shows the fold containing the dead-root error against the recovered root); the reverse-direction test pins that a live-root Contained verdict survives a root that dies later (defeats hardcoding Fatal); cr-pfc3-1's exhaustion boundary re-pinned on the new seam. SHA-verified restores; blit-core lib 464/0; workspace all green.
