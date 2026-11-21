# cr-ecm10-6: The line's cost grew with the number of patterns

**Severity**: MEDIUM — FAST: one line of a prompt took 400 ms of a command that takes 10
**Status**: Reviewed, found incomplete — `exclude-case-r11` (Codex openreview, 2026-11-21) reviewed `d3e33acc..bfa537f9`, the six `exclude-case-r10` fixes, with `guard_confirmed: true`: "bounding the work spent on the note [is] also [a] sound choice." The bound counts patterns asked about, and one question builds the whole filter as the option would read it: 10,000 literal includes, 217 ms with the line, 64 ms with the line as it was before this round, 52 ms with `--match-case`. The remaining work is cr-ecm11-6
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `d2f2b32d`
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `exclude-case-r10` over `df682eea..41b4bfdd` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/exclude-case-r10.codex.md`. The last of four commits that complete cr-ecm9-2.

## Evidence
A one-file release `copy --dry-run` with 1,000 `archiveNNN.tmp` exclusions and the covering glob `*.[Tt][Mm][Pp]` rose from 31 ms at base to 443 ms at head, median of five runs each. The reviewer: apply a comparison budget across the command and use the general case warning when it is exhausted.

Measured here (release, median of 9, the reviewer's command): 10.3 ms at `a13238b6`, **412.7 ms** before this round's fixes, **10.9 ms** after.

Cause. The line looks for a pattern it can show something about, in the order typed, and stopped at the first. When none has anything to show — every spelling covered by another pattern — it asked about every pattern in turn: two filters built for each, and, since cr-ecm9-2, a plain name put to the filters in every case of its letters (`archive000.tmp`: 1,024 spellings) to find out whether the option changes anything.

## Predicted observable failure
A long exclude list in which the first patterns' spellings are covered by a later one (a generated list beside a hand-written glob): the prompt, or `copy`'s note, takes hundreds of milliseconds, growing with the list.

## Intake
Admitted as the reviewer put it.

## What
- **A fixed number of patterns are asked about** (`MOST_PATTERNS_ASKED`, 16) for what to show — the first that could narrow, the first of all — and as many again for whether they read the same both ways. Past that, the line is the plain rule about the first pattern not asked about, and the rest are counted by their text, as plain text and wildcards always were.
- **Only a pattern that spells out alternatives is compared spelling by spelling.** Plain text cannot read the same both ways, so there is nothing to find out; it was being compared for an example in a case nobody typed.

## Guard proof
- `case_fold::tests::the_line_asks_a_bounded_number_of_questions`: the reviewer's 1,001 patterns — 256 questions put to a filter, under a bound of 2,000 (a thousand patterns asked about, or sixteen compared in full, are tens of thousands); the line is the rule with a count of 999. And the 1,000 without the covering glob: 14 questions, and the first pattern shown with its spelling as ever.
- Mutations, each restored byte for byte: every pattern asked about for what to show → the guard red; plain text compared spelling by spelling → red.
- Speed, `scratchpad` bench (release, one-file `copy --dry-run`): the reviewer's command 412.7 → 10.9 ms; 1,000 excludes with no covering glob 8.5 → 8.7 ms; 250 mixed excludes 26.9 → 26.8 ms; 50, 9.7 → 9.6 ms.

## Files changed
- `crates/blit-core/src/case_fold.rs` — `case_note_of` (the two searches bounded; the fallback), `Spellings::in_words`, `Spellings::with` (what is compared), the question counter (tests only); the guard.

## Known gaps
- Past the sixteenth pattern nothing is asked: a pattern further down that the option would narrow is counted and not shown, and a pattern there that reads the same both ways is counted too.
- A plain-text pattern whose lower and upper spellings are both covered gets the rule where it could have had an example in a mixed case (`KEep`).
- The bound is in patterns, not in time; a single pattern of many alternatives is still compared up to 1,024 spellings.
- Run on Linux only.
