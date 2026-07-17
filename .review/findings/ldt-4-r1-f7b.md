# ldt-4-r1-f7b — require exact blocked-ratio recomputation

**Severity**: LOW — the old absolute tolerance allowed a trace value to cross
an exact policy threshold while still matching its raw counters.
**Status**: Fixed and mutation-proved; neutral whole-change re-review pending.
**Branch**: `master` (repo policy forbids agent-created branches)
**Commit**: `f470b06`

## Evidence

`scripts/ldt4_rigw_analyze.py:1503-1513` recomputes `blocked_ratio` from
`sample_blocked_ns / (sample_elapsed_ns * sample_streams)` but previously
accepted an absolute difference up to `1e-12`. Policy replay then used the
emitted value. `math.nextafter(0.05, 0.0)` is only about `6.94e-18` below raw
counter ratio `0.05`, yet it changes the strict `< 0.05` step-up decision.

## Predicted observable failure

A corrupted or edited trace can carry counters exactly on a policy threshold
and an emitted ratio one floating-point step across it. The old integrity check
accepts the mismatch, and replay follows the emitted side of the branch rather
than the counters. This weakens immutable evidence validation even though the
serde producer itself emits consistent values.

## What

Require the serialized ratio to equal the analyzer's recomputation exactly.
Both producer and analyzer use the same integer numerator/denominator and IEEE
division, and JSON round-trips the producer `f64`; no evidence-specific
tolerance is needed.

## Approach

Replace the `math.isclose(..., abs_tol=1e-12)` comparison with direct floating-
point equality. Keep all finiteness, raw-counter, range-clamp, and policy replay
checks unchanged.

## Files changed

- `scripts/ldt4_rigw_analyze.py` — exact ratio/counter equality.
- `scripts/ldt4_rigw_analyze_test.py` — one-ULP threshold-crossing guard.

## Guard proof

- The full analyzer guard makes counters compute exactly `0.05`, emits the
  next representable float below it, and requires a ratio mismatch refusal.
- Restoring the old `1e-12` tolerance makes the new test fail because no error
  is raised. Exact restoration passes the focused guard and all 75 analyzer
  tests.

## Coder dispute

None.

## Known gaps

None. If a future producer changes the arithmetic order, that wire-neutral
observer contract must be changed and reviewed explicitly rather than hidden by
a tolerance that can cross policy thresholds.

## Reviewer comments

Claude Fable 5/max returned the bundled integrity candidate over exact
`e41b871..0e48721` with `guard_confirmed=true`. Intake split numeric tolerance
from duplicate-key parsing to preserve one finding per commit. Final fixed-SHA
whole-change re-review is pending.
