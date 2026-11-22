# cr-ecm23-4: a resume reused a destination another user owns

**Severity**: HIGH — a run as root resumed a private source in place into another user's `0600` file, and that user, its owner, could read the bytes as they landed
**Status**: Verified — `combined-r25` (Codex openreview, 2026-11-22) reviewed `c89b1eb1..19025839` — the six `combined-r24` fixes — with `guard_confirmed: true`: "The effective-user ownership check, decoding the Unix mode from the wire header, learning the staged file's identity before rename, and sending zero-block failures to the sink are all sound."
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `84e56bc9`
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `combined-r24` over `24d4064f..c89b1eb1` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r24.codex.md`. It completes cr-ecm22-4 (`8c1f1363`).

## Evidence
"A root-run restore of a private source over another user's `0600` file passes the group-and-other-bit check. That user still owns the inode and can read incoming bytes. Require compatible ownership on both local and wire paths before reusing it."

## Intake
Admitted. "Compatible ownership" is read from what a copy does: blit never gives a file an owner — no copy path calls `chown` — so every file a copy makes is its own user's. A file this run's user owns is reused; one another owns is replaced by a fresh partial, as a file the run makes is. A run as root resuming over files others own now copies them whole: correct for their owners, slower for that run.

## What
- `effective_user` (Unix): the run's effective user id, with a test seam (`RUN_AS`) to make a run root for one path — no test runs as root.
- Local (`open_resume_partial`): a destination another user owns is not reused, whatever this run may do to its mode.
- Wire (`compute_resume_block_hashes`): a file is hashed for reuse only when this run's user owns it; the exception for root is gone.
- CHANGELOG: the resume sentence says so.

## Guard proof
- `resume::tests::a_destination_another_owns_takes_a_fresh_partial` (Unix): the run made root for a `0600` destination this test's user owns — another file at the name, the source's bytes in it.
- `own_incomplete_copy_tests::a_wire_resume_reuses_only_a_file_this_run_owns` (Unix): its own file hashed; the same file to a run made root — no hash, `Whole`.
- Mutations, each restored byte for byte: a local resume not asking who owns the file (red); a wire resume taking root's run for the owner (red).
- The resume, sink and session tests (300) pass. Windows cross-clippy clean.

## Files changed
- `crates/blit-core/src/copy/file_copy/resume.rs` — `effective_user`, `RUN_AS`, `open_resume_partial`; guard.
- `crates/blit-core/src/copy/mod.rs` — their export to the crate.
- `crates/blit-core/src/transfer_session/mod.rs` — `compute_resume_block_hashes`; guard.
- `CHANGELOG.md` — the resume sentence.

## Known gaps
- Both guards return early when run as root, where the file is the run's own and there is nothing to model. Nothing here runs as root.
