# cr-ecm7-4: The line could show a pattern whose matches do not change

**Severity**: MEDIUM — the line offered, as names the option "would also match", names the pattern matched already, and passed over the pattern whose protection does change
**Status**: Fixed — awaiting review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix cr-ecm7-4`)
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `exclude-case-r7` over `3bc3ee47..cbab0c39` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/exclude-case-r7.codex.md`.

## Evidence
With `--exclude '{a,A}' --exclude '*.Doc'`, the prompt chose `{a,A}` and offered `{a,a}` and `{A,A}` as additional matches. Both were already covered. The actual change was that `x.doc` gained protection. The reviewer asked for a spelling that demonstrates a real difference, or neutral wording when one cannot be established.

Cause: the line showed the FIRST pattern whose text `--ignore-case` would write otherwise, and made that pattern's other spellings by rule — every letter lowered, every letter raised. `{a,A}` is written otherwise when widened and names exactly the same; lowered and raised it is `{a,a}` and `{A,A}`.

The cause under that one, and the reason this line has drawn a finding in five reviews running (cr-ecm3-4, cr-ecm4-3, cr-ecm5-4, cr-ecm6-2, and three of this review's four): everything the line says was made by rule from a pattern's TEXT and shown on trust. Each review found a text the rules got wrong, each fix was one more rule, and the rules were tested — cr-ecm6-2 put the spellings of 196 pattern shapes through the filter — but in a test, never in the line.

## Predicted observable failure
Any command with a pattern that spells out both cases itself (`{a,A}`, `[Jj]`-style classes beside letters, `{readme,README}`): the line names it, offers spellings that change nothing, and a later pattern that does change is only counted.

## Intake
Admitted. Done as a change to how the line is made, not as one more rule, so that the next text the rules get wrong costs a plainer line and not a false one.

## What
The line now puts each thing it is about to say to the filter itself, and says it only if the filter bears it out:
- **Each other spelling** must name only what `--ignore-case` would match, and something the pattern as typed does NOT. One that fails is dropped: `{a,A}` offers nothing; `{a,A}b` offers `{A,A}B` and no longer `{a,a}b`.
- **The text shown for the pattern** (its end wildcards trimmed) must name something, and nothing the pattern does not; else the pattern is shown whole.
- **A class said to stop matching a letter** must stop matching it.
- **A pattern with nothing left to say** is spoken of, in words (`--ignore-case reads it otherwise`), only when the option does change what it names; when it does not (`{a,A}`, `*.[Jj][Pp][Gg]`) it is neither shown nor counted, and a command with only such patterns gets no line.
- **Which pattern is shown**: the first with a spelling borne out (or a class narrowed), else the first that can only be spoken of in words.

How it asks: `FileFilter::probing` makes a filter of the one pattern, in its role (an `--exclude`, asked as a file and as a folder; an `--include`, asked as a file) — compiled on its own and kept out of the process-wide store of compiled sets, which holds what the run uses (cr-ecm2-3). The names asked with are read off each text's own syntax (`samples`): a wildcard run stands for `x`, a class for a member, an alternation for each alternative, sixteen names at most. Only names a walk can give are used (`walked`: no empty component, no `.` or `..`); a text no entry is ever called settles nothing.

Whether the option changes a pattern at all is asked with every name read off it in other cases: all lower, all upper, and each letter changed alone — every letter, since the names a pattern takes in one sequence of letters are joined by single changes of case — and with each letter a class would gain.

Cost, measured (release, median of 15, one-file `copy --dry-run`): 50 excludes 8.0 ms before, 8.4 ms now; 250 excludes 25.4 ms before and now. Only the pattern shown is asked about in full; of the patterns merely counted, only those holding braces or a class can read no differently, and the first sixteen of those are asked.

## Guard proof
- `case_fold::tests::the_case_note_claims_only_what_the_filter_bears_out`: the line to the letter for `{a,A}` with `*.Doc` in either order (`*.Doc`'s line, `{a,A}` not counted); no line for `{a,A}`, `{a,A}` with `{b,B}`, `*.[Jj][Pp][Gg]`; `{a,A}b`, `{1,Cache}` (the letters in the second alternative), `{ab,AB}` (changed in a way neither spelling shows: words).
- `case_fold::tests::each_claim_is_checked_against_the_filter`: each check asked directly — texts that are and are not faithful to a pattern, spellings that are and are not offered, classes that do and do not stop matching, patterns that do and do not read otherwise (a letter that changes past the eighth among them), and the names read off a text.
- `case_note::the_line_claims_only_what_the_mirror_then_does`, through the binary: the question's line for `--exclude '{a,A}' --exclude '*.Doc'` to the letter, then both mirrors — as typed `x.doc` is deleted and `x.Doc` kept; with `--ignore-case` both are kept; `a` is kept either way.
- Mutations, each restored byte for byte: a spelling offered on trust → all three guards red; every pattern whose text widens spoken of → all three; a pattern read no differently still counted → the first; the shown text taken on trust → the second; a class said to stop matching on trust → the second; a claim checked with any text, a walk's name or not → the second; only the first eight letters tried → the second; no name to ask with taken for no change → the second.

## Files changed
- `crates/blit-core/src/case_fold.rs` — `Role`, `Noted.role`, `case_note_of`'s choice, `Spellings::of_role`/`with`, `Asked`, `samples`, `walked`; the guards.
- `crates/blit-core/src/fs_enum.rs` — `FileFilter::probing`.
- `crates/blit-core/src/transfers/filter.rs` — each pattern is passed with its role.
- `crates/blit-cli/tests/case_note.rs` — the end-to-end guard.

## Known gaps
- A claim is borne out over the names read off the texts — sixteen at most — not over every name. The rules still carry the rest; what the checks add is that a rule's mistake has to survive the filter's own answer on the pattern's own names.
- The checks run under this machine's path rule. Under the other platform's rule — reachable only from tests — the rules stand unchecked, as before. On Windows the checks run with the Windows rule, and that has not been run: Linux only.
- `{a,A}b` is offered as `{A,A}B`: true, and clumsily written.
- Past sixteen counted patterns holding braces or a class, the count goes by the text and may include one the option reads no differently.
- The 196-shape guard of cr-ecm6-2 still covers excludes only; an include's claims are checked in the line, as an include, and by `each_claim…` not at all.
