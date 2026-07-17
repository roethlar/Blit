# ldt-4-live-f3 — distinguish a proven no-launch partial startup in teardown

**Severity**: MEDIUM — teardown falsely reports failure after a startup error
that occurred before any launcher or daemon could exist.
**Status**: Open; next one-finding repair under the active ldt-4 plan.
**Branch**: `master` (repo policy forbids agent-created branches)
**Commit**: pending

## Evidence

In retained live session `ldt4-20260717T052509Z-5a2265e202a4`, Windows daemon
startup created and flushed `daemon.toml`, then failed while creating its log
files. It never created or flushed `start.cmd`, never called
`Invoke-CimMethod`, and wrote no launcher PID, daemon PID, launch marker, or
identity record.

`stop_windows_daemon` nevertheless canonicalized and required `start.cmd`
before it could classify the partial state, so the EXIT trap reported
`Windows exact daemon teardown failed`. Independent read-only checks afterward
proved no matching process and no port 9031 listener. Runtime restoration
still completed exactly: the active daemon matched the durable pre-swap hash,
and the tested daemon remained retained.

## Predicted observable failure

Any startup exception before the fully flushed `start.cmd` boundary produces
a second, misleading teardown error even when no process was launched. That
obscures whether cleanup genuinely failed and prevents the harness from
proving the endpoint is safe for the next additive run.

## What

Make `stop_windows_daemon` recognize only the exact proven no-launch partial
state while preserving all existing exact PID, command, executable, parent,
and unique-match checks once launch may have occurred.

## Approach

Treat fully flushed `start.cmd` as the durable launch boundary because startup
creates and flushes it before `Invoke-CimMethod`. If it is absent, return
`STOPPED` only when both PID inputs are zero, all four post-launch artifacts
(`launcher.pid`, `daemon.pid`, `launch.ok`, and `daemon-identity.txt`) are
absent, and port 9031 is closed. Contradictory evidence or a listener fails
closed without enumerating or stopping any process. If `start.cmd` exists, the
current exact-ownership teardown path remains unchanged.

## Files changed

- `scripts/bench_ldt4_rigw.sh` — pending partial-start classification and
  offline structural guard.

## Guard proof

Pending. The guard must pin the flushed-start-file-before-process-create
ordering and the no-launch branch's PID/artifact/port refusals. Temporarily
disabling that branch and breaking its startup-order premise must each turn the
no-SSH self-test red; exact restoration must return green.

## Coder dispute

None.

## Known gaps

No live arm or transfer datum exists. Final Fable review and a fresh additive
live run wait for this repair.

## Reviewer comments

This finding came from the attached live launch, not a reviewer candidate.
