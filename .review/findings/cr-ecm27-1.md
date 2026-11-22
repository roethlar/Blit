# cr-ecm27-1: publication's rollback could unlink what was not its own, and left a file

**Severity**: HIGH — with a dangling destination link and the staged name's removal failing, acquisition returned an error but left a nine-byte file at the link's end; the rollback looked at the original pathname and unlinked through the held directory, and a file put there between its check and its unlink could be removed
**Status**: Reviewed, found incomplete — `combined-r29` (Codex openreview, 2026-11-22) reviewed `10145127..e3ea9517` — the five `combined-r28` fixes — with `guard_confirmed: true`: "I would keep the whole-file fallback for failed metadata reads, cleanup in the default `skip_record`, and Linux's atomic no-replace rename." "If another directory operation replaces the staged name, the exclusive rename can publish a different inode from the verified one the caller still holds. Error cleanup can likewise unlink the replacement. Use handle-based publication or a private staging directory held by descriptor." "After an injected staging-unlink failure, I retried the copy successfully. The destination and `.blit-partial-*` both contained `finished` and still named the same inode. Recover the staging alias safely or give the retry a fresh inode before writing payload bytes." The remaining work is cr-ecm28-3 and -5
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `0a70800e`
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `combined-r28` over `e025a3e2..10145127` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r28.codex.md`. It completes cr-ecm26-1 (`7615d648`).

## Evidence
"With a dangling destination symlink and an injected staging-name unlink failure, acquisition returned an error but left a nine-byte file at the symlink target. Rollback inspects the original pathname and unlinks through the held directory. A replacement between the identity check and unlink can also be removed. Prefer a single atomic no-replace rename and avoid rollback that can unlink a replacement."

## Intake
Admitted as the reviewer put it.

## What
`Resolved::publish_exclusive`:
- On Linux, the staged partial takes the name by one rename that will not replace (`renameat2(RENAME_NOREPLACE)`). Nothing has two names and nothing needs undoing.
- Where the filesystem has no such rename (`EINVAL`, `ENOSYS`, `EOPNOTSUPP`), or off Linux, it takes a second name (`linkat`, which fails if the name is taken) and the staged name is let go. If that cannot be done, the file fails and says so. Its partial is left at the name, unfinished and private, beside the staged name. The rollback by name is gone, so nothing that may be another's is removed.
- Where there is neither way, the file fails safely.
- Seams refuse the rename (`RENAME_FAULTS`), the second name and the staged name's removal, by path.

## Guard proof
- `resume::tests::a_partial_where_nothing_stood_is_made_exclusively` (Unix): with a file put at the name after the look — through the rename, through the second name, and with neither — that file is left as it is and the resume fails. With the staged name not let go, the resume fails naming the second name, and the partial at the name is unfinished (nine bytes for eight) and private.
- Mutations, each restored byte for byte: the rename replacing (red); the second name replacing (red); the staged name's removal not asked (red).
- The resume, sink and session tests (332) pass. Windows cross-clippy clean.

## Files changed
- `crates/blit-core/src/copy/file_copy/resume.rs` — `Resolved::publish_exclusive`, the rename seam; the guard rewritten.

## Known gaps
- macOS's exclusive rename (`renameatx_np(RENAME_EXCL)`) is cr-ecm27-6's, and blocked: nothing here can run or check macOS code.
- Unix only.
