# cr-ecm16-4: the case line printed a spelling that shares names with the pattern as typed, through `?` or another alternative

**Severity**: MEDIUM — the line names, as what `--ignore-case` would add, a text some names of which are copied either way
**Status**: Fixed, awaiting review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix cr-ecm16-4`)
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `combined-r17` over `13faafd2..7912da16` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r17.codex.md`. It completes cr-ecm15-2 (`0db7e38c`).

## Evidence
"For `--include '{a?,?b}'`, the CLI advertised `{A?,?B}` as an additional match. I verified that `Ab` was copied in both modes, while `Ax` was copied only with `--ignore-case`. Printing `Ax` gives the operator a true example without making a claim about every name in the glob."

cr-ecm15-2 SEARCHED for a name the spelling reads as and the pattern has as typed, trying each `*` holding the pattern's letters. A `?` holds a letter too, and another alternative can supply one: `Ab` is `A` and `b` in `{A?,…}`'s `?`, and `?b` as typed has it. The search did not try it, found nothing, and the spelling was printed.

## Intake
Admitted as the reviewer put it. A search can always miss a case; the fix turns the rule around — the spelling is printed only where it is PROVED that the two texts share no name, and otherwise the name that was checked, which is true whatever the answer.

## What
`cannot_share_a_name(typed, spelling, windows)` in place of `has_as_typed`. What a run's filter does was measured first: a pattern with no separator names an entry by its own name at any depth; a pattern with one is anchored at the root and names whatever lies below what it matches (`Cache/*` names `Cache/cache/x`, and not `x/Cache/y`). So:
- a one-component pattern is compared as a name; a path pattern component by component from the root, up to its first `**` — past it, one path can be named by both through ancestors of different depths (`a/Cache/cache` by `**/Cache` and `**/cache`);
- two components cannot name one name when, at some place a fixed way from either end (before the first `*` or alternation, or after the last), one holds a character the other's does not: a letter of the other case, or a class that does not hold it; two classes are compared on every printable ASCII character and one beyond.

What changes on the line, beside the reviewer's (`{a?,?b}` → `Ax`): `[A-C_]x` shows `ax` and `[A-C_]X` (was `[a-c_]x` — both hold `_x`); `**/*/Cache` shows `x/cache`, `x/CACHE`; `**/Cache/**` shows `cache/x`, `CACHE/x` and no longer says it "matches Cache/** only" (both share `Cache/cache/x`). Unchanged: every spelling whose letters begin or end a name or a path's first components — `IMG_*.jpg`, `a*b`, `?a?`, `[abc]`, `[Cc]ache`, `Cache/*`, `Cache/**`, `*/Cache`, and the Windows forms.

## Guard proof
- `case_fold::tests::a_spelling_that_reads_as_what_the_pattern_has_as_typed_is_not_printed` (extended): the reviewer's; classes apart and classes sharing a character; a path anchored at the root; a path past `**`.
- `case_fold::tests::the_case_note_shows_only_spellings_ignore_case_matches` and `the_case_note_says_what_the_first_lettered_pattern_matches` — three rows CHANGED, each for an overlap that is real (above), each with a comment saying so.
- `crates/blit-cli/tests/case_note.rs::the_line_keeps_what_is_under_a_folder_apart_from_the_folder` — its expected line changed the same way; its point (the line names something UNDER a folder, never a file of that name) holds, and its run is unchanged.
- Mutations, each restored byte for byte: a sole pattern's spelling taken to share no name, unproved (three guards red); no two texts ever shown apart (both case-line guards red, and five more); a path compared past its `**` (red); two classes taken to hold nothing in common (red).

## Files changed
- `crates/blit-core/src/case_fold.rs` — `cannot_share_a_name`, `components_apart`, `places`, `places_apart`; `has_as_typed` removed; guards.
- `crates/blit-cli/tests/case_note.rs` — one expected line.
- `CHANGELOG.md`.

## Known gaps
- The proof is sufficient, not exact: a spelling that shares no name with the pattern but cannot be shown to — two texts whose letters all lie between wildcards — gets the name checked printed in its place. That is true, and less general.
- The measurement of the filter's anchoring was made on this machine, through the filter itself; the rule depends on it.
- This is the line's fifteenth round. The owner's question — keep the line's design, or state the plain rule and no example — is open.
