# otp12-pf1-rigw-probe-record — exact live evidence and bounded interpretation

**Severity**: HIGH — an invalid or overclaimed P1 reading would select the
wrong counterfactual and consume more rig time without a gradeable result.
**Status**: Verified — Grok supplementary review and Claude Fable 5/max
authoritative review accepted the exact candidate without a corrective finding.
**Commit**: `7ecc2f9152fa5a4413ab928fcbdd17c78d6d7c05` (immutable reviewed head).

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
and result, prove pre-ldt-2 static-target orientation parity at eight for both
initiator layouts, provide
the requested two-layout descriptive phase report, and state the narrow
licensed result: then-current-build P1 non-reproduction, failed resolution
check, and no causal hypothesis grade.

## Approach

The complete q evidence directory is imported without changing a byte. The
README separates harness validity, historical static-target orientation
parity, wall-time medians,
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
  target epoch-7 `target_streams=live_streams=8` evidence for both roles,
  treating it as pre-ldt-2 static-policy evidence rather than adaptive tuning.
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

Grok 4.5/high supplied the owner-requested supplementary second eye and
returned schema-valid `accepted`, exact SHAs, and `guard_confirmed=true` with
no comments. Claude Fable 5/max then independently returned schema-valid
`accepted`, exact SHAs, and `guard_confirmed=true`. Claude recomputed the full
inventory, schedules, clocks, raw/exported phase identity, reports, summary
arithmetic, static-target orientation parity, and two-layout phase table, and
found the bounded interpretation exact.

Both reviewers retained fresh additive copies and proved analyzer green,
127/128 red, and exact restored green. Claude's first restore invoked `cp -i`,
blocked on its overwrite prompt, and was interrupted; that pass was not
counted. Claude then issued a separate explicit noninteractive restore, reran
green, and matched all six reports. The detached worktree ended clean at exact
`7ecc2f9`. Records:
`.review/results/otp12-pf1-rigw-probe-record-r1.grok.json` and
`.review/results/otp12-pf1-rigw-probe-record-r1.claude.json`.
