# fold-ls-twins: the `ls` guards' tree wrote `A.txt` beside `a.txt`

**Severity**: MEDIUM — two `ls` guards fail where the filesystem folds case (macOS, predicted); the code is right.
**Status**: Fixed; awaiting review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix fold-ls-twins`)
**Reviewer**: none — found by this session's run of the suite on a filesystem that folds case, 2026-11-26; for the next openreview.

## Evidence
Run with `TMPDIR` on a casefold tmpfs — mounted in a user and mount namespace of the run's own (`unshare -rm`), then the tests run as this user again in a nested one — the suite failed six tests. The filesystem folds case fully, as APFS does (`ß` to `ss`, measured on APFS 2026-10-19 in `MIRROR_GLOB_DEST_ONLY.md`), and pairs `ſ` with `s`. GitHub CI's macOS and Windows jobs have stopped at an earlier failure in every run since 2026-10-24, so none of these guards has run there.

`ls_prints_names_and_tells_a_file_from_a_directory_holding_one`: `left: ["!bang", "-dash", "0n", "Empty"]`, `right: […, "A.txt"]`; `ls_json_gives_the_entries_and_what_the_path_was`: `no A.txt`. Where the filesystem folds, `A.txt` was written over `a.txt`: one file, stored as `a.txt`.

## Intake
Admitted; both guards are `cfg(unix)`, so this is macOS's.

## What
- The tree writes `A.txt` only where the filesystem keeps it apart from `a.txt`, and says whether it did (from its listing).
- The listing's guard expects `A.txt` fourth, by its bytes, where it is there, and `Empty` where it is not.
- The entry dated long ago is `Z`, which every filesystem holds, so the JSON guard checks its time everywhere.

## Guard proof
- Both guards: green on Linux, and on the folding filesystem, where they failed.
- Mutations, restored byte for byte, each → red on the folding filesystem:
  - `A.txt` written over `a.txt` (the JSON guard: `a.txt` is no longer its 6 bytes);
  - the listing expected to hold `A.txt`.
- Checks:
  - native, Windows and macOS (`CARGO_FEATURE_PURE=1`) clippy are clean;
  - the workspace passes on Linux, with the round's fixes: 2158 tests over 57 binaries;
  - and with its temporary files on a filesystem that folds case (`fold_run.sh`: a casefold tmpfs in a user
    namespace, the tests run as this user): 2157 passed, the one failure a setgid test the namespace's unmapped
    groups refuse (`a_fresh_partial_takes_its_setgid_directorys_group`, `EINVAL`), green outside it.

## Files changed
- `crates/blit-cli/tests/ls.rs` — `tree`, the two guards.

## Known gaps
- On macOS the case twins are not compared; they cannot be made there.
