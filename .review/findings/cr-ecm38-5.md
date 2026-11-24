# cr-ecm38-5: a linked "not known" failure was counted unsaid, and failures withheld from a retry lost their names

**Severity**: MEDIUM — an eligible retry is withheld, and a known failure is reported only as a count. Brought in by cr-ecm37-3 and cr-ecm37-5. The reviewer found two faults under `--ignore-existing`.
- **Counted unsaid.** The destination's leftover account counted a "not known" failure as said only when its report entry's path was whole. A long name's entry is shortened, even though cr-ecm37-5 links it to its exact name. So the account was marked inexact, and under `--ignore-existing` a separate, eligible failure was withheld from the retry.
- **Lost names.** With a short name, the sibling was retried, but the failure withheld from the pass became part of an anonymous "not retried" count, without its name or reason.
**Status**: Fixed; awaiting review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix cr-ecm38-5`)
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli headless one-shot, openreview `combined-r39` over `6ee7ed5e..86d69ee3` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r39.codex.md`. It completes cr-ecm37-3 (`1d5be9a8`) and cr-ecm37-5 (`a57e4bd0`) in part.

## Evidence
"Under `--ignore-existing`, a shortened but explicitly linked failure with unknown disposition suppressed a separate, eligible retry. With a short name, the sibling retried successfully, but the known failure lost its name and reason. Honor the index when counting unknown dispositions and carry named failures forward alongside the anonymous remainder."

## Intake
Admitted as the reviewer put it.

## What
- **The account** (`SinkOutcome::wire_left_account`) counts a "not known" report entry as said when its path is whole or it is linked to its exact name (`failed_path_at`).
- **The retry** (`PassFailures`, the CLI):
  - `after_pass` is given the set the pass was given. The failures the report names that the pass was not given are carried forward by name: their report entries in `withheld`, their identities in `withheld_paths`. They are kept out of what the next pass is given, and still counted as not retried with the anonymous remainder (`unretried`).
  - `reported` writes the pass's own failures and the withheld ones together into every summary the retry writes: the summary itself, the local surface, and a delegated pull's. Each is linked to its exact name.

## Guard proof
- `sink::cr_win_1_tests::a_linked_unknown_is_said`: a long "not known" failure's shortened, linked entry leaves the leftover account exact.
- `transfers::retry::tests::a_withheld_failure_keeps_its_name_and_reason`: under `--ignore-existing`, a report of a "not known" `u.bin` and an ordinary `k.bin`.
  - The pass is given only `k.bin`, and it lands.
  - The run's summary still names `u.bin` with its reason, counted as not retried.
- Mutations, each restored byte for byte:
  - a linked "not known" counted unsaid → red;
  - the withheld not carried → red.
- Checks:
  - native, Windows and macOS (`CARGO_FEATURE_PURE=1`) clippy are clean;
  - the workspace passes on Linux: 2,129 tests over 57 binaries.

## Files changed
- `crates/blit-core/src/remote/transfer/sink.rs` — `wire_left_account`; guard.
- `crates/blit-cli/src/transfers/retry.rs` — `PassFailures.withheld`/`withheld_paths`, `after_pass`, `reported`, the three summary writers; guard.

## Known gaps
- The report's line for failures not retried still reads "the retry set could not name them all", including for the withheld failures it now names above it.
