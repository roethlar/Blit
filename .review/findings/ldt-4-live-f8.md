# ldt-4-live-f8 — keep the Windows client launch gate as one file path

**Severity**: MEDIUM — every Windows-client arm fails before client creation
because PowerShell splits the launch-gate expression and path-checks its parent
directory as a file.
**Status**: Closed — fixed, mutation-proved, tactically reviewed clean, and validated by the complete 96-arm rig-W run.
**Branch**: `master` (repo policy forbids agent-created branches)
**Commit**: `c2e12846bcb188f48f1c26a3c0977dbc0a52fa24`

## Evidence

Exact reviewed and staged harness `55fc5d5` passed fixtures, complete start and
runtime gates, Windows runtime preparation, and the f7 exact console-host/daemon
classification in retained session
`ldt4-20260721T210445Z-55fc5d5ff456`. Arm `ldt4-001` completed and retained a
byte-identical 1 GiB transfer in 1387 ms. Arm `ldt4-002` then started its q
daemon but the Windows client wrapper failed before generating any client file:

`registered Windows file is not plain:
D:\blit-test\ldt4-sessions\ldt4-20260721T210445Z-55fc5d5ff456\logs\ldt4-002`

The wrapper's prospective-file array ends with unparenthesized
`$dir + '/client-launch.ok'`. PowerShell evaluates the array operand ending in
`$dir` separately, so `Assert-Ldt4PlainPath ... File` receives the existing
per-arm directory. `ldt4-002`'s Windows log directory is empty, proving the
controller and client were not created.

The session is void: `runs.csv` contains one provisional data row plus its
header and must not be analyzed or graded. Both port 9031 listeners are closed,
no Blit process remains, and the prior active Windows daemon is restored at
SHA-256
`1510d8d04e503967baf250c19cfcd7af4363bc9a22038f68396ea6eb45890512`.

## Predicted observable failure

Every arm whose client runs on Windows fails its prospective-path guard on the
existing per-arm directory before controller script, PID, logs, launch gate,
client process, or timing.

## What

Parenthesize the Windows client launch-gate concatenation so the prospective
file array contains one exact `$dir/client-launch.ok` path.

## Approach

- Change only the final prospective-file array element to
  `($dir + '/client-launch.ok')`.
- Extend the Bash 3.2 structural self-test to require the exact parenthesized
  element and forbid the live-failing unparenthesized form.
- Mutation-prove removing those parentheses makes the self-test fail.

## Files changed

- `scripts/bench_ldt4_rigw.sh` — exact Windows client launch-gate path and
  structural guard.

## Guard proof

- Focused restored green: Bash syntax; 96-arm Bash 3.2 self-test with no SSH;
  all 77 analyzer tests.
- Production mutation: removing only the launch-gate element's parentheses
  made the static Bash self-test fail at the exact Windows client path guard.
  Restoring them returned the focused checks green.
- Full repository gates pass: rustfmt check, strict workspace clippy, and the
  complete workspace test suite.

## Coder dispute

None.

## Known gaps

None for this finding. Exact harness `96a4e3b03caf43ee368efadc779e3324248067f6`
includes the fix and completed all 96 valid arms. Retained evidence is recorded
at `docs/bench/ldt4-rigw-2026-07-21/`.

## Reviewer comments

Tactical Grok 4.5/high review returned clean with no findings for exact range
`f827822..c2e1284`. It independently removed only the production parentheses,
proved the exact path guard red, restored focused green, and left a clean exact
worktree. Record: `.review/results/ldt-4-live-f8-r1.grok-verdict.md`.
