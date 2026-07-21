# ldt-4-live-f12 — add a sustained controller-exercise cell

**Severity**: MEDIUM — the valid rig-W matrix completes before the live
controller can change membership, so ldt-4 cannot prove adaptive ADD/REMOVE
or role invariance under an actual transition.
**Status**: Open; implementation and guard proof pending.
**Branch**: `master` (repo policy forbids agent-created branches)
**Commit**: pending

## Evidence

Exact harness/analyzer `96a4e3b03caf43ee368efadc779e3324248067f6`
completed all 96 structurally valid arms in retained session
`ldt4-20260721T224319Z-96a4e3b03caf`. Independent recomputation matched all
six generated analysis files byte-for-byte.

Every arm reported floor = peak = final = 4 and no accepted ADD or REMOVE.
Seventy-four arms ended before any tuner sample; the other 22 produced one
sample. The analyzer therefore returned `REVIEW_REQUIRED` with arm review 0,
decision review 14, and performance review 2. Exact evidence and its inventory
digest are recorded at `docs/bench/ldt4-rigw-2026-07-21/`.

The production tuner samples every 500 ms. Before the first ADD it must raise
the cheap dials to their ceilings, satisfy two sustained samples, and clear
the four-tick settlement cooldown. The existing 1 GiB large arms complete in
roughly 0.9–1.1 seconds; the mixed and small cells are shorter. Repeating that
unchanged matrix cannot exercise the required transition.

## Predicted observable failure

Another run of only the registered 96 short arms can complete with perfect
payload integrity while again producing no membership change. It would remain
valid performance evidence but still could not establish adaptive behavior;
end-of-transfer sample-count differences can also keep generating role-pair
decision review without any transition to compare.

## What

Keep the original 96-arm fixed-fixture matrix and its performance grading
unchanged. Append one adjacent initiator-layout pair in each physical byte
direction using a dedicated 5 GiB sustained fixture. Treat these four arms as
controller-exercise evidence, not replacements for the fixed performance
cells. Require every sustained arm to accept at least one ADD above the
four-stream floor and require each role pair to agree on its accepted
membership-transition sequence.

## Approach

- Additively stage `src_sustained` on both endpoints as five distinct 1 GiB
  files copied from the already validated canonical large fixture. Verify the
  exact five-file/5 GiB manifests are byte-identical before any arm.
- Preserve the existing first 96 schedule rows byte-for-byte, then append four
  sustained rows: one adjacent source-init/destination-init pair for
  q→Windows and one counter-ordered adjacent pair for Windows→q.
- Before staging or running, prove enough q and Windows free space remains for
  the new stable source, all four retained destinations, and the existing
  33,000,000,000-byte floor. Do not delete or overwrite prior evidence,
  fixtures, or retained payloads.
- Extend the analyzer's exact schedule, fixture, runtime-gate, completion, and
  inventory contracts to the 100-arm matrix. Keep performance grading scoped
  to the original six cells. Add a separate sustained verdict that requires
  an accepted ADD in all four arms and exact accepted-operation/target parity
  within each initiator-layout pair.
- Add structural and analyzer tests that fail if the sustained rows disappear,
  the fixture is shortened, the original 96 rows change, or the required
  transition/parity verdict is weakened. Mutation-prove the new guard red,
  restore it, then run the full repository gates.

## Files expected

- `scripts/bench_ldt4_rigw.sh` — additive sustained fixture, registered four
  diagnostic arms, capacity gates, and structural guards.
- `scripts/ldt4_rigw_analyze.py` — exact 100-arm contract and sustained
  transition/parity verdict separate from fixed-cell performance.
- `scripts/ldt4_rigw_analyze_test.py` — valid sustained fixtures/traces and
  rejection tests for absent transitions or pair mismatch.

## Guard proof

Pending.

## Coder dispute

None.

## Known gaps

After implementation, review, and exact staging, one fresh quiet rig-W run is
required. The two fixed-cell performance findings from the valid 96-arm run
remain separate and must not be hidden by the longer diagnostic payload.

## Reviewer comments

Pending.
