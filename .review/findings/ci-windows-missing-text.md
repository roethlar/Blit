# ci-windows-missing-text: a removal guard expected Unix's words for a missing name

**Severity**: MEDIUM — a guard fails on Windows; the code gives the system's own words, as designed.
**Status**: Fixed (the guard); awaiting the next CI run on Windows
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix ci-windows-missing-text`)
**Reviewer**: none — found by GitHub CI run 45520218555 at `e2c2e08d`, 2026-11-25 — the run after the first's seven fixes, which reached the test binaries the first never ran; for the next openreview.

## Evidence
`service::admin::typed_operand_tests::an_operand_is_removed_as_it_was_typed` (Windows, the daemon):
- gave `(0, [(0, "The system cannot find the file specified.")])`;
- expected `(0, [(0, "No such file or directory")])`.

The operand ` report` (a space before it) names nothing, and its failure carries the system's words for that (`disk_usage::os_reason`). They are Windows' there.

## Intake
Admitted. The fault is the guard's assumption of a platform, not the code under it.

## What
The guard expects the system's own words for that name being missing: `os_reason` of the error `symlink_metadata` gives for it. What it guards is unchanged: the operand is taken as typed, fails as missing, and `report` stays.

## Guard proof
- Checks:
  - native, Windows and macOS (`CARGO_FEATURE_PURE=1`) clippy are clean;
  - the guard passes on Linux, where the words are what they were.
- Proof is the next CI run on Windows: this host is not Windows.

## Files changed
- `crates/blit-daemon/src/service/admin.rs` — the guard.

## Known gaps
- None known. The module's other guard expecting Unix's words, `a_path_that_is_not_text_is_read_by_its_bytes`, is Unix-only.
