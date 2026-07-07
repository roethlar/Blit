# otp-6b — GPT review adjudication

**Slice**: otp-6b (mirror on the session), commit `01d9c41`.
**Reviewer**: gpt-5.5 (codex-cli 0.142.5, read-only sandbox, reasoning xhigh).
**Raw review**: `.review/results/otp-6b.codex.md`.
**Codex verdict**: NEEDS FIXES — 1 High, 1 Medium.

## F1 (High) — case-insensitive-FS keep-set mismatch → data loss — ACCEPTED

**Codex** (`mirror_planner.rs:214`): the session mirror keep-set uses `CasefoldKey`,
which is exact on non-Windows and ASCII-only on Windows. On case-insensitive APFS
(or Unicode case variants on NTFS) the write/stat path treats `Foo.txt` and
`foo.txt` as the same file, but the delete planner treats the dest entry as absent
from the source set and deletes the just-kept file at SourceDone.

**Adjudication: ACCEPTED (real).** Verified: `CasefoldKey` (`mirror_planner.rs:33-54`)
folds only under `#[cfg(windows)]`; `#[cfg(not(windows))]` is an exact `PathBuf`
(case-sensitive). The old *local* mirror masks this because it enumerates source and
dest from the *same* FS, so both sides carry the same on-disk case. The session is
the first path to diff a **wire** source set (the source's on-disk case) against the
**dest** FS, so on a case-insensitive dest FS the two cases can diverge and the
updated file is deleted. This is genuine data loss on macOS's default APFS.

**Fix**: case-sensitivity is a property of the platform's default filesystem, which
is exactly what `CasefoldKey` exists to model — it just modelled only Windows.
Extend the folding variant to macOS: `#[cfg(any(windows, target_os = "macos"))]`
folds (posix-normalize + ASCII-lowercase), `#[cfg(not(...))]` stays exact. This
matches each platform's default FS (NTFS/APFS case-insensitive → fold; ext4/xfs
case-sensitive → exact), fixes the reported APFS data-loss, and improves the old
paths on macOS too (a strict improvement — fewer wrong deletions). Residual, now
documented, limitation: case-insensitive *mounts* on Linux and case-sensitive
*volumes* on macOS are rare misconfigurations a compile-time cfg can't detect
(rsync/robocopy share this platform-default model). Unicode (non-ASCII) case folding
is still approximate — the "ASCII-only" half codex noted — tracked as a known gap.

**Guard**: the fold is cfg-gated, so it changes only macOS/Windows behavior; a Linux
guard proof is impossible (Linux stays exact, unchanged). Added a
`#[cfg(any(windows, target_os = "macos"))]` unit test asserting `CasefoldKey` folds
case, plus a Linux-runnable test pinning that Linux deletion stays case-sensitive
(no regression). The Windows fold path already had coverage. Fix commit: `3c99557`.

## F2 (Medium) — Windows read-only extraneous deletes regress — ACCEPTED

**Codex** (`transfer_session/mod.rs:1730`): `mirror_delete_pass` calls
`remove_file`/`remove_dir` directly without clearing Windows read-only attributes;
the existing executors call `win_fs::clear_readonly_recursive` first, so a read-only
extraneous file fails to delete instead of filling `entries_deleted`.

**Adjudication: ACCEPTED (real).** Verified: `admin.rs:197-199` (daemon purge) and
`engine/mirror.rs:113-114` (local mirror) both `clear_readonly_recursive` before the
remove; `mirror_delete_pass` did not. On Windows a read-only extraneous entry would
error the whole pass.

**Fix**: add `#[cfg(windows)] crate::win_fs::clear_readonly_recursive(target)` before
each remove, matching the two existing executors. Windows-only; guard-proven on
windows-latest CI (not runnable on the Linux dev box). Fix commit: `3c99557`.

## Items codex confirmed / non-issues

- Test-count accounting directionally right (+5 mirror, −1 obsolete = +4); codex
  could not run the suite (read-only). Locally: `cargo test --workspace` → 1528 at
  the reviewed commit, 1529 after the fix (+1 Linux casefold test).
- Codex also noted a CLI/TUI `//` display cosmetic inconsistency
  (`display_endpoint`/`collapse_slashes` vs `Location::display`) — out of otp-6b
  scope, not data-affecting; not filed here.

**reviewer: gpt-5.5**
