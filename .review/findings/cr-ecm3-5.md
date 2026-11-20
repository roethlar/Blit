# cr-ecm3-5: The line about capital letters did not follow what the matcher does

**Severity**: MEDIUM — the line exists so that an operator knows, before a mirror deletes, how a pattern will be read; it was silent for some patterns whose reading depends on case, and spoke of patterns that named nothing
**Status**: Verified — `exclude-case-r4` (Codex openreview, 2026-11-20) reviewed `3bc3ee47..0669279f`, which holds this fix, with `guard_confirmed: true`, and required nothing further on this finding (it found the line still wrong for a negated class beside other letters, and for an exclude already reported as unmatchable — cr-ecm4-3)
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `9408536f`
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `exclude-case-r3` over `3bc3ee47..467773b4` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/exclude-case-r3.codex.md`.

## Evidence
The reviewer ran the built head:
- `*[a-z]` and `*ß` produced no line, although as typed a mirror deleted `TEST` and `CAPẞ` and `--ignore-case` kept them.
- `copy --exclude '*.cfg' --files-from LIST` printed the line about `*.cfg` while the listed `picked.cfg` was copied: a list of names overrides the patterns.
- A pattern typed with a line break made the "one line" two.

Cause: slice 2 decided which patterns to speak of by its own rule — a letter outside a class whose standard-library other case is one letter the option pairs it with — and not by asking the matcher. A class was copied and never counted as a letter (but `--ignore-case` gives a class its letters' other case); `ß` has no single upper form in the standard tables (but the option pairs it with `ẞ`). Slice 2's record states both as its readings.

## Predicted observable failure
An operator who types `--exclude '*[a-z]'` is not told that capital letters matter; one who uses `--files-from` is told about patterns that do nothing; a pattern holding a control character breaks the line.

## Intake
Admitted, all three parts.

## What
- A pattern is spoken of exactly when ignoring case would change what it names: when the pattern widened for `--ignore-case` differs from the pattern typed (`crates/blit-core/src/case_fold.rs`, `Spellings::of`). That is the matcher's own answer.
- The spellings shown come from the same pairs the matcher uses: each letter in the case the option pairs it with (`ẞ` for `ß`; `STRAẞE`, never `STRASSE`), a class as the class of its letters' other case (`[A-Z]` for `[a-z]`). A class that already takes both cases is the same in every spelling.
- Where ignoring case changes the reading in a way no spelling shows — a negated class takes LESS, a range across both cases — the line says so without an example: `"*[!a]" matches exactly as typed; capital letters matter (--ignore-case reads it otherwise).`
- No line when a list of names overrides the patterns: `--files-from`, and the list a retry sends again.
- The line is one line: a control character in a pattern or a file's name is written as its escape.

## Guard proof
- `case_fold::tests::the_case_note_follows_what_the_matcher_does`: the reviewer's two patterns to the letter; the form without an example, alone and with a count; nine patterns each noted exactly when widening changes them; and for six patterns every spelling shown is a name the widened pattern matches.
- `case_fold::tests::the_case_note_is_one_line_whatever_the_pattern_holds`.
- Slice 2's two unit guards, corrected: `[abc]` is noted (`[ABC]`), `straße` shows `STRAẞE`.
- Through the binary (`case_note.rs`): `the_line_speaks_of_every_pattern_whose_reading_depends_on_case` (the line for `*[a-z]` and `*ß` in the question, the entry deleted as typed and kept with the option), `no_line_when_a_list_of_names_overrides_the_patterns`, `a_pattern_holding_a_line_break_still_makes_one_line`.
- Mutations, each restored byte for byte: a pattern with no spelling to show passed over; a class the same in every spelling; a letter taking only the standard library's other case; a pattern that does not depend on case spoken of; a control character echoed as it is; a list of names not silencing the line — each red in the guards named for it.

## Files changed
- `crates/blit-core/src/case_fold.rs` — `Spellings::of`, `in_case`, `class_in_case`, `one_line`, the builder's two forms; guards.
- `crates/blit-cli/src/transfers/mod.rs` — no line under a list of names.
- `crates/blit-cli/tests/case_note.rs` — three end-to-end guards.

## Known gaps
- With `--retry N`, a whole rerun after a transient failure prints the note again with that attempt's banner (and `mirror` asks its question again, as it always has).
- Run on Linux only.
