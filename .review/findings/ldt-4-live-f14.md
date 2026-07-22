# ldt-4-live-f14 — validate exact SOURCE control action spelling

**Severity**: MEDIUM — all four horizon arms completed and retained, but the
analyzer refused the first real resize operation because its synthetic action
spelling did not match production evidence.
**Status**: Candidate implemented; tactical review pending.
**Branch**: `master` (repo policy forbids agent-created branches)
**Commit**: pending

## Evidence

Exact reviewed/staged harness `a0c3e3f18afd5528c6f636ee54708f4d8d5127e9`
completed all four 40 GiB arms in retained session
`ldt4-20260722T013314Z-a0c3e3f18afd`. Durations were 47,061, 48,077,
45,275, and 34,720 ms. Every payload was promoted to its unique retained path,
`runs.csv` has all four valid rows, `MEASUREMENTS-COMPLETE` exists, Windows
restored normally and byte-for-byte, and both daemon ports are closed.

The horizon successfully exercised the tuner. The q→Windows pair each emitted
41 samples and identical accepted REMOVE transitions `4→3→2→1`. The first
Windows→q arm emitted 33 samples and accepted ADD `4→5→6→7→8→9`; the second
emitted 31 samples and accepted REMOVE `4→3→2→1`. The latter pair is a material
transition mismatch that the analyzer must report, not hide.

Instead, final analysis stopped at the first arm with
`resize_proposed values disagree with epoch 1`, and the harness correctly
created `SESSION-VOID`. The arm-1 observer events carry
`action="DATA_PLANE_RESIZE_OP_REMOVE"` in `resize_proposed`,
`resize_send_begin`, `resize_sent`, and `source_settled`. ADD evidence carries
`DATA_PLANE_RESIZE_OP_ADD`. `_validate_control_lane` compares these fields to
the dial observer's shorthand `ADD`/`REMOVE`. Synthetic traces also emit that
shorthand in SOURCE control events, so all prior tests encoded the analyzer's
mistake. The earlier complete sessions had no accepted operation and could not
exercise this comparison.

## Predicted observable failure

Every structurally valid real session with any accepted ADD or REMOVE reaches
the SOURCE control-lane check and is refused at its first resize epoch, even
though the exact production event is well formed. This makes the adaptive
hardware acceptance analyzer incapable of grading the behavior it exists to
measure.

## What

Keep dial observer operations as `ADD`/`REMOVE`, but require the corresponding
SOURCE control event action to be the exact protobuf enum debug spelling
`DATA_PLANE_RESIZE_OP_ADD` or `DATA_PLANE_RESIZE_OP_REMOVE`. Make synthetic
SOURCE traces use those production strings. Do not relax or normalize arbitrary
action values.

## Approach

- Derive one exact expected SOURCE control action from each already validated
  dial operation and use it for `resize_proposed`, `resize_send_begin`,
  `resize_sent`, and `source_settled`.
- Change only synthetic SOURCE control events to the exact production spelling;
  keep dial samples/pending/settlement and DESTINATION custom actions unchanged.
- Add a focused assertion that the synthetic control lane contains the exact
  protobuf strings and no SOURCE shorthand.
- Mutation-prove that reverting the analyzer comparison to shorthand turns the
  focused valid-session guard red, restore it, and run full repository gates.
- Reanalyze a diagnostic copy of the retained void session without treating it
  as acceptance, so any next analyzer defect is found before another rig run.

## Files expected

- `scripts/ldt4_rigw_analyze.py` — exact SOURCE control action comparison.
- `scripts/ldt4_rigw_analyze_test.py` — production-shaped synthetic SOURCE
  events and spelling guard.

## Guard proof

All 88 analyzer tests pass with production-shaped synthetic SOURCE events.
Temporarily replacing the analyzer's exact enum-derived SOURCE action with the
dial shorthand made 27 tests fail at the first SOURCE resize event; restoring
the exact spelling returned all 88 tests to green. A focused assertion also
pins SOURCE events to the two exact protobuf debug strings while independently
pinning `dial_pending`/`dial_settlement` to `ADD`/`REMOVE`.

An additive copy of retained void session
`ldt4-20260722T013314Z-a0c3e3f18afd` was reanalyzed without `SESSION-VOID` as
diagnostic evidence only. The analyzer now validates all four arms and returns
the expected `REVIEW_REQUIRED`: arm review 3, decision review 1, performance
review 0. q→Windows transitions match REMOVE 4→1; Windows→q remains the
material ADD 4→9 versus REMOVE 4→1 mismatch. The original void session and all
endpoint payloads/evidence remain unchanged.

## Coder dispute

None.

## Known gaps

Full repository gates, tactical Opus review, exact restaging, and one fresh
additive live rerun remain. The void session and all endpoint payloads/evidence
stay retained unchanged.

## Reviewer comments

Pending.
