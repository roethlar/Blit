# cr-ecm5-4: The line's example dropped a directory the pattern requires

**Severity**: HIGH — the line said `--ignore-case` would also match a name it does not; a mirror run on that reading deleted it
**Status**: Fixed, awaiting re-review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: the commit carrying this record ("Fix cr-ecm5-4: …")
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `exclude-case-r5` over `3bc3ee47..585d3599` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/exclude-case-r5.codex.md`.

## Evidence
For `--exclude '*/Cache'`, the prompt said `--ignore-case` would also match `cache`. Running that mirror with `--ignore-case` deleted root-level `cache` and kept `sub/cache`.

Cause: `without_end_wildcards` (`crates/blit-core/src/case_fold.rs`, slice 2) trims the wildcards at a pattern's ends for showing, so that `*Cache`, `Cache*` and `**/Cache/**` are all shown `Cache`. It trimmed any run of stars and the separator beside it. `**/Cache` does name `Cache` at any depth, the top included; `*/Cache` names it only one level down, and `Cache/*` names what is directly inside it. Those are a directory level the pattern requires, not decoration.

## Predicted observable failure
`mirror --exclude '*/NAME'` or `'NAME/*'`: the line shows `NAME` and its other spellings without the level, and reads as protecting top-level entries the pattern never names, in either mode.

## Intake
Admitted.

## What
A star that stands for a whole directory level stays in what is shown: `"*/Cache" matches */Cache only; capital letters matter (--ignore-case would also match */cache, */CACHE).` Trimmed as before: a star run touching text (`*Cache`, `Cache*`), and `**/` at the start or `/**` at the end, repeatedly (`**/*/Cache` is shown `*/Cache`). Under the Windows rule the same with `\`.

## Guard proof
- `case_fold::tests::the_case_note_shows_only_spellings_ignore_case_matches`: seven patterns with what is shown and each other spelling, under both rules (`*/Cache`, `Cache/*`, `*/Cache/*`, `**/*/Cache`, `**/*Cache*/**`, `*\Cache`, `Cache\*`).
- `case_fold::tests::the_case_note_says_what_a_negated_class_would_stop_matching`'s rule — every spelling shown, read as a glob, admits only names the option matches — now runs over four of these patterns as well.
- `case_note::the_line_keeps_a_directory_level_the_pattern_requires`, through the binary: the reviewer's sequence — the line for `*/Cache` to the letter; with `--ignore-case`, `sub/cache` is kept and top-level `cache` deleted, as the line now shows.
- Mutations, each restored byte for byte: a leading required level trimmed → all three red; a trailing one trimmed → the two core guards red.

## Files changed
- `crates/blit-core/src/case_fold.rs` — `without_end_wildcards`; the guards' rows.
- `crates/blit-cli/tests/case_note.rs` — the end-to-end guard.

## Known gaps
- What `*Cache` is shown as — `Cache`, "matches Cache only" — still stands for "a name that ends in Cache"; that is the plan's own wording (AC-3).
- Run on Linux only.
