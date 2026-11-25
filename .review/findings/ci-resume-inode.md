# ci-resume-inode: the resume guard compared inode numbers across a removal and a creation

**Severity**: MEDIUM — a guard fails on a filesystem that reuses a freed inode number at once; the code is right.
**Status**: Verified — GitHub CI run 45520218555 at `e2c2e08d` (2026-11-25): blit-core's library tests, this guard among them, passed on Linux, macOS and Windows.
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `159a64c7`
**Reviewer**: none — found by GitHub CI run 45508559853 at `b6e6fab9`, 2026-11-24 — the first CI since 2026-10-24, after the owner pushed to GitHub; for the next openreview.

## Evidence
`copy::file_copy::resume::tests::a_destination_another_owns_takes_a_fresh_partial` (Linux): `assertion left != right failed: a fresh partial`, left and right both 9533449.

## Intake
Admitted. The fault is the guard's assumption of a platform, not the code under it.

## What
The guard keeps the file another owns open across the copy. Its inode stays allocated, so the fresh partial cannot be given its number. The guard then compares the fresh file's number with the held one's, and checks that the held one is the original.

## Guard proof
- Checks:
  - native, Windows and macOS (`CARGO_FEATURE_PURE=1`) clippy are clean;
  - the workspace passes on Linux: 2,142 tests over 57 binaries, the guard among them.
- Proof is the next CI run on the platform that failed: this host is not that platform, so no mutation here can show the guard red there.

## Files changed
- `crates/blit-core/src/copy/file_copy/resume.rs` — the guard.

## Known gaps
- None known.
