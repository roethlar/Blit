# fold-saved-cfg: a `mirror` guard wrote `SAVED.CFG` beside `saved.cfg`

**Severity**: MEDIUM — the guard fails where the filesystem folds case (macOS and Windows, predicted); the code is right.
**Status**: Fixed; awaiting review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix fold-saved-cfg`)
**Reviewer**: none — found by this session's run of the suite on a filesystem that folds case, 2026-11-26; for the next openreview.

## Evidence
Run with `TMPDIR` on a casefold tmpfs — mounted in a user and mount namespace of the run's own (`unshare -rm`), then the tests run as this user again in a nested one — the suite failed six tests. The filesystem folds case fully, as APFS does (`ß` to `ss`, measured on APFS 2026-10-19 in `MIRROR_GLOB_DEST_ONLY.md`), and pairs `ſ` with `s`. GitHub CI's macOS and Windows jobs have stopped at an earlier failure in every run since 2026-10-24, so none of these guards has run there.

`a_pattern_that_cannot_be_compiled_widened_is_refused_before_anything_is_deleted`: `left: ["s"]`, `right: ["s", "saved.cfg"]` — `SAVED.CFG` and `saved.cfg` were one file, stored as `SAVED.CFG`, which `*.cfg` as typed does not name.

## Intake
Admitted.

## What
Each spelling in a folder of its own (`upper/SAVED.CFG`, `lower/saved.cfg`), so on every filesystem each is itself; the patterns name each at any depth, as before.

## Guard proof
- Green on Linux and on the folding filesystem.
- The guard before the fix → red on the folding filesystem (the run above); a mutation of one anchor cannot put the two side by side again and keep the expectation.
- Checks:
  - native, Windows and macOS (`CARGO_FEATURE_PURE=1`) clippy are clean;
  - the workspace passes on Linux, with the round's fixes: 2158 tests over 57 binaries;
  - and with its temporary files on a filesystem that folds case (`fold_run.sh`: a casefold tmpfs in a user
    namespace, the tests run as this user): 2157 passed, the one failure a setgid test the namespace's unmapped
    groups refuse (`a_fresh_partial_takes_its_setgid_directorys_group`, `EINVAL`), green outside it.

## Files changed
- `crates/blit-cli/tests/ignore_case.rs` — the guard.

## Known gaps
- None known.
