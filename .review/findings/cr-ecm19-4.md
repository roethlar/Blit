# cr-ecm19-4: the working mode of every resume's partial

**Severity**: HIGH — the wire resume patched a partial of the usual mode, a fresh local partial could be unwritable or readable, and every ordinary destination was copied whole instead of patched
**Status**: Fixed on Unix, awaiting review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix cr-ecm19-4`)
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `combined-r20` over `17f4eb97..c15ebf5a` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r20.codex.md`. It completes cr-ecm18-2 (`7413b6c6`).

## Evidence
"The wire block writer left a `0644` destination at `0644` after patching it. A fresh local partial under umask `0200` was `0400`. The local symlink arm also bypasses the working-mode check, and a previously supported dangling destination link now fails with `ENOENT`. Select the reusable file before computing block hashes, establish its working permissions through the handle, and preserve contained-link behavior."

## Intake
Admitted as the reviewer put it. Seen in intake, and the larger defect: cr-ecm18-2 replaced every existing partial others could read — which is every ordinary destination, mode 644 — so `--resume` copied them whole where it exists to patch them in place (D-2026-07-09-1 Q2). Its own guard had the older resume tests make their destinations 600 to pass; they are ordinary files again here.

## What
One rule for every resume's partial (Unix): private and writable by its owner (600) while it is worked on, set through the handle it is patched through — whatever mode it had, whatever the mask — and the source's mode when the copy completes (cr-ecm16-1).
- Local (`open_resume_partial`): opened as it always was — a link at the name followed, a link to nothing making what it names — and made with mode 600 when made now; then `make_partial_private` through the handle. A destination there already is patched in place. Only one whose mode this run may not set (not its owner) is not reused: removed, and a fresh private one made.
- Wire: the file the blocks will patch is made private through the handle it is hashed through, before it is hashed; one whose mode may not be set is removed and none of it hashed (every block sent, into a fresh partial). The hold before the first block makes a missing partial with mode 600 and sets 600 through its handle; a block write that makes the file makes it 600.

## Guard proof
- `resume::tests::an_existing_destination_is_patched_in_place_and_private_meanwhile`: a 644 destination holding half the source — interrupted, it is the same file and 600; resumed, the same file, blocks skipped, the source's bytes and mode.
- `resume::tests::a_resume_through_a_link_works_on_what_it_names`: a link to nothing makes what it names (it failed with `ENOENT`); a link to a 644 file, interrupted — what it names is 600, the link stays.
- `resume::tests::a_partial_made_now_is_never_open_to_others`: a seam between the open and the mode set sees a fresh partial with no group or other bits.
- `sink::…::a_wire_partial_is_private_while_blocks_patch_it`: the hold makes a missing partial with no group or other bits (the same seam form) and 600; a 644 one becomes 600; a block write's file has no group or other bits.
- `transfer_session::…::a_resume_partial_is_made_private_before_it_is_hashed`: a 644 destination is 600 once hashed, its hashes all there.
- Mutations, each restored byte for byte, eight: a local partial made with the usual mask; no mode set through the handle; a link at the name not followed; an existing destination replaced (red, and twelve older resume tests); the wire hashing a partial as it stands; the hold made with the usual mask; the hold leaving a partial as it stands; a block write's file with the usual mask — each red.
- cr-ecm19-1's, cr-ecm18-2's and cr-ecm17-3's guards pass. The whole suite in the scratch copy: 1,966 passed, 0 failed. Windows cross-clippy clean.

## Files changed
- `crates/blit-core/src/copy/file_copy/resume.rs` — `open_resume_partial`, `make_partial_private`, a seam; guards; the older tests' destinations ordinary again.
- `crates/blit-core/src/copy/file_copy/mod.rs`, `crates/blit-core/src/copy/mod.rs` — export.
- `crates/blit-core/src/transfer_session/mod.rs` — `compute_resume_block_hashes`; guard.
- `crates/blit-core/src/remote/transfer/sink.rs` — `hold_partial_unfinished`, `patch_file_block`; guard.
- `CHANGELOG.md` — cr-ecm18-2's sentence corrected: a destination there is patched in place, private meanwhile.

## Known gaps
- A partial this run may write but not set the mode of, and may not remove either, is patched as it stands (as a copy into it would write it). Not reached by any guard: it needs a file owned by another user.
- A reader who opened an existing destination before the resume keeps reading it, the new bytes included — as with any copy into an existing file.
- A destination of mode 444 that this run owns is not opened for writing, as before and as an ordinary copy.
- The wire resume still opens its partial by name at each step (hash, hold, block, finish); which file each opens is not tied together. Not raised.
- Unix only.
