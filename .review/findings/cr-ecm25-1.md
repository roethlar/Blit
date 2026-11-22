# cr-ecm25-1: a local resume duplicated a handle for its metadata after its fresh partial was in place

**Severity**: HIGH — with the duplicate for the metadata tail refused (`EMFILE`), an eight-byte destination was left nine bytes of nothing before a byte of the source was written
**Status**: Reviewed, found incomplete — `combined-r27` (Codex openreview, 2026-11-22) reviewed `05ff50d7..e025a3e2` — the six `combined-r26` fixes — with `guard_confirmed: true`: "The patch takes the right approach by carrying the local writer through metadata stamping, staging private partials in a held directory, and discarding incomplete wire hashes." "With source `ABCD`, destination `xxABCD`, and its cursor at offset 2, the new public helper reported one skipped block and zero bytes transferred. It returned success with `xxAB` on disk. Seek to zero before comparison or use positional I/O." The remaining work is cr-ecm26-2
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `ce8ae0ea`
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `combined-r26` over `19025839..05ff50d7` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r26.codex.md`. It completes cr-ecm24-2 (`7778af82`).

## Evidence
"Injecting `EMFILE` at the clone used for metadata updates left nine zero bytes in place of an eight-byte destination before any source bytes were written. Carry the acquired handle through copying and metadata updates."

## Intake
Admitted as the reviewer put it.

## What
`resume_copy_through` copies through a partial the caller keeps (`&mut File`); `resume_copy_into` is it for a partial given away. The local route copies through the handle its acquisition returned and then keeps that same handle for its time, its mode and the check that it is still the file at the name. No handle to the partial is taken once the partial is had: the one written through, and the cleanup's, had while it was staged (cr-ecm24-2).

## Guard proof
- `sink::tests::a_local_resume_takes_no_handle_once_its_partial_is_had` (Linux, counting `/proc/self/fd`): a local resume of a `0600` source over a `0644` destination — two handles to the partial while it is copied into, and the source's bytes at the end.
- Mutation, restored byte for byte: a handle for the tail duplicated once the partial is had (red: three).
- The resume, sink and session tests (317) pass. Windows cross-clippy clean.

## Files changed
- `crates/blit-core/src/copy/file_copy/resume.rs` — `resume_copy_through`; `resume_copy_into` through it.
- `crates/blit-core/src/copy/file_copy/mod.rs`, `crates/blit-core/src/copy/mod.rs` — the export.
- `crates/blit-core/src/remote/transfer/sink.rs` — the local route; guard.

## Known gaps
- The guard counts descriptors through `/proc`, so it runs on Linux only.
