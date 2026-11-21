# cr-ecm10-5: The line printed a spelling it had not checked, and a count that changed with the order

**Severity**: MEDIUM — the question offered as "would also match" the very glob typed beside the pattern; the count of other patterns depended on where they stood
**Status**: Reviewed, found incomplete — `exclude-case-r11` (Codex openreview, 2026-11-21) reviewed `d3e33acc..bfa537f9`, the six `exclude-case-r10` fixes, with `guard_confirmed: true`. A spelling is now checked on the names it denotes — and the line then prints the spelling, not the name that passed: with `--include '*a' --exclude '[Aa]'` it said `would also match A`, both mirrors kept `A`, and the file the option deleted was `xA`. And the count still moves with the order once the patterns outnumber the budget (33 patterns: "1 more" or "2 more"). The remaining work is cr-ecm11-3 and cr-ecm11-5
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `b3eff1ef`
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `exclude-case-r10` over `df682eea..41b4bfdd` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/exclude-case-r10.codex.md`. The third of four commits that complete cr-ecm9-2.

## Evidence
With excludes `q[b-~]` and `Q[b-~]`, the prompt offered `Q[b-~]` as new even though the user had supplied that exact glob. `QB` was a genuine new match and would make an accurate example. The count also changed from two additional patterns to zero when `*.Doc` was moved from before `a, A` to after them. The reviewer: use the verified example directly and apply the same counting rule in either order.

Cause.
- **The spelling shown was checked beside names it does not denote.** cr-ecm9-2 asked, for a spelling, the names read off its text — and also the names read off the PATTERN, put in the spelling's case, so that `.O` (shown for `*.O`) would not fail on the one name `.O`. For `q[b-~]` that second set held `QB`, which the option does add. `Q[b-~]` denotes `Qb` … `Q~`, every one of them kept by the pattern typed beside it; `QB` is none of them.
- **Only the patterns typed AFTER the one shown were asked whether they read the same both ways.** One typed before it was counted unasked — or, when the search for a pattern to show had happened to ask it, not counted.

(The reviewer's own count, `*.Doc` before or after `a` and `A`, came right with cr-ecm10-1: `a` beside `A` is no longer called unchanged in one order and counted in the other. What is left is fixed here.)

## Predicted observable failure
A pattern with a class beside its own other-case spelling: the line offers that spelling as an additional match. A pattern that reads the same both ways (`{b,B}`) typed before the pattern shown: counted as "1 more pattern matches only as typed".

## Intake
Admitted as the reviewer put it. The example is not replaced by `QB`: no spelling of the pattern denotes it, and the line then states the rule.

## What
- **What is checked for a spelling is the whole pattern in that case, and the names IT denotes** (`Asked::would_also_match`): none of them may be a name the option does not match, and one of them must be a name the run would treat otherwise. The text shown is still the pattern without its end wildcards (`.O` for `*.O`); what is checked is `*.O`, which denotes `x.O` too.
- **Every pattern that could read the same both ways is asked, wherever it stands** (the first few in the order typed), and what is found of a pattern is kept by its place: shown, counted, or neither.

## Guard proof
- `case_fold::tests::the_case_note_claims_only_what_the_filter_bears_out`: `q[b-~]` beside `Q[b-~]` — the rule, no spelling; `*.Doc`, `a`, `A` in three orders — one line; `{b,B}` before and after `[!a].dat` — one line, nothing counted.
- `case_fold::tests::each_claim_is_checked_against_the_filter`: nine spellings offered or not on what they denote — `*/cache` for `*/Cache` offered, `cache` not (a level the pattern requires is gone); `Q[b-~]` offered for `q[b-~]` alone and not beside the pattern typed so, while `QB` is a name gained; `*.O` beside `??` offered, beside `*` not.
- `case_note.rs::the_line_claims_only_what_the_mirror_then_does`, through the binary: `q[b-~]` and `Q[b-~]` — the line, then `QB` deleted as typed and kept with the option, `Qb` and `qb` kept both ways.
- Mutations, each restored byte for byte: a spelling offered on a name it does not denote → all three guards red; only the patterns typed after the one shown asked about → the first.

## Files changed
- `crates/blit-core/src/case_fold.rs` — `Asked::would_also_match` (in place of `matched_only_ignoring_case`), `Spellings::with` (the whole pattern in each case), `case_note_of` (what was found of each pattern); the guards.
- `crates/blit-cli/tests/case_note.rs` — the case.

## Known gaps
- The line does not go looking for the name that changes (`QB`); where no spelling of the pattern is borne out it states the rule.
- A spelling with a wildcard in the middle is shown as a pattern (`img_*.jpg`), as the plan specifies; it is checked on the names it denotes with `*` as some text and as none.
- The checks run under this machine's path rule. Run on Linux only.
