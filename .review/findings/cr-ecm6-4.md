# cr-ecm6-4: The `--files-from` list was read again after the question

**Severity**: MEDIUM — the run and its own saved job disagreed about which files the command named
**Status**: Verified — `exclude-case-r7` (Codex openreview, 2026-11-21) reviewed `3bc3ee47..cbab0c39`, which holds this fix, with `guard_confirmed: true`, and required nothing further on this finding (it found the separators of a listed name sent as the sending platform writes them — cr-ecm7-1)
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `21b6469e`
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `exclude-case-r6` over `3bc3ee47..a13238b6` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/exclude-case-r6.codex.md`.

## Evidence
The reviewer changed `--files-from` from `first.txt` to `second.txt` while the confirmation was open. The mirror copied `second.txt`. Its saved job replayed `first.txt`.

Cause: `run_command` reads the list once before the transfer starts, for the job's record (`capture_files_from`, review cr-jl5-1), and keeps the text with the command. The filter did not use it: `filter::build` and `build_spec` (`crates/blit-core/src/transfers/filter.rs`) opened the file again when they were called, at dispatch — after the question, and again on every retry pass. cr-ecm5-3 closed the same gap for the excludes and recorded this one as a known gap.

## Predicted observable failure
`blit mirror --files-from LIST …` (or `copy`, with `--save` or without): a list edited between the start of the command and the answer to its question, or before a later retry, is the list that runs; the record and any saved job hold the earlier one.

## Intake
Admitted.

## What
The list the command read when it began is the list every filter of that command is built from: `FilterInputs.files_from_read` carries the content (or why it could not be read), and `build` / `build_spec` then do not open the file. A list that could not be read when the command began is refused in the same words, and is not tried again. A caller that hands no content — `check`, a program built on the library — has the file read where the filter is built, as before.

## Guard proof
- `transfers::filter::tests::a_list_is_built_from_what_its_caller_read` (every OS): a file that now says `second.txt` and a content read as `first.txt` — with the content given, the filter and the wire spec list `first.txt`; with none, the file; a list that could not be read is the error and the file is not tried; content alone, naming nothing, is a list in force.
- `case_note::the_list_read_before_the_question_is_the_list_that_runs` (every OS), through the binary: the reviewer's sequence — the list rewritten while the question is open, "y", and `first.txt` is copied and `second.txt` is not; `jobs run` of the saved job copies the same file.
- Mutations, each restored byte for byte: the file opened again whatever the caller read → both red; a list the caller could not read tried again → the first red; the command not handing its filters the list → the second red.

## Files changed
- `crates/blit-core/src/fs_enum.rs` — `FileFilter::files_from_text`.
- `crates/blit-core/src/transfers/filter.rs` — `FilterInputs.files_from_read`, `listed_names`; the guard.
- `crates/blit-cli/src/transfers/mod.rs` — the filter inputs carry the list read.
- `crates/blit-cli/src/check.rs`, `crates/blit-cli/src/jobs.rs` — the new field.
- `crates/blit-cli/tests/case_note.rs` — the end-to-end guard.

## Known gaps
- The list is read by `read_to_string`, so a list that is not UTF-8 is refused whole where it used to be refused at its first bad line; and the byte-order-mark question in `docs/STATE.md` stands as it was.
- Pattern files (`--exclude-from`, `--include-from`) were already read once, when the command begins (slice 3b).
- Run on Linux only.
