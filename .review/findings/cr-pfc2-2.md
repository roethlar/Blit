# cr-pfc2-2: Push progress reports destination-failed files complete

**Severity**: MEDIUM — initiator-SOURCE progress (verbose/JSON) counts a
file as completed that is absent or incomplete at the destination.
**Status**: Open — fix bound to slice pfc-4 (requires the wire failure
report that slice introduces)
**Branch**: — (default-branch mode)
**Commit**: (lands with pfc-4)

Reviewer provenance (generation pass): codex / gpt-5.6-sol / xhigh /
standard; codex-cli 0.146.0; range `4d2b888f..575e47e7`
(`.review/results/pfc-2-range.codex.json`, attempt 2, 2026-07-31).

## Evidence
`remote/transfer/pipeline.rs:509-521` filters completions against the
LOCAL sink outcome, but `DataPlaneSink` returns `SinkOutcome::written`
immediately after transmission (`sink.rs:1227-1236`); the in-stream sender
emits FileComplete unconditionally (`transfer_session/mod.rs:2962-2967`,
`:3150-3155`); pushes wire this source-side progress at
`session_client.rs:199-200`.

## Predicted observable failure
Initiator-SOURCE mirror: a destination open/flush/metadata failure is
contained AFTER the sender emitted FileComplete → CLI progress and
completed-file totals include a file the destination does not have.

## What
The source side has no destination confirmation to filter against — pfc-2
could only suppress completions on lanes that see the destination outcome.
The initiator learns of destination failures only when a failure report
crosses the wire, which is exactly pfc-4's `TransferSummary` extension.

## Approach
(bound to pfc-4) When the destination's failure report reaches the
initiator, reconcile completed-file totals: either emit FileComplete only
from destination-confirmed outcomes or subtract/annotate failed paths in
the final progress accounting before the summary renders. pfc-4's slice
text carries this requirement.

## Files changed
(lands with pfc-4)

## Guard proof
(planned with pfc-4) Initiator-SOURCE session with an injected destination
failure: final completed count excludes the failed file; red when the
reconciliation is reverted.

## Coder dispute (if any)
None. Scheduling into pfc-4 is dependency ordering, not deferral: the fix
consumes the wire vocabulary pfc-4 creates.

## Known gaps
Until pfc-4 lands, initiator-SOURCE progress can overcount completions for
destination-failed files (destination-side warn logs remain the only
signal). Mirror-only exposure in the interim (non-mirror sessions abort on
contained failures per the pfc-2 interlock).

## Reviewer comments
(pending pfc-4 fix + per-finding verification dispatch)
