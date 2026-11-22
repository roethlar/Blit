# cr-ecm19-5: a `/` inside a class anchored the pattern in the case line's proof

**Severity**: MEDIUM — the deletion prompt's case line names a spelling `--ignore-case` would add that matches a file the typed pattern already matches
**Status**: Fixed, awaiting review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix cr-ecm19-5`)
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `combined-r20` over `17f4eb97..c15ebf5a` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r20.codex.md`. It completes cr-ecm18-4 (`19161cc4`).

## Evidence
"The CLI advertised `A[X/]*1` as an additional match for `a[x/]*1`. Both patterns, run case-sensitively, copied `A/ax1`. One matches the whole path and the other its basename. A slash in the pattern text is insufficient to prove root anchoring."

The filter matches a pattern against an entry's own name or its whole relative path; no name holds a `/`, so a pattern that MUST match one matches the whole path only, from the root. The proof took any `/` in the text for such a one — one inside a class too, which may hold another character instead.

## Intake
Admitted as the reviewer put it.

## What
`cannot_share_a_name` takes a pattern as anchored only when one of its places is a separator itself (a trailing one aside) — read from its places, not its text. A class that may hold `/` anchors nothing; the two spellings are then told apart from the end only, and where they cannot be, the line shows a name it checked.

## Guard proof
- `case_fold::tests::a_spelling_that_reads_as_what_the_pattern_has_as_typed_is_not_printed` (a row added): `a[x/]*1` shows `AXx1`, not `A[X/]*1`; through the run's own filter, `AXx1` is taken with `--ignore-case` and not without, and `A/ax1` — the reviewer's — is taken either way.
- Mutation, restored byte for byte: any `/` in the text anchoring the pattern, as it was (red).
- The case-matching tests (21) and the CLI's case-line tests (16) pass.

## Files changed
- `crates/blit-core/src/case_fold.rs` — `cannot_share_a_name`; a row.

## Known gaps
- A class holding only `/` (`a[/]b`) must match one, and could anchor; it is read as not anchored, which only makes the line show a checked name more often.
- The line has been through eighteen rounds; the owner's question on its design is open.
