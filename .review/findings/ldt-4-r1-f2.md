# ldt-4-r1-f2 — refuse ambiguous Windows daemon baselines after a hard crash

**Severity**: MEDIUM — a rerun after untrapped termination could preserve the
test daemon as the supposed machine baseline and strand the true original.
**Status**: Fixed and mutation-proved; neutral whole-change re-review pending.
**Branch**: `master` (repo policy forbids agent-created branches)
**Commit**: `096304b`

## Evidence

`scripts/bench_ldt4_rigw.sh:954-1021` gives every swap intent and retained
original a new session tag. Restoration normally runs from `on_exit`, but
power loss or `SIGKILL` can skip it. A later run previously classified whatever
occupied `WIN_ACTIVE_DAEMON` as its own prior baseline without checking for an
older `retained-before-*` file or for bytes identical to the staged test build.

## Predicted observable failure

After a hard crash, a new run can complete and "restore" the staged test daemon
as the pre-run baseline. If the original existed, its exact bytes remain
stranded under the old retention name; if the active path was originally
absent, the staged daemon remains active. Nothing is deleted, but the documented
exact restoration invariant is silently false.

## What

Refuse a new swap before creating its intent when the dedicated active-runtime
directory contains any retained original from an earlier session or when the
current active daemon already hashes to the staged test daemon. Both states are
ambiguous and require operator adjudication; the harness does not delete,
rename, recover, or acknowledge them automatically.

## Approach

The generated PowerShell validates the staged hash first, inventories exact
`retained-before-*-blit-daemon.exe` entries, rejects directory/reparse matches,
then classifies the active path. An active/staged hash equality is refused.
The offline safety test pins both guards before intent creation and therefore
before either runtime mutation. Normal completion leaves only
`retained-tested-*`, so its retained evidence is not mistaken for an unresolved
baseline.

## Files changed

- `scripts/bench_ldt4_rigw.sh` — pre-intent stale-baseline guards and static
  ordering proof.

## Guard proof

- Changing the exact equality guard from `-ceq` to `-cne` makes the offline
  harness safety test exit 1 because the required fail-closed condition is no
  longer present; exact restoration returns `PASS (96 arms, no SSH)`.
- A read-only SSH PowerShell probe using only in-memory values returned
  `REFUSED|active daemon already matches staged test daemon; prior baseline is
  ambiguous` for equal hashes. It read or wrote no endpoint file.

## Coder dispute

None.

## Known gaps

The refusal is deliberately conservative when a legitimate pre-existing
daemon happens to be byte-identical to the staged build. Automatically deciding
whether that state is a baseline or crash residue would recreate the ambiguity
this fix closes.

## Reviewer comments

Claude Fable 5/max returned the candidate over exact
`e41b871..0e48721` with `guard_confirmed=true`. Intake admitted the hard-crash
misclassification and selected fail-closed refusal rather than automatic
recovery or deletion. Final fixed-SHA whole-change re-review is pending.
