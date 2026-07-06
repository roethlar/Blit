# otp-6b — codex review (raw findings)

**Reviewer**: gpt-5.5 via codex-cli 0.142.5, read-only sandbox, reasoning xhigh.
**Commit reviewed**: `01d9c41` (otp-6b: mirror on the session, the one delete rule).
**Prompt**: review the diff; check especially DATA-LOSS SAFETY (path/casefold
mismatch between wire source paths and dest enumeration, parent-chain kept-dir
derivation, containment, scan-complete guard, ordering), entries_deleted counts,
initiator/verb invariance, FilteredSubset vs All scope vs proto MirrorMode, test
count.

(Full reasoning/exec trace elided — durable content is the findings and verdict
below. Adjudication in `otp-6b.gpt-verdict.md`.)

## Findings

- `crates/blit-core/src/mirror_planner.rs:214` — **High** — The session mirror
  keep-set uses `CasefoldKey` on wire paths, but that key is exact on non-Windows
  and ASCII-only on Windows. On case-insensitive APFS or Unicode case variants on
  NTFS, the diff/write path can treat `Foo.txt` and `foo.txt` as the same file
  while the delete planner treats the dest entry as absent and deletes the
  just-kept file at `SourceDone`.

- `crates/blit-core/src/transfer_session/mod.rs:1730` — **Medium** — The new delete
  pass calls `remove_file` / `remove_dir` directly and does not clear Windows
  read-only attributes. The existing mirror executor does
  `win_fs::clear_readonly_recursive` before deletion, so session mirror regresses
  Windows read-only extraneous deletion and can fail instead of filling
  `entries_deleted`.

(Codex also noted a CLI/TUI `//`-in-display cosmetic inconsistency, out of otp-6b
scope and not data-affecting — not filed.)

## VERDICT

**NEEDS FIXES.** Test-count accounting looks directionally right (+5 mirror tests,
−1 obsolete refusal test = +4), but the suite was not run in the read-only
environment.
