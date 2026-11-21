# cr-ecm11-4: The line showed an example for a pattern `--ignore-case` cannot read

**Severity**: MEDIUM — the question recommends, by example, an option that refuses the same command
**Status**: Reviewed, found incomplete — `exclude-case-r12` (Codex openreview, 2026-11-21) reviewed `4d0f1a68..c5bab07a`, the six `exclude-case-r11` fixes, with `guard_confirmed: true`. A pattern that cannot itself be read with the option gets the rule; a pattern BESIDE one still gets its example — `*Cache` with the 249-brace pattern was offered `cache, CACHE`, and that command with `--ignore-case` is refused whole. This fix's guard pins that line. The remaining work is cr-ecm12-5
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `c2083493`
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `exclude-case-r11` over `d3e33acc..bfa537f9` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/exclude-case-r11.codex.md`. It completes cr-ecm10-1.

## Evidence
"An `s` inside 249 nested brace pairs was accepted as typed and produced an uppercase example. The same `copy --dry-run --ignore-case` exited 1 during filter validation. A failed probe should select the general wording."

Cause. Every claim the line makes is put to the filter — two filters holding the one pattern, as typed and as the option would read it. When they could not be built, the code fell back to what it does under the OTHER platform's path rule, where this machine's filter cannot be asked at all: the spellings made by rule, unchecked. A pattern too deep to widen is exactly one whose second filter cannot be built.

## Intake
Admitted as the reviewer put it.

## What
`Spellings::of_role`: when the pattern cannot be put to the filter in both readings, nothing is shown of it but the rule (`matches exactly as typed; capital letters matter (--ignore-case ignores them).`). The rules stand unchecked in one case only, the other platform's path rule.

## Guard proof
- `case_fold::tests::a_pattern_the_option_cannot_read_gets_the_rule_and_no_example`: the reviewer's pattern is one as typed and refused with the option; its line is the rule; beside `*Cache`, `*Cache` is shown with its spellings; eight pairs of braces, which both readings compile, still get a checked example.
- Mutation, restored byte for byte: a pattern that cannot be asked about shown by rule, as before → the guard red.

## Files changed
- `crates/blit-core/src/case_fold.rs` — `Spellings::of_role`; the guard.

## Known gaps
- The line still says `--ignore-case ignores them` of a pattern the option would refuse: it is the rule, and the refusal is the option's to make.
- Run on Linux only.
