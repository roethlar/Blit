# ldt-4-live-f2 — keep Windows daemon log paths as separate array elements

**Severity**: MEDIUM — every Windows-responder arm fails before daemon launch,
so the registered matrix cannot produce evidence.
**Status**: Fixed, mutation-proved, and tactically reviewed clean; live retry pending.
**Branch**: `master` (repo policy forbids agent-created branches)
**Commit**: `b9b8080c5463af894547aacad1395f86116ff657`

## Evidence

The exact reviewed harness `5a2265e` reached arm `ldt4-001` in retained live
session `ldt4-20260717T052509Z-5a2265e202a4`, then PowerShell rejected a
filename containing both intended log paths separated by a space. In
PowerShell, `@(\$dir + '/daemon.out',\$dir + '/daemon.err')` parses as one
addition expression and produces one string, not a two-element array.

No timing row was accepted. q and Windows port 9031 were closed afterward,
no session process remained, and Windows' active daemon hash exactly matched
the durable pre-swap hash. The tested daemon and all session evidence remain
retained.

## Predicted observable failure

Every arm with Windows as responder stops while creating its daemon log files.
Only `daemon.toml` exists; no launcher command or daemon process is created.
The session fails closed before a transfer can begin.

## What

Make each concatenated log path an explicit PowerShell array element and pin
that exact generated source form in the no-SSH harness self-test.

## Approach

The startup loop now uses
`@((\$dir + '/daemon.out'), (\$dir + '/daemon.err'))`. Parenthesizing each
concatenation makes PowerShell produce the two registered paths independently.
The static safety guard requires exactly that loop inside
`start_windows_daemon`.

## Files changed

- `scripts/bench_ldt4_rigw.sh` — correct the generated PowerShell array and
  guard its exact form.

## Guard proof

- An in-memory PowerShell probe reproduced the live behavior: the old
  expression had count 1 and one space-joined value; the corrected expression
  had count 2 and the exact two paths.
- Restoring only the live-failing loop made the 96-arm no-SSH self-test fail
  with `Windows daemon log paths are not two explicit array elements`.
  Restoring the fix returned `PASS (96 arms, no SSH)`.
- Bash syntax, all 75 analyzer tests, formatting, strict workspace clippy, the
  full workspace suite, documentation checks, and diff checks pass.

## Coder dispute

None.

## Known gaps

The same failed launch exposed separate partial-start teardown finding
`ldt-4-live-f3`, now fixed and mutation-proved at `a39f0c5`. Tactical Grok
review found no material defect at that exact head. A fresh live run remains;
further formal Fable openreviews are on owner-directed hold while that model is
out of capacity.

## Reviewer comments

Grok 4.5/high reviewed exact range `5a2265e..a39f0c5`, reproduced the old and
fixed PowerShell array behavior in memory, ran the complete Bash 3.2 no-SSH
self-test, and returned `clean` with no findings. This is tactical advisory
review, not formal acceptance. Record:
`.review/results/ldt-4-live-fixes-r1.grok-verdict.md`.
