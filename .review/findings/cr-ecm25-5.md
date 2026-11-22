# cr-ecm25-5: a local resume into a destination it could not read failed where the wire copies it whole

**Severity**: MEDIUM — `blit copy --resume` from a `0600` source to an owned `0004` destination in a writable directory exited 2, where the wire path already chose a fresh copy
**Status**: Reviewed, found incomplete — `combined-r27` (Codex openreview, 2026-11-22) reviewed `05ff50d7..e025a3e2` — the six `combined-r26` fixes — with `guard_confirmed: true`: "The patch takes the right approach by carrying the local writer through metadata stamping, staging private partials in a held directory, and discarding incomplete wire hashes." "An injected `ESTALE` on the final wire metadata check escaped the hash computation and propagates out of the grant sender, abandoning unrelated files. An injected local destination-block `EIO` failed that copy despite a readable source and writable directory. Discard unverifiable hashes and retry the affected file whole." "The new lookup sees the file type, yet a read-only FIFO can still reach `O_RDONLY` and wait indefinitely for a writer. Resolve special entries before attempting reuse and verify the opened descriptor is regular." The remaining work is cr-ecm26-3 and -5
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `660eb9d0`
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `combined-r26` over `19025839..05ff50d7` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r26.codex.md`. It completes cr-ecm24-4 (`f6cb0495`) for the local route.

## Evidence
"`blit copy --resume` from a `0600` source to an owned `0004` destination in a writable directory exited 2. The wire path already selects a fresh copy for this case. Make that decision locally before requiring read access to the old file."

## Intake
Admitted as the reviewer put it, with the wire's rule (cr-ecm24-4): a file that would be reused but can be neither read nor written (its own, `0200`) is replaced whole too.

## What
In `open_resume_partial_with`:
- What stands at the chain's end is looked at in the held directory, not followed (`Resolved::look`), before it is opened. A regular file that may not be reused — owned by another user, or granting others what its source does not — is replaced whole without being opened.
- A file this run can neither read nor write is replaced whole.
- The question is asked again of the file opened, which is what is patched.

## Guard proof
- `resume::tests::a_local_resume_into_a_destination_it_cannot_read_copies_it_whole` (Unix, not as root): a `0004`, a `0200` and a `0644` destination, each for a `0600` source — each holds the source's bytes, and the `0644` one is never opened (the opened-partial seam does not run).
- Mutations, each restored byte for byte: what stands there opened before it is asked (red); a file that cannot be opened failing the copy (red).
- The resume, sink and session tests (320) and the local-copy-mode tests pass. Windows cross-clippy clean.

## Files changed
- `crates/blit-core/src/copy/file_copy/resume.rs` — `Resolved::look`, `is_regular`, `open_resume_partial_with`; guard.

## Known gaps
- Unix only.
