# cr-ecm5-6: `--delete-scope all` deleted what an exclude names, and nothing said so

**Severity**: MEDIUM — the behaviour is the option's documented one; the question and the README read as if the pattern protected the entry
**Status**: Fixed, awaiting re-review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: the commit carrying this record ("Fix cr-ecm5-6: …")
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `exclude-case-r5` over `3bc3ee47..585d3599` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/exclude-case-r5.codex.md`.

## Evidence
The README says the case rule applies equally to source selection and deletion. With `--delete-scope all --exclude '*Cache'`, the usual case note appeared and the mirror deleted the matching destination-only `caseCache`.

Cause: `--delete-scope all` (older than this plan; its help says "deletes any destination file absent from the (filtered) source set, including files that wouldn't have been transferred in the first place") makes a filter choose what is copied and nothing else. This plan's line in the question, and its README sentence, describe what a pattern "matches" — which under the default scope is also what a mirror keeps, and under this one is not.

## Predicted observable failure
`blit mirror --delete-scope all --exclude PATTERN`: the question shows `"PATTERN" matches … only; capital letters matter …`, the operator reads the matching entries as kept, and they are deleted.

## Intake
Admitted. The deletion is what the option is for and is unchanged; what it says is fixed.

## What
- A `mirror --delete-scope all` with a filter of any kind (a pattern, a list, a size or age bound) shows, in place of the line about capital letters: `--delete-scope all: the filter chooses what is copied, not what is kept; destination entries it leaves out are deleted too.` In the question when one is asked, as a note on stderr from a dry run, and not at all with `-y`, like the line it replaces. `copy` is unaffected (it deletes nothing).
- `README.md`, `CHANGELOG.md` and the release notes say that a filter bounds what a mirror may delete unless `--delete-scope all`.

## Guard proof
- `case_note::delete_scope_all_says_a_filter_protects_nothing_at_the_destination`, through the binary: the line then the question, with no line about capital letters; answered yes, the destination's `bCache` — which the pattern names — is deleted, as said; a size bound alone brings the line; no filter, no line; the default scope keeps the line about capital letters; `-y` says nothing; a dry run prints it as a note; `copy` keeps its own note.
- Mutations, each restored byte for byte, each red: the scope saying nothing; only patterns counting as a filter; the line said with the question waived; the line said on a `copy`.

## Files changed
- `crates/blit-cli/src/transfers/mod.rs` — `delete_scope_note`.
- `crates/blit-cli/tests/case_note.rs` — the guard.
- `README.md`, `CHANGELOG.md`, `docs/RELEASE_NOTES_1_0.md` — the sentence.

## Known gaps
- Under this scope the line about capital letters is not shown at all, though case still decides what is COPIED there; one line was the plan's rule, and the deletion is the graver of the two.
- `docs/cli/blit.1.md` names neither the option's interplay with filters nor any filter option (an open question in `docs/STATE.md`).
- Run on Linux only.
