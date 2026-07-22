# ldt-4-live-f12 — add a sustained controller-exercise supplement

**Severity**: MEDIUM — the valid rig-W matrix completes before the live
controller can change membership, so ldt-4 cannot prove adaptive ADD/REMOVE
or role invariance under an actual transition.
**Status**: Superseded by `ldt-4-live-f13` after a structurally valid live run proved the five-file workload queued before the first tuner tick.
**Branch**: `master` (repo policy forbids agent-created branches)
**Commit**: `04e80082e12ce9836eda43afc70fb3b2d0eb07c9`

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

Keep the original 96-arm fixed-fixture evidence and its performance grading
unchanged. Run a separate four-arm supplement, bound to that valid evidence's
exact session and inventory digest, with one adjacent initiator-layout pair in
each physical byte direction using a dedicated 5 GiB sustained fixture. Require
every supplemental arm to accept at least one ADD above the four-stream floor
and require each role pair to agree on its accepted membership-transition
sequence.

## Approach

- Additively stage `src_sustained` on both endpoints as five distinct 1 GiB
  files copied from the already validated canonical large fixture. Verify the
  exact five-file/5 GiB manifests are byte-identical before any arm.
- Bind the supplement to the retained 96-arm session, evidence path, and exact
  copied-payload inventory digest. Its own schedule contains only four rows:
  one adjacent source-init/destination-init pair for q→Windows and one
  counter-ordered adjacent pair for Windows→q.
- Before staging or running, prove enough q and Windows free space remains for
  the new stable source, all four retained destinations, and the existing
  33,000,000,000-byte floor. Do not delete or overwrite prior evidence,
  fixtures, or retained payloads.
- Extend the analyzer with an exact sustained-supplement mode covering its
  four-arm schedule, parent binding, fixture, runtime-gate, completion, and
  inventory contracts. The sustained verdict requires an accepted ADD in all
  four arms and exact accepted-operation/target parity within each
  initiator-layout pair. Reason-only trailing-sample timing differences remain
  exported but do not override matching accepted membership transitions.
- Add structural and analyzer tests that fail if the sustained rows disappear,
  the fixture is shortened, the parent binding changes, or the required
  transition/parity verdict is weakened. Mutation-prove the new guard red,
  restore it, then run the full repository gates.

## Files changed

- `scripts/bench_ldt4_rigw.sh` — additive sustained fixture, registered four
  diagnostic arms, capacity gates, and structural guards.
- `scripts/ldt4_rigw_analyze.py` — exact four-arm supplemental contract and
  sustained transition/parity verdict separate from fixed-cell performance.
- `scripts/ldt4_rigw_analyze_test.py` — valid sustained fixtures/traces and
  rejection tests for absent transitions or pair mismatch.

## Guard proof

- Bash syntax and the four-arm no-SSH structural self-test pass.
- All 81 analyzer tests pass, including a valid supplemental matrix, exact
  parent binding, and missing-ADD/pair-mismatch review cases.
- Mutation 1: replacing the production accepted-ADD requirement with
  `if False` made
  `test_sustained_arm_without_add_and_pair_mismatch_require_review` fail
  because `arm_review_count` fell from 1 to 0. Restoring the requirement
  returned focused green.
- Mutation 2: replacing the production material transition comparison with an
  empty list made the same test fail because `decision_review_count` fell from
  1 to 0. Restoring the comparison returned focused green.
- Full repository gates pass: rustfmt check, strict workspace clippy, complete
  workspace tests, and `scripts/agent/check-docs.sh`.

## Coder dispute

None.

## Known gaps

The structurally valid live supplement did not satisfy this finding's adaptive
observable: all four arms completed, but every arm had zero tuner samples and
no accepted ADD. A new finding must keep SOURCE payload admission backpressured
across tuner ticks without exceeding q's additive retained-space floor. The two
fixed-cell performance findings from the valid 96-arm run stay separate and
cannot be hidden by a diagnostic payload.

## Reviewer comments

Tactical Grok 4.5/high review over exact
`12ae72c4b6a5f1c33803213c9c74dba12692a937..04e80082e12ce9836eda43afc70fb3b2d0eb07c9`
returned `clean`, no findings, with `guard_confirmed: true`. The first response
was discarded because its claimed tool work was absent from the exported
transcript; the same session then resumed, executed the checks, turned the
accepted-ADD guard red with a production mutation, restored exact bytes, and
finished clean. The primary agent independently rechecked exact identity, Bash
syntax, the four-arm self-test, and all 81 analyzer tests. Record:
`.review/results/ldt-4-live-f12-r1.grok-verdict.md`. This is tactical advisory
review, not formal `openreview` acceptance.

Exact additive staging is complete. Complete-history bundle
`/Users/michael/blit-ldt4-stage-04e8008.bundle` is retained on both Macs with
matching SHA-256
`f8c2e931576c6abf299892afa9ee28c11d80338853c2614248388aa8c6c081f8`.
New q checkout `/Users/michael/Dev/blit_v2_harness_04e8008` is clean and
detached at exact `04e80082e12ce9836eda43afc70fb3b2d0eb07c9`, has 1,974
commits and no replacement refs, and passes native Bash 3.2 syntax, the
four-arm no-SSH self-test, and all 81 analyzer tests. The bundle also carries
the later review-record commit, but executable code remains pinned to the
reviewed candidate.

Live session `ldt4-20260722T001611Z-04e80082e12c` completed all four arms with
no session void and normal byte-exact Windows restoration. Exact and
independently reproduced analysis is `REVIEW_REQUIRED`: arm review 4, decision
review 0, performance review 0. Every arm stayed at floor = peak = final = 4
with zero samples. SOURCE received terminal demand in 3.1–5.2 ms and sealed
membership in 3.3–5.4 ms, long before its 4.3–20.6 second data-plane drain
completed; five files were fully queued before the first 500 ms tuner tick.
Retained evidence: `docs/bench/ldt4-rigw-sustained-2026-07-22/`.
