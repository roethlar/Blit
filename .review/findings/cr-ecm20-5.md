# cr-ecm20-5: a `/` in one alternative of a brace group anchored the pattern in the case line's proof

**Severity**: MEDIUM — the deletion prompt's case line names a spelling `--ignore-case` would add that matches a file the typed pattern already matches
**Status**: Fixed, awaiting review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix cr-ecm20-5`)
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `combined-r21` over `c15ebf5a..b92c9a17` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r21.codex.md`. It completes cr-ecm19-5 (`f764e6b8`).

## Evidence
"The CLI advertised `A{/,X}*1` as an additional match for `a{/,x}*1`. Both case-sensitive runs copied `A/ax1`. A slash in one alternative cannot establish that every match starts at the root."

## Intake
Admitted as the reviewer put it.

## What
`must_part(pattern, windows)`: a pattern is anchored only by a separator outside every class AND every brace group (a trailing one aside) — read from the text, with classes skipped whole and brace depth counted; an escaped `/` on Unix is one too. `cannot_share_a_name` asks it in place of reading anchoring from the places, which lost where the braces were.

## Guard proof
- `case_fold::tests::a_spelling_that_reads_as_what_the_pattern_has_as_typed_is_not_printed` (a row added): `a{/,x}*1` shows `A/x1`, not `A{/,X}*1`; through the run's own filter, for it and for cr-ecm19-5's `a[x/]*1`, the name shown is taken with `--ignore-case` and not without, and `A/ax1` — the reviewer's — is taken either way.
- Mutations, each restored byte for byte: a `/` in a brace group anchoring (red); a `/` in a class anchoring (red).
- The case-matching tests (21) and the CLI's case-line tests (16) pass.

## Files changed
- `crates/blit-core/src/case_fold.rs` — `must_part`, `cannot_share_a_name`; a row.

## Known gaps
- A pattern every alternative of which holds a `/` (`{a/b,c/d}`) is read as not anchored: the line then shows a name it checked, more often than it must.
- The line has been through nineteen rounds; the owner's question on its design is open.
