# Release readiness

**Status:** Active release ledger
**As of:** local `master` at `43f156d`, 2026-07-22

This is the concise release boundary after D-2026-07-22-1. Correctness,
supported-platform CI, packaging, installation, and startup are release work.
Performance ceilings and hardware tuning are post-release work.

## Proven

- Local formatting, strict workspace clippy, workspace tests, and docs checks
  passed at `43f156d` before this docs-only audit.
- Linux and macOS tests, formatting, and clippy passed on published GitHub head
  `dcf9245` in CI run `29584631185` on 2026-07-17.
- ldt-1 through ldt-3 established one SOURCE-owned adaptive controller,
  acknowledged ADD/REMOVE membership, role-parity tests, and lifecycle closure.
- Every complete ldt-4 live payload had exact manifest identity and normal
  endpoint restoration. The complete and partial session classification is in
  `docs/bench/ldt4-evidence-audit-2026-07-22/`.

## Release blockers

1. **Windows CI is red.** Published run `29584631185` failed only
   `dial_token_write_stall_times_out_bounded_and_retryable`: Windows buffered
   the 64 MiB loopback handshake, so the test's forced-stall premise was false.
   Release artifact jobs were skipped. The exact local head has not run on
   hosted Windows. Finding: `release-win-ci-handshake-stall-test`.
2. **Windows directory-tree move can hang.** The integration test is ignored on
   Windows after repeated hangs; the product behavior remains unresolved.
3. **Windows attributes and alternate data streams are silently lost on the
   tar path.** This is a correctness blocker if the first release claims
   Windows metadata fidelity; otherwise the limitation must be explicit.
4. **Current release artifacts are unproved.** The exact local head is not on
   GitHub, the latest published release-build jobs were skipped, and install /
   startup smoke checks for the produced CLI and daemon artifacts are not
   recorded.

## Release-scope decisions still needed

- Decide whether live delegated/served-session progress completeness is in the
  first release. Three recorded Medium findings leave delegated progress
  silent and some daemon byte/file totals at zero; transfer correctness is not
  affected.
- Decide whether Windows metadata fidelity is promised in the first release.
  The loss itself is measured and not ambiguous.

## Deferred until after release

- P1 performance-invariance closure, ldt-4 causal tuning, fixed-cell throughput
  asymmetries, MTU follow-ups, and all further hardware matrices.
- Mac↔Mac Thunderbolt ceiling testing.
- Small-file and zero-copy performance work, performance refactors, and other
  ceiling optimization.

No new hardware result is required to make the release decision.
