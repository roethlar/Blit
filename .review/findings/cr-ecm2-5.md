# cr-ecm2-5: `blit move --help` offered two options `move` always refuses

**Severity**: LOW — help text; the refusal itself was already specific
**Status**: Fixed, awaiting re-review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: the commit carrying this record ("Fix cr-ecm2-5: …")
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `exclude-slice1-r2` over `3bc3ee47..a1fd6bae` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/exclude-slice1-r2.codex.md`.

## Evidence
`blit move --help` listed `--match-case` and `--ignore-case` under Filtering. `move` takes no pattern, and slice 1 made it refuse both by name on every invocation. `copy`, `mirror` and `move` share one set of option declarations (`TransferArgs`, `crates/blit-cli/src/cli.rs`), so what slice 1 added for two verbs appeared on the third.

## Predicted observable failure
A user reading `move`'s help is offered two options that can only fail.

## Intake
Admitted, for the two options this change added.

## What
`move`'s help no longer lists `--match-case` or `--ignore-case` (hidden on that verb alone). Both still parse there, so `blit move --ignore-case …` gets the same specific refusal as before rather than a parser's "unexpected argument". `copy`, `mirror` and `check` list them as before.

## Guard proof
- `ignore_case::help_offers_the_case_modes_only_where_they_are_taken`: through the binary, `--help` and `-h` of `copy`, `mirror` and `check` name both options; `move`'s name neither.
- `ignore_case::move_refuses_a_case_mode_it_has_no_pattern_for` (slice 1) is unchanged and green: the refusal still names the option.
- Mutations, each restored byte for byte: either hiding removed → the help guard red.

## Files changed
- `crates/blit-cli/src/cli.rs` — two attributes on the `Move` subcommand.
- `crates/blit-cli/tests/ignore_case.rs` — the guard.

## Known gaps
- **Older, not changed here, for the owner:** `move --help` lists fourteen more options that `move` refuses on every invocation, all from before this plan — `--dry-run`, `--detach`, `--exclude`, `--include`, `--files-from`, `--min-size`, `--max-size`, `--min-age`, `--max-age`, `--ignore-existing`, `--null`, `--force`, `--ignore-times`, `--size-only` (read from `run_move_inner`, 2026-11-20). The same two-line remedy applies to each. Left alone because it changes the help of an existing verb outside this review's range and outside any plan; recorded in `docs/STATE.md` as an open question.
- Run on Linux only.
