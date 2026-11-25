# ci-windows-df-words: a `df` guard expected Unix's words for a path that is not there

**Severity**: MEDIUM — a guard fails on Windows; the code gives the system's own words, as `os_reason` documents.
**Status**: Fixed (the guard); awaiting the next CI run on Windows
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix ci-windows-df-words`)
**Reviewer**: none — found by GitHub CI run 45715973925 at `4e12d761`, 2026-11-25 — the third run, which reached test binaries the second never ran; for the next openreview.

## Evidence
`du_df::df_reports_the_filesystem_that_holds_the_path` (Windows):
- gave `blit df: …/t/no-such-entry: The system cannot find the file specified.`;
- expected `…: No such file or directory`.

## Intake
Admitted. The fault is the guard's assumption of a platform, not the code under it.

## What
The guard expects the system's own words for that path being missing: `os_reason` of the error `symlink_metadata` gives for it. That is ci-windows-missing-text's treatment of the daemon's `rm`. The CLI's other guards that expect Unix's words for a missing path are Unix-only.

## Guard proof
- Checks:
  - native, Windows and macOS (`CARGO_FEATURE_PURE=1`) clippy are clean;
  - the guard passes on Linux, where the words are what they were.
- Proof is the next CI run on Windows: this host is not Windows.

## Files changed
- `crates/blit-cli/tests/du_df.rs` — the guard.

## Known gaps
- Whether a Windows daemon should give GNU's words, not the system's, is the owner's open question (`os_reason`); this guard follows the code as it stands.
