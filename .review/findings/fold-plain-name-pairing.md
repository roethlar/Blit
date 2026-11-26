# fold-plain-name-pairing: two `mirror` guards expected a plain-name exclude not to keep an entry the filesystem pairs with it

**Severity**: MEDIUM — the guards fail on macOS (predicted) and one on Windows (predicted), where the product keeps the entry by its documented rule.
**Status**: Fixed; awaiting review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix fold-plain-name-pairing`)
**Reviewer**: none — found by this session's run of the suite on a filesystem that folds case, 2026-11-26; for the next openreview.

## Evidence
Run with `TMPDIR` on a casefold tmpfs — mounted in a user and mount namespace of the run's own (`unshare -rm`), then the tests run as this user again in a nested one — the suite failed six tests. The filesystem folds case fully, as APFS does (`ß` to `ss`, measured on APFS 2026-10-19 in `MIRROR_GLOB_DEST_ONLY.md`), and pairs `ſ` with `s`. GitHub CI's macOS and Windows jobs have stopped at an earlier failure in every run since 2026-10-24, so none of these guards has run there.

- `mirror_ignore_case_keeps_a_class_one_byte_wide`: `--match-case: nor does the destination keep it` — `one/ſaved` kept; and then `two/Saved` kept.
- `mirror_ignore_case_pairs_letters_of_any_alphabet_and_nothing_more`: `STRASSE is named by nothing` — beside `STRAẞE`, one folder where `ß` folds to `ss`, and `straße` names it there.

## Intake
Admitted. As typed, `[s]aved` is the plain name `saved` (`plain_name`: a class of one member is how a character is written); with `--ignore-case` a class holding a letter is a glob (cr-ecm2-1), looked up nowhere. A plain-name exclude keeps the destination entry its filesystem pairs with it (cr-g3fix5-1) — the rule cr-ecm43-5's guard now checks too.

## What
- `[s]aved`: each folder asks the filesystem whether `saved` there is its entry, with `--match-case`; where it is, the entry is expected kept (`ſaved` where `ſ` pairs with `s`; `Saved` where case folds).
- `straße`: `STRASSE` in a destination of its own, expected kept only where the filesystem pairs it with `straße`; the rest of the case is as it was.

## Guard proof
- Both guards: green on Linux, and on the folding filesystem, where they failed.
- Mutations, restored byte for byte, each → red on the folding filesystem:
  - `STRASSE` expected gone wherever;
  - `ſaved` expected gone wherever;
  - `Saved` expected kept only when case is ignored;
  - a plain-name exclude not paired through the destination — the product's rule, which both guards now check where the filesystem folds.
- Checks:
  - native, Windows and macOS (`CARGO_FEATURE_PURE=1`) clippy are clean;
  - the workspace passes on Linux, with the round's fixes: 2158 tests over 57 binaries;
  - and with its temporary files on a filesystem that folds case (`fold_run.sh`: a casefold tmpfs in a user
    namespace, the tests run as this user): 2157 passed, the one failure a setgid test the namespace's unmapped
    groups refuse (`a_fresh_partial_takes_its_setgid_directorys_group`, `EINVAL`), green outside it.

## Files changed
- `crates/blit-cli/tests/ignore_case.rs` — the two guards.

## Known gaps
- Windows: that NTFS pairs `ſ` with `S` is an assumption (its upcase table); the guard asks the filesystem, so it holds either way.
