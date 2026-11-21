# cr-ecm11-5: The count of other patterns still moved with their order

**Severity**: MEDIUM — the same command, its patterns in another order, says "1 more pattern" or "2 more patterns"
**Status**: Reviewed — `exclude-case-r12` (Codex openreview, 2026-11-21) reviewed `4d0f1a68..c5bab07a`, the six `exclude-case-r11` fixes, with `guard_confirmed: true`, which required nothing further on this finding
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `48a14aa8`
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `exclude-case-r11` over `d3e33acc..bfa537f9` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/exclude-case-r11.codex.md`. It completes cr-ecm10-5.

## Evidence
"In a 33-pattern file containing `Visible.txt`, 31 case-equivalent alternatives, and `{z,2}`, moving `{z,2}` from last to second changed '1 more pattern' to '2 more patterns.' Use a neutral count of the remaining configured patterns that can be computed independently of the example-search budget."

Cause. The count left out a pattern whose two readings were proved the same (`{a,A}`). Proving it means asking, and only so many patterns are asked about (cr-ecm10-6): a pattern past that was counted unasked. Which patterns fell inside the budget depended on where they stood.

## Intake
Admitted as the reviewer put it. It reverses one clause of cr-ecm7-4 — a pattern that reads the same both ways is no longer left out of the COUNT. It is still never the pattern shown.

## What
The patterns counted are every other pattern the option could read otherwise by its TEXT — a letter in it that has another case, outside a class that holds both — which is what was typed and costs nothing to tell. Nothing asked of the filter enters the count, so neither the order nor the number of patterns can change it. The loop that asked the counted patterns is gone, and its questions with it.

`--exclude a --exclude A --exclude '{b,B}'` now says `2 more patterns` where it said `1 more`.

## Guard proof
- `case_fold::tests::the_case_note_claims_only_what_the_filter_bears_out`: the reviewer's 33 patterns with `{z,2}` second, eighteenth and last — one line, `32 more patterns`; `{b,B}` before and after `[!a].dat` — one line, `1 more pattern`; `{a,A}` beside `*.Doc`, both orders — `1 more pattern`; `{b,B}` with `[Cc]` — still no line.
- `case_note.rs::the_line_claims_only_what_the_mirror_then_does`, through the binary: `{a,A}` and `*.Doc`.
- Mutation, restored byte for byte: a pattern proved the same left out of the count when it was asked about, as before → both guards red.

## Files changed
- `crates/blit-core/src/case_fold.rs` — `case_note_of` (the count); the guards.
- `crates/blit-cli/tests/case_note.rs` — one line's count.

## Known gaps
- **The count includes patterns the option reads no differently** (`{a,A}`): "N more patterns match only as typed" is the rule said of each, and is true of each; it is not a count of patterns the option would change.
- Which pattern is SHOWN still depends on the order when more than sixteen stand before the first that has something to show.
- Run on Linux only.
