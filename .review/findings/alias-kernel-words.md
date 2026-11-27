# alias-kernel-words: a missing name followed by `.` was called no directory

**Severity**: LOW — the refusal is right, its word is not the kernel's. cr-ecm43-1's remaining half.
**Status**: Fixed; awaiting review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix alias-kernel-words`)
**Reviewer**: none — met by the reviewer's own probes in the event stream of `combined-r45` (Codex openreview over `34c77a48..ebc9d81c`, 2026-11-26; `.review/results/combined-r45.codex.events.jsonl`), which ended with no verdict when its service flagged the content; reproduced here by a guard; for the next openreview.

## Evidence
The reviewer's probe: an alias to `absent/.` beneath the root — `missing_slashdot kernel_error=Some(2) blit_error=Some(20)`: the kernel says the name is not there (`ENOENT`), blit said it was no directory (`ENOTDIR`).

## Intake
Admitted: cr-ecm43-1 says what the kernel refuses is refused in its words.

## What
A name that is not there, with anything after it — a `.` too — is not there (`ENOENT`).

## Guard proof
- `file_copy::…::an_alias_is_followed_as_the_kernel_follows_it`: for `leaf/` and `leaf/.`, `leaf` there and not, `open_to_write`'s error is the kernel's own for that path, with `openat2` and without.
- Mutation, restored byte for byte: `ENOTDIR` said of a missing name followed by `.` → red.
- Checks:
  - native, Windows and macOS (`CARGO_FEATURE_PURE=1`) clippy are clean;
  - the workspace passes on Linux, with the round's fixes: 2162 tests over 57 binaries;
  - and with its temporary files on a filesystem that folds case: 2161 passed, the one failure the setgid test
    the namespace's unmapped groups refuse, green outside it.

## Files changed
- `crates/blit-core/src/copy/file_copy/resume.rs` — the alias's resolution.
- `crates/blit-core/src/copy/file_copy/mod.rs` — guard.

## Known gaps
- None known.
