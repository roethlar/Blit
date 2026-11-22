# cr-ecm27-4: a partial made private kept its new mode on a late failure, locally and on the wire

**Severity**: MEDIUM — an `ESTALE` on the last length lookup left an owned `0444` file `0600`; the wire's hashed hold left a file `0600` when its writable reopen failed after the mode change
**Status**: Reviewed, found incomplete — `combined-r29` (Codex openreview, 2026-11-22) reviewed `10145127..e3ea9517` — the five `combined-r28` fixes — with `guard_confirmed: true`: "I would keep the whole-file fallback for failed metadata reads, cleanup in the default `skip_record`, and Linux's atomic no-replace rename." "With a 4,096-byte file-size limit, the public sink failed to mark an eight-byte partial unfinished at 4,097 bytes. It reported `EFBIG` and preserved all eight bytes, but changed the mode from `0644` to `0600`. Collect the required identity before changing the length and restore the mode on acquisition failure." The remaining work is cr-ecm28-4
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `50c6b3be`
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `combined-r28` over `e025a3e2..10145127` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r28.codex.md`. It completes cr-ecm26-4 (`c9942b72`).

## Evidence
"Injecting `ESTALE` at the final length lookup left an owned `0444` file at `0600`. Carry forward the length from the validated metadata or retain rollback through that last call. The wire's hashed-partial path needs the same protection if its writable reopen fails after `chmod`."

## Intake
Admitted as the reviewer put it. The rollback is kept armed through the last call.

## What
- Local (`open_resume_partial_with`): once a reused partial is made private — where it stood, or made writable first — every failure until it is had puts its mode back through the handle that changed it. That covers the writable reopen, the identity check, the cleanup handle and the last length look. A seam (`LEN_FAULTS`) fails the last look by path.
- Wire (`hold_hashed`): once the partial is made private, a refused writable reopen or a changed file puts its mode back through the read handle. A seam (`HASHED_REOPEN_FAULTS`) refuses the reopen by path.

## Guard proof
- `resume::tests::a_partial_whose_last_look_fails_gets_its_mode_back` (Unix, not as root): a `0644` destination reused where it stood and a `0444` one made writable, each with the last look failing — an error, the mode as it was.
- `sink::tests::a_hashed_hold_refused_its_reopen_gets_the_mode_back` (Unix): a `0644` destination held as hashed, the reopen refused — the mode as it was, and the file fails.
- Mutations, each restored byte for byte: a last look that fails leaving the new mode (red); the wire's refused reopen leaving it (red).
- The resume, sink and session tests (330) pass. Windows cross-clippy clean.

## Files changed
- `crates/blit-core/src/copy/file_copy/resume.rs` — `open_resume_partial_with`, `len_fault`, its seam; guard.
- `crates/blit-core/src/remote/transfer/sink.rs` — `hold_hashed`, `hashed_reopen_fault`, its seam; guard.

## Known gaps
- After the hashed hold's identity check, the partial is marked unfinished (one byte longer). A failure past that point leaves the mark, as a failed resume does.
