# cr-ecm24-2: a local resume published its fresh partial before it had every handle it needed

**Severity**: HIGH — with the cleanup handle's duplication refused (`EMFILE`), an eight-byte destination was left nine bytes of nothing before a byte of the source was written
**Status**: Fixed on Unix, awaiting review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix cr-ecm24-2`)
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `combined-r25` over `c89b1eb1..19025839` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r25.codex.md`. It completes cr-ecm23-1 (`399adf58`) for the local route.

## Evidence
"Injecting `EMFILE` at the cleanup-handle duplication left nine zero bytes in place of the original eight-byte file, before any source bytes were written. Acquire the necessary handles while the file is staged, or carry one handle through the operation."

## Intake
Admitted as the reviewer put it.

## What
`open_resume_partial_held` returns the partial, a second handle to it for the caller's cleanup, and its length. The second handle is had while a fresh partial is staged (in its `ready`), and once the in-place partial is had. The local route arms its cleanup with that handle (`PartialTarget::arm_held` takes it, and no longer duplicates). `open_resume_partial` is it without the second handle.

## Guard proof
- `resume::tests::a_resume_refused_its_cleanup_handle_leaves_the_destination` (Unix): a `0644` eight-byte destination, a `0600` source, the second handle refused (the `KEEP_FAULTS` seam) — an error, the destination as it stood, nothing beside it.
- Mutation, restored byte for byte: the second handle taken once the fresh partial is in place (red).
- The resume, sink and session tests (309) pass. Windows cross-clippy clean.

## Files changed
- `crates/blit-core/src/copy/file_copy/resume.rs` — `open_resume_partial_held`, `open_resume_partial_with`, `keep_handle`, its seam; guard.
- `crates/blit-core/src/copy/file_copy/mod.rs`, `crates/blit-core/src/copy/mod.rs` — the export.
- `crates/blit-core/src/remote/transfer/sink.rs` — `PartialTarget::arm_held`, the local route.

## Known gaps
- For a partial resumed in place, the second handle is had after it is made private: a refusal leaves its bytes and its 600 mode, as a failed resume leaves a partial.
