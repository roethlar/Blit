# cr-ecm25-4: a local resume refused its cleanup handle had already changed the destination

**Severity**: MEDIUM — with the cleanup's handle refused, a reusable `0644` destination was left `0600`, and where nothing had stood an empty file was left
**Status**: Reviewed, found incomplete — `combined-r27` (Codex openreview, 2026-11-22) reviewed `05ff50d7..e025a3e2` — the six `combined-r26` fixes — with `guard_confirmed: true`: "The patch takes the right approach by carrying the local writer through metadata stamping, staging private partials in a held directory, and discarding incomplete wire hashes." "Locally, a missing-name lookup is followed by an ordinary writable open, which can adopt a file created in between. On a filesystem that refuses hard links, the final existence check and `renameat` can likewise overwrite a concurrent creation. Select exclusive publication immediately and use an atomic no-replace operation, failing that file safely if unavailable. Also check staging-name removal after a successful hard link so a cleanup failure cannot silently leave a second name." "On Linux, an owned `0444` destination became `0600` when the writable reopen failed with `EMFILE`. Keep rollback armed on the original descriptor until acquisition succeeds." The remaining work is cr-ecm26-1 and -4
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `a8283606`
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `combined-r26` over `19025839..05ff50d7` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r26.codex.md`. It completes cr-ecm24-2 (`7778af82`).

## Evidence
"Refusing the cleanup handle changed a reusable file from `0644` to `0600`. With an absent destination, the same refusal left an empty file. Acquire the handle before changing an existing file's mode, and stage a new file until acquisition succeeds."

## Intake
Admitted as the reviewer put it.

## What
In `open_resume_partial_with`:
- Where nothing stands at the chain's end, the partial is made staged in the held directory — private, at its length, its cleanup's handle had — and takes the name only while nothing does (`Resolved::fresh_exclusive`, cr-ecm25-2). It is not opened by the name to be made.
- A file it reuses that it may write has its cleanup's handle had before it is made private.
- A read-only partial made writable to be had has its mode put back if its handle is then refused.
- The test seam that runs once a partial is open at its name runs, for one made now, once it takes the name.

## Guard proof
- `resume::tests::a_resume_refused_its_cleanup_handle_changes_nothing` (Unix): the cleanup's handle refused (the `KEEP_FAULTS` seam) for a reusable `0644` destination — its mode and bytes as they were — and for an absent one, where nothing is made.
- Mutations, each restored byte for byte: the cleanup's handle had after the file is made private (red); the partial where nothing stands made by the name (red).
- The resume, sink and session tests (319) pass, among them `a_partial_made_now_is_never_open_to_others`. Windows cross-clippy clean.

## Files changed
- `crates/blit-core/src/copy/file_copy/resume.rs` — `open_resume_partial_with`, the seam's word; guard.

## Known gaps
- The read-only partial's way back has no guard of its own: the seam refuses the handle only by path, and the guard's reusable file is writable.
