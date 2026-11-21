# cr-ecm13-6: The line validated less than the command does

**Severity**: MEDIUM — the question offers, by example, an option that refuses the same command
**Status**: Fixed, awaiting review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix cr-ecm13-6`)
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `exclude-case-r13` over `c99ad483..bd84649f` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/exclude-case-r13.codex.md`. It completes cr-ecm12-5.

## Evidence
"With `*Cache` and an absolute exclude outside the source containing `s` inside 249 brace pairs, the default note offered `cache, CACHE`. The same command with `--ignore-case` exited 1. The note omits outside-source patterns from validation, while the command validates them."

Cause. cr-ecm12-5 validates the run's whole filter as the option would read it before the line says anything concrete. An absolute exclude that lies outside the source matches nothing and is in no filter (cr-ecm4-3) — and is still held to being a pattern when the command is read (cr-ecm8-1, `refuse_unreadable`). The line asked the two lists and not that.

## Intake
Admitted as the reviewer put it.

## What
The line is told whether the command with the option would be refused for a pattern that is in neither list (`Whole::refused_with_the_option`), by the same function the command is refused by. Where it would, no pattern is asked about and the line is the rule, as for a pattern in the lists that the option cannot read.

## Guard proof
- `transfers::filter::…::the_case_note_offers_nothing_of_a_command_that_would_be_refused`: `*Cache` beside the reviewer's pattern, outside the source — the command validates as typed and is refused with `--ignore-case`, and the line is the rule; without that pattern, and beside an outside pattern the option can read, the command validates either way and the line has its example.
- `case_fold::…::a_pattern_the_option_cannot_read_gets_the_rule_and_no_example`: the same at the line itself.
- Mutations, each restored byte for byte: the patterns outside the source not asked; a command that would be refused elsewhere asked about all the same — two, each red on the guards named for it.

## Files changed
- `crates/blit-core/src/case_fold.rs` — `Whole::refused_with_the_option`; guard.
- `crates/blit-core/src/transfers/filter.rs` — `case_note_on`; guard.

## Known gaps
- The line and the command agree because they call one function for these patterns and validate one filter for the rest; nothing checks the two WHOLE validations against each other.
- Run on Linux only.
