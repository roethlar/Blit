# cr-ecm15-2: the case line printed a spelling that reads as names the pattern has as typed

**Severity**: MEDIUM — the line names, as what `--ignore-case` would add, a text some names of which are copied either way
**Status**: Reviewed, found incomplete — `combined-r17` (Codex openreview, 2026-11-21) reviewed `13faafd2..7912da16` — the nine `combined-r16` fixes and local-copy-mode — with `guard_confirmed: true`. "For `--include '{a?,?b}'`, the CLI advertised `{A?,?B}` as an additional match. I verified that `Ab` was copied in both modes, while `Ax` was copied only with `--ignore-case`." A name can be had both ways through ANOTHER alternative, with no `*` in it. The remaining work is cr-ecm16-4
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `0db7e38c`
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `combined-r16` over `35adeacb..13faafd2` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r16.codex.md`. It completes cr-ecm14-5 (`33ac6700`).

## Evidence
"With the sole include `1*a*2`, the note presents `1*A*2` as what `--ignore-case` gains. I verified that `1aA2` already copied in both modes. The checked example `1xAx2` distinguishes them. A single-pattern filter still needs that distinction."

Cause. cr-ecm14-5 prints a pattern's spelling in the other case — in place of the one name that was checked — only where the names tried are all the text can be answered for: a text of letters and alternatives, and ANY text when the pattern is the run's whole filter, "there is then no other pattern to treat one of its names otherwise". There is one: the pattern itself, as typed. A `*` holds any text, the letter in the other case among it, while the pattern's own letter is found beside it: `1aA2` is `1`, `a`, and `A` in the second `*`.

## Intake
Admitted as the reviewer put it.

## What
`Asked` keeps the pattern's text as typed. `has_as_typed(shown, windows)` asks whether some name the text to be printed reads as is one the pattern has as typed: only a `*` can make one (every other part of the two texts stands at a fixed place, where their letters differ), so the names tried are the ones the text reads as with each `*` holding the pattern's whole text as typed, less what is no letter of a name — with that in every `*`, the pattern as typed finds each of its letters, in order, wherever it may look for them. A sole pattern's spelling is printed only where there is no such name; otherwise the name that was checked is.

What changes on the line, for a sole pattern: `1*a*2` shows `1xAx2` (was `1*A*2`); `1*aB*2` shows `1xabx2, 1xABx2`; `*a*b*` shows `AxB`; `**/*Cache*/**` shows `cachex/x, CACHEx/x` (was `cache*/**, CACHE*/**` — which reads as `cacheCache/x`). Unchanged, their letters beginning or ending the name or standing a fixed way from either: `IMG_*.jpg`, `a*b`, `x*{a,b}*y`, `dir/*a*`, `?a?`, `*.txt`, `*cache*`.

## Guard proof
- `case_fold::tests::a_spelling_that_reads_as_what_the_pattern_has_as_typed_is_not_printed`: the reviewer's pattern; letters of two cases as typed; two runs of letters between wildcards; five patterns whose spelling stands; and, put to the filter itself in both modes, that `1xAx2` is taken with the option and not without while `1aA2` is taken either way.
- `case_fold::tests::the_case_note_shows_only_spellings_ignore_case_matches` — one row CHANGED: `**/*Cache*/**` expected its spellings and now expects the names checked. That row's own point (the directory level a pattern requires stays in the text shown as typed) is asserted as before.
- Mutations, each restored byte for byte: a sole pattern's spelling printed whatever it reads as, as it was (both guards red); never printed when it holds a `*` (the new guard red, and three others); each `*` tried holding a text of none of the pattern's letters (the new guard red).
- The whole suite in the scratch copy before porting: 1,931 passed, 0 failed.

## Files changed
- `crates/blit-core/src/case_fold.rs` — `Asked::typed`, `has_as_typed`, `would_also_match`; guards.
- `CHANGELOG.md`.

## Known gaps
- Beside OTHER patterns a text with a wildcard or a class was already never printed for its names (cr-ecm14-5); nothing there changes.
- The search is by one text in every `*`: it finds a name had both ways wherever one exists through `*` alone, by the argument above, and that argument is not a proof over classes and alternatives standing between two `*` — a text such as `*[ab]*` is shown as `[AB]`, by the rule for a class (cr-ecm12-4), unchanged here.
- This is the line's fourteenth round. The owner's question — keep the line's design, or state the plain rule and no example — is open, and this fix does not answer it.
