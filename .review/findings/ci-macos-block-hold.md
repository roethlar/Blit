# ci-macos-block-hold: a resume guard noted byte names on a destination that stores none

**Severity**: MEDIUM — a guard fails on macOS; the code refuses there as it should.
**Status**: Fixed (the guard); awaiting the next CI run on the platform that failed it
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix ci-macos-block-hold`)
**Reviewer**: none — found by GitHub CI run 45508559853 at `b6e6fab9`, 2026-11-24 — the first CI since 2026-10-24, after the owner pushed to GitHub; for the next openreview.

## Evidence
`remote::transfer::sink::cr_win_1_tests::a_closed_record_leaves_nothing_noted_for_it` (macOS): `validating block-hold path "failed.bin"`, caused by `filename is not valid UTF-8; the destination cannot store it`. The guard noted a byte name for every file, and on macOS `destination_can_store_raw_names` is false, so any byte name is refused. Production notes one only for a name that is not text.

## Intake
Admitted. The fault is the guard's assumption of a platform, not the code under it.

## What
The guard notes a byte name only where the destination stores one.

## Guard proof
- Checks:
  - native, Windows and macOS (`CARGO_FEATURE_PURE=1`) clippy are clean;
  - the workspace passes on Linux: 2,142 tests over 57 binaries, the guard among them.
- Proof is the next CI run on the platform that failed: this host is not that platform, so no mutation here can show the guard red there.

## Files changed
- `crates/blit-core/src/remote/transfer/sink.rs` — the guard.

## Known gaps
- On macOS the guard no longer covers a byte name being forgotten; there is none to forget.
