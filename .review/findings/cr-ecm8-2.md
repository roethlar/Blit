# cr-ecm8-2: The line could be absent, or name the wrong class, for patterns it should speak of

**Severity**: HIGH — the question was asked with no line at all for a pattern whose protection `--ignore-case` changes, and a confirmed mirror deleted what the option would have kept; for another pattern the line said the wrong name loses protection
**Status**: Reviewed, found incomplete — `exclude-case-r9` (Codex openreview, 2026-11-21) reviewed `3bc3ee47..ab7331c3`, which holds this fix, with `guard_confirmed: true`. Leaving a pattern out only on proof stands. Two things in it do not: a narrowing is claimed only when EVERY letter a class would lose is lost, so a real loss beside an unlost letter is not shown (`{[!ab].txt,A.txt}` loses `B.txt`, and the line spoke of `Keep`); and a gain is judged against the one pattern, not the filter (`--exclude a --exclude A` offered `A`, which the second pattern already keeps). The remaining work is cr-ecm9-2
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `e3b46470`
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `exclude-case-r8` over `3bc3ee47..f4203c31` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/exclude-case-r8.codex.md`.

## Evidence
With `--exclude '{Σ,σ}'`, the CLI showed only the ordinary delete prompt. Confirming deleted destination `ς`, which `--ignore-case` preserved. The ASCII pattern `{[a-c],A,B}` produced the same failure for `C`. Another pattern, `{[!a].1,A.1,[!b].2}`, attributed lost protection to `[!a]` and `A`, although `A.1` stayed protected and `B.2` lost protection. The reviewer: verify examples against the complete pattern in both modes; when sampling finds no example, show the case rule unless equivalence is established.

Cause, two faults in what cr-ecm7-4 added:
- **"No difference found" was taken for "no difference".** cr-ecm7-4 left a pattern out of the line when the names it tried — each name read off the pattern, all lowered, all raised, and with each letter's case changed — showed none. `ς` is a third member of `σ`'s case class, reached by neither lowering nor raising; and for the class `[a-c]` only the first letter it would gain was tried, `A`, which the alternative beside it already matches.
- **A narrowing was credited to the first class that could narrow, on the evidence of any name at all.** The check put the lost letter where "the first class written so" stands and then accepted a difference in ANY name read off the pattern, the other alternatives' names among them.

## Predicted observable failure
A pattern of alternatives that spells out some cases of a letter and not all (`{Σ,σ}`, `{ß,SS}`-like sets, `{k,K}` without the Kelvin sign), or letters beside a class: no line, and the mirror deletes what the option would keep. A pattern of several alternatives with negated classes: the line names a class and letter that lose nothing.

## Intake
Admitted, as the reviewer put it: an example must be borne out against the complete pattern in both modes, and a pattern is left out only when equivalence is established.

## What
- **A pattern is left out only on proof** (`Asked::compared`). For a pattern of plain text and alternatives — no wildcard, no class — the names it can match are the ones written in it, and `--ignore-case` adds every other case of each of their letters and nothing else. Every one of those spellings (every member of each letter's case class, not its lower and upper form) is put to both filters. If none is read differently, the option changes nothing and the pattern is neither shown nor counted (`{a,A}`, `{Σ,σ,ς}`). If some are, they are names the option would also match, asked of the complete pattern in both modes, and the line shows them: `"{Σ,σ}" matches {Σ,σ} only; capital letters matter (--ignore-case would also match ς).`
- **Where that cannot be done, nothing is concluded.** With a wildcard or a class in the pattern, or past 1,024 spellings, the comparison answers "unknown", and unless a spelling or a narrowing is borne out the line states the rule and claims no more: `"{[a-c],A,B}" matches exactly as typed; capital letters matter (--ignore-case ignores them).`
- **That form no longer claims a difference.** It read `(--ignore-case reads it otherwise)`, which says the option changes this pattern — the very thing not established. It now says what the option does.
- **A narrowing is credited where it stands** (`Asked::stops_matching`, `narrowed_classes`). Every negated class of the pattern that would take less is a candidate, with the byte it begins at; the lost letter is put at THAT class, and only names read through the alternative that holds it count (`samples` with a place). The first candidate for which EVERY letter it is said to lose is lost is the one named: `[!b] would stop matching B`. If none is borne out in full, no narrowing is claimed.
- **No rule-made spelling beside a class that would take less.** Such a spelling copies the class as written and reads as admitting what the option refuses there (cr-ecm4-3); it was withheld only when the narrowing was claimed. It is now withheld whenever the pattern holds such a class.

Cost, measured (release, median of 15, one-file `copy --dry-run`): 50 excludes 8.0 ms at `a13238b6`, 8.5 ms now; 250 excludes 25.5 and 25.6 ms. The comparison runs only for a pattern with no spelling and no narrowing borne out.

## Guard proof
- `case_fold::tests::the_case_note_claims_only_what_the_filter_bears_out`: the line to the letter for `{Σ,σ}` (the example `ς`), `{Σ,σ,ς}` (none), `{[a-c],A,B}` (the rule; and counted beside `Keep` where `{a,A}` is not), `{[!a].1,A.1,[!b].2}` (`[!b]`, `B`), `{ab,AB}` (the names `aB`, `Ab`), and three patterns whose narrowing is not borne out in full (the rule, no spelling).
- `case_fold::tests::each_claim_is_checked_against_the_filter`: `stops_matching` by place — false for `[!a]` and true for `[!b]` in the reviewer's pattern, false for `[!a-c]` beside `A.x` (one of its three letters stays matched); `compared` — the same for four patterns, `Differs(["ς"])` for `{Σ,σ}`, differing for three more, unknown for seven (a class, a wildcard, no name a walk gives, too many spellings); `samples` with a place gives only the names through it.
- `case_note::the_line_claims_only_what_the_mirror_then_does`, through the binary — the reviewer's three commands: the question's line to the letter, then both mirrors. `{Σ,σ}`: as typed `ς` is deleted, with the option kept. `{[a-c],A,B}`: as typed `C` is deleted, with the option kept. `{[!a].1,A.1,[!b].2}`: `A.1` kept both ways; `B.2` kept as typed and deleted with the option.
- Mutations, each restored byte for byte: a pattern left out when not every name could be asked about → the first and third guards red; only lower and upper case tried → all three; a narrowing credited when any name stops matching → all three; one lost letter enough to claim them all → the second; a rule-made spelling beside a class that would take less → the first; a pattern with a wildcard compared as if its names were all there are → the second; one with a class likewise → all three.

## Files changed
- `crates/blit-core/src/case_fold.rs` — `Spellings::with`, `Asked::stops_matching`, `Asked::compared` (in place of `reads_otherwise`), `Compared`, `only_text_and_alternatives`, `narrowed_classes`, `lost_runs`, `lost_letters`, `samples`/`expand`; the words form; the guards.
- `crates/blit-cli/tests/case_note.rs` — the three end-to-end cases; the reworded form.

## Known gaps
- For a pattern with a wildcard or a class, the line shows an example only when a rule-made spelling is borne out; otherwise it states the rule. It does not go looking for the name that changes (`C` for `{[a-c],A,B}`).
- A narrowing that is real for only some of a class's letters is not claimed at all; the line then states the rule.
- Equivalence is established by asking both filters about every spelling, up to 1,024; a longer plain pattern that happens to be written in every case is counted and spoken of as if it were not.
- The checks run under this machine's path rule; under the other platform's (tests only) the rules stand unchecked. Run on Linux only.
