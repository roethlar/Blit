# ldt-4-live-f8 — keep the Windows client launch gate as one file path

**Severity**: MEDIUM — every Windows-client arm fails before client creation
because PowerShell splits the launch-gate expression and path-checks its parent
directory as a file.
**Status**: Accepted; fix and guard pending.
**Branch**: `master` (repo policy forbids agent-created branches)
**Commit**: pending

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

- `scripts/bench_ldt4_rigw.sh` — pending exact Windows client launch-gate path.

## Guard proof

Pending.

## Coder dispute

None.

## Known gaps

Fix, focused/full gates, tactical review, additive staging, and a complete
valid live run remain.

## Reviewer comments

This finding came from the attached live launch, not a reviewer candidate.
