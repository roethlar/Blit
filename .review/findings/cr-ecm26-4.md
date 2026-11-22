# cr-ecm26-4: a read-only partial made writable kept its new mode when its reopen failed

**Severity**: MEDIUM — an owned `0444` destination became `0600` when its writable reopen was refused (`EMFILE`)
**Status**: Fixed on Unix, awaiting review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix cr-ecm26-4`)
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `combined-r27` over `05ff50d7..e025a3e2` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r27.codex.md`. It completes cr-ecm25-4 (`a8283606`).

## Evidence
"On Linux, an owned `0444` destination became `0600` when the writable reopen failed with `EMFILE`. Keep rollback armed on the original descriptor until acquisition succeeds."

## Intake
Admitted as the reviewer put it.

## What
In `open_resume_partial_with`, a read-only partial made writable to be had gets its mode back through the read-only handle that changed it on every failure until it is had: the writable reopen refused, the file found changed, or its cleanup's handle refused. A test seam (`REOPEN_FAULTS`) refuses the reopen by path.

## Guard proof
- `resume::tests::a_partial_made_writable_and_not_had_gets_its_mode_back` (Unix, not as root): an owned `0444` destination, a `0444` source, the reopen refused — an error, the file `0444` with its bytes.
- Mutation, restored byte for byte: a refused reopen leaving the mode it was given (red).
- The resume, sink and session tests (324) pass. Windows cross-clippy clean.

## Files changed
- `crates/blit-core/src/copy/file_copy/resume.rs` — `open_resume_partial_with`, `reopen_fault`, its seam; guard.

## Known gaps
- None known.
