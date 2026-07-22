# ldt-4-live-f15 — reverse horizon order before policy attribution

**Severity**: MEDIUM — the valid horizon evidence repeats a large
Windows→q decision split, but its fixed arm order aliases initiator role with
cold/warm Windows source state. Attributing that split to socket topology now
could tune the one shared controller against the wrong cause.
**Status**: Fixed, mutation-proved, and full-gate green; tactical review,
staging, and live execution remain.
**Branch**: `master` (repo policy forbids agent-created branches)
**Commit**: Pending

## Evidence

Exact reviewed harness/analyzer `7050a2997ac597a1b8982e7f4acbfa0b12572340`
completed fresh retained session `ldt4-20260722T022350Z-7050a2997ac5`. The
session is structurally valid, Windows restored normally and byte-for-byte,
and exact independently reproduced analysis lives at
`docs/bench/ldt4-rigw-horizon-2026-07-22/`. Its final inventory SHA-256 is
`c6ed0cf96b9d888d0611d9264e6be4bd3e67433afbd604e74b2ca07cf89a031a`.

q→Windows was role-invariant: SOURCE-init and DESTINATION-init both accepted
REMOVE `4→3→2→1` and completed in 47,661/47,740 ms. Windows→q split:
DESTINATION-init ran first, accepted ADD `4→5→6→7→8→9→10`, and completed in
45,282 ms; SOURCE-init ran second, accepted REMOVE `4→3→2→1`, and completed
in 34,710 ms. The earlier void f13 session used the same order and repeated
those two durations within 7/10 ms while reaching ADD 4→9 and REMOVE 4→1.

That repetition is not an independent role proof. The registered schedule
writes 80 GiB to the Windows destination before the first Windows-source arm,
then reads the same 40-file/40-GiB Windows source in two adjacent arms. The
first Windows-source sample (DESTINATION-init, first) transferred 459,276,288
bytes with blocked ratio 0.0588199; the second (SOURCE-init, second) transferred
618,659,840 bytes with blocked ratio 0.968834. The current policy grows from a
sustained low blocked ratio and shrinks from a sustained high one, so a
cold/warm source-production difference can explain the opposite decisions
without any role-dependent controller branch.

## Predicted observable failure

Repeating the current order can reproduce the same decision split indefinitely
while providing no new causal information. Changing controller constants from
that evidence could reward cold-source admission behavior or punish warm-source
blocking, then mis-tune both socket layouts because production has one shared
SOURCE policy.

## What

Add one separate `horizon_order` four-arm supplement. Reuse the exact validated
40-file/40-GiB sources, endpoints, volumes, controller code, and safety gates,
but create fresh additive destination/session roots and reverse both pair
orders exactly:

1. q→Windows: DESTINATION-init, then SOURCE-init.
2. Windows→q: SOURCE-init, then DESTINATION-init.

Bind the supplement to the exact valid horizon evidence path, session, and
inventory digest above in addition to the existing fixed-parent and sustained
predecessor bindings. Do not change controller policy or use these diagnostic
durations to regrade the fixed-cell performance findings.

## Approach

- Register `horizon_order` as a distinct analyzer matrix with an exact
  four-line schedule, marker, source/destination paths, volume bookends,
  capacity accounting, artifact identity, payload manifests, control traces,
  runtime restoration, and additive-retention contract. The current `fixed`,
  `sustained`, and `horizon` matrices remain accepted inputs unchanged.
- Add an exact `reference-evidence.txt` binding for horizon session
  `ldt4-20260722T022350Z-7050a2997ac5`, path
  `docs/bench/ldt4-rigw-horizon-2026-07-22`, and inventory SHA-256
  `c6ed0cf96b9d888d0611d9264e6be4bd3e67433afbd604e74b2ca07cf89a031a`.
  The analyzer fails closed if any field, matrix marker, or schedule identity
  differs.
- Classify each arm's accepted membership operation family as ADD-only,
  REMOVE-only, hold, or mixed while retaining its full epoch/target sequence.
  q→Windows is the control and must remain REMOVE-only with material transition
  parity; otherwise the causal result is `INCONCLUSIVE_CONTROL_CHANGED`.
- When the control is stable, classify Windows→q as `ORDER_TRACKING` if the new
  first arm (SOURCE-init) is ADD-only and the new second arm
  (DESTINATION-init) is REMOVE-only. Classify it as `ROLE_TRACKING` if
  SOURCE-init remains REMOVE-only and DESTINATION-init remains ADD-only despite
  reversal. Every hold, mixed, same-polarity, malformed, or otherwise partial
  outcome is `INCONCLUSIVE`, never silently forced into either explanation.
- Export the classification, exact transitions, reasons, samples, timings, and
  comparison to the bound valid horizon evidence. Structural validity is
  separate from a causal classification; no outcome is ldt-4 acceptance or a
  policy-change authorization by itself.
- Add synthetic positive cases for order tracking, role tracking, changed
  control, and inconclusive Windows→q evidence. Mutation-prove the exact
  reversed schedule and reference digest, plus at least one classifier branch.
  Reanalyze the retained fixed, sustained, and horizon evidence and require all
  six reports for each to remain byte-for-byte unchanged.
- Run native Bash 3.2 syntax/self-test, all analyzer tests, repository format,
  strict clippy, complete workspace tests, and docs gate. Use Claude Opus
  4.8/max for a tactical fixed-SHA review before exact additive staging and one
  live run; formal Fable remains held.

## Files expected

- `scripts/bench_ldt4_rigw.sh` — exact reversed supplement schedule, evidence
  binding, additive roots, gates, and self-test.
- `scripts/ldt4_rigw_analyze.py` — `horizon_order` validation and causal
  classification.
- `scripts/ldt4_rigw_analyze_test.py` — structural, classifier, mutation, and
  backward-compatibility guards.
- `docs/plan/LIVE_DIAL_TUNING.md`, `docs/STATE.md`, `REVIEW.md`, and
  `DEVLOG.md` — durable status and eventual evidence record.

## Guard proof

Native Bash 3.2.57 syntax and the four-arm no-SSH harness self-test pass. All
98 analyzer tests pass. The synthetic `horizon_order` cases produce the four
registered outcomes: `ORDER_TRACKING`, `ROLE_TRACKING`,
`INCONCLUSIVE_CONTROL_CHANGED`, and `INCONCLUSIVE`. The exact valid horizon
inventory and both exact source-manifest bytes are bound into the harness,
measurement marker, analyzer, summary, and negative guards.

Four isolated production mutations proved the new guards. Replacing
the reversed first-role tuple with the old horizon order made the literal
registration guard fail and the valid order case error on schedule identity.
Changing one nibble of the reference inventory digest made its literal guard
fail. Mislabeling the ADD-first classifier branch as role tracking made only
the order-tracking case fail while the genuine role-tracking case stayed green.
Disabling the exact source-manifest comparison made the dedicated guard fail
because a self-consistent alternate source then passed analysis. Exact
restoration returned all 98 tests and the harness self-test to green.

Fresh pre-analysis copies of the retained fixed, sustained, and horizon inputs
were reanalyzed with the changed analyzer. For each matrix, `input-files.csv`,
`arms.csv`, `dial-samples.csv`, `pairs.csv`, `summary.json`, and `summary.md`
matched the committed report byte-for-byte.

Repository format, strict workspace clippy, the complete workspace suite, and
the docs gate pass on the restored candidate tree.

## Coder dispute

None.

## Known gaps

Tactical Opus 4.8/max review, exact additive staging, live execution, evidence
retention, and independent analysis reproduction remain. No controller policy
change is authorized by this finding.

## Reviewer comments

Pending tactical review.
