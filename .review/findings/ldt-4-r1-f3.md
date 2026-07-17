# ldt-4-r1-f3 — normalize padded PIDs before exact q-client recovery

**Severity**: MEDIUM — the orphan-client fallback silently skipped common low
PIDs and could leave a stale writer alive during a later arm.
**Status**: Closed by clean neutral whole-change re-review at `4e0fdc3`.
**Branch**: `master` (repo policy forbids agent-created branches)
**Commit**: `0efa4e0`

## Evidence

`scripts/bench_ldt4_rigw.sh:9-11` removes spaces from global `IFS`.
`stop_q_client` previously iterated raw `ps -axo pid=` output and required each
candidate to match `^[0-9]+$`. macOS right-justifies the PID column, so values
such as `    1` and `  265` retained their padding under that `IFS` and failed
the regex; only naturally full-width values were considered.

## Predicted observable failure

If both the in-memory PID and its fsynced PID evidence are unavailable during
recovery, an owned client with a low PID is not found. The fallback returns as
though no client survives, allowing a later arm to start while the stale exact
client may still write its destination and contaminate or void evidence.

## What

Normalize the PID-only `ps` stream before the recovery loop. Every one-field
numeric line becomes its unpadded decimal value; malformed lines are ignored.
The existing exact command line and both trace-environment identity checks are
unchanged, so normalization broadens no kill authority.

## Approach

`normalize_q_client_pid_list` uses `awk` field parsing to strip column padding
and emit only numeric PIDs. `stop_q_client` consumes that normalized list, then
performs the same exact process command/environment comparison and ambiguity
refusal as before.

## Files changed

- `scripts/bench_ldt4_rigw.sh` — PID normalizer, fallback-loop use, and
  low-PID self-test.

## Guard proof

- The offline self-test requires padded inputs for PID 1 and 265 plus 12345 to
  normalize to exact newline-separated digits while a noise line is omitted.
- Mutating the helper to emit only values at least 10000 returns exit 1 with
  `q client PID normalization dropped padded low PIDs`; exact restoration
  returns `PASS (96 arms, no SSH)`.

## Coder dispute

None.

## Known gaps

This remains a recovery-only path. The normal path still prefers the owned
in-memory PID and the exclusively written PID file.

## Reviewer comments

Claude Fable 5/max returned the candidate over exact
`e41b871..0e48721` with `guard_confirmed=true`. Intake reproduced the padding
failure locally and admitted it. Claude Fable 5/max re-reviewed exact fixed
head `4e0fdc3` with an independent red/green guard and returned clean, closing
this finding.
