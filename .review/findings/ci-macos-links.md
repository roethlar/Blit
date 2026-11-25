# ci-macos-links: the link-chain guard ran its chain under a path that holds a link

**Severity**: MEDIUM — a guard fails on macOS; the code follows as far as the host does.
**Status**: Verified — GitHub CI run 45520218555 at `e2c2e08d` (2026-11-25): blit-core's library tests, this guard among them, passed on Linux, macOS and Windows.
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `c7efa611`
**Reviewer**: none — found by GitHub CI run 45508559853 at `b6e6fab9`, 2026-11-24 — the first CI since 2026-10-24, after the owner pushed to GitHub; for the next openreview.

## Evidence
`copy::file_copy::mode_tests::a_chain_of_forty_links_to_nothing_has_its_file_made` (macOS): `Too many levels of symbolic links` (`ELOOP`). The temporary directory lies under `/var`, a link to `/private/var`, and the system counts it: 32 links in the chain and one on the way, 33.

## Intake
Admitted. The fault is the guard's assumption of a platform, not the code under it.

## What
The guard canonicalizes its directory, so the chain of `MAX_FOLLOWED_LINKS` (32 on macOS) is the only links on the way. cr-ecm36-8's limit is not changed: the evidence is that one link more than the chain was walked.

## Guard proof
- Checks:
  - native, Windows and macOS (`CARGO_FEATURE_PURE=1`) clippy are clean;
  - the workspace passes on Linux: 2,142 tests over 57 binaries, the guard among them.
- Proof is the next CI run on the platform that failed: this host is not that platform, so no mutation here can show the guard red there.

## Files changed
- `crates/blit-core/src/copy/file_copy/mod.rs` — the guard.

## Known gaps
- If macOS refuses a chain of 32 even so, the limit itself is wrong for macOS. The next CI run says which.
