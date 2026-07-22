# ldt-4-live-f13-r1-f1 — pin the analyzer admission-horizon shape

**Severity**: LOW — the committed values agree, but analyzer-only shape drift
could pass every pre-rig gate and void the session only after the transfers.
**Status**: Fixed in candidate; tactical re-review pending.
**Branch**: `master` (repo policy forbids agent-created branches)
**Commit**: pending

## Evidence

`scripts/ldt4_rigw_analyze.py` registers horizon as 40 files and
42,949,672,960 bytes. The harness separately pins the same literals. However,
`AnalyzerTests.setUp` replaces `EXPECTED_FIXTURES` with `clear=True`, including
the synthetic horizon value `(2, 3)`, so none of its horizon sessions observes
the production tuple.

Claude Opus 4.8/max changed only the production analyzer tuple to 25 files and
26,843,545,600 bytes. All 86 analyzer tests and the exact harness self-test
remained green, proving the coverage gap. The committed candidate itself still
has matching 40-file/40-GiB values and is not behaviorally defective.

## Predicted observable failure

A future analyzer-only typo or partial revert can pass all local gates. The
harness then stages and transfers the correct 40-file fixture, but final live
analysis rejects `runs.csv` for a fixture total mismatch and marks the session
void after the expensive rig work.

## What

Add one literal assertion for the production analyzer horizon tuple outside
the class that installs the synthetic fixture patch.

## Approach

Put the assertion in `DialPolicyReplayTests`, which does not patch
`EXPECTED_FIXTURES`. Keep the synthetic horizon fixtures unchanged so the full
four-arm analyzer sessions remain fast.

## Files changed

- `scripts/ldt4_rigw_analyze_test.py` — literal production horizon-shape pin.

## Guard proof

- The new test passes with `(40, 42_949_672_960)`.
- Changing only the production analyzer tuple to `(25, 26_843_545_600)` makes
  exactly `test_horizon_fixture_shape_matches_registered_admission_horizon`
  fail; restoring the exact tuple returns all 87 analyzer tests green.
- Rustfmt, strict workspace clippy, the complete workspace suite, and the docs
  gate pass. The first workspace attempt hit an unrelated
  `test_admin_rm_directory` failure; that exact test passed alone and the full
  unchanged workspace rerun passed.

## Coder dispute

None. The finding predicts a concrete late session void and its mutation proved
the existing suite could not detect the drift.

## Known gaps

Claude Opus 4.8/max tactical re-review remains before exact staging.

## Reviewer comments

Reviewer: claude / claude-opus-4-8 / max / tactical advisory

Claude Code 2.1.217 reviewed exact
`75211b3a4725f8ae1952fa9f517cd593943e8b37..af13fdb444c94c29f9260fa710918c338d95dd5e`
in session `ec904253-4a0d-4eb9-b080-071b77fda80c`, returned one Low finding,
and reported `guard_confirmed: true`. The structured record is
`.review/results/ldt-4-live-f13-r1.opus.json`.
