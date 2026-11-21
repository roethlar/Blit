# cr-ecm7-2: The line's example dropped a doubled separator

**Severity**: HIGH — the line said `--ignore-case` would also match a name it does not; a mirror run on that reading deleted it
**Status**: Verified — `exclude-case-r8` (Codex openreview, 2026-11-21) reviewed `3bc3ee47..f4203c31`, which holds this fix, with `guard_confirmed: true`, and required nothing further on this finding
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `18d78c45`
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `exclude-case-r7` over `3bc3ee47..cbab0c39` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/exclude-case-r7.codex.md`.

## Evidence
For `--exclude '**//Cache'`, the prompt said `--ignore-case` would also match `cache`. A real mirror with that option deleted the destination-only `cache` and exited successfully. The reviewer: the formatter strips every separator after `**`; keep the full glob whenever shortening it would change what the example names.

Cause: the rule that makes the shown text trims `**/` from the start — `**/Cache` names `Cache` at any depth, the top included — and it trimmed every separator that followed (`trim_start_matches`). The second separator of `**//Cache` is the pattern's own. With it the pattern asks for a component with no name before `Cache`; no entry has one, so the pattern names nothing, and `Cache` names every entry called that.

## Predicted observable failure
`mirror --exclude '**//NAME'`: the line shows `NAME` and says the option would also match its other spellings; the pattern protects nothing with the option or without.

## Intake
Admitted.

Where this stood when the commit was made: cr-ecm7-4, the commit before, already stopped the false claim — it puts each spelling to the filter, `cache` and `CACHE` are not matched with the option, both are dropped, and the line falls to words. Its check on the shown text is the reviewer's request as written (a shortened text that names anything the pattern does not is not shown; the pattern is shown whole). What was left for this commit: the rule itself, still wrong wherever it stands unchecked, and nothing that pinned the case.

## What
- The rule trims one separator after `**`, and no more: `**//Cache` is `/Cache` by rule, `**///Cache` is `//Cache`.
- The line for `**//Cache` — and for `//Cache` and `Cache//**` — offers no spelling: `"**//Cache" matches exactly as typed; capital letters matter (--ignore-case reads it otherwise).` No entry is called `/Cache`, so there is no name to bear a spelling out with, and what cannot be asked is said in words.
- The rules are now run ALONE in the shape guard, beside the checked result (`Spellings::by_rule`): since cr-ecm7-4 a rule's mistake costs only a plainer line, and a guard that looked at the checked result alone would never see the rule go wrong.

Measured against rsync 3.5.2, which reads a doubled separator the same way: `--exclude '**//Cache'`, `'//Cache'` and `'sub//Cache'` each excluded nothing (`Cache/` and `sub/Cache/` were copied); `'**/Cache'` excluded both. blit says nothing about such a pattern either.

## Guard proof
- `case_fold::tests::every_spelling_the_line_shows_names_only_what_the_filter_names`: the shapes are now twelve beginnings (`**//`, `//`, `/`, `./`, `***/` added), four middles and nine endings (`//**`, `/` added) — 432 patterns — each taken by rule alone and as the line shows it, against the real `FileFilter` as an exclude, over twenty-six names as a file and as a folder. And the rule to the letter: `**//Cache` → `/Cache`, `**///Cache` → `//Cache`, `**/Cache` → `Cache`, and under the Windows rule `**\\Cache` → `\Cache`.
- `case_fold::tests::the_case_note_claims_only_what_the_filter_bears_out`: the line to the letter for `**//Cache`, `//Cache`, `Cache//**`.
- `case_note::the_line_claims_only_what_the_mirror_then_does`, through the binary: the question's line for `--exclude '**//Cache'` to the letter, then both mirrors — as typed and with `--ignore-case` alike, the destination's `cache` and `Cache` are both deleted. The line promises neither.
- Mutations, each restored byte for byte: every separator after `**` trimmed again → the shape guard red (by rule, `**//Cache` is shown as `Cache`, which names `Cache`); a spelling offered on trust → the other two red; a claim checked with any text, a walk's name or not → the other two; no name to ask with taken for no change → the other two.

## Files changed
- `crates/blit-core/src/case_fold.rs` — `without_end_wildcards`; `Spellings::by_rule`; the guards.
- `crates/blit-cli/tests/case_note.rs` — the end-to-end case.

## Known gaps
- With the checks in place the rule's correction is seen only where the rules stand alone: the guard's by-rule half, and the other platform's path rule (reachable only from tests).
- A pattern with a doubled separator protects nothing and blit accepts it without a word — as rsync does. Whether it should be refused or warned about, like an exclude that names nothing under the source, is not decided here.
- The shape guard covers excludes, on Linux, with ASCII names (cr-ecm6-2's gaps stand).
