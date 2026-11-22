# cr-ecm20-2: a wire resume's hash negotiation changed the destination

**Severity**: HIGH — before the source sends a block, hashing could change the destination's mode or remove it; a source that then skips the file, or goes away, leaves the user's file changed or gone
**Status**: Fixed on Unix, awaiting review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix cr-ecm20-2`)
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `combined-r21` over `c15ebf5a..b92c9a17` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r21.codex.md`. It completes cr-ecm19-4 (`af308007`).

## Evidence
"Before the source supplies a block, hashing can change the destination's mode or unlink it after a denied `chmod`. A source skip or disconnect can therefore leave the original changed or gone. If both operations fail, the receiver warns and can still patch with the old permissions. Keep negotiation read-only and require the writer to establish safe access before patching. A previously opened reader also retains access to a reused inode, which matters when the source is more private."

## Intake
Admitted as the reviewer put it.

## What
- Negotiation (`compute_resume_block_hashes`) changes nothing. It decides, without a write, whether the file is reused: one this run does not own, or one that grants others what the source does not (a reader who has it open would read what the blocks write), is not hashed — every block will come. It hands the sink what the hashes were made of (`ResumeBaseline`: `Hashed(version)` or `Whole`), through a new `TransferSink::note_resume_baseline` (passed through by the data plane's sink).
- The writer — the hold before the first block — makes the partial safe, by that word: hashed, it is had to be read, made private and writable (600) through that handle, and opened again to be written (the same file); nothing hashed, a file there is replaced by a fresh private partial (a link at the name is written through, as ever, to a file made private). If neither can be done, the failure is held for the file: its blocks are written no more, and it fails at its completion record — nothing is patched with its old permissions.
- With no word of the hashes, as before.

## Guard proof
- `transfer_session::…::hashing_a_resume_partial_changes_nothing`: a 644 file is hashed — its mode, number and time of last change the same afterwards; for a source of mode 600, it is not hashed (`Whole`), and nothing about it changes.
- `sink::…::a_wire_partial_is_made_safe_by_its_writer_or_not_written`: a hashed 644 partial is made 600 in place (same number, marked one byte longer); one nothing was hashed of is replaced by a fresh 600 partial, a reader who had it open keeping only what it held; one in a folder closed to removals, nothing hashed — its block is not written and its completion fails, the file as it was. Skipped, and says so, for a process a directory's mode does not bar.
- Mutations, each restored byte for byte, five: hashing making the file private; a file wider than its source hashed; a file nothing was hashed of patched in place; a hashed partial patched as it stands; a partial not had safely written all the same — each red.
- The whole suite in the scratch copy: 1,972 passed, 0 failed. Windows cross-clippy clean.

## Files changed
- `crates/blit-core/src/transfer_session/mod.rs` — `compute_resume_block_hashes`, `diff_chunk_and_send_needs`; guard (cr-ecm19-4's hash-time guard replaced).
- `crates/blit-core/src/remote/transfer/sink.rs` — `ResumeBaseline`, `note_resume_baseline`, `hold_partial`, the block payload; guard.
- `crates/blit-core/src/transfer_session/data_plane.rs` — pass-through.
- `CHANGELOG.md`.

## Known gaps
- A file this run does not own and may not remove is not resumed — it fails — where an ordinary copy would write into it. Not reached by any guard: it needs a file owned by another user.
- Whether the file a block patches is the file hashed is cr-ecm20-1's.
- Unix only.
