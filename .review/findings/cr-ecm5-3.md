# cr-ecm5-3: The excludes were resolved for the question and again for the run

**Severity**: HIGH — the mirror the operator confirmed was not the one the question described: it copied the entry shown as excluded (and could delete a destination entry shown as kept)
**Status**: Fixed, awaiting re-review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: the commit carrying this record ("Fix cr-ecm5-3: …")
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `exclude-case-r5` over `3bc3ee47..585d3599` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/exclude-case-r5.codex.md`.

## Evidence
An absolute exclude through a source alias was displayed as `first/Keep`. When the alias switched to `second` while the question was open, the confirmed mirror copied `first/Keep`. Re-resolving the pattern can likewise expose a destination-only `first/Keep` to deletion.

Cause: resolving an absolute `--exclude` reads the disk (a source reached through a link, the names the source stores), and every user of the excludes resolved for itself — the early validation, the warnings and the line in the question (cr-ecm4-3 made that line describe the resolved pattern), and each filter built at dispatch (`filter::build` / `build_spec`, `crates/blit-core/src/transfers/filter.rs`), the last after the answer.

## Predicted observable failure
`blit mirror --exclude /alias/Keep SRC/ DST/` where `/alias` is a link to a folder in the source: if the link's target changes between the question and the answer, the run excludes what the link names now, not what the question showed. The same for every retry of a command, minutes later.

## Intake
Admitted.

## What
The excludes are resolved once, when the command begins (`run_command`, after its pattern files are read), and the result travels with the command (`TransferArgs.resolved_excludes`). The validation, the warnings, the question's line, every filter built from the command and every retry of it use that one resolution: `FilterInputs.resolved` hands it to `filter::build`, `build_spec` and `validate`, which then read no disk for the excludes. `check` passes the resolution it has just reported on. A caller that hands none is resolved on the spot, as before.

## Guard proof
- `transfers::filter::tests::a_filter_is_built_from_the_resolution_it_is_handed` (every OS): an exclude that, resolved on the spot, matches nothing; handed the resolution `./first/Keep`, the filter excludes exactly that, the wire spec carries it, and validation judges it and not the typed text.
- `case_note::the_filter_the_question_showed_is_the_filter_that_runs` (Unix; an alias is a symbolic link): the reviewer's sequence through the binary — the question shows `matches first/Keep only`; the alias is pointed at `second`; "y"; `first/Keep` is left out and `second/Keep` is copied.
- Mutations, each restored byte for byte: a filter resolving again whatever it is handed → both guards red; the command not handing its filters the resolution → the second red; the command resolving nothing when it begins → the second red.

## Files changed
- `crates/blit-core/src/transfers/filter.rs` — `FilterInputs.resolved`, `resolved_patterns`; the guard.
- `crates/blit-cli/src/cli.rs` — `TransferArgs.resolved_excludes` (never a flag).
- `crates/blit-cli/src/transfers/mod.rs` — `resolve_excludes_once`; the warnings and the filter inputs use it.
- `crates/blit-cli/src/main.rs` — resolved when the command begins.
- `crates/blit-cli/src/check.rs` — the resolution reported is the one used.
- `crates/blit-cli/tests/case_note.rs` — the end-to-end guard.

## Known gaps
- The destination's own spelling of each excluded entry is still read when the deletions are planned, after the copy (cr-g3fix4-2); that reading is of the destination as it then stands, by design.
- A `--files-from` list is read when the filter is built, after the question; a list file changed in between is read as changed. Not this finding; not measured.
- A saved job resolves when it is RUN, against the disk of that day.
- The end-to-end guard cannot run on Windows; run on Linux only.
