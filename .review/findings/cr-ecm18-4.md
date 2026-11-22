# cr-ecm18-4: the case line's proof never tried a class on `/`

**Severity**: MEDIUM — the line names, as what `--ignore-case` would add, a spelling that shares a path with the pattern as typed through a class holding `/`
**Status**: Fixed, awaiting review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix cr-ecm18-4`)
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `combined-r19` over `ca377805..17f4eb97` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r19.codex.md`. It completes cr-ecm17-4 (`c5077057`).

## Evidence
"The CLI advertised `1[A/]2` as an additional match for `--include '1[a/]2'`. I ran both spellings case-sensitively and both copied `1/2`. The proof omits `/`, which a class can match in a relative path. The verified filename `1A2` would make the warning accurate."

cr-ecm17-4 measured that a `*` crosses separators, and still left `/` out of the characters two classes are tried on — taking a class for a place within one name.

## Intake
Admitted as the reviewer put it.

## What
Classes are tried on every character from 1 to 127, `/` among them, and on one beyond.

## Guard proof
- `case_fold::tests::a_spelling_that_reads_as_what_the_pattern_has_as_typed_is_not_printed` (extended): `1[a/]2` shows `1A2`, the name checked, not `1[A/]2`.
- Mutation, restored byte for byte: `/` left out of the trials, as it was (red).
- `case_note` (16) and `pattern_files` (8) pass unchanged.

## Files changed
- `crates/blit-core/src/case_fold.rs` — `class_trials`; guard.

## Known gaps
- None beyond cr-ecm17-4's.
