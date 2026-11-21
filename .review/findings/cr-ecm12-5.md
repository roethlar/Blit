# cr-ecm12-5: A pattern beside the one shown could make the advertised alternative impossible

**Severity**: MEDIUM — the question offers, by example, an option that refuses the same command
**Status**: Reviewed, found incomplete — `exclude-case-r13` (Codex openreview, 2026-11-21) reviewed `c99ad483..bd84649f`, the five `exclude-case-r12` fixes, with `guard_confirmed: true`. The filter that is validated leaves out an absolute exclude that lies outside the source, and the command validates it: `*Cache` beside such a pattern, an `s` inside 249 brace pairs, was offered `cache, CACHE`, and the command with `--ignore-case` exits 1. The remaining work is cr-ecm13-6
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `8f1fe40e`
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `exclude-case-r12` over `4d0f1a68..c5bab07a` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/exclude-case-r12.codex.md`. It completes cr-ecm11-4.

## Evidence
"With `*Cache` and a second exclude consisting of `s` inside 249 brace pairs, the default note offered `cache` and `CACHE`. The same command with `--ignore-case` exited 1 during filter validation. Require the complete alternative filter to validate before making a concrete claim about that run. The new test currently expects the misleading example."

Cause. cr-ecm11-4 gave the rule to a pattern that cannot itself be read with the option. Whether the RUN can be read with it depends on every pattern, and no other pattern was asked.

## Intake
Admitted as the reviewer put it.

## What
Before anything is asked of any pattern, the run's whole filter as the option would read it is validated (`Complete::with_the_option`):
- it reads: the line as before;
- the option would refuse it: no pattern is asked about, and the line is the rule about the first pattern;
- it holds more than 64 patterns and is not built (cr-ecm11-6), so it is NOT KNOWN: nothing concrete is said either — no example, for an exclude or an include — and a class that would take less by its text is still named, with `may` (cr-ecm10-1: a warning that protection may shrink stays wherever it cannot be ruled out).

So a command with more than 64 patterns gets the rule, or `may`, and never an example. 1,000 excludes with nothing covering them had `would also match ARCHIVE000.TMP`; they now have the rule.

Speed (release, medians of 7): 64 includes 4.5 ms with the line (4.5 before, 3.7 without it); 10,000 includes 46.3 ms (48.3 before, 38.3 without it).

## Guard proof
- `case_fold::tests::a_pattern_the_option_cannot_read_gets_the_rule_and_no_example`: `*Cache` beside the 249-brace pattern, in both orders — the rule about the first, a count of one; `*Cache` alone keeps its example.
- `case_fold::tests::the_line_asks_a_bounded_number_of_questions`: 1,000 excludes, covered or not — the rule; no question put to a filter.
- `case_fold::tests::the_line_does_not_build_a_long_filter_a_second_time`: past the bound a class that could take less is still named, with `may`.
- Mutations, each restored byte for byte: a filter the option would refuse taken for one it reads; patterns asked about though the whole filter was not read; past the bound a class that could take less not named — three, each red on the guards named for it.

## Files changed
- `crates/blit-core/src/case_fold.rs` — `Complete::with_the_option`, `WithTheOption`, `case_note_of`; the guards.
- `CHANGELOG.md`.

## Known gaps
- **Past 64 patterns the line never shows an example**, where for excludes alone it used to.
- Validating means building the second filter for every line within the bound: about a millisecond at 64 patterns.
- Run on Linux only.
