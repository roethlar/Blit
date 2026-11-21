# cr-uv2-2: On Windows a removal checks a path and then acts on it

**Severity**: HIGH — a directory above the operand, changed while a removal runs, can turn the clearing of attributes or the removal elsewhere; one entry that cannot be removed leaves its removable neighbours
**Status**: Admitted — NOT FIXED. Blocked: the fix is Windows-only code that nothing in this session can run, and no Windows host answers (2026-11-21: `10.1.10.77`, `.173`, `.177` and `.222` all time out or have no route). See "Why it is not fixed here".
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: —
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `utility-slices-r2` over `4841b2d4..3cd49e40` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/utility-slices-r2.codex.md`. It is the known gap cr-uv1-2 recorded.

## Evidence
"From source inspection, a concurrent change to an ancestor can redirect attribute clearing or removal outside the checked directory. A locked child can also leave removable siblings behind, with one error attributed to the whole operand. Use handles anchored at the module root for traversal, attribute changes, and deletion. Report each failed entry and count successful removals directly." (`crates/blit-core/src/deletion.rs`, the `#[cfg(not(unix))] remove_operand`.)

What that code does, read from the source — nothing here has run on Windows:
1. `path_safety::verify_contained(root, target)`: where the operand lies, asked of its PATH.
2. `win_fs::clear_readonly_recursive(target)`: a walk by path, clearing the read-only attribute.
3. `std::fs::remove_dir_all(target)` (or `remove_file`): by path again. Inside the tree the standard library holds handles and follows no link; the way TO the tree is looked up afresh at each of the three steps.
4. The standard library's removal stops at the first entry it cannot remove; what had not gone yet stays, and one failure is reported for the operand. The count of what went is taken by walking the tree before and after.

## Predicted observable failure
On a Windows daemon, a user who can write inside a module replaces a directory on the way to someone else's `rm` operand with a junction between step 1 and step 2 or 3: attributes are cleared, or a tree removed, where the junction points. And `blit rm -r` of a tree with one file held open by another program: the files after it in the listing stay, with one message.

## Intake
Admitted as the reviewer put it. On Unix both halves are fixed (cr-uv1-2, cr-uv1-5, cr-uv2-4).

## Why it is not fixed here
- **What the fix is.** Windows has no call that opens a name IN an open directory except the native one (`NtCreateFile` with `OBJECT_ATTRIBUTES.RootDirectory`), which the standard library uses inside `remove_dir_all` and does not offer. Holding the directories open is not enough: a directory held without delete-sharing cannot be renamed or removed, but an EMPTY one can still be made a junction in place by anyone who may write its attributes, and sharing modes do not stop that. So every act has to go through a handle opened in the handle before it — the Unix `anchored` module again, on the native interface: a listing by `GetFileInformationByHandleEx`, read-only cleared by `SetFileInformationByHandle(FileBasicInfo)`, removal by `FileDispositionInfoEx` (POSIX semantics, read-only ignored) with `FileDispositionInfo` where a filesystem lacks it, each entry opened with `FILE_FLAG_OPEN_REPARSE_POINT` and a reparse point removed as the link it is. About three hundred lines of `unsafe`, none of it in the tree today (`win_fs.rs` holds no handle-level file call).
- **Why not written blind.** It would replace the removal every Windows `rm`, `move` and `mirror` uses with code that has never run. It can be type-checked here (`--target x86_64-pc-windows-gnu`) and no more: there is no Windows machine, the cross target cannot be linked on this host (no MinGW toolchain), and Wine is not Windows where reparse points and delete dispositions are concerned. A guard that cannot go red where it is written is not evidence (`.agents/repo-guidance.md`), and a mistake in this code is a removal that fails, or removes, on a platform nobody is watching.
- **What unblocks it.** Any Windows host with the Rust toolchain (the dev VM, `netwatch-01`, or CI's Windows runner once the tree reaches GitHub). The work is then: the `anchored` module for Windows as above; `remove_operand` shared between the two systems, so that failures are reported per entry, counted directly, and the removal asks whether it is called off between entries (cr-uv2-3) on Windows too; the Unix guards' twins — a directory swapped for a junction mid-walk and on the way, a child held open, read-only entries — run there.

## Guard proof
None: nothing was changed.

## Files changed
None.

## Known gaps
- Everything above. Until it is fixed, on Windows: `rm`, and the removal that ends a `move`, check the operand's place by its path; a tree stops at the first entry that cannot be removed (AC-12 holds on Unix only); a removal that has begun a tree is not called off before it ends.
