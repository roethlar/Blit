# cr-ecm24-7: a directory that can be searched but not listed was refused a fresh partial

**Severity**: MEDIUM — a resume that replaces a destination in a directory this run may write and search but not list (0300) failed with `EACCES`; it succeeded before cr-ecm23-3, which brought this in
**Status**: Fixed on Unix, awaiting review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix cr-ecm24-7`)
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `combined-r25` over `c89b1eb1..19025839` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r25.codex.md`. It completes cr-ecm23-3 (`6bf9c2f9`).

## Evidence
"A fresh replacement in a `0300` directory succeeded at base and failed with `EACCES` at head. The operation needs write and search permission. The new `O_RDONLY` open also demands permission to list the directory. Use the existing `TO_SEARCH` flags."

## Intake
Admitted as the reviewer put it.

## What
`held::directory_unlinked` opens the directory it holds with `TO_SEARCH` — `O_PATH` on Linux, `O_SEARCH` where the system has it — as the passages on the way are opened. Making, naming and removing a file in it asks only that it be written and searched.

## Guard proof
- `resume::tests::a_fresh_partial_is_made_in_a_directory_that_cannot_be_listed` (Unix, not as root): a `0644` destination in a `0300` folder resumed from a `0600` source — the source's bytes at the name.
- Mutation, restored byte for byte: the directory held to be read (red).
- The resume, sink and session tests (303) pass. Windows cross-clippy clean.

## Files changed
- `crates/blit-core/src/held.rs` — `directory_unlinked`.
- `crates/blit-core/src/copy/file_copy/resume.rs` — guard.

## Known gaps
- Where the system has neither `O_PATH` nor `O_SEARCH`, `TO_SEARCH` is `O_RDONLY`, as it already is for the passages on the way.
