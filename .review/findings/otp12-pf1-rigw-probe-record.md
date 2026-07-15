# otp12-pf1-rigw-probe-record — exact live evidence and bounded interpretation

**Severity**: HIGH — an invalid or overclaimed P1 reading would select the
wrong counterfactual and consume more rig time without a gradeable result.
**Status**: In progress — candidate record complete; Claude Fable 5/max review pending.
**Commit**: supplied as the immutable reviewed head at dispatch.

## Evidence

Exact registered session `20260715T211759Z.30531` at reviewed build
`8e019ef5e948b94a7aca7cb3a8d0be41204742af` produced a sole exact
`SESSION-COMPLETE` marker, 128 valid/drained/exit-zero arms, and an analyzer
PASS. The complete 290-file payload is copied byte-for-byte under
`docs/bench/otp12-pf1-rigw-2026-07-15/`; its pre-README inventory digest is
`1e8d815c74761f34f247eeccc931cccc6d0e69ea73baed68accd45b97d86e51f`.

## Predicted observable failure

Treating the point medians as a causal recovery would report P1 fixed even
though the registered `N_resolution=329 ms` exceeds the historical gap and
the current target baseline is reversed/invariance-passing. It would also
ignore a failing gRPC control and violate the active plan's rule that a
counterfactual on a rig without the finding proves nothing.

## What

Preserve the exact accepted evidence, independently recompute its inventory
and result, prove live eight-stream parity for both initiator layouts, provide
the requested two-layout descriptive phase report, and state the narrow
licensed result: current-build P1 non-reproduction, failed resolution check,
and no causal hypothesis grade.

## Approach

The complete q evidence directory is imported without changing a byte. The
README separates harness validity, live stream parity, wall-time medians,
resolution, endpoint-local phase spans, and remaining pf-1 gates. Current
state and review records advance only to a reviewed probe record; they do not
amend the preregistered plan or declare P1 fixed.

## Files changed

- `docs/bench/otp12-pf1-rigw-2026-07-15/` — exact 290-file evidence payload
  plus its bounded human-readable interpretation.
- `.review/findings/otp12-pf1-rigw-harness.md` — record the completed live gate.
- `.review/findings/otp12-pf1-rigw-probe-record.md` — review contract.
- `REVIEW.md`, `docs/STATE.md`, `DEVLOG.md` — current status and next gate.

## Guard proof

- Independently hash the 290 imported files excluding only the new README and
  require the recorded inventory digest.
- Re-run `scripts/otp12pf_rigw_analyze.py` against a disposable copy and
  require byte-identical generated reports plus analyzer acceptance. Removing
  one `runs.csv` data row from that copy must fail closed at 127/128; exact
  restoration must return acceptance.
- Independently recompute all eight summary rows, the target observer bias and
  resolution, control ratios, clock-sample selections, phase inventories, and
  target epoch-7 `target_streams=live_streams=8` evidence for both roles.
- Review every interpretive sentence against
  `docs/plan/OTP12_PERF_FINDINGS.md`, especially the positive-gap,
  counterfactual, resolution, and hard-gate requirements.

## Known gaps

- No P1 wall-time counterfactual exists, so no causal grade is possible.
- The small-fixture/P2 probes and pinned `0f922de` historical control remain.
- A new P1 experiment needs an owner-approved plan amendment before more rig
  time; this record does not supply or approve that amendment.
- The generated report's preserved prose typo and q's nonfatal recents
  warnings do not affect any measured or validated field.

## Reviewer comments

Pending Claude Fable 5/max review of the immutable candidate.
