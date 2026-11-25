# ci-listing-case: two listing guards made `A.txt` beside `a.txt`

**Severity**: MEDIUM — the guards fail on a case-insensitive filesystem (macOS, Windows); the code is right.
**Status**: Verified — GitHub CI run 45520218555 at `e2c2e08d` (2026-11-25): blit-core's library tests, this guard among them, passed on Linux, macOS and Windows.
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `ccd1d177`
**Reviewer**: none — found by GitHub CI run 45508559853 at `b6e6fab9`, 2026-11-24 — the first CI since 2026-10-24, after the owner pushed to GitHub; for the next openreview.

## Evidence
`listing::tests::a_directory_is_listed_into_and_its_entries_are_not_looked_up` (macOS, Windows): the names lacked `A.txt`. `listing::tests::an_operand_that_is_no_directory_is_itself` (macOS, Windows): `a.txt` was 1 byte, not 6. On such a filesystem `A.txt` is `a.txt`, and writing it rewrote the file.

## Intake
Admitted. The fault is the guard's assumption of a platform, not the code under it.

## What
The tree makes `A.txt` only where the filesystem keeps it apart from `a.txt`. The listing guard expects `A.txt` only where the directory lists it.

## Guard proof
- Checks:
  - native, Windows and macOS (`CARGO_FEATURE_PURE=1`) clippy are clean;
  - the workspace passes on Linux: 2,142 tests over 57 binaries, the guard among them.
- Proof is the next CI run on the platform that failed: this host is not that platform, so no mutation here can show the guard red there.

## Files changed
- `crates/blit-core/src/listing.rs` — the tree and the guard.

## Known gaps
- On a case-insensitive filesystem the guard no longer covers two names of one spelling in one directory, which such a filesystem cannot hold.
