# cr-ecm14-5: the line printed a spelling where its claim rested on the names it had tried

**Severity**: MEDIUM — the question's line says the option would match names that both runs leave out
**Status**: Reviewed, found incomplete — `combined-r16` (Codex openreview, 2026-11-21) reviewed `35adeacb..13faafd2` — utilities slices 5 and 6 and the eleven `combined-r15` fixes — with `guard_confirmed: true`. A pattern that is the whole filter is not always answered for by its spelling: "With the sole include `1*a*2`, the note presents `1*A*2` as what `--ignore-case` gains. I verified that `1aA2` already copied in both modes." The remaining work is cr-ecm15-2
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `33ac6700`
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `combined-r15` over `b8273ac1..35adeacb` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r15.codex.md`. It completes cr-ecm13-7 (`b7139bd9`).

## Evidence
"With `--include 'a?b' --exclude '?1?'`, the note offered `A?B`. Enabling `--ignore-case` added `AxB`, while `A1B` stayed excluded. Showing `AxB` accurately describes the change the complete filter demonstrated."

Cause. cr-ecm13-7 prints a spelling — the pattern in another case — only when EVERY name it reads as is treated otherwise with the option, and found those names by reading them off the spelling's own text (`samples_with`): each alternative in turn, a class's member, `x` for a wildcard. For a text of letters and alternatives those are all its names. For a text that still holds a wildcard or a class they are the few that were tried: `A?B` was read as `AxB` alone, and `A1B`, which another pattern of the run leaves out either way, was never asked about.

## Intake
Admitted as the reviewer put it.

## What
A spelling is printed for the names tried only when they are all it can be answered for (`would_also_match`):
- when the text is letters and alternatives, as before — its names are the ones tried; or
- when the pattern is the run's WHOLE filter, whatever it holds: there is then no other pattern to treat one of its names otherwise, and what the option does with the spelling is what it does with the pattern (`--exclude 'IMG_*.JPG'` alone is still offered `img_*.jpg`).

Beside other patterns, for a text that still holds a wildcard or a class, the name that was tried — and shown to change — is printed: `AxB`.

## Guard proof
- `case_fold::tests::the_case_note_claims_only_what_the_filter_bears_out`, extended: the reviewer's command gives `would also match AxB`; `a?b` alone gives `A?B`, the spelling; a class beside another pattern (`a[xy1]b` beside `?1?`) gives `AXB`, and a star in the middle of a text (`img_*.jpg` beside `*1*`) gives `IMG_x.JPG`; a text of alternatives beside another pattern (`{axb,ayb}` beside `?1?`) is still printed as its spelling.
- `crates/blit-cli/tests/case_note.rs::the_line_claims_only_what_the_mirror_then_does` (end to end), one more case: the reviewer's command prints the line with `AxB`, and the two mirrors — as typed, and with `--ignore-case` — then do what it says: `AxB` is taken only with the option, `A1B` by neither, `axb` by both.
- Every guard of the line that stood before holds unchanged: the first form of this fix, which asked only whether the text holds a wildcard, moved six of them (a pattern alone was no longer offered its spelling) and made one claim that the option did not bear out; it was not kept.
- Mutations, each restored byte for byte: a text with a wildcard printed for the names tried whatever stands beside it (the unit guard and the end-to-end one red); a pattern that is the whole filter treated like any other (the unit guard red, and eight more); a text of letters and alternatives printed only when it is the whole filter (the unit guard red, and five more).

## Files changed
- `crates/blit-core/src/case_fold.rs` — the rule in `would_also_match`; guard.
- `crates/blit-cli/tests/case_note.rs` — guard.

## Known gaps
- **For a pattern alone the spelling is still printed on the strength of the names tried.** The argument that nothing else can treat one of its names otherwise is an argument, not a check: `would_also_match` still refuses a spelling one of whose tried names the option does not match, and tries no others.
- The name printed is one that was tried (`AxB`, `IMG_x.JPG`): a made-up name that shows the change, not a file the run has seen.
- "Beside other patterns" is any second pattern in either list, whether or not it could touch the names in question; the line is then more cautious than it needs to be, never less.
- This is the line's thirteenth round. The owner's choice between the line as built and the plain rule alone (`docs/STATE.md`) is still open.
