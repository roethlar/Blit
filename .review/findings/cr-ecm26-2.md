# cr-ecm26-2: the copy through a kept handle compared from wherever its cursor stood

**Severity**: HIGH — `resume_copy_through` with source `ABCD` and a destination `xxABCD` whose cursor stood at 2 reported a skipped block, wrote nothing, and returned success with `xxAB` on disk
**Status**: Verified — `combined-r28` (Codex openreview, 2026-11-22) reviewed `e025a3e2..10145127` — the six `combined-r27` fixes — with `guard_confirmed: true`: "The cursor reset, local rewrite after a failed block read, and whole-file fallback after the final wire metadata check all fit that approach. I would keep them"
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `e5b8ec9f`
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `combined-r27` over `05ff50d7..e025a3e2` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r27.codex.md`. It completes cr-ecm25-1 (`ce8ae0ea`).

## Evidence
"With source `ABCD`, destination `xxABCD`, and its cursor at offset 2, the new public helper reported one skipped block and zero bytes transferred. It returned success with `xxAB` on disk. Seek to zero before comparison or use positional I/O."

## Intake
Admitted as the reviewer put it. The comparison tracked the destination's cursor on the assumption that the handle was opened at 0, which a handle the caller keeps need not be.

## What
`resume_copy_through` seeks the destination handle to the start before it compares, as it already did the source.

## Guard proof
- `resume::tests::a_copy_through_a_kept_handle_compares_from_the_start`: `ABCD` into `xxABCD` through a handle at offset 2 — `ABCD` on disk, four bytes written.
- Mutation, restored byte for byte: the comparison from wherever the cursor stands (red).
- The resume, sink and session tests (321) pass. Windows cross-clippy clean.

## Files changed
- `crates/blit-core/src/copy/file_copy/resume.rs` — `resume_copy_through`; guard.

## Known gaps
- None known.
