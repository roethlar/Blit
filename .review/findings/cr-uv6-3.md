# cr-uv6-3: on Windows a move's account knows a file by its size and the time it was written

**Severity**: HIGH — on Windows, a file replaced at the source by another of the same length and last-write time can be removed by a move as if it were the one that was sent
**Status**: Admitted — NOT FIXED. Blocked: the fix is Windows-only code in the path that deletes a source, nothing in this session can run it, and no Windows machine answers (2026-11-22: `michael@10.1.10.222` and `michael@10.1.10.173`, "No route to host").
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: —
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `combined-r17` over `13faafd2..7912da16` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r17.codex.md`. It is the Windows half of cr-uv5-2, which that finding's record left open.

## Evidence
"The documented Windows gap still allows a replacement with the same length and last-write time to satisfy a move ticket. Capture file identity and change information from a handle, or retain the source when its version cannot be established." (`crates/blit-core/src/moved.rs`, `Version::of`: on Windows `ctime_nanos`, `device` and `inode` are 0.)

## What it needs (not done)
- `moved::Version::of` on Windows: the volume serial number and the file index (as `transfers::filter::entry_identity` already reads them for another purpose, through `GetFileInformationByHandle`), and the change time (`GetFileInformationByHandleEx` with `FileBasicInfo`, `ChangeTime`), from a handle opened on the entry itself (`FILE_FLAG_OPEN_REPARSE_POINT | FILE_FLAG_BACKUP_SEMANTICS`).
- The walk's look (`FsTransferSource`'s manifest task, cr-uv5-2) and the removal's look (`deletion::met_by_path`, the non-Unix removal) must read the same three things the same way.
- Where any of them cannot be read, the account must not hold the file — the source stays (the reviewer's second remedy).
- Guards that can only go red on Windows: a replacement of the same length and time written is kept; a file changed only in its change time is kept.

## What unblocks it
A Windows machine this session can reach (the Windows dev VM at `10.1.10.222` needs the Mac's `en11` up, per `.agents/machines.md`), or the owner's word to land it unrun — which this work has not done for any Windows code in the deleting path.
