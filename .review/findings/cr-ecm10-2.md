# cr-ecm10-2: The line judged includes and excludes apart

**Severity**: HIGH — the question named a loss that does not happen, while `--ignore-case` gives up another file the line did not mention
**Status**: Verified — `exclude-case-r11` (Codex openreview, 2026-11-21) reviewed `d3e33acc..bfa537f9`, the six `exclude-case-r10` fixes, with `guard_confirmed: true`: "Using both pattern lists … [is] also [a] sound choice." What building the second filter costs for a long include list is cr-ecm11-6
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `efe3321b`
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `exclude-case-r10` over `df682eea..41b4bfdd` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/exclude-case-r10.codex.md`. The first of four commits that complete cr-ecm9-2 (with cr-ecm10-1, -5, -6).

## Evidence
With `--exclude '[!a].bin' --exclude '[!b].txt' --include '*.txt'`, the prompt singled out the loss of `A` from `[!a].bin`. Both mirrors kept `A.bin`. The file that `--ignore-case` actually deleted was `B.txt`. The reviewer: each proposed example should pass through both rule lists in both modes before it gets priority in the prompt.

Cause. cr-ecm9-2 gave the line "the lists the filter holds" — and asked each list as a filter of its own (`Listed`): an exclude's claim against the excludes, an include's against the includes. What a run does with a name is decided by both: a file no include names is out of the run's scope whatever the excludes say of it.

## Predicted observable failure
Any command with both `--include` and `--exclude` where an exclude holds a negated class: the line names a letter whose files the run leaves alone either way, and can pass over the pattern whose loss is real. In the other direction, an include's "would also match" for a name an exclude takes out again under the option.

## Intake
Admitted as the reviewer put it.

## What
- **One filter, both lists** (`Complete`, in place of one `Listed` for each role): the run's complete filter as typed and — made only when something has to be asked of it — as the option would read it. The lists are the run's own, so the sets compiled for the run are the ones asked.
- **A loss is a file the RUN stops naming** (`Asked::lost`): the pattern names it as typed and not with the option, and so does the complete filter. A file, and not a folder of that name: an include says nothing of folders, so beside one an exclude can stop naming a folder whose files the run goes on leaving alone.
- **A gain is a name the RUN treats otherwise** (`Asked::gained`): for an exclude, the filter as typed does not already leave it out; for an include, the filter as the option would read it — excludes too — lets it in.

Through the binary: `--exclude '[!a].bin' --exclude '[!b].txt' --include '*.txt'` asks with `"[!b].txt" matches exactly as typed, and 2 more patterns match only as typed; capital letters matter (--ignore-case changes what they match: [!b] would stop matching B).`; both mirrors keep `A.bin`, and `B.txt` is kept as typed and deleted with the option.

## Guard proof
- `case_fold::tests::each_claim_is_checked_against_the_filter`: with those three patterns, nothing is lost at `[!a]` and `B` is at `[!b]`; an include `*.Txt` gains `x.txt` alone, and does not beside an exclude `*.TXT`.
- `case_fold::tests::the_case_note_claims_only_what_the_filter_bears_out`: the line above to the letter; an include alone offered `.txt, .TXT`; beside an exclude that takes them out again in every case, no spelling.
- `case_note.rs::the_line_claims_only_what_the_mirror_then_does`, through the binary: the reviewer's command — the line, then both mirrors and what each leaves. The guard takes includes now.
- Mutations, each restored byte for byte: the run's filter asked without its includes → all three guards red; an include's gain not put to the excludes read the same way → the first two; a folder the run stops naming counted as a file's loss → all three.

## Files changed
- `crates/blit-core/src/case_fold.rs` — `Complete` (in place of `Listed`, `WholeFilters`), `Asked::gained`, `Asked::lost`, `in_both_cases` (the two spellings, moved out of `Spellings::with` unchanged); the guards.
- `crates/blit-cli/tests/case_note.rs` — the case, and includes in the guard.

## Known gaps
- A change that happens only to FOLDERS is not shown as a loss; where a class could take less and no file shows it, cr-ecm10-1 keeps the warning.
- Names are tried at the top of the tree; cr-ecm10-1 is about what that can and cannot establish.
- The checks run under this machine's path rule. Run on Linux only.
