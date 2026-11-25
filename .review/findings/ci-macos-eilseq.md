# ci-macos-eilseq: a move guard made a name that is not text on a filesystem that refuses one

**Severity**: MEDIUM — a guard fails on macOS; the code is right.
**Status**: Verified — GitHub CI run 45520218555 at `e2c2e08d` (2026-11-25): blit-core's library tests, this guard among them, passed on Linux, macOS and Windows.
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `95234b92`
**Reviewer**: none — found by GitHub CI run 45508559853 at `b6e6fab9`, 2026-11-24 — the first CI since 2026-10-24, after the owner pushed to GitHub; for the next openreview.

## Evidence
`transfer_session::own_incomplete_copy_tests::a_moves_account_holds_each_file_as_the_walk_saw_it` (macOS): `Illegal byte sequence` making `bad\xff.bin`. APFS refuses a name that is not UTF-8.

## Intake
Admitted. The fault is the guard's assumption of a platform, not the code under it.

## What
The guard makes that file, and asks the account about it, only where the platform takes such a name (`destination_can_store_raw_names`); its counts follow.

## Guard proof
- Checks:
  - native, Windows and macOS (`CARGO_FEATURE_PURE=1`) clippy are clean;
  - the workspace passes on Linux: 2,142 tests over 57 binaries, the guard among them.
- Proof is the next CI run on the platform that failed: this host is not that platform, so no mutation here can show the guard red there.

## Files changed
- `crates/blit-core/src/transfer_session/mod.rs` — the guard.

## Known gaps
- On macOS the guard no longer covers a byte name in a move's account; macOS has none.
