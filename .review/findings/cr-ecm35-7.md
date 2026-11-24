# cr-ecm35-7: a retry that could not start counted nothing as not retried

**Severity**: MEDIUM — the report understates what was left undone. With `--retries` on, a run whose failed files could not be named for a retry runs no retry pass. The case: its one failure's name was over the wire's budget. It reported the failures (`files_failed = 1`) but `files_not_retried = 0`, as though nothing had been held back.
**Status**: Fixed; awaiting review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix cr-ecm35-7`)
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli headless one-shot, openreview `combined-r36` over `dc14afc4..d2494965` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r36.codex.md`. It completes cr-ecm34-7 (`727df1e0`) in part.

## Evidence
"With one requested name over the wire budget and one retry enabled, the CLI returned exit 2 with `files_failed = 1` and `files_not_retried = 0`. Account for the unnamed remainder before the empty-set exit."

## Intake
Admitted as the reviewer put it.

## What
`run_retry_passes_until` leaves the loop when the retry set is empty. When that happens before any pass has run, every failure the report counts is counted as not retried (`unretried = files_failed`). The failure block then says so on its line, and `--json` carries it in `files_not_retried`.

An empty set after a pass is left as it was: those failures were retried, and the remainder no pass was given is carried by `after_pass` as before.

## Guard proof
- `transfers::retry::tests::a_retry_that_cannot_start_counts_its_failures_as_not_retried`: one failure, no name, one retry. No pass runs, and the failure is counted not retried.
- Mutation, restored byte for byte: nothing counted → red.
- Checks:
  - native clippy is clean;
  - the CLI package passes: 509 tests, Linux.

## Files changed
- `crates/blit-cli/src/transfers/retry.rs` — the empty-set exit; guard.

## Known gaps
- None known.
