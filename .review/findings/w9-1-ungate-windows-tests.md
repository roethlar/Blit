# w9-1-ungate-windows-tests — remove blanket #[cfg(unix)] from remote transfer tests

**Branch**: `master` (owner-authorized branchless session 2026-06-11)
**Commit**: `9324559`
**Source finding**: tests-cfg-unix-gating-blocks-windows-transfer-coverage (reviewer: high) — `docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md`

## What

Eight remote-transfer test files were entirely `#[cfg(unix)]`-gated despite
containing no unix-only APIs, so the windows-latest CI job (and the manual
ps1 parity runner) compiled out mirror purge, resume, TCP/gRPC fallback,
checksum negotiation, and remote-to-remote delegation coverage — a false
parity signal. Removed all 27 gates.

## Approach

Pure mechanical deletion of `#[cfg(unix)]` lines, each verified to sit
directly on a `#[test]` fn:
remote_parity (5), remote_resume (3), remote_tcp_fallback (1),
remote_checksum_negotiation (2), remote_remote (5), remote_pull_mirror (3),
remote_push_single_file (2), remote_transfer_edges (6).

Re-verified the finding's claim before deleting: zero grep hits for
`PermissionsExt | std::os::unix | symlink | libc | chmod` in all eight
files, and the shared harness already selects `blit.exe` /
`blit-daemon.exe` on Windows.

Kept gated (genuinely unix-dependent per the same grep):
f2_chroot_containment (4), local_move_semantics (2),
remote_push_mirror_safety (1), remote_regression (3). Per-test refinement
of those four (gating only the chmod/symlink assertions) is a possible
follow-up, not in this slice.

## Files changed

8 files under `crates/blit-cli/tests/` (27 deleted lines, nothing else).

## Tests added

None added or removed on unix — suite stays 1341. On Windows this *adds*
~27 tests to what `cargo test --workspace` compiles and runs.

## Known gaps

- Windows execution is unverified from this machine (macOS). The slice
  spec anticipates this: the next windows-latest CI run or
  `scripts/windows/run-blit-tests.ps1` run triages any genuine platform
  failures into their own findings. Until that run happens, these tests
  are compiled-but-unproven on Windows.
- The four kept-gated files still blanket-gate some tests that may have
  only one unix-specific assertion; refining them per-assertion was left
  out of scope.
