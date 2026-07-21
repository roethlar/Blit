# ldt-4-live-f10 — let fixed q quietness recover from registered load history

**Severity**: MEDIUM — the unchanged q load ceiling can void a healthy session
because its two-minute recovery window expires while the benchmark's one-minute
load history is still decaying between registered arms.
**Status**: Fixed, mutation-proved, and full-gate green; tactical review pending.
**Branch**: `master` (repo policy forbids agent-created branches)
**Commit**: pending

## Evidence

Exact reviewed and staged harness `ef9ef0b` cleared every prior live finding,
including the exact f9 arm-26 teardown boundary, and completed 38 byte-identical
arms in retained session `ldt4-20260721T214439Z-ef9ef0b6f531`. Before pair 4
of the Windows-to-q small-file cell, the runtime quiet gate waited outside all
timed arms for its full 120-second budget, then voided at
`load1=3.53 Spotlight=0.0`.

The immediately preceding accepted gate samples were 1.55 before pair 1, 3.00
before pair 2, and 2.37 before pair 3. A read-only post-failure audit found no
conflicting process, no q or Windows port-9031 listener, no Blit process,
Time Machine stopped, and no material current CPU consumer. q's load1 then
decayed to 1.98 without intervention. This is stale one-minute load history,
not evidence authorizing a higher quietness threshold.

Windows' active daemon was restored to the prior SHA-256
`1510d8d04e503967baf250c19cfcd7af4363bc9a22038f68396ea6eb45890512`;
the tested daemon remains retained at SHA-256
`ae414e649cf64f042f9d2a61639371c57d2fc3107cc426fe5a27a057b6630322`.

The session is void: `runs.csv` contains 38 provisional data rows plus its
header. No analyzer ran, and no row may be analyzed or graded.

## Predicted observable failure

After enough registered arms raise q's one-minute load history, a between-pair
runtime gate can remain above 3.0 for slightly longer than 120 seconds despite
no current contaminating work. The harness voids instead of continuing to wait
outside measured timing for the same fixed bar.

## What

Keep the exact q load1 ≤3.0 and Spotlight ≤10.0 quietness thresholds and the
five-second resampling cadence, but give the fail-closed recovery window five
minutes for load history to decay.

## Approach

- Change only q's quiet-gate recovery deadline from 120 to 300 seconds.
- Preserve every conflicting-process, Time Machine, numeric, load, Spotlight,
  and sample-cadence check.
- Extend the Bash 3.2 structural self-test to require the five-minute bound and
  forbid the live-failing two-minute bound; mutation-prove it red.

## Files changed

- `scripts/bench_ldt4_rigw.sh` — five-minute q quietness recovery and its
  exact structural guard.

## Guard proof

- Focused restored green: Bash syntax; 96-arm Bash 3.2 self-test with no SSH;
  all 77 analyzer tests.
- Production mutation: changing only q's recovery deadline from 300 back to
  120 seconds made the static Bash self-test fail at
  `q quiet gate does not retain five-minute load-history recovery`. Restoring
  300 returned the focused checks green.
- Full repository gates pass: rustfmt check, strict workspace clippy, and the
  complete workspace test suite.

## Coder dispute

None.

## Known gaps

Tactical review, additive staging, and a complete valid live run remain.

## Reviewer comments

This finding came from the attached live launch, not a reviewer candidate.
