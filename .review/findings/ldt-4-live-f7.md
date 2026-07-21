# ldt-4-live-f7 — recognize Windows console host without weakening daemon identity

**Severity**: MEDIUM — every Windows-responder arm fails before its client can
launch because the startup gate mistakes the platform console host for a
second daemon child.
**Status**: Accepted; fix and guard pending.
**Branch**: `master` (repo policy forbids agent-created branches)
**Commit**: pending

## Evidence

Exact reviewed and staged harness `21fe468` passed fixtures, the complete start
environment gate (including stable q identity `Q/Q`), runtime quietness, and
Windows runtime preparation in retained session
`ldt4-20260721T204038Z-21fe468af129`. First arm `ldt4-001` launched the exact
Windows daemon, which logged that it was listening on port 9031, then the
harness failed closed on `daemon child count=2` before any client or transfer.
`runs.csv` contains only its header and `runtime-gates.csv` contains the one
pre-arm gate.

An additive isolated diagnostic retained at
`D:/blit-test/ldt4-diagnostics/f7-20260721T204302Z` reproduced the exact process
shape on an unused port. At 250, 500, 1000, and 2000 ms the launcher had exactly
two direct children: one system
`C:/Windows/System32/conhost.exe` and one exact staged `blit-daemon.exe` with
the registered config command. Identity-scoped diagnostic teardown left zero
children and closed its port.

The live harness teardown also closed both port 9031 listeners, left no Blit
process, and restored the prior active Windows daemon byte-for-byte at SHA-256
`1510d8d04e503967baf250c19cfcd7af4363bc9a22038f68396ea6eb45890512`.

## Predicted observable failure

Every Windows-responder arm starts the correct daemon and console-host sidecar,
then voids on the raw-child count before PID identity evidence, daemon
readiness, client launch, or timing.

## What

Require the observed exact two-child Windows launcher topology: one registered
daemon by name/path/command and one system console host by name/path. Continue
to reject a missing, duplicate, or unrecognized child, and record the console
host PID/path alongside daemon identity.

## Approach

- Partition the launcher's direct children into exact daemon and exact system
  `conhost.exe` matches.
- Require raw child count two and exactly one member in each class; include all
  child identities in a fail-closed mismatch error.
- Preserve existing exact daemon recovery/teardown and listener checks.
- Extend the Bash 3.2 structural self-test to pin the partition, exact counts,
  and console-host evidence; mutation-prove the old raw-one-child rule red.

## Files changed

- `scripts/bench_ldt4_rigw.sh` — pending exact console-host/daemon partition.

## Guard proof

Pending.

## Coder dispute

None.

## Known gaps

Fix, focused/full gates, tactical review, additive staging, and a completed
live arm/run remain.

## Reviewer comments

This finding came from the attached live launch and isolated process-topology
diagnostic, not a reviewer candidate.
