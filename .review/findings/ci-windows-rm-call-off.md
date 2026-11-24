# ci-windows-rm-call-off: the removal call-off guard expected Windows to stop part way

**Severity**: MEDIUM — a guard fails on Windows; the code does what its own comment says Windows does.
**Status**: Fixed (the guard); awaiting the next CI run on the platform that failed it
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix ci-windows-rm-call-off`)
**Reviewer**: none — found by GitHub CI run 45508559853 at `b6e6fab9`, 2026-11-24 — the first CI since 2026-10-24, after the owner pushed to GitHub; for the next openreview.

## Evidence
`deletion::rm_tests::a_removal_called_off_stops_where_it_is` (Windows): `unwrap_err()` on `Ok(101)`. Off Unix the tree is the standard library's to remove, whole or not at all, and the question is put once, before it. So the call-off at the twentieth question never came, and the tree went whole. The guard expected an error and an untouched tree there.

## Intake
Admitted. The fault is the guard's assumption of a platform, not the code under it.

## What
Under Unix the guard is unchanged: it stops between entries, and the rest is removed after. Elsewhere it expects what happens there: the tree removed whole, 101 entries.

## Guard proof
- Checks:
  - native, Windows and macOS (`CARGO_FEATURE_PURE=1`) clippy are clean;
  - the workspace passes on Linux: 2,142 tests over 57 binaries, the guard among them.
- Proof is the next CI run on the platform that failed: this host is not that platform, so no mutation here can show the guard red there.

## Files changed
- `crates/blit-core/src/deletion.rs` — the guard.

## Known gaps
- Off Unix a removal can be called off only before its operand is touched (cr-uv2-3's design there).
