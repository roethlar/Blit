# cr-ecm28-5: a partial left with a second name was patched in place on retry

**Severity**: MEDIUM — after a failed publication left its staged name beside the destination, a retry patched the destination in place, and both names held the finished bytes, the same inode
**Status**: Verified on Unix — `combined-r30` (Codex openreview, 2026-11-22) reviewed `e3ea9517..fd603571` — the five `combined-r29` fixes — with `guard_confirmed: true`: "I would keep the nonblocking opens for resume hashing, the Unix hard-link check, private staging, and permission rollback through the unfinished-length update." Its Windows half is cr-ecm29-3's, blocked: "The wire hash decision accepts every non-Unix file, and local resume opens the existing file for writing. On NTFS, resuming one name can therefore change another backup snapshot. Check the link count through the Windows file handle and give a multiply linked destination a fresh file."
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `6d7be846`
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `combined-r29` over `10145127..e3ea9517` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r29.codex.md`. It completes cr-ecm27-1 (`0a70800e`).

## Evidence
"After an injected staging-unlink failure, I retried the copy successfully. The destination and `.blit-partial-*` both contained `finished` and still named the same inode. Recover the staging alias safely or give the retry a fresh inode before writing payload bytes."

## Intake
Admitted. The retry gets a fresh inode: a destination is reused in place only while it has one name. That covers a staged name left behind and an ordinary hard link alike. Patched in place, a hard-linked destination changed every other name with it — a backup tree's other snapshots, say.

## What
- Local (`open_resume_partial_with`): the file opened is reused only while its link count is 1; otherwise a fresh partial takes its name, and the other names keep what they had.
- Wire (`compute_resume_block_hashes`): a file with more than one name is not hashed for reuse, and every block comes into a fresh partial.
- CHANGELOG: the resume sentence says so.

## Guard proof
- `resume::tests::a_destination_with_a_second_name_is_not_patched_in_place` (Unix): a `0600` destination with a hard link, resumed from a `0600` source — another inode at the name with the source's bytes, the link's name as it was.
- `own_incomplete_copy_tests::a_destination_with_a_second_name_is_sent_whole` (Unix): no hashes, `Whole`.
- Mutations, each restored byte for byte: a local destination's names not counted (red); a wire destination's (red).
- The resume, sink and session tests (335) pass. Windows cross-clippy clean.

## Files changed
- `crates/blit-core/src/copy/file_copy/resume.rs` — `open_resume_partial_with`; guard.
- `crates/blit-core/src/transfer_session/mod.rs` — `compute_resume_block_hashes`; guard.
- `CHANGELOG.md` — the resume sentence.

## Known gaps
- The staged name a failed publication leaves is not removed by the retry. It stays a second name for the old partial until something removes it.
