# Release readiness

**Status:** Active release ledger
**As of:** local `master` at `9c399cc`, 2026-07-22

This is the concise release boundary after D-2026-07-22-3. Every known broken
behavior is release work regardless of its internal classification. Optional
performance ceilings and hardware tuning remain post-release work.

## Proven

- Local formatting, strict workspace clippy, workspace tests, and docs checks
  passed at `43f156d` before this docs-only audit.
- Linux and macOS tests, formatting, and clippy passed on published GitHub head
  `dcf9245` in CI run `29584631185` on 2026-07-17.
- ldt-1 through ldt-3 established one SOURCE-owned adaptive controller,
  acknowledged ADD/REMOVE membership, role-parity tests, and lifecycle closure.
- The historical P1 initiator discrepancy is closed without another transfer:
  the pre-fix code made SOURCE/DESTINATION initiation settle at 3/2 workers,
  `a76b785..42b9b38` mutation-proved parity, post-fix `8e019ef` no longer
  failed the target cell, and ldt-2 retains adaptive role parity. Evidence:
  `docs/bench/p1-evidence-reconciliation-2026-07-22/`.
- Every complete ldt-4 live payload had exact manifest identity and normal
  endpoint restoration. The complete and partial session classification is in
  `docs/bench/ldt4-evidence-audit-2026-07-22/`.

## Release blockers

1. **Hosted Windows confirmation is pending.** Published run `29584631185`
   failed because Windows buffered the guard's 64 MiB loopback handshake.
   rel-1 now exercises the same production timeout through a deterministic
   two-byte/one-byte in-memory blocked writer with local mutation proof. The
   exact fix has not run on hosted Windows because publication is owner-gated.
   Finding: `release-win-ci-handshake-stall-test`.
2. **CLI integration daemon startup is flaky.** A rel-1 workspace run failed
   two `blit_utils` tests with connection refused; both isolated reruns passed.
   The fixture needs bounded positive readiness, not a timing assumption.
   Finding: `release-cli-daemon-test-startup-race`.
3. **Windows directory-tree move can hang.** The integration test is ignored on
   Windows after repeated hangs; the product behavior remains unresolved.
4. **Windows attributes and alternate data streams are silently lost on the
   tar path.** Full fidelity is required for this release.
5. **Progress reporting is incomplete.** Delegated progress can be silent and
   served-session byte/file totals can remain zero through daemon, RPC, CLI,
   and TUI consumers.
6. **P2 is unresolved.** Unified TCP small-file push is 10–20% slower than the
   old path in retained same-session evidence. It requires code attribution and
   a direct software guard, not another hardware matrix.
7. **Current release artifacts are unproved.** The exact local head is not on
   GitHub, the latest published release-build jobs were skipped, and install /
   startup smoke checks for the produced CLI and daemon artifacts are not
   recorded.

## Deferred until after release

- ldt-4 causal tuning, unexplained ceiling asymmetries not tied to a known
  broken behavior, MTU follow-ups, and further hardware matrices.
- Mac↔Mac Thunderbolt ceiling testing.
- Small-file and zero-copy performance work, performance refactors, and other
  ceiling optimization.

No new hardware result is required to make the release decision.
