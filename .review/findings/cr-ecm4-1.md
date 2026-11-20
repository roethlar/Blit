# cr-ecm4-1: The pattern-file refusal advised a spelling that names other entries

**Severity**: HIGH — a user who follows the message has a mirror delete the very entry the message said how to keep
**Status**: Verified — `exclude-case-r5` (Codex openreview, 2026-11-20) reviewed `3bc3ee47..585d3599`, which holds this fix, with `guard_confirmed: true`, and required nothing further on this finding
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `004ac102`
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `exclude-case-r4` over `3bc3ee47..0669279f` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/exclude-case-r4.codex.md`.

## Evidence
For the pattern-file line `- [1].jpg`, blit refused the run and said: to name an entry really called `- [1].jpg`, write `[-] [1].jpg`. The reviewer followed that in a mirror: the literal `- [1].jpg` was deleted and `- 1.jpg` kept. `[-] [[]1[]].jpg` keeps the intended file.

Cause: `rsync_rule` (`crates/blit-core/src/fs_enum.rs`, slice 3b) put the sign in a class and copied the rest of the line as it was. Whatever glob syntax the name held stayed syntax: `[1]` a class, `*` a wildcard.

## Predicted observable failure
For a rule line whose name holds `*`, `?`, `[`, `]`, `{` or `}`, the advised spelling is a glob that names other entries and, for a class, not the entry itself.

## Intake
Admitted.

## What
The advised spelling writes the whole name literally: the sign in a class, and the rest through the one encoder that already writes a disk's names as patterns (`literal_pattern`, moved to `fs_enum.rs`; `transfers::filter`'s `literal_glob` is now a call to it). `- [1].jpg` → `[-] [[]1[]].jpg`; `- *.jpg` → `[-] [*].jpg`.

## Guard proof
- `fs_enum::tests::an_rsync_rule_in_a_pattern_file_is_refused_with_its_line`: seven rule lines with the spelling advised for each; each spelling validates, names the entry, and names none of six look-alikes (the name with a wildcard filled in, a class read as one of its members).
- `pattern_files::an_rsync_rule_line_refuses_the_run_and_a_class_names_the_real_entry`: the reviewer's sequence through the binary — the refusal advises `[-] [[]1[]].jpg`; the file rewritten with it, the mirror keeps `- [1].jpg` and deletes `- 1.jpg`.
- Mutation (the rest of the line copied as it is): both guards red; restored byte for byte.

## Files changed
- `crates/blit-core/src/fs_enum.rs` — `rsync_rule`, `literal_pattern`; the guard's rows.
- `crates/blit-core/src/transfers/filter.rs` — `literal_glob` calls the one encoder.
- `crates/blit-cli/tests/pattern_files.rs` — the end-to-end sequence.

## Known gaps
- The message tells a user how to name an entry really called like the rule. It does not say what an rsync user most likely meant by `- *.jpg` — the bare pattern `*.jpg` on its own line.
- Run on Linux only.
