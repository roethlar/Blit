# cr-ecm7-3: A pattern the line only counted could be one `--ignore-case` narrows

**Severity**: MEDIUM — the line described what the option adds for one pattern and said nothing of a later pattern that, with the option, stops protecting names; a mirror run with it deleted one
**Status**: Verified — `exclude-case-r8` (Codex openreview, 2026-11-21) reviewed `3bc3ee47..f4203c31`, which holds this fix, with `guard_confirmed: true`, and required nothing further on which pattern is shown (it found the narrowing itself attributed to the wrong class in a pattern of several alternatives — cr-ecm8-2, on the check cr-ecm7-4 added)
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `cce1f877`
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `exclude-case-r7` over `3bc3ee47..cbab0c39` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/exclude-case-r7.codex.md`.

## Evidence
With `--exclude Keep --exclude '[!a].dat'`, the prompt described the extra spellings of `Keep` and counted the second pattern. The exact-mode mirror kept `A.dat`. The ignore-case mirror deleted it. The reviewer: preserve the one-line format and say when another exclusion would stop protecting names.

Cause: the line shows one pattern and counts the rest, and the one shown was the first the option reads otherwise. Its three forms say different things — the option would match more (`would also match keep, KEEP`), or would match LESS (`[!a] would stop matching A`, cr-ecm4-3) — and which was said went by which pattern came first. Of the two, the second is the one an operator must not miss: with it, something protected stops being protected.

## Predicted observable failure
Any command whose first lettered pattern is plain and whose later one holds a negated class with a letter in it: the line reads as "the option only adds matches", and with the option the later pattern stops excluding the letter's other case.

## Intake
Admitted.

## What
A pattern the option would narrow is the one shown, wherever it stands among the patterns; the rest are counted, as before, and the line keeps its one-line form:

`"[!a].dat" matches exactly as typed, and 1 more pattern matches only as typed; capital letters matter (--ignore-case changes what they match: [!a] would stop matching A).`

Found cheaply: every pattern's text is looked over for a negated class that would take less (`has_narrowed_class` — a class inside braces included), and only a pattern holding one is put to the filter, where the class must really stop matching the letter (cr-ecm7-4's check). With no such pattern the choice is cr-ecm7-4's: the first with a spelling borne out, else the first spoken of in words.

## Guard proof
- `case_fold::tests::the_case_note_claims_only_what_the_filter_bears_out`: the line to the letter for `Keep` then `[!a].dat`, and for `Keep`, `*.tmp`, then `{x,[!a]}.dat` — the narrowed pattern third, its class inside braces.
- `case_note::the_line_claims_only_what_the_mirror_then_does`, through the binary: the question's line for `--exclude Keep --exclude '[!a].dat'` to the letter, then both mirrors — as typed `A.dat` and `b.dat` are kept and `keep` is deleted; with `--ignore-case` `A.dat` is deleted, `b.dat` and `keep` are kept. Each is what the line says.
- `case_fold::tests::the_case_note_follows_what_the_matcher_does` and the table guards still hold the cases where the narrowed pattern comes first.
- Mutations, each restored byte for byte: a narrowed pattern shown only when it comes first → both guards red; one sought only among the first two patterns → the first red; a class inside braces not looked at → the first red.

## Files changed
- `crates/blit-core/src/case_fold.rs` — `case_note_of`'s choice, `has_narrowed_class`; the guards.
- `crates/blit-cli/tests/case_note.rs` — the end-to-end case.

## Known gaps
- One pattern is shown. With two narrowed patterns the second is counted (`changes what they match`), its class not named.
- When a narrowed pattern is shown, what the option would ADD for the other patterns is not spelled out; the line says the option changes what they match.
- Every pattern holding such a class is put to the filter until one is borne out; a long list of them that the filter does not bear out (patterns that name nothing) is each compiled once for the line.
- Run on Linux only.
