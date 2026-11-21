# cr-ecm9-1: A rule made from an exclude was not validated as it is executed

**Severity**: HIGH — a confirmed `mirror --ignore-case` deleted a file the question's line had just said the option would also match
**Status**: Fixed, awaiting review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix cr-ecm9-1`)
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `exclude-case-r9` over `3bc3ee47..ab7331c3` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/exclude-case-r9.codex.md`.

## Evidence
On Linux, the prompt for `--exclude 'keep\/'` advertised `KEEP\/` as an additional match. The `--ignore-case` mirror kept the file `keep\` and deleted `KEEP\`. Supplying `KEEP\/` explicitly protected `KEEP\`. The reviewer: trimming the trailing slash leaves a dangling escape in the derived rule, and validation admits it into a case-sensitive text fallback; validate every derived rule and refuse an unrepresentable rule before confirmation.

Cause. `validate_globs` compiled each exclude AS TYPED, and `keep\/` is a glob as typed: an escaped `/`. The rules a filter runs are not the patterns as typed but what `exclude_rule` reads from each — its trailing `/` dropped, its leading `./` stripped — and that text, `keep\`, was compiled on trust. The glob compiler refuses an escape of nothing, and a rule it refuses is compared as text (`CompiledPatterns::refused`), in the typed case only, whatever the case mode. So the filter held a rule `--ignore-case` could not widen, and the line, which reads the pattern's letters, offered a spelling that rule never matches.

## Predicted observable failure
Any exclude whose rule ends in a lone `\` once its trailing `/` is gone (`keep\/`, `a/keep\/`, `keep\/\/`), on Linux or macOS: under `--ignore-case` the rule matches only the typed case, and a mirror deletes the other spellings the option is said to keep. Under the Windows path rule `\` is a separator and the rule is `keep`; not affected.

## Intake
Admitted as the reviewer put it: every derived rule is validated, and one that cannot be compiled refuses the filter before anything is asked.

## What
- `FileFilter::validate_globs` builds the exclude rules as before and then reads the two sets it built: a rule held for comparison as text is one the compiler refused. If there is one, the filter is refused, naming the pattern as typed and the rule read from it: `--exclude pattern "keep\\/" cannot be used: the rule read from it, "keep\\", is not a valid glob: …` (with `--ignore-case`, `cannot be used with --ignore-case`).
- Nothing more is compiled for the check — the sets are the ones the run uses, built once and kept — except, when it fails, the one rule named, for the compiler's own words.
- The refusal comes where every filter refusal comes: before `mirror` asks, at a daemon at session open, and from a saved job.

Cost, measured (release, median of 15, one-file `copy --dry-run`): 250 excludes 26.7 ms at `a13238b6`, 26.4 ms now. A first version compiled each rule a second time to ask whether it compiles, and took 64.6 ms; that version was never committed.

## Guard proof
- `fs_enum::tests::a_rule_that_does_not_compile_refuses_the_filter`: the rule read from `keep\/` is `keep\` under the Unix rule and `keep` under the Windows one; in both case modes `keep\/`, `a/keep\/`, `./keep\/` and `keep\/\/` refuse the filter, the message naming the pattern as typed; `keep\\/` (a name ending in a backslash), `keep/`, `./keep`, `k\ee\p/` and `a/b/` pass; and no pattern that passes holds a rule compared as text.
- `case_note::an_exclude_whose_rule_does_not_compile_is_refused_before_the_question` (Unix), through the binary: `mirror --exclude 'keep\/'` with no mode, `--ignore-case` and `--match-case`, answered `y` — refused, the question never printed, and the destination's `keep\`, `KEEP\` and `extra.txt` all still there, nothing copied.
- Mutation, restored byte for byte: a rule the compiler refused is let through → both guards red.

## Files changed
- `crates/blit-core/src/fs_enum.rs` — `validate_globs`; the unit guard.
- `crates/blit-cli/tests/case_note.rs` — the end-to-end guard.
- `CHANGELOG.md` — the refusal, under Patterns.

## Known gaps
- The end-to-end guard is `cfg(unix)`: under the Windows path rule the pattern is valid and names a folder. The unit guard states that reading and runs on both; nothing here was run on Windows.
- The text comparison for a refused rule stays in `CompiledPatterns`. No filter that passes validation reaches it; a filter that was never validated still can.
- Run on Linux only.
