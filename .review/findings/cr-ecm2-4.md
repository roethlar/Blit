# cr-ecm2-4: The class refusal advised a pattern that is refused in turn

**Severity**: LOW — a wrong hint, no wrong transfer: the user follows the message and is refused again
**Status**: Fixed, awaiting re-review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: the commit carrying this record ("Fix cr-ecm2-4: …")
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `exclude-slice1-r2` over `3bc3ee47..a1fd6bae` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/exclude-slice1-r2.codex.md`.

## Evidence
For `[aé][bß].txt` the refusal said `Write "{a,é}[bß].txt" instead` — which the next run refuses for its second class. `{a,é}{b,ß}.txt` passes. `class_beyond_ascii_refusal` (`crates/blit-core/src/fs_enum.rs`) rewrote the first such class and copied the rest.

## Predicted observable failure
A pattern with more than one class holding a letter beyond ASCII takes one failed run per class to correct.

## Intake
Admitted.

## What
The refusal rewrites every such class of the pattern, and advises a pattern only when it is one the filter accepts (no such class left, and a valid glob). When one of the classes has no rewrite — a range, a negated class — that class is the one the message names, with the advice to list the letters as alternatives.

## Guard proof
- `fs_enum::tests::a_class_holding_a_letter_beyond_ascii_is_refused`: four new rows — two classes (`{a,é}{b,ß}.txt`), three classes among an ASCII one (`éxüy[ab]ñ`), and two where a class with no rewrite follows or precedes a plain one (that class is named). For every row that advises a pattern, the advised pattern is validated in both case modes.
- `exclude_contract::a_class_holding_a_letter_beyond_ascii_refuses_the_run`: through the binary, `[aé][bß].txt` is refused advising `{a,é}{b,ß}.txt`, and that pattern is then accepted.
- Mutations, each restored byte for byte: only the first class rewritten → both guards red; a class with no rewrite rewritten anyway → both guards red.

## Files changed
- `crates/blit-core/src/fs_enum.rs` — `class_beyond_ascii_refusal` (now given the pattern and the platform rule, and returning nothing when no class is at fault); its caller; the guards.
- `crates/blit-cli/tests/exclude_contract.rs` — the end-to-end rows.

## Known gaps
- Run on Linux only.
