# cr-ecm12-4: The line still printed a text other than the name it had checked

**Severity**: MEDIUM — the question names `[A]` as gained where the file the option's mirror deletes is `xA`
**Status**: Fixed, awaiting review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix cr-ecm12-4`)
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `exclude-case-r12` over `4d0f1a68..c5bab07a` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/exclude-case-r12.codex.md`. It completes cr-ecm11-3.

## Evidence
"With `--exclude '[Aa]' --include '*[a]'`, it said `--ignore-case would also match [A]`. Both mirrors kept destination-only `A`. The folded mirror deleted `xA`, which the default mirror kept. Print the verified filename `xA` here. The presence of glob syntax in `[A]` does not establish the displayed claim."

Cause. cr-ecm11-3 prints the name that was checked — unless the text to print still held a wildcard or a class, which was kept "as a pattern". `[A]` is such a text, and as a name it is `A`.

## Intake
Admitted as the reviewer put it.

## What
The spelling is printed only when the name that showed the change is the text's OWN — one of the names that text itself reads as. `IMG_*.JPG` is still printed for `img_*.jpg`: its own name `IMG_x.JPG` is the one checked. Otherwise the name is printed, and the line does not say what the pattern matches "only".

## Guard proof
- `case_fold::tests::the_case_note_claims_only_what_the_filter_bears_out`: the reviewer's command — `"*[a]" matches exactly as typed; capital letters matter (--ignore-case would also match xA).`; `img_*.jpg` keeps its pattern spelling.
- `case_note.rs::the_line_claims_only_what_the_mirror_then_does`, through the binary: the line, then both mirrors — `xA` kept as typed and deleted with the option, `A` and `a` kept by both.
- Mutation, restored byte for byte: a text that holds glob syntax printed whatever name was checked, as before → both guards red.

## Files changed
- `crates/blit-core/src/case_fold.rs` — `Spellings::with`; `reads_as_one_name` removed; the guard.
- `crates/blit-cli/tests/case_note.rs` — the case.

## Known gaps
- As cr-ecm11-3's: a name with `x` in it is an example; the first half of the line is the pattern's text.
- Run on Linux only.
