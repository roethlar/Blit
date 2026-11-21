# cr-ecm9-2: The line did not show a real loss first, and judged a gain against one pattern

**Severity**: HIGH — the question was asked with a line about another pattern while `--ignore-case` would stop protecting a file; a confirmed mirror under that option deleted it
**Status**: Fixed, awaiting review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix cr-ecm9-2`)
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `exclude-case-r9` over `3bc3ee47..ab7331c3` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/exclude-case-r9.codex.md`. Completes cr-ecm8-2, which that review found incomplete.

## Evidence
With `--exclude Keep --exclude '{[!ab].txt,A.txt}'`, the prompt described only the additional spellings of `Keep`. The exact-mode mirror kept both `A.txt` and `B.txt`. The `--ignore-case` mirror deleted `B.txt`. The reviewer: the warning requires every candidate letter to lose its match, so the alternative that preserves `A.txt` hides the loss of `B.txt`; one verified loss should be enough to show it. Check gains against the complete filter too: `--exclude a --exclude A` advertised `A` as an additional match, although both runs already protected it.

Cause, two things cr-ecm8-2 left:
- **A loss was all or nothing.** `stops_matching` answered yes only when EVERY letter a class would lose was lost at that class. `[!ab]` would lose `A` and `B`; `A.txt` stays matched by the alternative beside it, so nothing was claimed and the pattern fell to "counted", behind `Keep`.
- **Every check asked a filter of the one pattern.** A spelling was offered when the pattern as typed did not match it and the pattern with the option did. Whether the FILTER keeps that name — another pattern may — was never asked.

## Predicted observable failure
Two or more patterns where one narrows for some of a class's letters only: the line speaks of another pattern, and the mirror under the option deletes what the line did not mention. Patterns that between them spell a name in several cases (`a` and `A`, `*.jpg` and `*.JPG`): the line offers as "would also match" a name already kept.

## Intake
Admitted as the reviewer put it: one verified loss is enough and is shown first; gains are judged against the complete filter. Losses are judged against it too, for the same reason — a name another pattern still keeps with the option is not lost.

## What
- **The line is given the lists the filter holds** (`Whole`: the resolved excludes and the includes as sent). Each list is asked as a filter of its own — as typed, and, only when a loss has to be checked, as `--ignore-case` would read it. They are the run's own lists, so the sets compiled for the run are the ones asked.
- **A loss is the letters really lost** (`Asked::lost_here`, in place of `stops_matching`). For a class that would take less, each letter it is said to lose is put where that class stands and asked about: lost if the pattern as typed names it, the pattern with the option does not, and no pattern of the list names it with the option. The claim is made with those letters, however few: `[!ab] would stop matching B`; `[!a-c] would stop matching B-C` beside an alternative `A.x`.
- **A gain is a name the filter does not already keep** (`matched_only_ignoring_case`, `compared`). A spelling is offered only if the option names it through this pattern and the whole list as typed does not. A plain pattern the option changes only in ways another pattern already covers is neither shown nor counted: `--exclude a --exclude A` has no line.
- **What a spelling stands for is asked, not only its text.** The text shown for `*.o` is `.O`, a name `??` happens to take; the names it is shown for (`x.O`) are what is asked about as well, so a pattern beside `??` is still described and one beside `*` is not.

Lines after the fix, from the binary:
- `--exclude Keep --exclude '{[!ab].txt,A.txt}'`: `"{[!ab].txt,A.txt}" matches exactly as typed, and 1 more pattern matches only as typed; capital letters matter (--ignore-case changes what they match: [!ab] would stop matching B).` As typed the mirror keeps `A.txt` and `B.txt`; with the option it keeps `A.txt` and `keep` and deletes `B.txt`.
- `--exclude a --exclude A`: no line.

Cost, measured (release, median of 15, one-file `copy --dry-run`): 50 excludes 9.4 ms at `a13238b6`, 9.7 ms now; 250 excludes 26.7 and 26.4 ms; `mirror --yes` with 250, 30.7 and 30.7 ms.

## Guard proof
- `case_fold::tests::the_case_note_claims_only_what_the_filter_bears_out`: the two lines above to the letter; `{[!a-c].x,A.x}` → `B-C`; `a` + `A`, and with `{b,B}` beside them, no line; with `*.Doc` beside them, `*.Doc`'s line; `[!a].dat` + `A.dat` → no loss claimed, `A.dat`'s gain shown; two patterns none of whose class letters is lost → the rule and no claim.
- `case_fold::tests::each_claim_is_checked_against_the_filter`: `lost_here` letter by letter for nine patterns alone and two among others (`[!a].dat` beside `A.dat` loses nothing, beside `Keep` loses `A`); `matched_only_ignoring_case` and `compared` alone and among others (`a` beside `A`: not offered, the same; beside `b`: `A`); `*.o` beside `??` offered, beside `*` not.
- `case_fold::tests::the_case_note_says_what_the_first_lettered_pattern_matches`: a pattern beside `*` gets no line; beside `??` its line is unchanged.
- `case_note::the_line_claims_only_what_the_mirror_then_does`, through the binary — the reviewer's two commands: the line to the letter, then both mirrors and what each leaves.
- Mutations, each restored byte for byte: a loss claimed only when every letter of the class is lost → the first and the end-to-end guard red; a spelling judged against the one pattern → the first, second and end-to-end; a plain pattern compared with itself, not the filter → the first and second; a letter counted lost when the one pattern stops matching it → the first and second; a spelling judged by its own text alone, not by the names it is shown for → the second and third.

## Files changed
- `crates/blit-core/src/case_fold.rs` — `Whole`, `Listed`, `WholeFilters`; `case_note_of`, `Spellings::of_role`/`with`, `Asked::about`, `Asked::lost_here`, `matched_only_ignoring_case`, `compared`, `written`; the guards.
- `crates/blit-core/src/transfers/filter.rs` — `case_note_on` passes the lists.
- `crates/blit-cli/tests/case_note.rs` — the two end-to-end cases.

## Known gaps
- A change that needs BOTH lists to see is not looked for: an include and an exclude are each judged within their own list. A name an include would newly admit and an exclude already leaves out is still offered as "would also match".
- For a pattern with a wildcard or a class, the names asked about are the ones read off the pattern (a wildcard stands as one sample). A name another pattern covers only for some of what the wildcard can be is judged by that sample.
- A loss is looked for only at negated classes, the one place the option makes a single pattern take less.
- The checks run under this machine's path rule; under the other platform's (tests only) the rules stand unchecked. Run on Linux only.
