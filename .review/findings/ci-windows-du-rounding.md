# ci-windows-du-rounding: a `du` guard expected a parent's 1K blocks to cover its children's, rounded each on its own

**Severity**: MEDIUM — a guard fails on Windows; the code counts as documented.
**Status**: Fixed (the guard); awaiting the next CI run on Windows
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix ci-windows-du-rounding`)
**Reviewer**: none — found by GitHub CI run 45715973925 at `4e12d761`, 2026-11-25 — the third run, which reached test binaries the second never ran; for the next openreview.

## Evidence
`du_df::du_prints_the_unix_commands_lines` (Windows): `assertion failed: blocks(&operand) >= blocks(&under("a")) + blocks(&under("b"))`.

Each line's bytes are rounded up to whole 1K blocks on their own. Off Unix an entry counts the bytes its files hold (`Looked::allocated`: "its length elsewhere"), so `a` and `b` rounded up apart can show one block more between them than their parent rounded once. Unix counts allocated blocks, whole multiples.

## Intake
Admitted. The fault is the guard's assumption of a platform, not the code under it.

## What
Off Unix the guard allows that one block. On Unix it is exact, as before.

## Guard proof
- Checks:
  - native, Windows and macOS (`CARGO_FEATURE_PURE=1`) clippy are clean;
  - all 10 `du_df` guards pass on Linux.
- Proof is the next CI run on Windows: this host is not Windows.

## Files changed
- `crates/blit-cli/tests/du_df.rs` — the guard.

## Known gaps
- Whether a Windows `du` should count allocation, not length, is a question for the plan, not this guard: the plan's reference is GNU's on Unix, and `held.rs` documents lengths elsewhere.
