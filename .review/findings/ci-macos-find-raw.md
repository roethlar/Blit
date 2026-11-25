# ci-macos-find-raw: the `find -delete` fixture made a name that is not text on a filesystem that refuses one

**Severity**: MEDIUM — two guards fail on macOS before they begin; the code is right.
**Status**: Fixed (the guard); awaiting the next CI run on macOS
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix ci-macos-find-raw`)
**Reviewer**: none — found by GitHub CI run 45715973925 at `4e12d761`, 2026-11-25 — the third run, which reached test binaries the second never ran; for the next openreview.

## Evidence
`find::find_delete_asks_before_anything_is_removed` and `find::find_delete_changes_no_read_only_module_and_removes_no_module` (macOS): `Illegal byte sequence` at `find.rs:995`, writing `bad\xff.tmp`. APFS refuses a name that is not UTF-8, and the fixture `sized_and_aged` unwrapped the write.

## Intake
Admitted. The fault is the guard's assumption of a platform, not the code under it.

## What
The fixture makes that file only where the filesystem takes it. The guards' only claim on it — that nothing named `t/bad…` is left after `-y` — holds either way.

## Guard proof
- Checks:
  - native, Windows and macOS (`CARGO_FEATURE_PURE=1`) clippy are clean;
  - all 14 `find` guards pass on Linux, where the file is made, as before.
- Proof is the next CI run on macOS: this host's filesystems take such a name.

## Files changed
- `crates/blit-cli/tests/find.rs` — `sized_and_aged`.

## Known gaps
- On macOS these guards do not cover a byte name being removed; macOS has none.
