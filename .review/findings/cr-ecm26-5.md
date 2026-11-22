# cr-ecm26-5: a FIFO at the name could block the resume's open

**Severity**: MEDIUM — a FIFO at the destination that this run could only read reached an `O_RDONLY` open, which waits for a writer, and the resume with it
**Status**: Fixed on Unix, awaiting review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix cr-ecm26-5`)
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `combined-r27` over `05ff50d7..e025a3e2` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r27.codex.md`. It completes cr-ecm25-5 (`660eb9d0`).

## Evidence
"The new lookup sees the file type, yet a read-only FIFO can still reach `O_RDONLY` and wait indefinitely for a writer. Resolve special entries before attempting reuse and verify the opened descriptor is regular."

## Intake
Admitted as the reviewer put it.

## What
`open_resume_partial_with`: what the look finds at the name that is not a regular file — a FIFO, a socket, a device — is replaced whole and never opened. A directory there refuses the rename, and the resume fails, as it did when its open was refused. What is opened is checked to be a regular file, or it too is replaced.

## Guard proof
- `resume::tests::a_fifo_at_the_name_is_replaced_not_opened` (Unix): a `0444` FIFO at the name, a `0600` source — the resume ends within its bound, with the source's bytes in a regular file.
- Mutation, restored byte for byte: what is not a regular file opened (red: the resume waited on the FIFO).
- The resume, sink and session tests (323) pass. Windows cross-clippy clean.

## Files changed
- `crates/blit-core/src/copy/file_copy/resume.rs` — `open_resume_partial_with`; guard.

## Known gaps
- A regular file put in a FIFO's place between the look and the open could still make a read-only open wait. The open does not ask not to wait, and the check after it comes once it returns.
