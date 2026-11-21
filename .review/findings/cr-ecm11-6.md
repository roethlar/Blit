# cr-ecm11-6: One question built the run's whole filter a second time

**Severity**: MEDIUM — FAST: one line of a prompt took 120 ms of a command that takes 38
**Status**: Fixed, awaiting review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix cr-ecm11-6`)
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `exclude-case-r11` over `d3e33acc..bfa537f9` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/exclude-case-r11.codex.md`. It completes cr-ecm10-6.

## Evidence
"A one-file release dry run with 10,000 literal includes took a median **217 ms** with the head note, **64 ms** with the base note implementation, and **52 ms** with explicit `--match-case`, over five runs each. One candidate triggers compilation of the widened list. Use the general wording when that compilation would exceed the budget."

Measured here (release, one-file `copy --dry-run`, medians of 7, N literal `--include` patterns; the line / `--match-case`):

| N | before | after | `--match-case` |
| --- | --- | --- | --- |
| 64 | 4.6 ms | 4.6 ms | 3.7 ms |
| 256 | 7.0 ms | 5.4 ms | 4.2 ms |
| 1,000 | 18.8 ms | 8.4 ms | 6.6 ms |
| 10,000 | 160.2 ms | 47.7 ms | 38.5 ms |

Cause. cr-ecm10-6 bounded how many patterns are ASKED ABOUT. An include's example is asked of the run's whole filter as the option would read it (cr-ecm10-2) — the one filter here the run does not build for itself — and that filter was built, whole, the first time it was asked anything: about 12 microseconds a pattern.

## Intake
Admitted as the reviewer put it.

## What
`Complete::ignoring` is `None` when the run's two lists hold more than `MOST_PATTERNS_WIDENED` (64) patterns: about one millisecond. Past that nothing is asked of it and what it would have settled is not claimed — an include's example is not shown (the line states the rule), and a class that could take less is said to with `may`.

What is left at 10,000 patterns, 9 ms over `--match-case`, is one pass over the patterns' text; it was there before the line checked anything.

## Guard proof
- `case_fold::tests::the_line_does_not_build_a_long_filter_a_second_time`: 10,000 includes — the filter built again zero times, the line the rule and a count of 9,999; 64 includes — built once, the example shown; 64 includes and `[!a]*.dat` — not built, `[!a] may stop matching A`.
- Mutations, each restored byte for byte: the whole filter built again however long, as before → the guard red; a loss that cannot be asked about taken as shown → red.

## Files changed
- `crates/blit-core/src/case_fold.rs` — `Complete::ignoring`, `MOST_PATTERNS_WIDENED`, `Asked::gained`, `Asked::lost`; the guard.

## Known gaps
- Past 64 patterns an include gets no example and a narrowing class gets `may` where a name could have shown the loss.
- The bound is in patterns, not in time.
- Run on Linux only.
