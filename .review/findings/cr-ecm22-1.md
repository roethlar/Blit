# cr-ecm22-1: a fresh partial was renamed into place before it was ready

**Severity**: HIGH — a resume whose fresh partial cannot be given its length leaves the destination an empty file
**Status**: Fixed on Unix, awaiting review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix cr-ecm22-1`)
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `combined-r23` over `5d2f8def..24d4064f` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r23.codex.md`. It completes cr-ecm21-3 (`abe9d4a1`).

## Evidence
"With a 256 KiB process file-size limit, Linux `copy --resume` exited 2 and replaced an 18-byte destination with an empty file. The rename succeeds before setting the unfinished length fails. Set the required mode and length on the staged file before renaming it, on both local and wire paths."

## Intake
Admitted as the reviewer put it.

## What
`fresh_partial(dst, unfinished)` gives the fresh partial its mode AND its length — one byte longer than its source — before it is renamed over the destination; if either is refused, it is removed and the destination is as it stood. The local resume (`open_resume_partial`, which now takes the source's length) and the wire's hold pass the length.

## Guard proof
- `local_copy_mode::a_resume_that_cannot_ready_its_partial_leaves_the_destination`: `blit copy --resume` of a 300 KiB private source over an 18-byte destination others can read, under a 256 KiB file-size limit (`setrlimit` in the child, `SIGXFSZ` ignored) — the copy fails and the destination keeps its 18 bytes.
- Mutation, restored byte for byte: the partial renamed before its length is set (red).
- The resume and sink tests (129) and the local-copy-mode tests pass. Windows cross-clippy clean.

## Files changed
- `crates/blit-core/src/copy/file_copy/resume.rs` — `fresh_partial`, `open_resume_partial`.
- `crates/blit-core/src/remote/transfer/sink.rs` — the local route's call, `hold_whole`.
- `crates/blit-cli/tests/local_copy_mode.rs`, `crates/blit-cli/Cargo.toml` (`libc` for the test), `Cargo.lock` — guard.

## Known gaps
- Unix only.
