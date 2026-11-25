# ci-macos-case-mirror: a case-note guard asked the mirror whether the filesystem folds case

**Severity**: MEDIUM — a guard fails on a filesystem that folds case (macOS); the code is right.
**Status**: Reviewed, found incomplete — `combined-r43` (Codex openreview, 2026-11-25) reviewed `e2c2e08d..4e12d761` — the `combined-r42` fixes and GitHub CI's three — with `guard_confirmed: true`: "I would keep the granted-header identity, the failure-bounded disposition tracking, the explicit log-delivery field, and the independent `openat2` probe. I would require the following corrections." "The new probe detects folding independently, but both post-mirror loops then skip every entry. An incorrect deletion of an unambiguous name such as `xA` can pass. Assert the outcomes for distinct entries and exercise colliding spellings in separate fixtures." The remaining work is cr-ecm42-7
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `3df41965`
**Reviewer**: none — found by GitHub CI run 45520218555 at `e2c2e08d`, 2026-11-25 — the run after the first's seven fixes, which reached the test binaries the first never ran; for the next openreview.

## Evidence
`case_note::the_line_claims_only_what_the_mirror_then_does` (macOS): `["[Aa]"], as typed: b`, true where false was expected.

The guard skips its as-typed checks where the filesystem folds case, and it learned that from `dst/KEEP.1`, which exists only where the mirror copied `keep.1`. The case with `--include '{a,b}'` leaves `keep.1` out, so on APFS, which folds, the guard read "no fold". It then expected `B` kept and `b` gone, which there are one entry.

## Intake
Admitted. The fault is the guard's assumption of a platform, not the code under it.

## What
The guard asks the filesystem itself: it writes `fold-probe` and looks for `FOLD-PROBE`.

## Guard proof
- Checks:
  - native, Windows and macOS (`CARGO_FEATURE_PURE=1`) clippy are clean;
  - all 16 `case_note` guards pass on Linux, where nothing folds, as before.
- Proof is the next CI run on macOS: this host's filesystems do not fold case.

## Files changed
- `crates/blit-cli/tests/case_note.rs` — the guard.

## Known gaps
- On a filesystem that folds, the guard still checks only the line, not what each mirror kept. That is as before for every case.
