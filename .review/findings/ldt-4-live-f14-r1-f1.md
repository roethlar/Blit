# ldt-4-live-f14-r1-f1 — reject SOURCE action relaxation

**Severity**: LOW — exact committed behavior is correct, but deleting both
SOURCE action comparisons still passed every analyzer test.
**Status**: Candidate implemented; tactical re-review pending.
**Branch**: `master` (repo policy forbids agent-created branches)
**Commit**: pending

## Evidence

Claude Opus 4.8/max independently confirmed that the f14 candidate derives
only the exact protobuf SOURCE action strings and that its shorthand mutation
makes 27 tests fail. It then replaced both new action comparisons with constant
`False`, deleting SOURCE action enforcement while leaving fixture bytes
unchanged. All 88 tests still passed.

The fixture spelling assertion proves synthetic evidence matches production,
but no negative test gives the analyzer a wrong SOURCE action. A later
relaxation or normalization could therefore pass pre-rig gates even though the
exact action contract is the reason f14 exists.

## Predicted observable failure

An analyzer edit that accepts dial shorthand or ignores SOURCE action values
can pass all local tests and then accept malformed control-lane evidence during
a hardware run. The suite would prove only fixture spelling, not enforcement.

## What

Add negative valid-session guards for both SOURCE action comparison sites.
Mutate one epoch-1 `resize_proposed` action and one epoch-1 `source_settled`
action independently from `DATA_PLANE_RESIZE_OP_ADD` to shorthand `ADD`, then
require the exact control-lane error in each case.

## Approach

- Reuse the existing `SyntheticSession` read/mutate/write pattern.
- Keep production code and fixture spelling unchanged.
- Mutation-prove the tests by deleting each relevant analyzer action
  comparison, observing its dedicated negative guard fail, then restore and
  run all gates.

## Files expected

- `scripts/ldt4_rigw_analyze_test.py` — two rejection guards.

## Guard proof

- At exact code bytes, all 90 analyzer tests pass.
- Replacing only the `resize_proposed` SOURCE action comparison with constant
  `False` makes exactly
  `test_resize_proposed_rejects_dial_shorthand_action` fail because no
  `AnalysisError` is raised; restoration returns green.
- Replacing only the `source_settled` SOURCE action comparison with constant
  `False` makes exactly
  `test_source_settled_rejects_dial_shorthand_action` fail for the same reason;
  restoration returns green.
- The production analyzer and synthetic fixture remain byte-identical to the
  reviewed f14 candidate; this slice adds rejection coverage only.

## Coder dispute

None. The reviewer demonstrated a concrete fail-open mutation with a fully
green suite.

## Known gaps

Full gates and tactical re-review remain. Exact restaging and live execution
belong to parent `ldt-4-live-f14`.

## Reviewer comments

Reviewer: claude / claude-opus-4-8 / max / tactical advisory

Session `7a84f4a9-dab8-496a-a509-f2a28880cce2` reviewed exact resolved range
`679253c7e2f12f4e313f0bfc26d2d044ce377e61..8385d2334b155cd1044fb9c11fb3a33f2e8078e0`,
returned this one Low finding, and reported `guard_confirmed: true` with a clean
review worktree. The originally supplied long base SHA was invalid; short
`679253c` resolved to the exact base recorded here.
