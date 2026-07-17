# ldt-4-live-f3 — distinguish a proven no-launch partial startup in teardown

**Severity**: MEDIUM — teardown falsely reports failure after a startup error
that occurred before any launcher or daemon could exist.
**Status**: Fixed, mutation-proved, and tactically reviewed clean; live retry pending.
**Branch**: `master` (repo policy forbids agent-created branches)
**Commit**: `a39f0c570191d65f197e4ab58eade375ec52e6d6`

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

- `scripts/bench_ldt4_rigw.sh` — partial-start classification and offline
  structural guard.

## Guard proof

- The no-SSH self-test pins one durable `start.cmd` flush before the sole
  process-create call, validates the partial paths before classification, and
  requires the ordered PID, four-marker, and listener refusals before the only
  successful return. It rejects any process enumeration or stop operation in
  that branch.
- Replacing the missing-start condition with false makes the self-test fail
  with `Windows no-launch teardown branch is not unique`. Weakening the exact
  durable flush makes it fail with
  `Windows start command durable flush is not unique`. Exact restoration
  returns `PASS (96 arms, no SSH)` after each mutation.
- The exact current `stop_windows_daemon` function was streamed in memory over
  SSH to q and executed against the retained `ldt4-001` partial state on
  Windows. It returned `LDT4-NO-LAUNCH-TEARDOWN|PASS`; that branch contains no
  registered session or runtime file write.
- Bash syntax, all 75 analyzer tests, formatting, strict workspace clippy,
  documentation, and diff checks pass. One full workspace run had the known
  transient temporary-daemon failure in `test_admin_list_modules`; that exact
  test passed alone and the complete workspace rerun passed without a code
  change.

## Coder dispute

None.

## Known gaps

No completed/timed live arm or transfer datum exists. Tactical Grok review found no
material defect at exact `a39f0c5`; a fresh additive live run cleared the
no-launch teardown fault, then exposed separate generated start-command finding
`ldt-4-live-f4`, fixed and mutation-proved at `d53b5fd`. The owner put
further formal Fable openreviews on hold while that model is out of capacity.

## Reviewer comments

Grok 4.5/high reviewed exact range `5a2265e..a39f0c5`, audited every partial
startup window and the unchanged launched-state ownership path, ran the Bash
3.2 self-test and in-memory PowerShell parser probes, and returned `clean` with
no findings. This is tactical advisory review, not formal acceptance. Record:
`.review/results/ldt-4-live-fixes-r1.grok-verdict.md`.
