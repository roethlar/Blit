# ldt-4-r1-f6 — guard every independent Python replay reason branch

**Severity**: LOW — analyzer replay drift in untested decision branches would
first surface by refusing or misreading an expensive live session.
**Status**: Fixed and mutation-proved; neutral whole-change re-review pending.
**Branch**: `master` (repo policy forbids agent-created branches)
**Commit**: review-fix commit pending final record

## Evidence

`scripts/ldt4_rigw_analyze.py:1370-1390` independently transcribes production
`hysteresis`, `cooldown`, `sustain`, and `bound` selection. The original 72
analyzer tests exercised emitted `cheap-up`, `add`, `rebaseline`, `cheap-down`,
`remove`, and `idle` paths, but contained no positive `cooldown`, `sustain`, or
`bound` event. Rust tests already guard production's complete ten-reason
taxonomy; the missing protection was specifically in the Python replay.

## Predicted observable failure

A future Python transcription regression in those branches could still pass
the analyzer suite, then reject valid runtime evidence or accept a wrong reason
sequence. The first detection could be after the complete 96-arm hardware run.

## What

Add one fast table-driven unit guard that drives the independent replay through
`hysteresis`, `cooldown`, `sustain`, and a ceiling `bound`. It also requires
those four plus the fixture-guarded reasons to equal the analyzer's complete
`SAMPLE_REASONS` set.

## Approach

The test constructs only the replay state needed for each decision and checks
both the exact returned reason and absence of a resize proposal. It does not
duplicate the 96-arm synthetic evidence setup and adds no real-emitter fixture
as a pre-hardware requirement.

## Files changed

- `scripts/ldt4_rigw_analyze_test.py` — focused replay reason table and
  taxonomy-closure assertion.

## Guard proof

- Changing the production replay's `cooldown` result to `sustain` makes the new
  focused test fail with an exact expected/observed reason error.
- Exact restoration passes the focused guard and all 73 analyzer tests.

## Coder dispute

The broader suggestion that all synthetic tests must be replaced by a live
emitter fixture is not admitted before the first live run. The concrete Python
branch gap is admitted and closed.

## Known gaps

The first accepted hardware trace can seed a separate real-output regression
fixture after its immutable evidence has passed review.

## Reviewer comments

Claude Fable 5/max returned the candidate over exact
`e41b871..0e48721` with `guard_confirmed=true`. Intake narrowed it to the three
unguarded Python branches plus taxonomy closure. Final fixed-SHA whole-change
re-review is pending.
